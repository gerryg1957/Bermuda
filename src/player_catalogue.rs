use std::{collections::HashSet, time::Duration};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Deserialize;

use crate::player_directory::{PlayerAliasResolution, resolve_player_alias_for_source};

const SUPPLIED_CATALOGUE_JSON: &str = include_str!("../data/player_catalogue.json");
const PLAYER_CATALOGUE_BUSY_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayerCatalogue {
    pub version: u64,
    pub players: Vec<CataloguePlayer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CataloguePlayer {
    pub key: String,
    pub preferred_name: String,

    #[serde(default)]
    pub aliases: Vec<CatalogueAlias>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CatalogueNameResolution<'a> {
    Unrecognised,
    Unique(&'a CataloguePlayer),
    Ambiguous(Vec<&'a CataloguePlayer>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CataloguePlayerMaterialisation {
    pub player_id: i64,
    pub created: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedPlayerIdentity {
    pub(crate) player_id: Option<i64>,
    pub(crate) catalogue_derived: bool,
}

impl ResolvedPlayerIdentity {
    const UNRESOLVED: Self = Self {
        player_id: None,
        catalogue_derived: false,
    };

    fn local(player_id: i64) -> Self {
        Self {
            player_id: Some(player_id),
            catalogue_derived: false,
        }
    }

    fn catalogue(player_id: i64) -> Self {
        Self {
            player_id: Some(player_id),
            catalogue_derived: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CatalogueAlias {
    pub name: String,

    #[serde(default)]
    pub notes: Option<String>,
}

impl PlayerCatalogue {
    pub fn supplied() -> Result<Self> {
        Self::from_json(SUPPLIED_CATALOGUE_JSON)
            .context("parsing Bermuda supplied player catalogue")
    }

    /// Prepare the bundled Bermuda player catalogue for an opened database.
    ///
    /// This is the normal production entry point for identity-aware services.
    /// It parses the bundled catalogue once, synchronises and reconciles the
    /// database when its catalogue data version is older, then returns the
    /// parsed catalogue for callers such as the importer that need it for
    /// subsequent source-name resolution.
    pub fn prepare_supplied(connection: &mut Connection) -> Result<Self> {
        let catalogue = Self::supplied()?;

        catalogue
            .synchronise(connection)
            .context("preparing Bermuda supplied player catalogue")?;

        Ok(catalogue)
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let catalogue: Self =
            serde_json::from_str(json).context("parsing player catalogue JSON")?;

        catalogue.validate()?;

        Ok(catalogue)
    }

    pub fn resolve_name(&self, name: &str) -> CatalogueNameResolution<'_> {
        /*
         * Catalogue resolution is deliberately exact and conservative.
         *
         * This is identity resolution for imported source text, not the
         * user-facing search facility. It therefore does not trim, fold case,
         * perform fuzzy matching, or otherwise guess that two spellings are
         * equivalent.
         *
         * A player is included at most once even if the same spelling appears
         * both as that player's preferred name and as one of their aliases.
         */
        let matches = self
            .players
            .iter()
            .filter(|player| {
                player.preferred_name == name
                    || player.aliases.iter().any(|alias| alias.name == name)
            })
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [] => CatalogueNameResolution::Unrecognised,
            [player] => CatalogueNameResolution::Unique(player),
            _ => CatalogueNameResolution::Ambiguous(matches),
        }
    }

    pub fn materialise_player(
        &self,
        connection: &mut Connection,
        catalogue_key: &str,
    ) -> Result<CataloguePlayerMaterialisation> {
        /*
         * Public catalogue fields mean callers could construct or mutate a
         * PlayerCatalogue without going through from_json(), so validate
         * before using catalogue data to create a local identity.
         */
        self.validate()?;

        let player = self
            .players
            .iter()
            .find(|player| player.key == catalogue_key)
            .with_context(|| {
                format!("player catalogue contains no identity with key {catalogue_key:?}")
            })?;

        /*
         * IMMEDIATE makes the find-or-create operation atomic with respect to
         * other SQLite writers. The private helper deliberately accepts an
         * existing transaction so later catalogue-derived metadata linking can
         * materialise and assign an identity in one atomic operation.
         */
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("starting catalogue player materialisation")?;

        let result = materialise_player_in_transaction(&transaction, player)?;

        transaction
            .commit()
            .context("committing catalogue player materialisation")?;

        Ok(result)
    }

    /// Synchronise the supplied catalogue and reconcile catalogue-owned
    /// player interpretations when the bundled catalogue version changes.
    ///
    /// The supplied tables and every metadata repair are committed atomically.
    /// Local assignments and explicitly catalogue-suppressed occurrences are
    /// never rewritten by this operation.
    ///
    /// Returns true when a catalogue version was applied and false when the
    /// database was already at this exact supplied-data version.
    pub fn synchronise(&self, connection: &mut Connection) -> Result<bool> {
        self.validate()?;

        let data_version = i64::try_from(self.version)
            .context("player catalogue version exceeds SQLite integer range")?;

        let observed_version = read_catalogue_data_version(connection)?;

        match observed_version {
            Some(version) if version > data_version => {
                bail!(
                    "project player catalogue version {version} is newer than \
                     this Bermuda catalogue version {data_version}"
                );
            }

            Some(version) if version == data_version => return Ok(false),

            _ => {}
        }

        /*
         * As with schema migration, the first read is only a fast path.
         * Acquire the SQLite write lock and then read the version again so two
         * Bermuda connections cannot both act on the same stale observation.
         */
        connection
            .busy_timeout(PLAYER_CATALOGUE_BUSY_TIMEOUT)
            .context("setting player catalogue synchronisation busy timeout")?;

        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("locking database for supplied player catalogue synchronisation")?;

        let locked_version = read_catalogue_data_version(&transaction)?;

        match locked_version {
            Some(version) if version > data_version => {
                bail!(
                    "project player catalogue version {version} is newer than \
                     this Bermuda catalogue version {data_version}"
                );
            }

            Some(version) if version == data_version => {
                transaction
                    .commit()
                    .context("committing no-op player catalogue synchronisation")?;

                return Ok(false);
            }

            _ => {}
        }

        synchronise_supplied_tables_in_transaction(&transaction, self, data_version)?;

        reconcile_game_metadata_in_transaction(&transaction, self)?;

        transaction
            .commit()
            .context("committing supplied player catalogue reconciliation")?;

        Ok(true)
    }

    fn validate(&self) -> Result<()> {
        if self.version == 0 {
            bail!("player catalogue version must be greater than zero");
        }

        let mut player_keys = HashSet::new();

        for player in &self.players {
            if player.key.trim().is_empty() {
                bail!("player catalogue key must not be empty");
            }

            if player.preferred_name.trim().is_empty() {
                bail!(
                    "player catalogue preferred name must not be empty for key {:?}",
                    player.key
                );
            }

            if !player_keys.insert(player.key.as_str()) {
                bail!("duplicate player catalogue key {:?}", player.key);
            }

            let mut alias_names = HashSet::new();

            for alias in &player.aliases {
                if alias.name.trim().is_empty() {
                    bail!(
                        "player catalogue alias must not be empty for key {:?}",
                        player.key
                    );
                }

                if !alias_names.insert(alias.name.as_str()) {
                    bail!(
                        "duplicate player catalogue alias {:?} for key {:?}",
                        alias.name,
                        player.key
                    );
                }
            }
        }

        Ok(())
    }
}

fn read_catalogue_data_version(connection: &Connection) -> Result<Option<i64>> {
    connection
        .query_row(
            "SELECT data_version FROM player_catalogue_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("reading supplied player catalogue version")
}

fn synchronise_supplied_tables_in_transaction(
    transaction: &Transaction<'_>,
    catalogue: &PlayerCatalogue,
    data_version: i64,
) -> Result<()> {
    /*
     * This helper owns only the supplied player_catalogue_* tables. Its caller
     * owns the transaction so table replacement and metadata reconciliation
     * either both commit or both roll back.
     */
    transaction
        .execute("DELETE FROM player_catalogue_aliases", [])
        .context("clearing supplied player catalogue aliases")?;

    transaction
        .execute("DELETE FROM player_catalogue_players", [])
        .context("clearing supplied player catalogue players")?;

    transaction
        .execute("DELETE FROM player_catalogue_state", [])
        .context("clearing supplied player catalogue state")?;

    {
        let mut insert_player = transaction
            .prepare(
                r#"
                INSERT INTO player_catalogue_players(
                    catalogue_key,
                    preferred_name
                )
                VALUES (?1, ?2)
                "#,
            )
            .context("preparing supplied catalogue player insertion")?;

        for player in &catalogue.players {
            insert_player
                .execute(params![player.key.as_str(), player.preferred_name.as_str(),])
                .with_context(|| format!("storing supplied catalogue player {:?}", player.key))?;
        }
    }

    {
        let mut insert_alias = transaction
            .prepare(
                r#"
                INSERT INTO player_catalogue_aliases(
                    catalogue_key,
                    name,
                    notes
                )
                VALUES (?1, ?2, ?3)
                "#,
            )
            .context("preparing supplied catalogue alias insertion")?;

        for player in &catalogue.players {
            for alias in &player.aliases {
                insert_alias
                    .execute(params![
                        player.key.as_str(),
                        alias.name.as_str(),
                        alias.notes.as_deref(),
                    ])
                    .with_context(|| {
                        format!(
                            "storing supplied catalogue alias {:?} for {:?}",
                            alias.name, player.key
                        )
                    })?;
            }
        }
    }

    transaction
        .execute(
            r#"
            INSERT INTO player_catalogue_state(id, data_version)
            VALUES (1, ?1)
            "#,
            [data_version],
        )
        .context("recording supplied player catalogue version")?;

    Ok(())
}

/// Resolve one imported/source player spelling using Bermuda's complete
/// precedence rules.
///
/// Exact local source-specific aliases win over exact local global aliases.
/// Local ambiguity is a stop condition. Only a name unrecognised by the local
/// layer may fall through to one unique exact supplied-catalogue identity.
pub(crate) fn resolve_player_identity_in_transaction(
    transaction: &Transaction<'_>,
    source_id: i64,
    player_catalogue: &PlayerCatalogue,
    name: Option<&str>,
) -> Result<ResolvedPlayerIdentity> {
    let Some(name) = name else {
        return Ok(ResolvedPlayerIdentity::UNRESOLVED);
    };

    match resolve_player_alias_for_source(transaction, source_id, name)? {
        PlayerAliasResolution::Unique(player_id) => {
            return Ok(ResolvedPlayerIdentity::local(player_id));
        }

        PlayerAliasResolution::Ambiguous => {
            return Ok(ResolvedPlayerIdentity::UNRESOLVED);
        }

        PlayerAliasResolution::Unrecognised => {}
    }

    let catalogue_player = match player_catalogue.resolve_name(name) {
        CatalogueNameResolution::Unique(player) => player,

        CatalogueNameResolution::Unrecognised | CatalogueNameResolution::Ambiguous(_) => {
            return Ok(ResolvedPlayerIdentity::UNRESOLVED);
        }
    };

    let materialisation = materialise_player_in_transaction(transaction, catalogue_player)?;

    Ok(ResolvedPlayerIdentity::catalogue(materialisation.player_id))
}

fn reconcile_game_metadata_in_transaction(
    transaction: &Transaction<'_>,
    player_catalogue: &PlayerCatalogue,
) -> Result<()> {
    /*
     * Resolve each source/name pair once rather than once per game occurrence.
     *
     * Eligible rows are exactly:
     *   - ordinary unresolved occurrences; and
     *   - catalogue-derived occurrences from the previous catalogue version.
     *
     * Local assignments are excluded by the player-id/provenance predicate.
     * Explicitly suppressed occurrences are excluded independently.
     */
    let source_names = {
        let mut statement = transaction
            .prepare(
                r#"
                SELECT
                    gs.source_id,
                    gm.black_player AS raw_name
                FROM game_metadata AS gm
                JOIN game_sources AS gs
                    ON gs.id = gm.game_source_id
                WHERE gm.black_player IS NOT NULL
                  AND TRIM(gm.black_player) <> ''
                  AND gm.black_player_catalogue_suppressed = 0
                  AND (
                        gm.black_player_id IS NULL
                        OR gm.black_player_catalogue_derived = 1
                      )

                UNION

                SELECT
                    gs.source_id,
                    gm.white_player AS raw_name
                FROM game_metadata AS gm
                JOIN game_sources AS gs
                    ON gs.id = gm.game_source_id
                WHERE gm.white_player IS NOT NULL
                  AND TRIM(gm.white_player) <> ''
                  AND gm.white_player_catalogue_suppressed = 0
                  AND (
                        gm.white_player_id IS NULL
                        OR gm.white_player_catalogue_derived = 1
                      )

                ORDER BY source_id, raw_name
                "#,
            )
            .context("preparing player catalogue reconciliation candidates")?;

        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .context("reading player catalogue reconciliation candidates")?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("collecting player catalogue reconciliation candidates")?
    };

    for (source_id, raw_name) in source_names {
        let identity = resolve_player_identity_in_transaction(
            transaction,
            source_id,
            player_catalogue,
            Some(&raw_name),
        )?;

        let catalogue_derived = if identity.catalogue_derived {
            1_i64
        } else {
            0_i64
        };

        transaction
            .execute(
                r#"
                UPDATE game_metadata
                SET black_player_id = ?1,
                    black_player_catalogue_derived = ?2,
                    black_player_catalogue_suppressed = 0
                WHERE black_player = ?3
                  AND black_player_catalogue_suppressed = 0
                  AND (
                        black_player_id IS NULL
                        OR black_player_catalogue_derived = 1
                      )
                  AND EXISTS (
                        SELECT 1
                        FROM game_sources AS gs
                        WHERE gs.id = game_metadata.game_source_id
                          AND gs.source_id = ?4
                      )
                "#,
                params![
                    identity.player_id,
                    catalogue_derived,
                    raw_name.as_str(),
                    source_id,
                ],
            )
            .with_context(|| {
                format!(
                    "reconciling Black player name {:?} for source {}",
                    raw_name, source_id
                )
            })?;

        transaction
            .execute(
                r#"
                UPDATE game_metadata
                SET white_player_id = ?1,
                    white_player_catalogue_derived = ?2,
                    white_player_catalogue_suppressed = 0
                WHERE white_player = ?3
                  AND white_player_catalogue_suppressed = 0
                  AND (
                        white_player_id IS NULL
                        OR white_player_catalogue_derived = 1
                      )
                  AND EXISTS (
                        SELECT 1
                        FROM game_sources AS gs
                        WHERE gs.id = game_metadata.game_source_id
                          AND gs.source_id = ?4
                      )
                "#,
                params![
                    identity.player_id,
                    catalogue_derived,
                    raw_name.as_str(),
                    source_id,
                ],
            )
            .with_context(|| {
                format!(
                    "reconciling White player name {:?} for source {}",
                    raw_name, source_id
                )
            })?;
    }

    Ok(())
}

pub(crate) fn materialise_player_in_transaction(
    transaction: &Transaction<'_>,
    player: &CataloguePlayer,
) -> Result<CataloguePlayerMaterialisation> {
    /*
     * Materialisation is key-based, never name-based.
     *
     * A local player with the same preferred_name but no catalogue_key is
     * therefore not silently merged with the supplied identity. The unique
     * players.catalogue_key index remains the database-level invariant.
     */
    let created = transaction
        .execute(
            r#"
            INSERT INTO players(preferred_name, catalogue_key)
            SELECT ?1, ?2
            WHERE NOT EXISTS (
                SELECT 1
                FROM players
                WHERE catalogue_key = ?2
            )
            "#,
            params![player.preferred_name.as_str(), player.key.as_str(),],
        )
        .with_context(|| format!("materialising supplied catalogue player {:?}", player.key))?
        == 1;

    let player_id: i64 = transaction
        .query_row(
            r#"
            SELECT id
            FROM players
            WHERE catalogue_key = ?1
            "#,
            [player.key.as_str()],
            |row| row.get(0),
        )
        .with_context(|| format!("reading materialised catalogue player {:?}", player.key))?;

    Ok(CataloguePlayerMaterialisation { player_id, created })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::database;

    #[test]
    fn resolves_catalogue_names_without_guessing() -> Result<()> {
        let catalogue = PlayerCatalogue::from_json(
            r#"
            {
              "version": 1,
              "players": [
                {
                  "key": "test:player-a",
                  "preferred_name": "Player A",
                  "aliases": [
                    {
                      "name": "Alias A"
                    },
                    {
                      "name": "Player A"
                    }
                  ]
                },
                {
                  "key": "test:player-b",
                  "preferred_name": "Player B",
                  "aliases": [
                    {
                      "name": "Shared Alias"
                    }
                  ]
                },
                {
                  "key": "test:player-c",
                  "preferred_name": "Player C",
                  "aliases": [
                    {
                      "name": "Shared Alias"
                    }
                  ]
                }
              ]
            }
            "#,
        )?;

        match catalogue.resolve_name("Player A") {
            CatalogueNameResolution::Unique(player) => {
                assert_eq!(player.key, "test:player-a");
            }
            other => panic!("expected unique preferred-name match, got {other:?}"),
        }

        match catalogue.resolve_name("Alias A") {
            CatalogueNameResolution::Unique(player) => {
                assert_eq!(player.key, "test:player-a");
            }
            other => panic!("expected unique alias match, got {other:?}"),
        }

        /*
         * Matching both preferred name and alias on the same identity must
         * still yield one player, not a false ambiguity.
         */
        match catalogue.resolve_name("Player A") {
            CatalogueNameResolution::Unique(player) => {
                assert_eq!(player.key, "test:player-a");
            }
            other => panic!("expected one deduplicated identity, got {other:?}"),
        }

        assert_eq!(
            catalogue.resolve_name("player a"),
            CatalogueNameResolution::Unrecognised
        );

        assert_eq!(
            catalogue.resolve_name(" Player A "),
            CatalogueNameResolution::Unrecognised
        );

        match catalogue.resolve_name("Shared Alias") {
            CatalogueNameResolution::Ambiguous(players) => {
                let keys = players
                    .iter()
                    .map(|player| player.key.as_str())
                    .collect::<Vec<_>>();

                assert_eq!(keys, vec!["test:player-b", "test:player-c"]);
            }
            other => panic!("expected ambiguous catalogue match, got {other:?}"),
        }

        assert_eq!(
            catalogue.resolve_name("Unknown Player"),
            CatalogueNameResolution::Unrecognised
        );

        Ok(())
    }

    #[test]
    fn materialises_catalogue_identity_without_guessing_or_overwriting() -> Result<()> {
        let temporary_directory = tempdir()?;
        let database_root = temporary_directory.path().join("database");

        database::initialise(&database_root)?;
        let mut connection = database::open(&database_root)?;

        let catalogue = PlayerCatalogue::from_json(
            r#"
            {
              "version": 1,
              "players": [
                {
                  "key": "test:player-a",
                  "preferred_name": "Player A",
                  "aliases": [
                    {
                      "name": "Alias A"
                    }
                  ]
                }
              ]
            }
            "#,
        )?;

        /*
         * Same display name, but local-only. Bermuda must not infer that this
         * is the supplied identity.
         */
        connection.execute(
            r#"
            INSERT INTO players(preferred_name)
            VALUES ('Player A')
            "#,
            [],
        )?;

        let local_only_id = connection.last_insert_rowid();

        connection.execute(
            r#"
            INSERT INTO player_aliases(
                player_id,
                name,
                source_id,
                notes
            )
            VALUES (?1, 'Local Alias A', NULL, 'local-only identity')
            "#,
            [local_only_id],
        )?;

        let first = catalogue.materialise_player(&mut connection, "test:player-a")?;

        assert!(first.created);
        assert_ne!(first.player_id, local_only_id);

        let player_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))?;

        assert_eq!(player_count, 2);

        let (materialised_name, materialised_key): (String, Option<String>) = connection
            .query_row(
                r#"
            SELECT preferred_name, catalogue_key
            FROM players
            WHERE id = ?1
            "#,
                [first.player_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

        assert_eq!(materialised_name, "Player A");
        assert_eq!(materialised_key.as_deref(), Some("test:player-a"));

        /*
         * Supplied aliases stay in the supplied catalogue layer. Merely
         * materialising the player must not copy them into player_aliases.
         */
        let alias_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_aliases", [], |row| row.get(0))?;

        assert_eq!(alias_count, 1);

        /*
         * Simulate a user choosing a different local display name.
         */
        connection.execute(
            r#"
            UPDATE players
            SET preferred_name = 'User display for A'
            WHERE id = ?1
            "#,
            [first.player_id],
        )?;

        /*
         * Even if a later supplied catalogue changes its preferred spelling,
         * the already-materialised local row is reused and not overwritten.
         */
        let revised_catalogue = PlayerCatalogue::from_json(
            r#"
            {
              "version": 2,
              "players": [
                {
                  "key": "test:player-a",
                  "preferred_name": "Player A revised"
                }
              ]
            }
            "#,
        )?;

        let second = revised_catalogue.materialise_player(&mut connection, "test:player-a")?;

        assert!(!second.created);
        assert_eq!(second.player_id, first.player_id);

        let preserved_name: String = connection.query_row(
            "SELECT preferred_name FROM players WHERE id = ?1",
            [first.player_id],
            |row| row.get(0),
        )?;

        assert_eq!(preserved_name, "User display for A");

        let local_only_key: Option<String> = connection.query_row(
            "SELECT catalogue_key FROM players WHERE id = ?1",
            [local_only_id],
            |row| row.get(0),
        )?;

        assert_eq!(local_only_key, None);

        let final_alias_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_aliases", [], |row| row.get(0))?;

        assert_eq!(final_alias_count, 1);

        /*
         * Materialisation must require an actual supplied catalogue identity.
         */
        let error = catalogue
            .materialise_player(&mut connection, "test:missing")
            .expect_err("unknown catalogue key must not materialise");

        assert!(
            error
                .to_string()
                .contains("player catalogue contains no identity")
        );

        Ok(())
    }

    #[test]
    fn synchronises_catalogue_without_deleting_local_identity_data() -> Result<()> {
        let temporary_directory = tempdir()?;
        let database_root = temporary_directory.path().join("database");

        database::initialise(&database_root)?;
        let mut connection = database::open(&database_root)?;

        let first = PlayerCatalogue::from_json(
            r#"
            {
              "version": 1,
              "players": [
                {
                  "key": "test:player-a",
                  "preferred_name": "Player A",
                  "aliases": [
                    {
                      "name": "Alias A",
                      "notes": "first catalogue"
                    }
                  ]
                },
                {
                  "key": "test:player-b",
                  "preferred_name": "Player B",
                  "aliases": [
                    {
                      "name": "Shared Alias"
                    }
                  ]
                }
              ]
            }
            "#,
        )?;

        assert!(first.synchronise(&mut connection)?);

        let first_version: i64 = connection.query_row(
            "SELECT data_version FROM player_catalogue_state WHERE id = 1",
            [],
            |row| row.get(0),
        )?;

        let first_player_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_catalogue_players", [], |row| {
                row.get(0)
            })?;

        let first_alias_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_catalogue_aliases", [], |row| {
                row.get(0)
            })?;

        assert_eq!(first_version, 1);
        assert_eq!(first_player_count, 2);
        assert_eq!(first_alias_count, 2);

        /*
         * Materialise one supplied identity into the local/user layer and add
         * local information to it. The next catalogue version deliberately
         * removes test:player-a; synchronisation must not delete this row or
         * its local alias.
         */
        connection.execute(
            r#"
            INSERT INTO players(preferred_name, catalogue_key)
            VALUES ('User display for A', 'test:player-a')
            "#,
            [],
        )?;

        let local_player_id = connection.last_insert_rowid();

        connection.execute(
            r#"
            INSERT INTO player_aliases(
                player_id,
                name,
                source_id,
                notes
            )
            VALUES (?1, 'User Alias A', NULL, 'local user knowledge')
            "#,
            [local_player_id],
        )?;

        let second = PlayerCatalogue::from_json(
            r#"
            {
              "version": 2,
              "players": [
                {
                  "key": "test:player-b",
                  "preferred_name": "Player B revised",
                  "aliases": [
                    {
                      "name": "Shared Alias"
                    }
                  ]
                },
                {
                  "key": "test:player-c",
                  "preferred_name": "Player C",
                  "aliases": [
                    {
                      "name": "Shared Alias"
                    }
                  ]
                }
              ]
            }
            "#,
        )?;

        assert!(second.synchronise(&mut connection)?);

        let second_version: i64 = connection.query_row(
            "SELECT data_version FROM player_catalogue_state WHERE id = 1",
            [],
            |row| row.get(0),
        )?;

        assert_eq!(second_version, 2);

        let catalogue_player_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_catalogue_players", [], |row| {
                row.get(0)
            })?;

        let catalogue_alias_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM player_catalogue_aliases", [], |row| {
                row.get(0)
            })?;

        assert_eq!(catalogue_player_count, 2);
        assert_eq!(catalogue_alias_count, 2);

        let removed_catalogue_player_count: i64 = connection.query_row(
            r#"
            SELECT COUNT(*)
            FROM player_catalogue_players
            WHERE catalogue_key = 'test:player-a'
            "#,
            [],
            |row| row.get(0),
        )?;

        assert_eq!(removed_catalogue_player_count, 0);

        let revised_name: String = connection.query_row(
            r#"
            SELECT preferred_name
            FROM player_catalogue_players
            WHERE catalogue_key = 'test:player-b'
            "#,
            [],
            |row| row.get(0),
        )?;

        assert_eq!(revised_name, "Player B revised");

        /*
         * Ambiguous supplied names remain representable. The same exact
         * alias may point at more than one catalogue identity.
         */
        let shared_alias_count: i64 = connection.query_row(
            r#"
            SELECT COUNT(*)
            FROM player_catalogue_aliases
            WHERE name = 'Shared Alias'
            "#,
            [],
            |row| row.get(0),
        )?;

        assert_eq!(shared_alias_count, 2);

        let (local_name, local_catalogue_key): (String, Option<String>) = connection.query_row(
            r#"
                SELECT preferred_name, catalogue_key
                FROM players
                WHERE id = ?1
                "#,
            [local_player_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        assert_eq!(local_name, "User display for A");
        assert_eq!(local_catalogue_key.as_deref(), Some("test:player-a"));

        let local_alias_count: i64 = connection.query_row(
            r#"
            SELECT COUNT(*)
            FROM player_aliases
            WHERE player_id = ?1
              AND name = 'User Alias A'
              AND notes = 'local user knowledge'
            "#,
            [local_player_id],
            |row| row.get(0),
        )?;

        assert_eq!(local_alias_count, 1);

        Ok(())
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ReconciledSideState {
        raw_name: Option<String>,
        player_id: Option<i64>,
        catalogue_derived: i64,
        catalogue_suppressed: i64,
    }

    fn reconciliation_test_connection() -> Result<rusqlite::Connection> {
        let connection = rusqlite::Connection::open_in_memory()?;

        connection.execute_batch(
            r#"
            CREATE TABLE players (
                id              INTEGER PRIMARY KEY,
                preferred_name  TEXT NOT NULL,
                catalogue_key   TEXT
            );

            CREATE UNIQUE INDEX players_catalogue_key
                ON players(catalogue_key)
                WHERE catalogue_key IS NOT NULL;

            CREATE TABLE player_aliases (
                id          INTEGER PRIMARY KEY,
                player_id   INTEGER NOT NULL,
                name        TEXT NOT NULL,
                source_id   INTEGER,
                notes       TEXT
            );

            CREATE TABLE game_sources (
                id          INTEGER PRIMARY KEY,
                source_id   INTEGER NOT NULL
            );

            CREATE TABLE game_metadata (
                game_source_id INTEGER PRIMARY KEY,
                black_player TEXT,
                white_player TEXT,
                black_player_id INTEGER,
                white_player_id INTEGER,
                black_player_catalogue_derived INTEGER NOT NULL DEFAULT 0
                    CHECK (black_player_catalogue_derived IN (0, 1)),
                white_player_catalogue_derived INTEGER NOT NULL DEFAULT 0
                    CHECK (white_player_catalogue_derived IN (0, 1)),
                black_player_catalogue_suppressed INTEGER NOT NULL DEFAULT 0
                    CHECK (black_player_catalogue_suppressed IN (0, 1)),
                white_player_catalogue_suppressed INTEGER NOT NULL DEFAULT 0
                    CHECK (white_player_catalogue_suppressed IN (0, 1))
            );

            CREATE TABLE player_catalogue_state (
                id              INTEGER PRIMARY KEY CHECK (id = 1),
                data_version    INTEGER NOT NULL CHECK (data_version >= 0)
            );

            CREATE TABLE player_catalogue_players (
                catalogue_key   TEXT PRIMARY KEY,
                preferred_name  TEXT NOT NULL
            );

            CREATE TABLE player_catalogue_aliases (
                id              INTEGER PRIMARY KEY,
                catalogue_key   TEXT NOT NULL,
                name            TEXT NOT NULL,
                notes           TEXT
            );

            CREATE UNIQUE INDEX player_catalogue_alias_assignment
                ON player_catalogue_aliases(catalogue_key, name);
            "#,
        )?;

        Ok(connection)
    }

    fn reconciled_side_state(
        connection: &rusqlite::Connection,
        game_source_id: i64,
        black: bool,
    ) -> Result<ReconciledSideState> {
        let sql = if black {
            r#"
            SELECT
                black_player,
                black_player_id,
                black_player_catalogue_derived,
                black_player_catalogue_suppressed
            FROM game_metadata
            WHERE game_source_id = ?1
            "#
        } else {
            r#"
            SELECT
                white_player,
                white_player_id,
                white_player_catalogue_derived,
                white_player_catalogue_suppressed
            FROM game_metadata
            WHERE game_source_id = ?1
            "#
        };

        connection
            .query_row(sql, [game_source_id], |row| {
                Ok(ReconciledSideState {
                    raw_name: row.get(0)?,
                    player_id: row.get(1)?,
                    catalogue_derived: row.get(2)?,
                    catalogue_suppressed: row.get(3)?,
                })
            })
            .context("reading reconciled player-side state")
    }

    #[test]
    fn catalogue_reconciliation_respects_local_ambiguity_and_suppression() -> Result<()> {
        let mut connection = reconciliation_test_connection()?;

        connection.execute_batch(
            r#"
            INSERT INTO players(id, preferred_name)
            VALUES
                (101, 'Local Winner'),
                (102, 'Ambiguous Local A'),
                (103, 'Ambiguous Local B'),
                (104, 'Local Locked');

            INSERT INTO player_aliases(player_id, name, source_id)
            VALUES
                (101, 'Local Name', 1),
                (102, 'Ambiguous Local', 1),
                (103, 'Ambiguous Local', 1);

            INSERT INTO game_sources(id, source_id)
            VALUES
                (10, 1),
                (11, 1),
                (12, 1),
                (13, 1);

            INSERT INTO game_metadata(
                game_source_id,
                black_player,
                white_player,
                black_player_id,
                white_player_id,
                black_player_catalogue_derived,
                white_player_catalogue_derived,
                black_player_catalogue_suppressed,
                white_player_catalogue_suppressed
            )
            VALUES
                (
                    10,
                    'Catalogue Name',
                    'Local Name',
                    NULL,
                    NULL,
                    0,
                    0,
                    0,
                    0
                ),
                (
                    11,
                    'Ambiguous Local',
                    'Shared Catalogue',
                    NULL,
                    NULL,
                    0,
                    0,
                    0,
                    0
                ),
                (
                    12,
                    'Suppressed Name',
                    'Unknown',
                    NULL,
                    NULL,
                    0,
                    0,
                    1,
                    0
                ),
                (
                    13,
                    'Local Locked',
                    'Catalogue Name',
                    104,
                    NULL,
                    0,
                    0,
                    0,
                    0
                );
            "#,
        )?;

        let catalogue = PlayerCatalogue::from_json(
            r#"
            {
              "version": 1,
              "players": [
                {
                  "key": "test:catalogue-only",
                  "preferred_name": "Catalogue Name"
                },
                {
                  "key": "test:shadowed",
                  "preferred_name": "Catalogue Shadow",
                  "aliases": [
                    { "name": "Local Name" }
                  ]
                },
                {
                  "key": "test:local-ambiguous",
                  "preferred_name": "Catalogue Local Ambiguous",
                  "aliases": [
                    { "name": "Ambiguous Local" }
                  ]
                },
                {
                  "key": "test:shared-a",
                  "preferred_name": "Shared A",
                  "aliases": [
                    { "name": "Shared Catalogue" }
                  ]
                },
                {
                  "key": "test:shared-b",
                  "preferred_name": "Shared B",
                  "aliases": [
                    { "name": "Shared Catalogue" }
                  ]
                },
                {
                  "key": "test:suppressed",
                  "preferred_name": "Suppressed Name"
                },
                {
                  "key": "test:local-locked",
                  "preferred_name": "Local Locked"
                }
              ]
            }
            "#,
        )?;

        assert!(catalogue.synchronise(&mut connection)?);

        let catalogue_player_id: i64 = connection.query_row(
            r#"
            SELECT id
            FROM players
            WHERE catalogue_key = 'test:catalogue-only'
            "#,
            [],
            |row| row.get(0),
        )?;

        assert_eq!(
            reconciled_side_state(&connection, 10, true)?,
            ReconciledSideState {
                raw_name: Some("Catalogue Name".to_owned()),
                player_id: Some(catalogue_player_id),
                catalogue_derived: 1,
                catalogue_suppressed: 0,
            }
        );

        assert_eq!(
            reconciled_side_state(&connection, 10, false)?,
            ReconciledSideState {
                raw_name: Some("Local Name".to_owned()),
                player_id: Some(101),
                catalogue_derived: 0,
                catalogue_suppressed: 0,
            }
        );

        /*
         * Local ambiguity is a stop condition. Supplied ambiguity is also
         * unresolved rather than guessed.
         */
        assert_eq!(
            reconciled_side_state(&connection, 11, true)?,
            ReconciledSideState {
                raw_name: Some("Ambiguous Local".to_owned()),
                player_id: None,
                catalogue_derived: 0,
                catalogue_suppressed: 0,
            }
        );

        assert_eq!(
            reconciled_side_state(&connection, 11, false)?,
            ReconciledSideState {
                raw_name: Some("Shared Catalogue".to_owned()),
                player_id: None,
                catalogue_derived: 0,
                catalogue_suppressed: 0,
            }
        );

        /*
         * Explicit suppression wins over a supplied catalogue match.
         */
        assert_eq!(
            reconciled_side_state(&connection, 12, true)?,
            ReconciledSideState {
                raw_name: Some("Suppressed Name".to_owned()),
                player_id: None,
                catalogue_derived: 0,
                catalogue_suppressed: 1,
            }
        );

        /*
         * Existing local ownership is not rewritten even when the raw spelling
         * also exists in the supplied catalogue.
         */
        assert_eq!(
            reconciled_side_state(&connection, 13, true)?,
            ReconciledSideState {
                raw_name: Some("Local Locked".to_owned()),
                player_id: Some(104),
                catalogue_derived: 0,
                catalogue_suppressed: 0,
            }
        );

        assert_eq!(
            reconciled_side_state(&connection, 13, false)?,
            ReconciledSideState {
                raw_name: Some("Catalogue Name".to_owned()),
                player_id: Some(catalogue_player_id),
                catalogue_derived: 1,
                catalogue_suppressed: 0,
            }
        );

        let stored_version: i64 = connection.query_row(
            "SELECT data_version FROM player_catalogue_state WHERE id = 1",
            [],
            |row| row.get(0),
        )?;

        assert_eq!(stored_version, 1);

        /*
         * Reopening against the same bundled data version is a genuine no-op.
         */
        assert!(!catalogue.synchronise(&mut connection)?);

        Ok(())
    }

    #[test]
    fn catalogue_version_change_repairs_only_catalogue_owned_interpretations() -> Result<()> {
        let mut connection = reconciliation_test_connection()?;

        connection.execute_batch(
            r#"
            INSERT INTO players(id, preferred_name)
            VALUES (201, 'Local Retarget');

            INSERT INTO game_sources(id, source_id)
            VALUES
                (20, 1),
                (21, 1),
                (22, 1),
                (23, 1),
                (24, 1);

            INSERT INTO game_metadata(
                game_source_id,
                black_player
            )
            VALUES
                (20, 'Stable'),
                (21, 'Removed'),
                (22, 'Retarget'),
                (23, 'Rekeyed'),
                (24, 'New Name');
            "#,
        )?;

        let first = PlayerCatalogue::from_json(
            r#"
            {
              "version": 1,
              "players": [
                {
                  "key": "test:stable",
                  "preferred_name": "Stable"
                },
                {
                  "key": "test:old",
                  "preferred_name": "Old Player",
                  "aliases": [
                    { "name": "Removed" },
                    { "name": "Retarget" },
                    { "name": "Rekeyed" }
                  ]
                }
              ]
            }
            "#,
        )?;

        assert!(first.synchronise(&mut connection)?);

        let stable_id: i64 = connection.query_row(
            "SELECT id FROM players WHERE catalogue_key = 'test:stable'",
            [],
            |row| row.get(0),
        )?;

        let old_id: i64 = connection.query_row(
            "SELECT id FROM players WHERE catalogue_key = 'test:old'",
            [],
            |row| row.get(0),
        )?;

        assert_eq!(
            reconciled_side_state(&connection, 20, true)?.player_id,
            Some(stable_id)
        );
        assert_eq!(
            reconciled_side_state(&connection, 21, true)?.player_id,
            Some(old_id)
        );
        assert_eq!(
            reconciled_side_state(&connection, 22, true)?.player_id,
            Some(old_id)
        );
        assert_eq!(
            reconciled_side_state(&connection, 23, true)?.player_id,
            Some(old_id)
        );
        assert_eq!(
            reconciled_side_state(&connection, 24, true)?.player_id,
            None
        );

        /*
         * Local knowledge added after catalogue version 1 must take precedence
         * when version 2 re-evaluates the old catalogue-derived Retarget link.
         */
        connection.execute(
            r#"
            INSERT INTO player_aliases(player_id, name, source_id)
            VALUES (201, 'Retarget', 1)
            "#,
            [],
        )?;

        let second = PlayerCatalogue::from_json(
            r#"
            {
              "version": 2,
              "players": [
                {
                  "key": "test:stable",
                  "preferred_name": "Stable"
                },
                {
                  "key": "test:old",
                  "preferred_name": "Old Player"
                },
                {
                  "key": "test:new",
                  "preferred_name": "New Name",
                  "aliases": [
                    { "name": "Rekeyed" }
                  ]
                },
                {
                  "key": "test:catalogue-retarget",
                  "preferred_name": "Catalogue Retarget",
                  "aliases": [
                    { "name": "Retarget" }
                  ]
                }
              ]
            }
            "#,
        )?;

        assert!(second.synchronise(&mut connection)?);

        let new_id: i64 = connection.query_row(
            "SELECT id FROM players WHERE catalogue_key = 'test:new'",
            [],
            |row| row.get(0),
        )?;

        /*
         * A still-valid catalogue interpretation remains catalogue-owned.
         */
        assert_eq!(
            reconciled_side_state(&connection, 20, true)?,
            ReconciledSideState {
                raw_name: Some("Stable".to_owned()),
                player_id: Some(stable_id),
                catalogue_derived: 1,
                catalogue_suppressed: 0,
            }
        );

        /*
         * Bermuda withdrawing an interpretation makes it ordinarily
         * unresolved, not user-suppressed.
         */
        assert_eq!(
            reconciled_side_state(&connection, 21, true)?,
            ReconciledSideState {
                raw_name: Some("Removed".to_owned()),
                player_id: None,
                catalogue_derived: 0,
                catalogue_suppressed: 0,
            }
        );

        /*
         * Local exact knowledge added since the previous catalogue version
         * replaces the old catalogue interpretation.
         */
        assert_eq!(
            reconciled_side_state(&connection, 22, true)?,
            ReconciledSideState {
                raw_name: Some("Retarget".to_owned()),
                player_id: Some(201),
                catalogue_derived: 0,
                catalogue_suppressed: 0,
            }
        );

        /*
         * A supplied spelling may legitimately move to a different stable
         * catalogue identity.
         */
        assert_eq!(
            reconciled_side_state(&connection, 23, true)?,
            ReconciledSideState {
                raw_name: Some("Rekeyed".to_owned()),
                player_id: Some(new_id),
                catalogue_derived: 1,
                catalogue_suppressed: 0,
            }
        );

        /*
         * Previously unresolved historical metadata may acquire an identity
         * when the newer catalogue learns that spelling.
         */
        assert_eq!(
            reconciled_side_state(&connection, 24, true)?,
            ReconciledSideState {
                raw_name: Some("New Name".to_owned()),
                player_id: Some(new_id),
                catalogue_derived: 1,
                catalogue_suppressed: 0,
            }
        );

        assert_eq!(
            connection.query_row(
                "SELECT data_version FROM player_catalogue_state WHERE id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            2
        );

        /*
         * An older Bermuda binary must not silently downgrade a project whose
         * supplied catalogue state was written by a newer catalogue version.
         */
        let downgrade_error = first
            .synchronise(&mut connection)
            .expect_err("catalogue downgrade must be refused");

        assert!(downgrade_error.to_string().contains("newer"));

        assert_eq!(
            connection.query_row(
                "SELECT data_version FROM player_catalogue_state WHERE id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            2
        );

        Ok(())
    }

    #[test]
    fn supplied_catalogue_parses() -> Result<()> {
        let catalogue = PlayerCatalogue::supplied()?;

        assert_eq!(catalogue.version, 1);
        assert!(catalogue.players.is_empty());

        Ok(())
    }

    #[test]
    fn parses_player_aliases_and_optional_notes() -> Result<()> {
        let catalogue = PlayerCatalogue::from_json(
            r#"
            {
              "version": 7,
              "players": [
                {
                  "key": "test:player-a",
                  "preferred_name": "Player A",
                  "aliases": [
                    {
                      "name": "Alias A1"
                    },
                    {
                      "name": "Alias A2",
                      "notes": "test provenance note"
                    }
                  ]
                },
                {
                  "key": "test:player-b",
                  "preferred_name": "Player B"
                }
              ]
            }
            "#,
        )?;

        assert_eq!(catalogue.version, 7);
        assert_eq!(catalogue.players.len(), 2);

        let player_a = &catalogue.players[0];
        assert_eq!(player_a.key, "test:player-a");
        assert_eq!(player_a.preferred_name, "Player A");
        assert_eq!(player_a.aliases.len(), 2);

        assert_eq!(player_a.aliases[0].name, "Alias A1");
        assert_eq!(player_a.aliases[0].notes, None);

        assert_eq!(player_a.aliases[1].name, "Alias A2");
        assert_eq!(
            player_a.aliases[1].notes.as_deref(),
            Some("test provenance note")
        );

        let player_b = &catalogue.players[1];
        assert_eq!(player_b.key, "test:player-b");
        assert_eq!(player_b.preferred_name, "Player B");
        assert!(player_b.aliases.is_empty());

        Ok(())
    }

    #[test]
    fn rejects_semantically_invalid_catalogues() {
        let cases = [
            (
                r#"{"version":0,"players":[]}"#,
                "version must be greater than zero",
            ),
            (
                r#"{
                    "version":1,
                    "players":[
                        {"key":"","preferred_name":"Player A"}
                    ]
                }"#,
                "key must not be empty",
            ),
            (
                r#"{
                    "version":1,
                    "players":[
                        {"key":"test:a","preferred_name":""}
                    ]
                }"#,
                "preferred name must not be empty",
            ),
            (
                r#"{
                    "version":1,
                    "players":[
                        {"key":"test:a","preferred_name":"Player A"},
                        {"key":"test:a","preferred_name":"Player B"}
                    ]
                }"#,
                "duplicate player catalogue key",
            ),
            (
                r#"{
                    "version":1,
                    "players":[
                        {
                            "key":"test:a",
                            "preferred_name":"Player A",
                            "aliases":[
                                {"name":"Alias A"},
                                {"name":"Alias A"}
                            ]
                        }
                    ]
                }"#,
                "duplicate player catalogue alias",
            ),
        ];

        for (json, expected_message) in cases {
            let error = PlayerCatalogue::from_json(json).expect_err("invalid catalogue must fail");

            assert!(
                error.to_string().contains(expected_message),
                "expected {expected_message:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn rejects_malformed_catalogue() {
        let error = PlayerCatalogue::from_json(
            r#"
            {
              "version": 1,
              "players": [
                {
                  "preferred_name": "Missing key"
                }
              ]
            }
            "#,
        )
        .expect_err("catalogue without player key must fail");

        assert!(error.to_string().contains("parsing player catalogue JSON"));
    }
}
