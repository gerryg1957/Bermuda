use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{database, project::Project};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerIdentity {
    pub id: i64,
    pub preferred_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerAlias {
    pub id: i64,
    pub player_id: i64,
    pub name: String,
    pub source_id: Option<i64>,
    pub source_name: Option<String>,
    pub source_version: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedPlayerName {
    pub source_id: i64,
    pub source_name: String,
    pub source_version: String,
    pub name: String,
    pub occurrence_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAliasResolutionPreview {
    pub alias_id: i64,
    pub player_id: i64,
    pub source_id: i64,
    pub source_name: String,
    pub source_version: String,
    pub name: String,
    pub unresolved_black_count: u64,
    pub unresolved_white_count: u64,
    pub catalogue_black_count: u64,
    pub catalogue_white_count: u64,
    pub already_linked_count: u64,
    pub conflicting_link_count: u64,
    pub competing_alias_count: u64,
}

impl SourceAliasResolutionPreview {
    pub fn unresolved_count(&self) -> u64 {
        self.unresolved_black_count + self.unresolved_white_count
    }

    pub fn catalogue_count(&self) -> u64 {
        self.catalogue_black_count + self.catalogue_white_count
    }

    pub fn assignable_count(&self) -> u64 {
        self.unresolved_count() + self.catalogue_count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAliasResolutionResult {
    pub alias_id: i64,
    pub player_id: i64,
    pub source_id: i64,
    pub source_name: String,
    pub source_version: String,
    pub name: String,
    pub linked_black_count: u64,
    pub linked_white_count: u64,
}

impl SourceAliasResolutionResult {
    pub fn linked_count(&self) -> u64 {
        self.linked_black_count + self.linked_white_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSide {
    Black,
    White,
}

pub struct PlayerDirectory {
    connection: Connection,
}

impl PlayerDirectory {
    pub fn open(database_root: &Path) -> Result<Self> {
        let connection = database::open(database_root)?;

        Ok(Self { connection })
    }

    pub fn open_project(project: &Project) -> Result<Self> {
        Self::open(&project.database_root())
    }

    pub fn create_player(&self, preferred_name: &str) -> Result<PlayerIdentity> {
        let preferred_name = required_text(preferred_name, "preferred player name")?;

        self.connection
            .execute(
                "INSERT INTO players(preferred_name) VALUES (?1)",
                [preferred_name],
            )
            .context("creating player identity")?;

        Ok(PlayerIdentity {
            id: self.connection.last_insert_rowid(),
            preferred_name: preferred_name.to_owned(),
        })
    }

    pub fn rename_player(&self, player_id: i64, preferred_name: &str) -> Result<()> {
        let preferred_name = required_text(preferred_name, "preferred player name")?;

        let changed = self
            .connection
            .execute(
                r#"
                UPDATE players
                SET preferred_name = ?1
                WHERE id = ?2
                "#,
                params![preferred_name, player_id],
            )
            .with_context(|| format!("renaming player {player_id}"))?;

        if changed == 0 {
            bail!("player {player_id} does not exist");
        }

        Ok(())
    }

    pub fn list_players(&self) -> Result<Vec<PlayerIdentity>> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT id, preferred_name
                FROM players
                ORDER BY preferred_name COLLATE NOCASE, id
                "#,
            )
            .context("preparing player list")?;

        let rows = statement
            .query_map([], |row| {
                Ok(PlayerIdentity {
                    id: row.get(0)?,
                    preferred_name: row.get(1)?,
                })
            })
            .context("reading player list")?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("collecting player list")
    }

    pub fn add_alias(
        &self,
        player_id: i64,
        name: &str,
        source_id: Option<i64>,
        notes: Option<&str>,
    ) -> Result<PlayerAlias> {
        self.require_player(player_id)?;

        if let Some(source_id) = source_id {
            self.require_source(source_id)?;
        }

        let name = required_text(name, "player alias")?;
        let notes = optional_text(notes);

        self.connection
            .execute(
                r#"
                INSERT INTO player_aliases(
                    player_id,
                    name,
                    source_id,
                    notes
                )
                VALUES (?1, ?2, ?3, ?4)
                "#,
                params![player_id, name, source_id, notes],
            )
            .with_context(|| format!("adding alias {name:?} to player {player_id}"))?;

        let alias_id = self.connection.last_insert_rowid();

        self.get_alias(alias_id)
    }

    /// Remove a curated alias.
    ///
    /// For a source-specific alias, this also restores metadata linked by
    /// that exact source/name assertion to the unresolved state. The raw
    /// PB/PW strings are never changed.
    ///
    /// A global alias is only a search/identity assertion and therefore has
    /// no source-specific metadata links to undo.
    pub fn remove_alias(&self, alias_id: i64) -> Result<()> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .context("starting player-alias removal transaction")?;

        let alias: Option<(i64, String, Option<i64>)> = transaction
            .query_row(
                r#"
                SELECT player_id, name, source_id
                FROM player_aliases
                WHERE id = ?1
                "#,
                [alias_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .with_context(|| format!("reading player alias {alias_id}"))?;

        let Some((player_id, name, source_id)) = alias else {
            bail!("player alias {alias_id} does not exist");
        };

        if let Some(source_id) = source_id {
            transaction
                .execute(
                    r#"
                    UPDATE game_metadata
                    SET black_player_id = NULL,
                        black_player_catalogue_derived = 0
                    WHERE black_player_id = ?1
                      AND black_player = ?2
                      AND black_player_catalogue_derived = 0
                      AND EXISTS (
                          SELECT 1
                          FROM game_sources AS gs
                          WHERE gs.id = game_metadata.game_source_id
                            AND gs.source_id = ?3
                      )
                    "#,
                    params![player_id, &name, source_id],
                )
                .with_context(|| format!("unlinking Black-player metadata for alias {alias_id}"))?;

            transaction
                .execute(
                    r#"
                    UPDATE game_metadata
                    SET white_player_id = NULL,
                        white_player_catalogue_derived = 0
                    WHERE white_player_id = ?1
                      AND white_player = ?2
                      AND white_player_catalogue_derived = 0
                      AND EXISTS (
                          SELECT 1
                          FROM game_sources AS gs
                          WHERE gs.id = game_metadata.game_source_id
                            AND gs.source_id = ?3
                      )
                    "#,
                    params![player_id, &name, source_id],
                )
                .with_context(|| format!("unlinking White-player metadata for alias {alias_id}"))?;
        }

        let changed = transaction
            .execute("DELETE FROM player_aliases WHERE id = ?1", [alias_id])
            .with_context(|| format!("removing player alias {alias_id}"))?;

        if changed != 1 {
            bail!(
                "expected to remove player alias {alias_id}, but the row disappeared during the transaction"
            );
        }

        transaction
            .commit()
            .context("committing player-alias removal")?;

        Ok(())
    }

    /// Remove an entire player identity without deleting or rewriting any
    /// imported game metadata.
    ///
    /// All numeric identity links are cleared first. Aliases then disappear
    /// through the player_aliases ON DELETE CASCADE relationship.
    pub fn delete_player(&self, player_id: i64) -> Result<String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .context("starting player-identity removal transaction")?;

        let preferred_name: Option<String> = transaction
            .query_row(
                "SELECT preferred_name FROM players WHERE id = ?1",
                [player_id],
                |row| row.get(0),
            )
            .optional()
            .with_context(|| format!("reading player identity {player_id}"))?;

        let Some(preferred_name) = preferred_name else {
            bail!("player identity {player_id} does not exist");
        };

        transaction
            .execute(
                r#"
                UPDATE game_metadata
                SET black_player_id = NULL,
                    black_player_catalogue_derived = 0
                WHERE black_player_id = ?1
                "#,
                [player_id],
            )
            .with_context(|| format!("unlinking Black-player metadata for player {player_id}"))?;

        transaction
            .execute(
                r#"
                UPDATE game_metadata
                SET white_player_id = NULL,
                    white_player_catalogue_derived = 0
                WHERE white_player_id = ?1
                "#,
                [player_id],
            )
            .with_context(|| format!("unlinking White-player metadata for player {player_id}"))?;

        let changed = transaction
            .execute("DELETE FROM players WHERE id = ?1", [player_id])
            .with_context(|| format!("removing player identity {player_id}"))?;

        if changed != 1 {
            bail!(
                "expected to remove player identity {player_id}, but the row disappeared during the transaction"
            );
        }

        transaction
            .commit()
            .context("committing player-identity removal")?;

        Ok(preferred_name)
    }

    pub fn aliases_for_player(&self, player_id: i64) -> Result<Vec<PlayerAlias>> {
        self.require_player(player_id)?;

        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                    pa.id,
                    pa.player_id,
                    pa.name,
                    pa.source_id,
                    s.name,
                    s.version,
                    pa.notes
                FROM player_aliases AS pa
                LEFT JOIN sources AS s
                    ON s.id = pa.source_id
                WHERE pa.player_id = ?1
                ORDER BY
                    pa.name COLLATE NOCASE,
                    pa.source_id IS NOT NULL,
                    s.name COLLATE NOCASE,
                    s.version COLLATE NOCASE,
                    pa.id
                "#,
            )
            .with_context(|| format!("preparing aliases for player {player_id}"))?;

        let rows = statement
            .query_map([player_id], player_alias_from_row)
            .with_context(|| format!("reading aliases for player {player_id}"))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .with_context(|| format!("collecting aliases for player {player_id}"))
    }

    /// Return every player identity explicitly known by this exact name.
    ///
    /// Search semantics deliberately differ from import-time resolution:
    /// ambiguity broadens a search rather than forcing Bermuda to choose one
    /// identity. Source-specific aliases are therefore also valid search
    /// names for that identity in games from other sources.
    pub fn player_ids_for_search_name(&self, name: &str) -> Result<Vec<i64>> {
        player_ids_for_search_name_on(&self.connection, name)
    }

    pub fn preview_source_alias_resolution(
        &self,
        alias_id: i64,
    ) -> Result<SourceAliasResolutionPreview> {
        preview_source_alias_resolution_on(&self.connection, alias_id)
    }

    pub fn apply_source_alias_resolution(
        &self,
        alias_id: i64,
    ) -> Result<SourceAliasResolutionResult> {
        /*
         * Re-run the complete preview inside the same transaction that will
         * perform the updates. The write therefore never relies on an older
         * preview supplied by a caller.
         */
        let transaction = self
            .connection
            .unchecked_transaction()
            .context("starting source-alias resolution transaction")?;

        let result = apply_source_alias_resolution_on(&transaction, alias_id)?;

        transaction
            .commit()
            .context("committing source-alias resolution")?;

        Ok(result)
    }

    /// Assign one exact source spelling to a Bermuda player identity.
    ///
    /// The source-specific alias and every corresponding unresolved metadata
    /// link are created in one transaction. If the existing conflict guards
    /// refuse the resolution, a newly inserted alias is rolled back as well.
    ///
    /// `name` is deliberately preserved exactly. It comes from imported
    /// source metadata and is not normalised, case-folded, or fuzzy-matched.
    pub fn assign_source_name_to_player(
        &self,
        player_id: i64,
        source_id: i64,
        name: &str,
    ) -> Result<SourceAliasResolutionResult> {
        if name.trim().is_empty() {
            bail!("source player name must not be empty");
        }

        let transaction = self
            .connection
            .unchecked_transaction()
            .context("starting source-player assignment transaction")?;

        let player_exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM players WHERE id = ?1)",
                [player_id],
                |row| row.get(0),
            )
            .with_context(|| format!("checking player {player_id}"))?;

        if player_exists == 0 {
            bail!("player {player_id} does not exist");
        }

        let source_exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sources WHERE id = ?1)",
                [source_id],
                |row| row.get(0),
            )
            .with_context(|| format!("checking source {source_id}"))?;

        if source_exists == 0 {
            bail!("source {source_id} does not exist");
        }

        /*
         * Reuse an existing identical assignment to the same identity.
         * The schema's partial unique index guarantees there can be at most
         * one such source-specific alias.
         */
        let existing_alias_id = transaction
            .query_row(
                r#"
                SELECT id
                FROM player_aliases
                WHERE player_id = ?1
                  AND source_id = ?2
                  AND name = ?3
                "#,
                params![player_id, source_id, name],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .context("checking for an existing source-specific alias")?;

        let alias_id = match existing_alias_id {
            Some(alias_id) => alias_id,

            None => {
                transaction
                    .execute(
                        r#"
                        INSERT INTO player_aliases(
                            player_id,
                            name,
                            source_id,
                            notes
                        )
                        VALUES (?1, ?2, ?3, NULL)
                        "#,
                        params![player_id, name, source_id],
                    )
                    .with_context(|| {
                        format!(
                            "creating source alias {name:?} for player {player_id} \
                             and source {source_id}"
                        )
                    })?;

                transaction.last_insert_rowid()
            }
        };

        /*
         * This is the same guarded operation used by the public bulk resolver.
         * A competing alias, conflicting existing link, or row-count invariant
         * failure aborts this outer transaction too.
         */
        let result = apply_source_alias_resolution_on(&transaction, alias_id)?;

        transaction
            .commit()
            .context("committing source-player assignment")?;

        Ok(result)
    }

    /// Create a new Bermuda player and assign one exact source spelling to it.
    ///
    /// Player creation, source-specific alias creation, and linking every
    /// unresolved occurrence all happen in one transaction. If the existing
    /// source-alias guards refuse the assignment, neither the player nor the
    /// alias survives.
    ///
    /// The preferred name is normalised in the same way as create_player().
    /// The imported source spelling itself remains exact.
    pub fn create_player_and_assign_source_name(
        &self,
        preferred_name: &str,
        source_id: i64,
        name: &str,
    ) -> Result<(PlayerIdentity, SourceAliasResolutionResult)> {
        let preferred_name = required_text(preferred_name, "preferred player name")?;

        if name.trim().is_empty() {
            bail!("source player name must not be empty");
        }

        let transaction = self
            .connection
            .unchecked_transaction()
            .context("starting new-player source assignment transaction")?;

        let source_exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sources WHERE id = ?1)",
                [source_id],
                |row| row.get(0),
            )
            .with_context(|| format!("checking source {source_id}"))?;

        if source_exists == 0 {
            bail!("source {source_id} does not exist");
        }

        transaction
            .execute(
                "INSERT INTO players(preferred_name) VALUES (?1)",
                [preferred_name],
            )
            .context("creating player identity for source assignment")?;

        let player = PlayerIdentity {
            id: transaction.last_insert_rowid(),
            preferred_name: preferred_name.to_owned(),
        };

        transaction
            .execute(
                r#"
                INSERT INTO player_aliases(
                    player_id,
                    name,
                    source_id,
                    notes
                )
                VALUES (?1, ?2, ?3, NULL)
                "#,
                params![player.id, name, source_id],
            )
            .with_context(|| {
                format!(
                    "creating source alias {name:?} for new player {} \
                     and source {source_id}",
                    player.id
                )
            })?;

        let alias_id = transaction.last_insert_rowid();

        /*
         * Reuse exactly the same conflict detection, updates, and invariant
         * checks as an assignment to an existing player.
         */
        let result = apply_source_alias_resolution_on(&transaction, alias_id)?;

        transaction
            .commit()
            .context("committing new-player source assignment")?;

        Ok((player, result))
    }

    pub fn unresolved_names(&self) -> Result<Vec<UnresolvedPlayerName>> {
        /*
         * Each row describes a source spelling which is still unlinked in
         * source-specific game metadata. Black and white occurrences are
         * combined so the count reflects how often that spelling still needs
         * an identity decision for the source.
         */
        let mut statement = self
            .connection
            .prepare(
                r#"
                WITH unresolved AS (
                    SELECT
                        gs.source_id AS source_id,
                        s.name AS source_name,
                        s.version AS source_version,
                        gm.black_player AS player_name
                    FROM game_metadata AS gm
                    JOIN game_sources AS gs
                        ON gs.id = gm.game_source_id
                    JOIN sources AS s
                        ON s.id = gs.source_id
                    WHERE gm.black_player IS NOT NULL
                      AND TRIM(gm.black_player) <> ''
                      AND gm.black_player_id IS NULL

                    UNION ALL

                    SELECT
                        gs.source_id AS source_id,
                        s.name AS source_name,
                        s.version AS source_version,
                        gm.white_player AS player_name
                    FROM game_metadata AS gm
                    JOIN game_sources AS gs
                        ON gs.id = gm.game_source_id
                    JOIN sources AS s
                        ON s.id = gs.source_id
                    WHERE gm.white_player IS NOT NULL
                      AND TRIM(gm.white_player) <> ''
                      AND gm.white_player_id IS NULL
                )
                SELECT
                    source_id,
                    source_name,
                    source_version,
                    player_name,
                    COUNT(*)
                FROM unresolved
                GROUP BY
                    source_id,
                    source_name,
                    source_version,
                    player_name
                ORDER BY
                    player_name COLLATE NOCASE,
                    source_name COLLATE NOCASE,
                    source_version COLLATE NOCASE,
                    source_id
                "#,
            )
            .context("preparing unresolved player-name list")?;

        let rows = statement
            .query_map([], |row| {
                let occurrence_count: i64 = row.get(4)?;

                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    occurrence_count,
                ))
            })
            .context("reading unresolved player names")?;

        let mut unresolved = Vec::new();

        for row in rows {
            let (source_id, source_name, source_version, name, occurrence_count) =
                row.context("reading unresolved player-name row")?;

            unresolved.push(UnresolvedPlayerName {
                source_id,
                source_name,
                source_version,
                name,
                occurrence_count: u64::try_from(occurrence_count)
                    .context("negative unresolved player-name occurrence count")?,
            });
        }

        Ok(unresolved)
    }

    pub fn link_source_player(
        &self,
        game_source_id: i64,
        side: PlayerSide,
        player_id: i64,
    ) -> Result<String> {
        self.require_player(player_id)?;

        let source_name = self.source_player_name(game_source_id, side)?;

        let (id_column, catalogue_derived_column) = match side {
            PlayerSide::Black => ("black_player_id", "black_player_catalogue_derived"),
            PlayerSide::White => ("white_player_id", "white_player_catalogue_derived"),
        };

        let sql = format!(
            "UPDATE game_metadata
             SET {id_column} = ?1,
                 {catalogue_derived_column} = 0
             WHERE game_source_id = ?2"
        );

        let changed = self
            .connection
            .execute(&sql, params![player_id, game_source_id])
            .with_context(|| {
                format!(
                    "linking {side:?} player for game source {game_source_id} \
                     to player {player_id}"
                )
            })?;

        if changed == 0 {
            bail!("game source metadata {game_source_id} does not exist");
        }

        Ok(source_name)
    }

    pub fn unlink_source_player(&self, game_source_id: i64, side: PlayerSide) -> Result<()> {
        self.require_game_metadata(game_source_id)?;

        let (id_column, catalogue_derived_column) = match side {
            PlayerSide::Black => ("black_player_id", "black_player_catalogue_derived"),
            PlayerSide::White => ("white_player_id", "white_player_catalogue_derived"),
        };

        let sql = format!(
            "UPDATE game_metadata
             SET {id_column} = NULL,
                 {catalogue_derived_column} = 0
             WHERE game_source_id = ?1"
        );

        self.connection
            .execute(&sql, [game_source_id])
            .with_context(|| {
                format!("unlinking {side:?} player for game source {game_source_id}")
            })?;

        Ok(())
    }

    fn get_alias(&self, alias_id: i64) -> Result<PlayerAlias> {
        get_alias_from_connection(&self.connection, alias_id)
    }

    fn require_player(&self, player_id: i64) -> Result<()> {
        let exists: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM players WHERE id = ?1)",
                [player_id],
                |row| row.get(0),
            )
            .with_context(|| format!("checking player {player_id}"))?;

        if !exists {
            bail!("player {player_id} does not exist");
        }

        Ok(())
    }

    fn require_source(&self, source_id: i64) -> Result<()> {
        let exists: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sources WHERE id = ?1)",
                [source_id],
                |row| row.get(0),
            )
            .with_context(|| format!("checking source {source_id}"))?;

        if !exists {
            bail!("source {source_id} does not exist");
        }

        Ok(())
    }

    fn require_game_metadata(&self, game_source_id: i64) -> Result<()> {
        let exists: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM game_metadata WHERE game_source_id = ?1)",
                [game_source_id],
                |row| row.get(0),
            )
            .with_context(|| format!("checking game source metadata {game_source_id}"))?;

        if !exists {
            bail!("game source metadata {game_source_id} does not exist");
        }

        Ok(())
    }

    fn source_player_name(&self, game_source_id: i64, side: PlayerSide) -> Result<String> {
        let name_column = match side {
            PlayerSide::Black => "black_player",
            PlayerSide::White => "white_player",
        };

        let sql = format!("SELECT {name_column} FROM game_metadata WHERE game_source_id = ?1");

        let name = self
            .connection
            .query_row(&sql, [game_source_id], |row| {
                row.get::<_, Option<String>>(0)
            })
            .optional()
            .with_context(|| {
                format!("reading {side:?} player name for game source {game_source_id}")
            })?;

        let Some(name) = name else {
            bail!("game source metadata {game_source_id} does not exist");
        };

        let Some(name) = name.filter(|name| !name.trim().is_empty()) else {
            bail!("game source metadata {game_source_id} has no {side:?} player name");
        };

        Ok(name)
    }
}

