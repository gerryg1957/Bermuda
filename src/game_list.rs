use anyhow::{Context, Result};
use rusqlite::Connection;

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

        ORDER BY {order_by}

        LIMIT ?1
        OFFSET ?2
        "#
    );

    let mut statement = connection
        .prepare(&sql)
        .context("preparing game-list query")?;

    let rows = statement
        .query_map([i64::from(query.limit), i64::from(query.offset)], |row| {
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
        })
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
}
