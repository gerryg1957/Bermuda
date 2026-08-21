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
    pub already_linked_count: u64,
    pub conflicting_link_count: u64,
    pub competing_alias_count: u64,
}

impl SourceAliasResolutionPreview {
    pub fn unresolved_count(&self) -> u64 {
        self.unresolved_black_count + self.unresolved_white_count
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

    pub fn remove_alias(&self, alias_id: i64) -> Result<()> {
        let changed = self
            .connection
            .execute("DELETE FROM player_aliases WHERE id = ?1", [alias_id])
            .with_context(|| format!("removing player alias {alias_id}"))?;

        if changed == 0 {
            bail!("player alias {alias_id} does not exist");
        }

        Ok(())
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

    pub fn preview_source_alias_resolution(
        &self,
        alias_id: i64,
    ) -> Result<SourceAliasResolutionPreview> {
        let alias = self.get_alias(alias_id)?;

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
         * This is deliberately a read-only preview.
         *
         * Exact source spellings are inspected without rewriting PB/PW or
         * assigning any player IDs. Existing assignments are reported
         * separately so a later explicit bulk operation can refuse unsafe
         * or ambiguous changes.
         */
        let (
            unresolved_black_count,
            unresolved_white_count,
            already_linked_count,
            conflicting_link_count,
        ): (i64, i64, i64, i64) = self
            .connection
            .query_row(
                r#"
                WITH occurrences(side, raw_name, linked_player_id) AS (
                    SELECT
                        0,
                        gm.black_player,
                        gm.black_player_id
                    FROM game_metadata AS gm
                    JOIN game_sources AS gs
                        ON gs.id = gm.game_source_id
                    WHERE gs.source_id = ?1

                    UNION ALL

                    SELECT
                        1,
                        gm.white_player,
                        gm.white_player_id
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
                            WHEN linked_player_id = ?2
                            THEN 1
                            ELSE 0
                        END
                    ), 0),
                    COALESCE(SUM(
                        CASE
                            WHEN linked_player_id IS NOT NULL
                             AND linked_player_id <> ?2
                            THEN 1
                            ELSE 0
                        END
                    ), 0)
                FROM occurrences
                WHERE raw_name = ?3
                "#,
                params![source_id, alias.player_id, &alias.name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .with_context(|| {
                format!("previewing source alias {alias_id} for source {source_id}")
            })?;

        let competing_alias_count: i64 = self
            .connection
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
            .with_context(|| {
                format!("checking competing assignments for source alias {alias_id}")
            })?;

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
            already_linked_count: u64::try_from(already_linked_count)
                .context("negative already-linked player count")?,
            conflicting_link_count: u64::try_from(conflicting_link_count)
                .context("negative conflicting player-link count")?,
            competing_alias_count: u64::try_from(competing_alias_count)
                .context("negative competing-alias count")?,
        })
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

        let id_column = match side {
            PlayerSide::Black => "black_player_id",
            PlayerSide::White => "white_player_id",
        };

        let sql = format!("UPDATE game_metadata SET {id_column} = ?1 WHERE game_source_id = ?2");

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

        let id_column = match side {
            PlayerSide::Black => "black_player_id",
            PlayerSide::White => "white_player_id",
        };

        let sql = format!("UPDATE game_metadata SET {id_column} = NULL WHERE game_source_id = ?1");

        self.connection
            .execute(&sql, [game_source_id])
            .with_context(|| {
                format!("unlinking {side:?} player for game source {game_source_id}")
            })?;

        Ok(())
    }

    fn get_alias(&self, alias_id: i64) -> Result<PlayerAlias> {
        self.connection
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
