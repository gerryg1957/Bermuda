use anyhow::{Context, Result};
use rusqlite::{Connection, params};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameColumn {
    BlackPlayer,
    WhitePlayer,
    Date,
    Result,
    Event,
}

impl GameColumn {
    pub const fn sql_expression(self) -> &'static str {
        match self {
            Self::BlackPlayer => "selected_metadata.black_player",
            Self::WhitePlayer => "selected_metadata.white_player",
            Self::Date => "selected_metadata.played_date",
            Self::Result => "selected_metadata.result",
            Self::Event => "selected_metadata.event",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub const fn sql_keyword(self) -> &'static str {
        match self {
            Self::Ascending => "ASC",
            Self::Descending => "DESC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortField {
    pub column: GameColumn,
    pub direction: SortDirection,
}

impl SortField {
    pub const fn ascending(column: GameColumn) -> Self {
        Self {
            column,
            direction: SortDirection::Ascending,
        }
    }

    pub const fn descending(column: GameColumn) -> Self {
        Self {
            column,
            direction: SortDirection::Descending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerColour {
    Black,
    White,

    #[default]
    Either,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameResultFilter {
    #[default]
    Any,
    BlackWin,
    WhiteWin,
    Jigo,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameListRow {
    pub game_id: i64,
    pub black_player: Option<String>,
    pub white_player: Option<String>,
    pub game_date: Option<String>,
    pub result: Option<String>,
    pub event: Option<String>,
    pub matched_move: Option<u32>,
    pub match_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameListQuery {
    pub player: Option<String>,
    pub colour: PlayerColour,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub result: GameResultFilter,
    pub sort_fields: Vec<SortField>,
    pub offset: u32,
    pub limit: u32,
}

impl Default for GameListQuery {
    fn default() -> Self {
        Self {
            player: None,
            colour: PlayerColour::Either,
            date_from: None,
            date_to: None,
            result: GameResultFilter::Any,
            sort_fields: vec![
                SortField::descending(GameColumn::Date),
                SortField::ascending(GameColumn::BlackPlayer),
                SortField::ascending(GameColumn::WhitePlayer),
            ],
            offset: 0,
            limit: 200,
        }
    }
}

pub fn list_games(connection: &Connection, query: &GameListQuery) -> Result<Vec<GameListRow>> {
    let order_by = order_by_clause(query);
    let player_condition = match query.colour {
        PlayerColour::Black => "selected_metadata.black_player = ?3",
        PlayerColour::White => "selected_metadata.white_player = ?3",
        PlayerColour::Either => {
            "(selected_metadata.black_player = ?3 OR selected_metadata.white_player = ?3)"
        }
    };

    let sql = format!(
        r#"
        WITH ranked_metadata AS (
            SELECT
                game_sources.game_id,
                game_metadata.black_player,
                game_metadata.white_player,
                game_metadata.played_date,
                game_metadata.result,
                game_metadata.event,

                ROW_NUMBER() OVER (
                    PARTITION BY game_sources.game_id
                    ORDER BY
                        (
                            (game_metadata.black_player IS NOT NULL) +
                            (game_metadata.white_player IS NOT NULL) +
                            (game_metadata.played_date IS NOT NULL) +
                            (game_metadata.result IS NOT NULL) +
                            (game_metadata.event IS NOT NULL)
                        ) DESC,
                        game_sources.id ASC
                ) AS metadata_rank

            FROM game_sources

            LEFT JOIN game_metadata
                ON game_metadata.game_source_id = game_sources.id
        ),

        selected_metadata AS (
            SELECT
                game_id,
                black_player,
                white_player,
                played_date,
                result,
                event

            FROM ranked_metadata

            WHERE metadata_rank = 1
        )

        SELECT
            games.id,
            selected_metadata.black_player,
            selected_metadata.white_player,
            selected_metadata.played_date,
            selected_metadata.result,
            selected_metadata.event

        FROM games

        LEFT JOIN selected_metadata
    ON selected_metadata.game_id = games.id

WHERE (
    ?3 IS NULL
    OR {player_condition}
)

ORDER BY {order_by}

        LIMIT ?1
        OFFSET ?2
        "#
    );

    let mut statement = connection
        .prepare(&sql)
        .context("preparing game-list query")?;

    let rows = statement
        .query_map(
            params![
                i64::from(query.limit),
                i64::from(query.offset),
                query.player.as_deref(),
            ],
            |row| {
                Ok(GameListRow {
                    game_id: row.get(0)?,
                    black_player: row.get(1)?,
                    white_player: row.get(2)?,
                    game_date: row.get(3)?,
                    result: row.get(4)?,
                    event: row.get(5)?,
                    matched_move: None,
                    match_count: None,
                })
            },
        )
        .context("querying game list")?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("reading game-list rows")
}

fn order_by_clause(query: &GameListQuery) -> String {
    let mut fields: Vec<String> = query
        .sort_fields
        .iter()
        .map(|field| {
            format!(
                "{} {}",
                field.column.sql_expression(),
                field.direction.sql_keyword()
            )
        })
        .collect();

    // Ensure stable ordering when several games have identical metadata.
    fields.push("games.id ASC".to_owned());

    fields.join(", ")
}

// Test section

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_query_has_stable_multi_column_sorting() {
        let query = GameListQuery::default();

        assert_eq!(
            query.sort_fields,
            vec![
                SortField::descending(GameColumn::Date),
                SortField::ascending(GameColumn::BlackPlayer),
                SortField::ascending(GameColumn::WhitePlayer),
            ]
        );

        assert_eq!(query.result, GameResultFilter::Any);
        assert_eq!(query.limit, 200);
        assert_eq!(query.offset, 0);
    }

    #[test]
    fn columns_map_to_fixed_sql_expressions() {
        assert_eq!(
            GameColumn::BlackPlayer.sql_expression(),
            "selected_metadata.black_player"
        );
        assert_eq!(
            GameColumn::Date.sql_expression(),
            "selected_metadata.played_date"
        );
        assert_eq!(
            GameColumn::Result.sql_expression(),
            "selected_metadata.result"
        );
    }

    #[test]
    fn directions_map_to_sql_keywords() {
        assert_eq!(SortDirection::Ascending.sql_keyword(), "ASC");
        assert_eq!(SortDirection::Descending.sql_keyword(), "DESC");
    }

    #[test]
    fn creates_multi_column_ordering_with_stable_tie_breaker() {
        let query = GameListQuery {
            sort_fields: vec![
                SortField::descending(GameColumn::Date),
                SortField::ascending(GameColumn::BlackPlayer),
            ],
            ..GameListQuery::default()
        };

        assert_eq!(
            order_by_clause(&query),
            concat!(
                "selected_metadata.played_date DESC, ",
                "selected_metadata.black_player ASC, ",
                "games.id ASC"
            )
        );
    }

    fn populated_test_connection() -> Result<Connection> {
        let connection = Connection::open_in_memory().context("opening in-memory test database")?;

        connection.execute_batch(
            r#"
        CREATE TABLE games (
            id              INTEGER PRIMARY KEY,
            canonical_hash  BLOB NOT NULL UNIQUE,
            board_size      INTEGER NOT NULL,
            move_count      INTEGER NOT NULL,
            move_file       TEXT NOT NULL UNIQUE,
            created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE sources (
            id              INTEGER PRIMARY KEY,
            name            TEXT NOT NULL,
            version         TEXT NOT NULL,
            created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

            UNIQUE(name, version)
        );

        CREATE TABLE game_sources (
            id              INTEGER PRIMARY KEY,
            game_id         INTEGER NOT NULL,
            source_id       INTEGER NOT NULL,
            original_path   TEXT NOT NULL,
            imported_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

            FOREIGN KEY(game_id) REFERENCES games(id),
            FOREIGN KEY(source_id) REFERENCES sources(id),

            UNIQUE(source_id, original_path)
        );

        CREATE TABLE game_metadata (
            game_source_id  INTEGER PRIMARY KEY,
            black_player    TEXT,
            white_player    TEXT,
            played_date     TEXT,
            event           TEXT,
            result          TEXT,
            komi            REAL,
            handicap        INTEGER,

            FOREIGN KEY(game_source_id)
                REFERENCES game_sources(id)
                ON DELETE CASCADE
        );

        INSERT INTO games (
            id,
            canonical_hash,
            board_size,
            move_count,
            move_file
        )
        VALUES
            (1, X'01', 19, 120, 'games/01.moves'),
            (2, X'02', 19, 210, 'games/02.moves');

        INSERT INTO sources (id, name, version)
        VALUES
            (1, 'GoGoD', '2026'),
            (2, 'go4go', '2026');

        INSERT INTO game_sources (
            id,
            game_id,
            source_id,
            original_path
        )
        VALUES
            (1, 1, 1, 'gogod/game-one.sgf'),
            (2, 1, 2, 'go4go/game-one.sgf'),
            (3, 2, 1, 'gogod/game-two.sgf');

        -- The first source for game 1 has incomplete metadata.
        INSERT INTO game_metadata (
            game_source_id,
            black_player,
            white_player
        )
        VALUES (
            1,
            'Alpha',
            'Beta'
        );

        -- The second source describes the same canonical game more fully.
        INSERT INTO game_metadata (
            game_source_id,
            black_player,
            white_player,
            played_date,
            event,
            result
        )
        VALUES (
            2,
            'Alpha',
            'Beta',
            '2026-04-15',
            'Spring Tournament',
            'B+R'
        );

        INSERT INTO game_metadata (
            game_source_id,
            black_player,
            white_player,
            played_date,
            event,
            result
        )
        VALUES (
            3,
            'Gamma',
            'Delta',
            '2025-11-03',
            'Autumn Tournament',
            'W+2.5'
        );
        "#,
        )?;

        Ok(connection)
    }

    #[test]
    fn lists_one_row_per_canonical_game_and_uses_best_metadata() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery::default();
        let games = list_games(&connection, &query)?;

        assert_eq!(games.len(), 2);

        let first_game = games
            .iter()
            .find(|game| game.game_id == 1)
            .expect("canonical game 1 should be returned");

        assert_eq!(first_game.black_player.as_deref(), Some("Alpha"));
        assert_eq!(first_game.white_player.as_deref(), Some("Beta"));
        assert_eq!(first_game.game_date.as_deref(), Some("2026-04-15"));
        assert_eq!(first_game.event.as_deref(), Some("Spring Tournament"));
        assert_eq!(first_game.result.as_deref(), Some("B+R"));

        let second_game = games
            .iter()
            .find(|game| game.game_id == 2)
            .expect("canonical game 2 should be returned");

        assert_eq!(second_game.black_player.as_deref(), Some("Gamma"));
        assert_eq!(second_game.white_player.as_deref(), Some("Delta"));

        Ok(())
    }

    #[test]
    fn filters_by_black_player() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            player: Some("Alpha".to_owned()),
            colour: PlayerColour::Black,
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![1]
        );

        let wrong_colour_query = GameListQuery {
            player: Some("Beta".to_owned()),
            colour: PlayerColour::Black,
            ..GameListQuery::default()
        };

        assert!(list_games(&connection, &wrong_colour_query)?.is_empty());

        Ok(())
    }

    #[test]
    fn filters_by_white_player() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            player: Some("Beta".to_owned()),
            colour: PlayerColour::White,
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![1]
        );

        let wrong_colour_query = GameListQuery {
            player: Some("Alpha".to_owned()),
            colour: PlayerColour::White,
            ..GameListQuery::default()
        };

        assert!(list_games(&connection, &wrong_colour_query)?.is_empty());

        Ok(())
    }

    #[test]
    fn filters_by_player_of_either_colour() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            player: Some("Delta".to_owned()),
            colour: PlayerColour::Either,
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![2]
        );

        Ok(())
    }
}
