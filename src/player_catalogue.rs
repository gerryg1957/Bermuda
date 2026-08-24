use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, TransactionBehavior, params};
use serde::Deserialize;

const SUPPLIED_CATALOGUE_JSON: &str = include_str!("../data/player_catalogue.json");

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
         * Materialisation is key-based, never name-based.
         *
         * A local player with the same preferred_name but no catalogue_key is
         * therefore not silently merged with the supplied identity.
         *
         * IMMEDIATE makes the find-or-create operation atomic with respect to
         * other SQLite writers. The unique players.catalogue_key index remains
         * the final database-level invariant.
         */
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("starting catalogue player materialisation")?;

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

        transaction
            .commit()
            .context("committing catalogue player materialisation")?;

        Ok(CataloguePlayerMaterialisation { player_id, created })
    }

    pub fn synchronise(&self, connection: &mut Connection) -> Result<()> {
        /*
         * Validation is repeated here deliberately. PlayerCatalogue's fields
         * are public, so callers are not required to have constructed it via
         * from_json().
         */
        self.validate()?;

        let data_version = i64::try_from(self.version)
            .context("player catalogue version exceeds SQLite integer range")?;

        /*
         * The supplied catalogue is replaced as one SQLite transaction.
         *
         * This operation deliberately owns only player_catalogue_* tables.
         * In particular it must not rewrite players, player_aliases,
         * game_metadata, or source PB/PW strings.
         */
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("starting supplied player catalogue synchronisation")?;

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

            for player in &self.players {
                insert_player
                    .execute(params![player.key.as_str(), player.preferred_name.as_str(),])
                    .with_context(|| {
                        format!("storing supplied catalogue player {:?}", player.key)
                    })?;
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

            for player in &self.players {
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

        transaction
            .commit()
            .context("committing supplied player catalogue synchronisation")?;

        Ok(())
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
    fn synchronises_only_supplied_catalogue_data() -> Result<()> {
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

        first.synchronise(&mut connection)?;

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

        second.synchronise(&mut connection)?;

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