pub(crate) fn player_ids_for_search_name_on(
    connection: &Connection,
    name: &str,
) -> Result<Vec<i64>> {
    /*
     * Preferred names and aliases are both curated identity assertions.
     *
     * Unlike import-time resolution, alias source scope does not restrict
     * searching: once a spelling is known to denote an identity, searching
     * for that spelling should find that identity wherever it plays.
     *
     * UNION deliberately removes duplicate IDs when, for example, a player's
     * preferred name is also one of that player's aliases.
     */
    let mut statement = connection
        .prepare(
            r#"
            SELECT id AS player_id
            FROM players
            WHERE preferred_name COLLATE NOCASE = ?1

            UNION

            SELECT player_id
            FROM player_aliases
            WHERE name COLLATE NOCASE = ?1

            ORDER BY player_id
            "#,
        )
        .context("preparing case-insensitive player search-name lookup")?;

    let rows = statement
        .query_map([name], |row| row.get::<_, i64>(0))
        .with_context(|| format!("resolving case-insensitive player search name {name:?}"))?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("collecting player IDs for case-insensitive search name")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAliasResolution {
    Unrecognised,
    Unique(i64),
    Ambiguous,
}

pub(crate) fn resolve_player_alias_for_source(
    connection: &Connection,
    source_id: i64,
    name: &str,
) -> Result<PlayerAliasResolution> {
    /*
     * Exact source-specific aliases take precedence over global aliases.
     *
     * The query deliberately returns at most two distinct player IDs. One
     * candidate is unambiguous; zero candidates means unresolved; two means
     * ambiguous and must also remain unresolved.
     *
     * If any source-specific candidate exists, global aliases are not
     * considered. Thus a global assertion can never override or "repair" an
     * ambiguous source-specific assertion.
     */
    let mut statement = connection
        .prepare(
            r#"
            WITH candidates(player_id, priority) AS (
                SELECT
                    player_id,
                    0
                FROM player_aliases
                WHERE source_id = ?1
                  AND name = ?2

                UNION ALL

                SELECT
                    player_id,
                    1
                FROM player_aliases
                WHERE source_id IS NULL
                  AND name = ?2
            ),
            best_priority(priority) AS (
                SELECT MIN(priority)
                FROM candidates
            )
            SELECT DISTINCT player_id
            FROM candidates
            WHERE priority = (
                SELECT priority
                FROM best_priority
            )
            ORDER BY player_id
            LIMIT 2
            "#,
        )
        .context("preparing exact player-alias lookup")?;

    let rows = statement
        .query_map(params![source_id, name], |row| row.get::<_, i64>(0))
        .with_context(|| format!("resolving exact player alias {name:?} for source {source_id}"))?;

    let player_ids = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collecting exact player-alias candidates")?;

    match player_ids.as_slice() {
        [] => Ok(PlayerAliasResolution::Unrecognised),
        [player_id] => Ok(PlayerAliasResolution::Unique(*player_id)),
        [_, _] => Ok(PlayerAliasResolution::Ambiguous),
        _ => unreachable!("alias query is limited to two rows"),
    }
}

fn get_alias_from_connection(connection: &Connection, alias_id: i64) -> Result<PlayerAlias> {
    connection
        .query_row(
            r#"
            SELECT
                pa.id,
                pa.player_id,
                pa.name,
                pa.source_id,
                s.name,
                s.version,
                pa.notes
            FROM player_aliases AS pa
            LEFT JOIN sources AS s
                ON s.id = pa.source_id
            WHERE pa.id = ?1
            "#,
            [alias_id],
            player_alias_from_row,
        )
        .optional()
        .context("reading player alias")?
        .with_context(|| format!("player alias {alias_id} does not exist"))
}

fn apply_source_alias_resolution_on(
    connection: &Connection,
    alias_id: i64,
) -> Result<SourceAliasResolutionResult> {
    let preview = preview_source_alias_resolution_on(connection, alias_id)?;

    if preview.competing_alias_count != 0 {
        bail!(
            "source alias {alias_id} has {} competing alias assignment(s)",
            preview.competing_alias_count
        );
    }

    if preview.conflicting_link_count != 0 {
        bail!(
            "source alias {alias_id} has {} occurrence(s) already linked \
             to another player",
            preview.conflicting_link_count
        );
    }

    let linked_black = connection
        .execute(
            r#"
            UPDATE game_metadata
            SET black_player_id = ?1,
                black_player_catalogue_derived = 0
            WHERE (
                    black_player_id IS NULL
                    OR black_player_catalogue_derived = 1
                  )
              AND black_player = ?2
              AND EXISTS (
                  SELECT 1
                  FROM game_sources AS gs
                  WHERE gs.id = game_metadata.game_source_id
                    AND gs.source_id = ?3
              )
            "#,
            params![preview.player_id, &preview.name, preview.source_id],
        )
        .context("linking unresolved Black-player source aliases")?;

    let linked_white = connection
        .execute(
            r#"
            UPDATE game_metadata
            SET white_player_id = ?1,
                white_player_catalogue_derived = 0
            WHERE (
                    white_player_id IS NULL
                    OR white_player_catalogue_derived = 1
                  )
              AND white_player = ?2
              AND EXISTS (
                  SELECT 1
                  FROM game_sources AS gs
                  WHERE gs.id = game_metadata.game_source_id
                    AND gs.source_id = ?3
              )
            "#,
            params![preview.player_id, &preview.name, preview.source_id],
        )
        .context("linking unresolved White-player source aliases")?;

    let linked_black_count =
        u64::try_from(linked_black).context("oversized Black-player update count")?;

    let linked_white_count =
        u64::try_from(linked_white).context("oversized White-player update count")?;

    /*
     * The preview and updates occur under the caller's transaction. A mismatch
     * is an invariant failure and must roll the whole operation back.
     */
    let expected_black_count = preview.unresolved_black_count + preview.catalogue_black_count;
    let expected_white_count = preview.unresolved_white_count + preview.catalogue_white_count;

    if linked_black_count != expected_black_count || linked_white_count != expected_white_count {
        bail!(
            "source alias {alias_id} changed unexpectedly during resolution: \
             preview expected {} Black and {} White assignable occurrence(s), \
             update found {} Black and {} White",
            expected_black_count,
            expected_white_count,
            linked_black_count,
            linked_white_count
        );
    }

    Ok(SourceAliasResolutionResult {
        alias_id: preview.alias_id,
        player_id: preview.player_id,
        source_id: preview.source_id,
        source_name: preview.source_name,
        source_version: preview.source_version,
        name: preview.name,
        linked_black_count,
        linked_white_count,
    })
}

fn preview_source_alias_resolution_on(
    connection: &Connection,
    alias_id: i64,
) -> Result<SourceAliasResolutionPreview> {
    let alias = get_alias_from_connection(connection, alias_id)?;

    let Some(source_id) = alias.source_id else {
        bail!(
            "player alias {alias_id} is global; \
             source-alias resolution requires a source-specific alias"
        );
    };

    let source_name = alias
        .source_name
        .clone()
        .with_context(|| format!("source {source_id} has no name"))?;

    let source_version = alias
        .source_version
        .clone()
        .with_context(|| format!("source {source_id} has no version"))?;

    /*
     * This helper performs the same read-only safety inspection for both
     * ordinary previews and the transaction immediately before a bulk write.
     */
    let (
        unresolved_black_count,
        unresolved_white_count,
        catalogue_black_count,
        catalogue_white_count,
        already_linked_count,
        conflicting_link_count,
    ): (i64, i64, i64, i64, i64, i64) = connection
        .query_row(
            r#"
            WITH occurrences(
                side,
                raw_name,
                linked_player_id,
                catalogue_derived
            ) AS (
                SELECT
                    0,
                    gm.black_player,
                    gm.black_player_id,
                    gm.black_player_catalogue_derived
                FROM game_metadata AS gm
                JOIN game_sources AS gs
                    ON gs.id = gm.game_source_id
                WHERE gs.source_id = ?1

                UNION ALL

                SELECT
                    1,
                    gm.white_player,
                    gm.white_player_id,
                    gm.white_player_catalogue_derived
                FROM game_metadata AS gm
                JOIN game_sources AS gs
                    ON gs.id = gm.game_source_id
                WHERE gs.source_id = ?1
            )
            SELECT
                COALESCE(SUM(
                    CASE
                        WHEN side = 0
                         AND linked_player_id IS NULL
                        THEN 1
                        ELSE 0
                    END
                ), 0),
                COALESCE(SUM(
                    CASE
                        WHEN side = 1
                         AND linked_player_id IS NULL
                        THEN 1
                        ELSE 0
                    END
                ), 0),
                COALESCE(SUM(
                    CASE
                        WHEN side = 0
                         AND linked_player_id IS NOT NULL
                         AND catalogue_derived = 1
                        THEN 1
                        ELSE 0
                    END
                ), 0),
                COALESCE(SUM(
                    CASE
                        WHEN side = 1
                         AND linked_player_id IS NOT NULL
                         AND catalogue_derived = 1
                        THEN 1
                        ELSE 0
                    END
                ), 0),
                COALESCE(SUM(
                    CASE
                        WHEN linked_player_id = ?2
                         AND catalogue_derived = 0
                        THEN 1
                        ELSE 0
                    END
                ), 0),
                COALESCE(SUM(
                    CASE
                        WHEN linked_player_id IS NOT NULL
                         AND linked_player_id <> ?2
                         AND catalogue_derived = 0
                        THEN 1
                        ELSE 0
                    END
                ), 0)
            FROM occurrences
            WHERE raw_name = ?3
            "#,
            params![source_id, alias.player_id, &alias.name],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .with_context(|| format!("previewing source alias {alias_id} for source {source_id}"))?;

    let competing_alias_count: i64 = connection
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM player_aliases
            WHERE source_id = ?1
              AND name = ?2
              AND player_id <> ?3
            "#,
            params![source_id, &alias.name, alias.player_id],
            |row| row.get(0),
        )
        .with_context(|| format!("checking competing assignments for source alias {alias_id}"))?;

    Ok(SourceAliasResolutionPreview {
        alias_id,
        player_id: alias.player_id,
        source_id,
        source_name,
        source_version,
        name: alias.name,
        unresolved_black_count: u64::try_from(unresolved_black_count)
            .context("negative unresolved Black-player count")?,
        unresolved_white_count: u64::try_from(unresolved_white_count)
            .context("negative unresolved White-player count")?,
        catalogue_black_count: u64::try_from(catalogue_black_count)
            .context("negative catalogue-derived Black-player count")?,
        catalogue_white_count: u64::try_from(catalogue_white_count)
            .context("negative catalogue-derived White-player count")?,
        already_linked_count: u64::try_from(already_linked_count)
            .context("negative already-linked player count")?,
        conflicting_link_count: u64::try_from(conflicting_link_count)
            .context("negative conflicting player-link count")?,
        competing_alias_count: u64::try_from(competing_alias_count)
            .context("negative competing-alias count")?,
    })
}

fn player_alias_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlayerAlias> {
    Ok(PlayerAlias {
        id: row.get(0)?,
        player_id: row.get(1)?,
        name: row.get(2)?,
        source_id: row.get(3)?,
        source_name: row.get(4)?,
        source_version: row.get(5)?,
        notes: row.get(6)?,
    })
}

fn required_text<'a>(value: &'a str, description: &str) -> Result<&'a str> {
    let value = value.trim();

    if value.is_empty() {
        bail!("{description} must not be empty");
    }

    Ok(value)
}

fn optional_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::project_manager::ProjectManager;
    use tempfile::tempdir;

    fn add_source_metadata(project: &Project) -> Result<()> {
        let connection = database::open(&project.database_root())?;

        connection.execute_batch(
            r#"
            INSERT INTO sources(id, name, version)
            VALUES (1, 'GoGoD', '2026');

            INSERT INTO games(
                id,
                canonical_hash,
                board_size,
                move_count,
                move_file
            )
            VALUES (
                1,
                X'01',
                19,
                200,
                'games/01.moves'
            );

            INSERT INTO game_sources(
                id,
                game_id,
                source_id,
                original_path
            )
            VALUES (
                10,
                1,
                1,
                'gogod/test.sgf'
            );

            INSERT INTO game_metadata(
                game_source_id,
                black_player,
                white_player
            )
            VALUES (
                10,
                'Cho Chikun',
                'Kobayashi Satoru'
            );
            "#,
        )?;

        Ok(())
    }

    #[test]
    fn creates_renames_and_lists_players() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        let directory = project.player_directory()?;

        let player = directory.create_player("  Cho Chikun  ")?;

        assert_eq!(player.preferred_name, "Cho Chikun");

        directory.rename_player(player.id, "Cho Chikun 9p")?;

        assert_eq!(
            directory.list_players()?,
            vec![PlayerIdentity {
                id: player.id,
                preferred_name: "Cho Chikun 9p".to_owned(),
            }]
        );

        assert!(directory.create_player("   ").is_err());
        assert!(directory.rename_player(player.id, "").is_err());
        assert!(directory.rename_player(999_999, "Nobody").is_err());

        Ok(())
    }

    #[test]
    fn search_names_are_case_insensitive() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;
        let directory = project.player_directory()?;

        let player = directory.create_player("Lee Sedol")?;

        directory.add_alias(
            player.id,
            "Yi Se-tol",
            None,
            Some("case-insensitive search regression test"),
        )?;

        assert_eq!(
            directory.player_ids_for_search_name("lee sedol")?,
            vec![player.id]
        );

        assert_eq!(
            directory.player_ids_for_search_name("yi se-TOL")?,
            vec![player.id]
        );

        Ok(())
    }

    #[test]
    fn aliases_are_explicit_and_can_represent_ambiguous_names() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        {
            let connection = database::open(&project.database_root())?;

            connection.execute(
                "INSERT INTO sources(id, name, version) VALUES (1, 'GoGoD', '2026')",
                [],
            )?;
        }

        let directory = project.player_directory()?;

        let first = directory.create_player("First Lee")?;
        let second = directory.create_player("Second Lee")?;

        let first_alias =
            directory.add_alias(first.id, "Lee", Some(1), Some("confirmed from source"))?;

        let second_alias = directory.add_alias(second.id, "Lee", Some(1), None)?;

        assert_eq!(first_alias.name, "Lee");
        assert_eq!(first_alias.source_name.as_deref(), Some("GoGoD"));
        assert_eq!(first_alias.source_version.as_deref(), Some("2026"));
        assert_eq!(first_alias.notes.as_deref(), Some("confirmed from source"));

        /*
         * The same literal source spelling may genuinely be ambiguous.
         * Bermuda therefore allows it to be attached to two identities.
         */
        assert_eq!(second_alias.name, "Lee");

        /*
         * Repeating the identical assignment to the same identity is not
         * useful and is blocked by the schema's partial UNIQUE index.
         */
        assert!(directory.add_alias(first.id, "Lee", Some(1), None).is_err());

        assert_eq!(directory.aliases_for_player(first.id)?.len(), 1);

        directory.remove_alias(first_alias.id)?;

        assert!(directory.aliases_for_player(first.id)?.is_empty());
        assert!(directory.remove_alias(first_alias.id).is_err());

        Ok(())
    }

    #[test]
    fn creates_player_and_assigns_source_name_in_one_transaction() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        add_source_metadata(&project)?;

        let directory = project.player_directory()?;

        let (player, result) =
            directory.create_player_and_assign_source_name("  Cho Chikun  ", 1, "Cho Chikun")?;

        assert_eq!(player.preferred_name, "Cho Chikun");
        assert_eq!(result.player_id, player.id);
        assert_eq!(result.source_id, 1);
        assert_eq!(result.name, "Cho Chikun");
        assert_eq!(result.linked_count(), 1);

        assert_eq!(
            directory.list_players()?,
            vec![PlayerIdentity {
                id: player.id,
                preferred_name: "Cho Chikun".to_owned(),
            }]
        );

        let aliases = directory.aliases_for_player(player.id)?;

        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name, "Cho Chikun");
        assert_eq!(aliases[0].source_id, Some(1));

        let connection = database::open(&project.database_root())?;

        let (raw_name, linked_id): (String, Option<i64>) = connection.query_row(
            r#"
                SELECT black_player, black_player_id
                FROM game_metadata
                WHERE game_source_id = 10
                "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        /*
         * The source text remains exactly as imported.
         */
        assert_eq!(raw_name, "Cho Chikun");
        assert_eq!(linked_id, Some(player.id));

        Ok(())
    }

    #[test]
    fn new_player_is_rolled_back_when_source_assignment_is_unsafe() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        add_source_metadata(&project)?;

        let directory = project.player_directory()?;

        let existing = directory.create_player("Existing Cho")?;

        directory.add_alias(
            existing.id,
            "Cho Chikun",
            Some(1),
            Some("existing explicit assignment"),
        )?;

        let error = directory
            .create_player_and_assign_source_name("New Cho", 1, "Cho Chikun")
            .expect_err("competing source alias must refuse new-player assignment");

        assert!(error.to_string().contains("competing"));

        /*
         * The attempted new identity was created inside the failed
         * transaction and therefore must not survive.
         */
        assert_eq!(
            directory.list_players()?,
            vec![PlayerIdentity {
                id: existing.id,
                preferred_name: "Existing Cho".to_owned(),
            }]
        );

        assert_eq!(directory.aliases_for_player(existing.id)?.len(), 1);

        let connection = database::open(&project.database_root())?;

        let linked_id: Option<i64> = connection.query_row(
            r#"
            SELECT black_player_id
            FROM game_metadata
            WHERE game_source_id = 10
            "#,
            [],
            |row| row.get(0),
        )?;

        assert_eq!(linked_id, None);

        Ok(())
    }

    #[test]
    fn assigns_source_name_atomically_and_reuses_existing_alias() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        add_source_metadata(&project)?;

        let directory = project.player_directory()?;
        let player = directory.create_player("Cho Chikun")?;

        let result = directory.assign_source_name_to_player(player.id, 1, "Cho Chikun")?;

        assert_eq!(result.player_id, player.id);
        assert_eq!(result.source_id, 1);
        assert_eq!(result.name, "Cho Chikun");
        assert_eq!(result.linked_black_count, 1);
        assert_eq!(result.linked_white_count, 0);

        let aliases = directory.aliases_for_player(player.id)?;

        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name, "Cho Chikun");
        assert_eq!(aliases[0].source_id, Some(1));

        /*
         * Repeating the same explicit assignment reuses the existing alias.
         * The game is already linked, so there is nothing left to update.
         */
        let repeated = directory.assign_source_name_to_player(player.id, 1, "Cho Chikun")?;

        assert_eq!(repeated.alias_id, result.alias_id);
        assert_eq!(repeated.linked_count(), 0);
        assert_eq!(directory.aliases_for_player(player.id)?.len(), 1);

        let connection = database::open(&project.database_root())?;

        let (raw_name, linked_id): (String, Option<i64>) = connection.query_row(
            r#"
            SELECT black_player, black_player_id
            FROM game_metadata
            WHERE game_source_id = 10
            "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        assert_eq!(raw_name, "Cho Chikun");
        assert_eq!(linked_id, Some(player.id));

        Ok(())
    }

    #[test]
    fn source_name_assignment_rolls_back_new_alias_when_resolution_is_unsafe() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        add_source_metadata(&project)?;

        let directory = project.player_directory()?;

        let intended = directory.create_player("Intended Cho")?;
        let competing = directory.create_player("Competing Cho")?;

        directory.add_alias(
            competing.id,
            "Cho Chikun",
            Some(1),
            Some("deliberately competing assignment"),
        )?;

        let error = directory
            .assign_source_name_to_player(intended.id, 1, "Cho Chikun")
            .expect_err("competing source alias must refuse assignment");

        assert!(error.to_string().contains("competing"));

        /*
         * assign_source_name_to_player inserted the intended alias inside its
         * transaction. Refusal must therefore roll that insertion back too.
         */
        assert!(directory.aliases_for_player(intended.id)?.is_empty());

        let connection = database::open(&project.database_root())?;

        let linked_id: Option<i64> = connection.query_row(
            r#"
            SELECT black_player_id
            FROM game_metadata
            WHERE game_source_id = 10
            "#,
            [],
            |row| row.get(0),
        )?;

        assert_eq!(linked_id, None);

        Ok(())
    }

    #[test]
    fn removing_source_alias_restores_metadata_without_changing_source_name() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;
        add_source_metadata(&project)?;

        let directory = project.player_directory()?;
        let player = directory.create_player("Cho Chikun")?;

        let result = directory.assign_source_name_to_player(player.id, 1, "Cho Chikun")?;

        {
            let connection = database::open(&project.database_root())?;

            let (raw_name, linked_id): (String, Option<i64>) = connection.query_row(
                r#"
                    SELECT black_player, black_player_id
                    FROM game_metadata
                    WHERE game_source_id = 10
                    "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            assert_eq!(raw_name, "Cho Chikun");
            assert_eq!(linked_id, Some(player.id));
        }

        directory.remove_alias(result.alias_id)?;

        {
            let connection = database::open(&project.database_root())?;

            let (raw_name, linked_id): (String, Option<i64>) = connection.query_row(
                r#"
                    SELECT black_player, black_player_id
                    FROM game_metadata
                    WHERE game_source_id = 10
                    "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            /*
             * Removing the alias removes Bermuda's interpretation only.
             * The source spelling imported from the SGF survives unchanged.
             */
            assert_eq!(raw_name, "Cho Chikun");
            assert_eq!(linked_id, None);
        }

        assert!(directory.aliases_for_player(player.id)?.is_empty());

        assert!(
            directory
                .unresolved_names()?
                .iter()
                .any(|name| name.source_id == 1 && name.name == "Cho Chikun")
        );

        assert!(directory.remove_alias(result.alias_id).is_err());

        Ok(())
    }

    #[test]
    fn deleting_player_identity_restores_metadata_without_changing_source_name() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;
        add_source_metadata(&project)?;

        let directory = project.player_directory()?;

        let (player, _) =
            directory.create_player_and_assign_source_name("Cho Chikun", 1, "Cho Chikun")?;

        let removed_name = directory.delete_player(player.id)?;

        assert_eq!(removed_name, "Cho Chikun");
        assert!(directory.list_players()?.is_empty());

        {
            let connection = database::open(&project.database_root())?;

            let (raw_name, linked_id): (String, Option<i64>) = connection.query_row(
                r#"
                    SELECT black_player, black_player_id
                    FROM game_metadata
                    WHERE game_source_id = 10
                    "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            assert_eq!(raw_name, "Cho Chikun");
            assert_eq!(linked_id, None);

            let alias_count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM player_aliases WHERE player_id = ?1",
                [player.id],
                |row| row.get(0),
            )?;

            assert_eq!(alias_count, 0);
        }

        assert!(
            directory
                .unresolved_names()?
                .iter()
                .any(|name| name.source_id == 1 && name.name == "Cho Chikun")
        );

        assert!(directory.delete_player(player.id).is_err());

        Ok(())
    }

    #[test]
    fn previews_source_alias_resolution_without_changing_metadata() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        add_source_metadata(&project)?;

        let directory = project.player_directory()?;

        let player = directory.create_player("Cho Chikun")?;

        let alias = directory.add_alias(
            player.id,
            "Cho Chikun",
            Some(1),
            Some("confirmed GoGoD spelling"),
        )?;

        let preview = directory.preview_source_alias_resolution(alias.id)?;

        assert_eq!(
            preview,
            SourceAliasResolutionPreview {
                alias_id: alias.id,
                player_id: player.id,
                source_id: 1,
                source_name: "GoGoD".to_owned(),
                source_version: "2026".to_owned(),
                name: "Cho Chikun".to_owned(),
                unresolved_black_count: 1,
                unresolved_white_count: 0,
                catalogue_black_count: 0,
                catalogue_white_count: 0,
                already_linked_count: 0,
                conflicting_link_count: 0,
                competing_alias_count: 0,
            }
        );

        assert_eq!(preview.unresolved_count(), 1);

        /*
         * Previewing must not itself assign an identity.
         */
        {
            let connection = database::open(&project.database_root())?;

            let (raw_name, player_id): (String, Option<i64>) = connection.query_row(
                r#"
                    SELECT black_player, black_player_id
                    FROM game_metadata
                    WHERE game_source_id = 10
                    "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            assert_eq!(raw_name, "Cho Chikun");
            assert_eq!(player_id, None);
        }

        directory.link_source_player(10, PlayerSide::Black, player.id)?;

        let preview = directory.preview_source_alias_resolution(alias.id)?;

        assert_eq!(preview.unresolved_count(), 0);
        assert_eq!(preview.already_linked_count, 1);
        assert_eq!(preview.conflicting_link_count, 0);

        let other_player = directory.create_player("Different Cho Chikun")?;

        directory.link_source_player(10, PlayerSide::Black, other_player.id)?;

        let preview = directory.preview_source_alias_resolution(alias.id)?;

        assert_eq!(preview.already_linked_count, 0);
        assert_eq!(preview.conflicting_link_count, 1);

        directory.add_alias(
            other_player.id,
            "Cho Chikun",
            Some(1),
            Some("deliberately ambiguous test assignment"),
        )?;

        let preview = directory.preview_source_alias_resolution(alias.id)?;

        assert_eq!(preview.competing_alias_count, 1);

        let global_alias = directory.add_alias(player.id, "Global Cho", None, None)?;

        let error = directory
            .preview_source_alias_resolution(global_alias.id)
            .expect_err("global aliases must not use source-specific preview");

        assert!(error.to_string().contains("global"));

        Ok(())
    }

    #[test]
    fn applies_source_alias_resolution_only_when_safe() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        add_source_metadata(&project)?;

        /*
         * Add the same raw spelling in another source. Resolving the GoGoD
         * alias must not touch it.
         */
        {
            let connection = database::open(&project.database_root())?;

            connection.execute_batch(
                r#"
                INSERT INTO sources(id, name, version)
                VALUES (2, 'Other Source', '1');

                INSERT INTO games(
                    id,
                    canonical_hash,
                    board_size,
                    move_count,
                    move_file
                )
                VALUES (
                    2,
                    X'02',
                    19,
                    150,
                    'games/02.moves'
                );

                INSERT INTO game_sources(
                    id,
                    game_id,
                    source_id,
                    original_path
                )
                VALUES (
                    20,
                    2,
                    2,
                    'other/test.sgf'
                );

                INSERT INTO game_metadata(
                    game_source_id,
                    black_player,
                    white_player
                )
                VALUES (
                    20,
                    'Cho Chikun',
                    'Someone Else'
                );
                "#,
            )?;
        }

        let directory = project.player_directory()?;

        let player = directory.create_player("Cho Chikun")?;

        let alias = directory.add_alias(
            player.id,
            "Cho Chikun",
            Some(1),
            Some("confirmed GoGoD spelling"),
        )?;

        let result = directory.apply_source_alias_resolution(alias.id)?;

        assert_eq!(
            result,
            SourceAliasResolutionResult {
                alias_id: alias.id,
                player_id: player.id,
                source_id: 1,
                source_name: "GoGoD".to_owned(),
                source_version: "2026".to_owned(),
                name: "Cho Chikun".to_owned(),
                linked_black_count: 1,
                linked_white_count: 0,
            }
        );

        assert_eq!(result.linked_count(), 1);

        {
            let connection = database::open(&project.database_root())?;

            let (raw_name, linked_id): (String, Option<i64>) = connection.query_row(
                r#"
                    SELECT black_player, black_player_id
                    FROM game_metadata
                    WHERE game_source_id = 10
                    "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            /*
             * Source text is still exactly what was imported.
             */
            assert_eq!(raw_name, "Cho Chikun");
            assert_eq!(linked_id, Some(player.id));

            /*
             * The identical spelling belonging to another source was not
             * touched by this source-specific alias.
             */
            let other_source_id: Option<i64> = connection.query_row(
                r#"
                SELECT black_player_id
                FROM game_metadata
                WHERE game_source_id = 20
                "#,
                [],
                |row| row.get(0),
            )?;

            assert_eq!(other_source_id, None);
        }

        /*
         * Re-applying an already resolved alias is harmless.
         */
        let result = directory.apply_source_alias_resolution(alias.id)?;

        assert_eq!(result.linked_count(), 0);

        /*
         * An existing link to a different identity blocks the whole bulk
         * operation.
         */
        directory.unlink_source_player(10, PlayerSide::Black)?;

        let other_player = directory.create_player("Different Cho Chikun")?;

        directory.link_source_player(10, PlayerSide::Black, other_player.id)?;

        let error = directory
            .apply_source_alias_resolution(alias.id)
            .expect_err("conflicting existing links must block bulk resolution");

        assert!(error.to_string().contains("another player"));

        /*
         * A competing alias assignment also blocks the operation.
         */
        directory.unlink_source_player(10, PlayerSide::Black)?;

        let competing_alias = directory.add_alias(
            other_player.id,
            "Cho Chikun",
            Some(1),
            Some("deliberately ambiguous test assignment"),
        )?;

        let error = directory
            .apply_source_alias_resolution(alias.id)
            .expect_err("competing aliases must block bulk resolution");

        assert!(error.to_string().contains("competing"));

        {
            let connection = database::open(&project.database_root())?;

            let linked_id: Option<i64> = connection.query_row(
                r#"
                SELECT black_player_id
                FROM game_metadata
                WHERE game_source_id = 10
                "#,
                [],
                |row| row.get(0),
            )?;

            assert_eq!(linked_id, None);
        }

        directory.remove_alias(competing_alias.id)?;

        let result = directory.apply_source_alias_resolution(alias.id)?;

        assert_eq!(result.linked_count(), 1);

        Ok(())
    }

    #[test]
    fn local_source_alias_overrides_catalogue_but_not_local_assignment() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;
        add_source_metadata(&project)?;

        let directory = project.player_directory()?;
        let local_player = directory.create_player("Local Cho")?;

        let alias = directory.add_alias(
            local_player.id,
            "Cho Chikun",
            Some(1),
            Some("explicit local source knowledge"),
        )?;

        let catalogue_player_id = {
            let connection = database::open(&project.database_root())?;

            connection.execute(
                r#"
                INSERT INTO players(preferred_name, catalogue_key)
                VALUES ('Catalogue Cho', 'test:catalogue-cho')
                "#,
                [],
            )?;

            let catalogue_player_id = connection.last_insert_rowid();

            connection.execute(
                r#"
                UPDATE game_metadata
                SET black_player_id = ?1,
                    black_player_catalogue_derived = 1
                WHERE game_source_id = 10
                "#,
                [catalogue_player_id],
            )?;

            catalogue_player_id
        };

        assert_ne!(catalogue_player_id, local_player.id);

        /*
         * A catalogue assignment is not a conflict with deliberate local
         * source-specific knowledge. It is explicitly replaceable.
         */
        let preview = directory.preview_source_alias_resolution(alias.id)?;

        assert_eq!(preview.unresolved_black_count, 0);
        assert_eq!(preview.unresolved_white_count, 0);
        assert_eq!(preview.catalogue_black_count, 1);
        assert_eq!(preview.catalogue_white_count, 0);
        assert_eq!(preview.catalogue_count(), 1);
        assert_eq!(preview.assignable_count(), 1);
        assert_eq!(preview.already_linked_count, 0);
        assert_eq!(preview.conflicting_link_count, 0);

        let result = directory.apply_source_alias_resolution(alias.id)?;

        assert_eq!(result.linked_black_count, 1);
        assert_eq!(result.linked_white_count, 0);

        {
            let connection = database::open(&project.database_root())?;

            let (raw_name, linked_player_id, catalogue_derived): (String, Option<i64>, i64) =
                connection.query_row(
                    r#"
                SELECT
                    black_player,
                    black_player_id,
                    black_player_catalogue_derived
                FROM game_metadata
                WHERE game_source_id = 10
                "#,
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;

            assert_eq!(raw_name, "Cho Chikun");
            assert_eq!(linked_player_id, Some(local_player.id));
            assert_eq!(catalogue_derived, 0);
        }

        /*
         * Once another user/local identity owns the row, the same alias must
         * again see a genuine conflict. Local knowledge never silently
         * overwrites other local knowledge.
         */
        let other_local_player = directory.create_player("Other Local Cho")?;

        directory.link_source_player(10, PlayerSide::Black, other_local_player.id)?;

        let preview = directory.preview_source_alias_resolution(alias.id)?;

        assert_eq!(preview.catalogue_count(), 0);
        assert_eq!(preview.assignable_count(), 0);
        assert_eq!(preview.already_linked_count, 0);
        assert_eq!(preview.conflicting_link_count, 1);

        let error = directory
            .apply_source_alias_resolution(alias.id)
            .expect_err("different local assignment must remain a conflict");

        assert!(error.to_string().contains("another player"));

        {
            let connection = database::open(&project.database_root())?;

            let (linked_player_id, catalogue_derived): (Option<i64>, i64) = connection.query_row(
                r#"
                    SELECT
                        black_player_id,
                        black_player_catalogue_derived
                    FROM game_metadata
                    WHERE game_source_id = 10
                    "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            assert_eq!(linked_player_id, Some(other_local_player.id));
            assert_eq!(catalogue_derived, 0);
        }

        Ok(())
    }

    #[test]
    fn explicit_player_assignment_becomes_local_provenance() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        add_source_metadata(&project)?;

        let supplied_player_id = {
            let connection = database::open(&project.database_root())?;

            connection.execute(
                r#"
                INSERT INTO players(preferred_name, catalogue_key)
                VALUES ('Supplied Cho', 'test:cho')
                "#,
                [],
            )?;

            let supplied_player_id = connection.last_insert_rowid();

            connection.execute(
                r#"
                UPDATE game_metadata
                SET black_player_id = ?1,
                    black_player_catalogue_derived = 1
                WHERE game_source_id = 10
                "#,
                [supplied_player_id],
            )?;

            supplied_player_id
        };

        let directory = project.player_directory()?;
        let local_player = directory.create_player("Local Cho")?;

        assert_ne!(supplied_player_id, local_player.id);

        /*
         * An explicit user assignment overrides the catalogue interpretation
         * and therefore becomes local provenance.
         */
        let source_name = directory.link_source_player(10, PlayerSide::Black, local_player.id)?;

        assert_eq!(source_name, "Cho Chikun");

        {
            let connection = database::open(&project.database_root())?;

            let (raw_name, player_id, catalogue_derived): (String, Option<i64>, i64) = connection
                .query_row(
                r#"
                SELECT
                    black_player,
                    black_player_id,
                    black_player_catalogue_derived
                FROM game_metadata
                WHERE game_source_id = 10
                "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;

            assert_eq!(raw_name, "Cho Chikun");
            assert_eq!(player_id, Some(local_player.id));
            assert_eq!(catalogue_derived, 0);
        }

        /*
         * Seed catalogue provenance again so unlink proves that it clears
         * both the identity and its provenance flag.
         */
        {
            let connection = database::open(&project.database_root())?;

            connection.execute(
                r#"
                UPDATE game_metadata
                SET black_player_catalogue_derived = 1
                WHERE game_source_id = 10
                "#,
                [],
            )?;
        }

        directory.unlink_source_player(10, PlayerSide::Black)?;

        {
            let connection = database::open(&project.database_root())?;

            let (raw_name, player_id, catalogue_derived): (String, Option<i64>, i64) = connection
                .query_row(
                r#"
                SELECT
                    black_player,
                    black_player_id,
                    black_player_catalogue_derived
                FROM game_metadata
                WHERE game_source_id = 10
                "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;

            assert_eq!(raw_name, "Cho Chikun");
            assert_eq!(player_id, None);
            assert_eq!(catalogue_derived, 0);
        }

        Ok(())
    }

    #[test]
    fn links_and_unlinks_identity_without_changing_source_name() -> Result<()> {
        let temporary_directory = tempdir()?;
        let project_root = temporary_directory.path().join("test-project");

        let project = ProjectManager::new().create("Test Project", &project_root)?;

        add_source_metadata(&project)?;

        let directory = project.player_directory()?;

        assert_eq!(
            directory.unresolved_names()?,
            vec![
                UnresolvedPlayerName {
                    source_id: 1,
                    source_name: "GoGoD".to_owned(),
                    source_version: "2026".to_owned(),
                    name: "Cho Chikun".to_owned(),
                    occurrence_count: 1,
                },
                UnresolvedPlayerName {
                    source_id: 1,
                    source_name: "GoGoD".to_owned(),
                    source_version: "2026".to_owned(),
                    name: "Kobayashi Satoru".to_owned(),
                    occurrence_count: 1,
                },
            ]
        );

        let player = directory.create_player("Cho Chikun")?;

        let source_name = directory.link_source_player(10, PlayerSide::Black, player.id)?;

        assert_eq!(source_name, "Cho Chikun");

        {
            let connection = database::open(&project.database_root())?;

            let (black_player, black_player_id): (String, Option<i64>) = connection.query_row(
                r#"
                    SELECT black_player, black_player_id
                    FROM game_metadata
                    WHERE game_source_id = 10
                    "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            /*
             * The SGF/source spelling remains untouched. The numeric ID is
             * Bermuda's interpretation alongside it.
             */
            assert_eq!(black_player, "Cho Chikun");
            assert_eq!(black_player_id, Some(player.id));
        }

        assert_eq!(
            directory.unresolved_names()?,
            vec![UnresolvedPlayerName {
                source_id: 1,
                source_name: "GoGoD".to_owned(),
                source_version: "2026".to_owned(),
                name: "Kobayashi Satoru".to_owned(),
                occurrence_count: 1,
            }]
        );

        directory.unlink_source_player(10, PlayerSide::Black)?;

        assert_eq!(directory.unresolved_names()?.len(), 2);

        Ok(())
    }
}
