use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

use crate::game_date::played_date_sort_key;

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
            Self::BlackPlayer => {
                "COALESCE(black_identity.preferred_name, selected_metadata.black_player)"
            }
            Self::WhitePlayer => {
                "COALESCE(white_identity.preferred_name, selected_metadata.white_player)"
            }
            Self::Date => "selected_metadata.played_date_sort",
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

#[derive(Debug, Clone, PartialEq)]
pub struct GameListRow {
    pub game_id: i64,

    /*
     * black_player / white_player are the literal spellings from the
     * selected source metadata. The display fields are Bermuda's preferred
     * presentation when a confirmed identity exists.
     */
    pub black_player: Option<String>,
    pub white_player: Option<String>,
    pub black_player_id: Option<i64>,
    pub white_player_id: Option<i64>,
    pub black_player_display: Option<String>,
    pub white_player_display: Option<String>,

    pub game_date: Option<String>,
    pub result: Option<String>,
    pub event: Option<String>,
    pub komi: Option<f32>,
    pub matched_move: Option<u32>,
    pub match_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameListPlayerMetadata<'a> {
    pub source_name: Option<&'a str>,
    pub player_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameListQuery {
    pub player: Option<String>,
    pub versus: Option<String>,

    /*
     * These ID sets are populated when the same query is used for in-memory
     * filtering. SQL catalogue filtering resolves the typed names directly.
     */
    pub resolved_player_ids: Vec<i64>,
    pub resolved_versus_ids: Vec<i64>,

    pub colour: PlayerColour,
    pub event: Option<String>,
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
            versus: None,
            resolved_player_ids: Vec::new(),
            resolved_versus_ids: Vec::new(),
            colour: PlayerColour::Either,
            event: None,
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

impl GameListQuery {
    pub fn matches_metadata(
        &self,
        black_player: GameListPlayerMetadata<'_>,
        white_player: GameListPlayerMetadata<'_>,
        game_date: Option<&str>,
        result: Option<&str>,
        event: Option<&str>,
    ) -> bool {
        let players_match = match (self.player.as_deref(), self.versus.as_deref()) {
            (None, None) => true,

            (None, Some(_)) => false,

            (Some(player), None) => match self.colour {
                PlayerColour::Black => {
                    player_metadata_matches(player, &self.resolved_player_ids, black_player)
                }

                PlayerColour::White => {
                    player_metadata_matches(player, &self.resolved_player_ids, white_player)
                }

                PlayerColour::Either => {
                    player_metadata_matches(player, &self.resolved_player_ids, black_player)
                        || player_metadata_matches(player, &self.resolved_player_ids, white_player)
                }
            },

            (Some(player), Some(versus)) => match self.colour {
                PlayerColour::Black => {
                    player_metadata_matches(player, &self.resolved_player_ids, black_player)
                        && player_metadata_matches(versus, &self.resolved_versus_ids, white_player)
                }

                PlayerColour::White => {
                    player_metadata_matches(player, &self.resolved_player_ids, white_player)
                        && player_metadata_matches(versus, &self.resolved_versus_ids, black_player)
                }

                PlayerColour::Either => {
                    (player_metadata_matches(player, &self.resolved_player_ids, black_player)
                        && player_metadata_matches(versus, &self.resolved_versus_ids, white_player))
                        || (player_metadata_matches(
                            player,
                            &self.resolved_player_ids,
                            white_player,
                        ) && player_metadata_matches(
                            versus,
                            &self.resolved_versus_ids,
                            black_player,
                        ))
                }
            },
        };

        if !players_match {
            return false;
        }

        if let Some(expected_event) = self.event.as_deref() {
            let Some(stored_event) = event else {
                return false;
            };

            if !stored_event
                .to_ascii_lowercase()
                .contains(&expected_event.to_ascii_lowercase())
            {
                return false;
            }
        }

        let played_date_sort = game_date.and_then(played_date_sort_key);

        if let Some(date_from) = normalise_date_from(self.date_from.as_deref()) {
            let Some(stored_date) = played_date_sort.as_deref() else {
                return false;
            };

            if stored_date < date_from.as_str() {
                return false;
            }
        }

        if let Some(date_to) = normalise_date_to(self.date_to.as_deref()) {
            let Some(stored_date) = played_date_sort.as_deref() else {
                return false;
            };

            if stored_date > date_to.as_str() {
                return false;
            }
        }

        result_matches(self.result, result)
    }
}

fn player_metadata_matches(
    expected_source_name: &str,
    resolved_player_ids: &[i64],
    metadata: GameListPlayerMetadata<'_>,
) -> bool {
    match metadata.player_id {
        /*
         * Once Bermuda has a confirmed identity for this side, identity is
         * authoritative. The literal source spelling must not bypass it.
         */
        Some(player_id) => resolved_player_ids.contains(&player_id),

        /*
         * Unidentified metadata remains searchable by its PB/PW text.
         * Player-name search is case-insensitive, matching Catalogue SQL.
         */
        None => metadata
            .source_name
            .is_some_and(|name| name.eq_ignore_ascii_case(expected_source_name)),
    }
}

fn starts_with_ascii_case_insensitive(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

fn result_matches(filter: GameResultFilter, result: Option<&str>) -> bool {
    match filter {
        GameResultFilter::Any => true,

        GameResultFilter::BlackWin => {
            result.is_some_and(|value| starts_with_ascii_case_insensitive(value, "B+"))
        }

        GameResultFilter::WhiteWin => {
            result.is_some_and(|value| starts_with_ascii_case_insensitive(value, "W+"))
        }

        GameResultFilter::Jigo => result.is_some_and(|value| {
            starts_with_ascii_case_insensitive(value, "Jigo") || value == "Draw" || value == "0"
        }),

        GameResultFilter::Void => {
            result.is_some_and(|value| starts_with_ascii_case_insensitive(value, "Void"))
        }
    }
}

fn player_side_condition(name_column: &str, player_id_column: &str, parameter: &str) -> String {
    format!(
        "(({player_id_column} IS NULL AND {name_column} COLLATE NOCASE = {parameter}) \
         OR \
         ({player_id_column} IS NOT NULL AND (\
             EXISTS (\
                 SELECT 1 \
                 FROM players AS filter_player \
                 WHERE filter_player.id = {player_id_column} \
                   AND filter_player.preferred_name COLLATE NOCASE = {parameter}\
             ) \
             OR EXISTS (\
                 SELECT 1 \
                 FROM player_aliases AS filter_alias \
                 WHERE filter_alias.player_id = {player_id_column} \
                   AND filter_alias.name COLLATE NOCASE = {parameter}\
             )\
         )))"
    )
}

fn player_condition(colour: PlayerColour, parameter: &str) -> String {
    let black = player_side_condition(
        "selected_metadata.black_player",
        "selected_metadata.black_player_id",
        parameter,
    );

    let white = player_side_condition(
        "selected_metadata.white_player",
        "selected_metadata.white_player_id",
        parameter,
    );

    match colour {
        PlayerColour::Black => black,
        PlayerColour::White => white,
        PlayerColour::Either => format!("({black} OR {white})"),
    }
}

fn matchup_condition(
    colour: PlayerColour,
    player_parameter: &str,
    versus_parameter: &str,
) -> String {
    let black_player = player_side_condition(
        "selected_metadata.black_player",
        "selected_metadata.black_player_id",
        player_parameter,
    );

    let white_player = player_side_condition(
        "selected_metadata.white_player",
        "selected_metadata.white_player_id",
        player_parameter,
    );

    let black_versus = player_side_condition(
        "selected_metadata.black_player",
        "selected_metadata.black_player_id",
        versus_parameter,
    );

    let white_versus = player_side_condition(
        "selected_metadata.white_player",
        "selected_metadata.white_player_id",
        versus_parameter,
    );

    match colour {
        PlayerColour::Black => {
            format!("({black_player} AND {white_versus})")
        }

        PlayerColour::White => {
            format!("({white_player} AND {black_versus})")
        }

        PlayerColour::Either => format!(
            "(({black_player} AND {white_versus}) \
             OR \
             ({white_player} AND {black_versus}))"
        ),
    }
}

fn result_condition(result: GameResultFilter) -> &'static str {
    match result {
        GameResultFilter::Any => "1 = 1",
        GameResultFilter::BlackWin => "selected_metadata.result LIKE 'B+%'",
        GameResultFilter::WhiteWin => "selected_metadata.result LIKE 'W+%'",
        GameResultFilter::Jigo => {
            "(selected_metadata.result LIKE 'Jigo%' \
             OR selected_metadata.result = 'Draw' \
             OR selected_metadata.result = '0')"
        }
        GameResultFilter::Void => "selected_metadata.result LIKE 'Void%'",
    }
}

fn is_year_only(value: &str) -> bool {
    value.len() == 4 && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn normalise_date_from(value: Option<&str>) -> Option<String> {
    value.map(|value| {
        if is_year_only(value) {
            format!("{value}-01-01")
        } else {
            value.to_owned()
        }
    })
}

fn normalise_date_to(value: Option<&str>) -> Option<String> {
    value.map(|value| {
        if is_year_only(value) {
            format!("{value}-12-31")
        } else {
            value.to_owned()
        }
    })
}

pub fn list_games(connection: &Connection, query: &GameListQuery) -> Result<Vec<GameListRow>> {
    let order_by = order_by_clause(query);
    let player_condition = player_condition(query.colour, "?3");
    let matchup_condition = matchup_condition(query.colour, "?3", "?4");
    let result_condition = result_condition(query.result);
    let date_from = normalise_date_from(query.date_from.as_deref());
    let date_to = normalise_date_to(query.date_to.as_deref());

    let sql = format!(
        r#"
        WITH ranked_metadata AS (
            SELECT
                game_sources.game_id,
                game_metadata.black_player,
                game_metadata.white_player,
                game_metadata.black_player_id,
                game_metadata.white_player_id,
                game_metadata.played_date,
                game_metadata.played_date_sort,
                game_metadata.result,
                game_metadata.event,
                game_metadata.komi,

                ROW_NUMBER() OVER (
                    PARTITION BY game_sources.game_id
                    ORDER BY
                        (
                            (game_metadata.black_player IS NOT NULL) +
                            (game_metadata.white_player IS NOT NULL) +
                            (game_metadata.played_date IS NOT NULL) +
                            (game_metadata.result IS NOT NULL) +
                            (game_metadata.event IS NOT NULL) +
                            (game_metadata.komi IS NOT NULL)
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
                black_player_id,
                white_player_id,
                played_date,
                played_date_sort,
                result,
                event,
                komi

            FROM ranked_metadata

            WHERE metadata_rank = 1
        )

        SELECT
            games.id,
            selected_metadata.black_player,
            selected_metadata.white_player,
            selected_metadata.black_player_id,
            selected_metadata.white_player_id,
            COALESCE(
                black_identity.preferred_name,
                selected_metadata.black_player
            ),
            COALESCE(
                white_identity.preferred_name,
                selected_metadata.white_player
            ),
            selected_metadata.played_date,

            selected_metadata.result,
            selected_metadata.event,
            selected_metadata.komi

        FROM games

        LEFT JOIN selected_metadata
            ON selected_metadata.game_id = games.id

        LEFT JOIN players AS black_identity
            ON black_identity.id = selected_metadata.black_player_id

        LEFT JOIN players AS white_identity
            ON white_identity.id = selected_metadata.white_player_id

WHERE (
    (
        ?4 IS NULL
        AND (
            ?3 IS NULL
            OR {player_condition}
        )
    )
    OR (
        ?4 IS NOT NULL
        AND ?3 IS NOT NULL
        AND {matchup_condition}
    )
)

AND (
    ?5 IS NULL
    OR selected_metadata.event LIKE '%' || ?5 || '%' COLLATE NOCASE
)

AND (
    ?6 IS NULL
    OR selected_metadata.played_date_sort >= ?6
)

AND (
    ?7 IS NULL
    OR selected_metadata.played_date_sort <= ?7
)

AND (
    {result_condition}
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
                query.versus.as_deref(),
                query.event.as_deref(),
                date_from.as_deref(),
                date_to.as_deref(),
            ],
            |row| {
                Ok(GameListRow {
                    game_id: row.get(0)?,
                    black_player: row.get(1)?,
                    white_player: row.get(2)?,
                    black_player_id: row.get(3)?,
                    white_player_id: row.get(4)?,
                    black_player_display: row.get(5)?,
                    white_player_display: row.get(6)?,
                    game_date: row.get(7)?,
                    result: row.get(8)?,
                    event: row.get(9)?,
                    komi: row.get(10)?,
                    matched_move: None,
                    match_count: None,
                })
            },
        )
        .context("querying game list")?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("reading game-list rows")
}

pub fn count_games(connection: &Connection, query: &GameListQuery) -> Result<u64> {
    let player_condition = player_condition(query.colour, "?1");
    let matchup_condition = matchup_condition(query.colour, "?1", "?2");
    let result_condition = result_condition(query.result);
    let date_from = normalise_date_from(query.date_from.as_deref());
    let date_to = normalise_date_to(query.date_to.as_deref());

    let sql = format!(
        r#"
        WITH ranked_metadata AS (
            SELECT
                game_sources.game_id,
                game_metadata.black_player,
                game_metadata.white_player,
                game_metadata.black_player_id,
                game_metadata.white_player_id,
                game_metadata.played_date,
                game_metadata.played_date_sort,
                game_metadata.result,
                game_metadata.event,
                game_metadata.komi,

                ROW_NUMBER() OVER (
                    PARTITION BY game_sources.game_id
                    ORDER BY
                        (
                            (game_metadata.black_player IS NOT NULL) +
                            (game_metadata.white_player IS NOT NULL) +
                            (game_metadata.played_date IS NOT NULL) +
                            (game_metadata.result IS NOT NULL) +
                            (game_metadata.event IS NOT NULL) +
                            (game_metadata.komi IS NOT NULL)
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
                black_player_id,
                white_player_id,
                played_date,
                played_date_sort,
                result,
                event

            FROM ranked_metadata

            WHERE metadata_rank = 1
        )

        SELECT COUNT(*)

        FROM games

        LEFT JOIN selected_metadata
            ON selected_metadata.game_id = games.id

        WHERE (
            (
                ?2 IS NULL
                AND (
                    ?1 IS NULL
                    OR {player_condition}
                )
            )
            OR (
                ?2 IS NOT NULL
                AND ?1 IS NOT NULL
                AND {matchup_condition}
            )
        )

        AND (
            ?3 IS NULL
            OR selected_metadata.event LIKE '%' || ?3 || '%' COLLATE NOCASE
        )

        AND (
            ?4 IS NULL
            OR selected_metadata.played_date_sort >= ?4
        )

        AND (
            ?5 IS NULL
            OR selected_metadata.played_date_sort <= ?5
        )

        AND (
            {result_condition}
        )
        "#
    );

    let count: i64 = connection
        .query_row(
            &sql,
            params![
                query.player.as_deref(),
                query.versus.as_deref(),
                query.event.as_deref(),
                date_from.as_deref(),
                date_to.as_deref(),
            ],
            |row| row.get(0),
        )
        .context("counting catalogue games")?;

    u64::try_from(count).context("catalogue game count was negative")
}

pub fn get_game(connection: &Connection, game_id: i64) -> Result<GameListRow> {
    let sql = r#"
        WITH ranked_metadata AS (
            SELECT
                game_sources.game_id,
                game_metadata.black_player,
                game_metadata.white_player,
                game_metadata.black_player_id,
                game_metadata.white_player_id,
                game_metadata.played_date,
                game_metadata.played_date_sort,
                game_metadata.result,
                game_metadata.event,
                game_metadata.komi,

                ROW_NUMBER() OVER (
                    PARTITION BY game_sources.game_id
                    ORDER BY
                        (
                            (game_metadata.black_player IS NOT NULL) +
                            (game_metadata.white_player IS NOT NULL) +
                            (game_metadata.played_date IS NOT NULL) +
                            (game_metadata.result IS NOT NULL) +
                            (game_metadata.event IS NOT NULL) +
                            (game_metadata.komi IS NOT NULL)
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
                black_player_id,
                white_player_id,
                played_date,
                played_date_sort,
                result,
                event,
                komi

            FROM ranked_metadata

            WHERE metadata_rank = 1
        )

        SELECT
            games.id,
            selected_metadata.black_player,
            selected_metadata.white_player,
            selected_metadata.black_player_id,
            selected_metadata.white_player_id,
            COALESCE(
                black_identity.preferred_name,
                selected_metadata.black_player
            ),
            COALESCE(
                white_identity.preferred_name,
                selected_metadata.white_player
            ),
            selected_metadata.played_date,
            selected_metadata.result,
            selected_metadata.event,
            selected_metadata.komi

        FROM games

        LEFT JOIN selected_metadata
            ON selected_metadata.game_id = games.id

        LEFT JOIN players AS black_identity
            ON black_identity.id = selected_metadata.black_player_id

        LEFT JOIN players AS white_identity
            ON white_identity.id = selected_metadata.white_player_id

        WHERE games.id = ?1
    "#;

    let game = connection
        .query_row(sql, [game_id], |row| {
            Ok(GameListRow {
                game_id: row.get(0)?,
                black_player: row.get(1)?,
                white_player: row.get(2)?,
                black_player_id: row.get(3)?,
                white_player_id: row.get(4)?,
                black_player_display: row.get(5)?,
                white_player_display: row.get(6)?,
                game_date: row.get(7)?,
                result: row.get(8)?,
                event: row.get(9)?,
                komi: row.get(10)?,
                matched_move: None,
                match_count: None,
            })
        })
        .optional()
        .context("reading game from catalogue")?;

    game.with_context(|| format!("game {game_id} does not exist"))
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
            "COALESCE(black_identity.preferred_name, selected_metadata.black_player)"
        );
        assert_eq!(
            GameColumn::Date.sql_expression(),
            "selected_metadata.played_date_sort"
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
                "selected_metadata.played_date_sort DESC, ",
                "COALESCE(black_identity.preferred_name, selected_metadata.black_player) ASC, ",
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

        CREATE TABLE players (
            id              INTEGER PRIMARY KEY,
            preferred_name  TEXT NOT NULL
        );

        CREATE TABLE player_aliases (
            id              INTEGER PRIMARY KEY,
            player_id       INTEGER NOT NULL,
            name            TEXT NOT NULL,
            source_id       INTEGER,
            notes           TEXT,
            FOREIGN KEY(player_id) REFERENCES players(id)
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
            black_player_id INTEGER REFERENCES players(id),
            white_player_id INTEGER REFERENCES players(id),
            played_date     TEXT,
            played_date_sort  TEXT,
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
    played_date_sort,
    event,
    result
)
VALUES (
    2,
    'Alpha',
    'Beta',
    '2026-04-15',
    '2026-04-15',
    'Spring Tournament',
    'B+R'
);

  INSERT INTO game_metadata (
    game_source_id,
    black_player,
    white_player,
    played_date,
    played_date_sort,
    event,
    result
)
VALUES (
    3,
    'Gamma',
    'Delta',
    '2025-11-03',
    '2025-11-03',
    'Autumn Tournament',
    'W+2.5'
);

        INSERT INTO players (id, preferred_name)
        VALUES
            (10, 'Preferred Alpha'),
            (20, 'Preferred Beta');

        /*
         * Alpha/Beta preserve the existing source-spelling tests while now
         * exercising identity-aware filtering. A. Alpha is another confirmed
         * source-specific alias. Shared Name deliberately denotes both
         * identities so search ambiguity can broaden rather than fail.
         */
        INSERT INTO player_aliases(
            id,
            player_id,
            name,
            source_id
        )
        VALUES
            (1, 10, 'Alpha', 1),
            (2, 20, 'Beta', 1),
            (3, 10, 'A. Alpha', 1),
            (4, 10, 'Shared Name', NULL),
            (5, 20, 'Shared Name', NULL);

        /*
         * Only the preferred metadata row for canonical game 1 is linked.
         * The literal Alpha/Beta source strings remain unchanged.
         */
        UPDATE game_metadata
        SET
            black_player_id = 10,
            white_player_id = 20
        WHERE game_source_id = 2;
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

        /*
         * Source spelling and Bermuda display identity are both retained.
         */
        assert_eq!(first_game.black_player.as_deref(), Some("Alpha"));
        assert_eq!(first_game.white_player.as_deref(), Some("Beta"));
        assert_eq!(first_game.black_player_id, Some(10));
        assert_eq!(first_game.white_player_id, Some(20));
        assert_eq!(
            first_game.black_player_display.as_deref(),
            Some("Preferred Alpha")
        );
        assert_eq!(
            first_game.white_player_display.as_deref(),
            Some("Preferred Beta")
        );
        assert_eq!(first_game.game_date.as_deref(), Some("2026-04-15"));
        assert_eq!(first_game.event.as_deref(), Some("Spring Tournament"));
        assert_eq!(first_game.result.as_deref(), Some("B+R"));

        let second_game = games
            .iter()
            .find(|game| game.game_id == 2)
            .expect("canonical game 2 should be returned");

        assert_eq!(second_game.black_player.as_deref(), Some("Gamma"));
        assert_eq!(second_game.white_player.as_deref(), Some("Delta"));
        assert_eq!(second_game.black_player_id, None);
        assert_eq!(second_game.white_player_id, None);

        /*
         * Unresolved names simply display their source spelling.
         */
        assert_eq!(second_game.black_player_display.as_deref(), Some("Gamma"));
        assert_eq!(second_game.white_player_display.as_deref(), Some("Delta"));

        Ok(())
    }

    #[test]
    fn in_memory_filter_matches_database_filter_semantics() -> Result<()> {
        let connection = populated_test_connection()?;

        let all_games = list_games(
            &connection,
            &GameListQuery {
                limit: u32::MAX,
                ..GameListQuery::default()
            },
        )?;

        let queries = vec![
            GameListQuery {
                player: Some("Alpha".to_owned()),
                colour: PlayerColour::Black,
                ..GameListQuery::default()
            },
            GameListQuery {
                player: Some("Preferred Alpha".to_owned()),
                colour: PlayerColour::Black,
                ..GameListQuery::default()
            },
            GameListQuery {
                player: Some("A. Alpha".to_owned()),
                colour: PlayerColour::Black,
                ..GameListQuery::default()
            },
            GameListQuery {
                player: Some("Shared Name".to_owned()),
                colour: PlayerColour::Black,
                ..GameListQuery::default()
            },
            GameListQuery {
                player: Some("Shared Name".to_owned()),
                colour: PlayerColour::White,
                ..GameListQuery::default()
            },
            GameListQuery {
                player: Some("Delta".to_owned()),
                colour: PlayerColour::Either,
                ..GameListQuery::default()
            },
            GameListQuery {
                player: Some("Alpha".to_owned()),
                versus: Some("Beta".to_owned()),
                colour: PlayerColour::Either,
                ..GameListQuery::default()
            },
            GameListQuery {
                versus: Some("Beta".to_owned()),
                ..GameListQuery::default()
            },
            GameListQuery {
                event: Some("SPRING".to_owned()),
                ..GameListQuery::default()
            },
            GameListQuery {
                date_from: Some("2026".to_owned()),
                ..GameListQuery::default()
            },
            GameListQuery {
                date_to: Some("2025".to_owned()),
                ..GameListQuery::default()
            },
            GameListQuery {
                result: GameResultFilter::BlackWin,
                ..GameListQuery::default()
            },
            GameListQuery {
                result: GameResultFilter::WhiteWin,
                ..GameListQuery::default()
            },
            GameListQuery {
                player: Some("Alpha".to_owned()),
                colour: PlayerColour::Black,
                event: Some("spring".to_owned()),
                date_from: Some("2026".to_owned()),
                date_to: Some("2026".to_owned()),
                result: GameResultFilter::BlackWin,
                ..GameListQuery::default()
            },
        ];

        for mut query in queries {
            query.limit = u32::MAX;

            let mut database_ids = list_games(&connection, &query)?
                .into_iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>();

            let mut memory_query = query.clone();

            if let Some(name) = memory_query.player.clone() {
                memory_query.resolved_player_ids =
                    crate::player_directory::player_ids_for_search_name_on(&connection, &name)?;
            }

            if let Some(name) = memory_query.versus.clone() {
                memory_query.resolved_versus_ids =
                    crate::player_directory::player_ids_for_search_name_on(&connection, &name)?;
            }

            let mut memory_ids = all_games
                .iter()
                .filter(|game| {
                    memory_query.matches_metadata(
                        GameListPlayerMetadata {
                            source_name: game.black_player.as_deref(),
                            player_id: game.black_player_id,
                        },
                        GameListPlayerMetadata {
                            source_name: game.white_player.as_deref(),
                            player_id: game.white_player_id,
                        },
                        game.game_date.as_deref(),
                        game.result.as_deref(),
                        game.event.as_deref(),
                    )
                })
                .map(|game| game.game_id)
                .collect::<Vec<_>>();

            database_ids.sort_unstable();
            memory_ids.sort_unstable();

            assert_eq!(memory_ids, database_ids, "query was {query:?}");
        }

        Ok(())
    }

    #[test]
    fn filters_identified_players_by_preferred_name_alias_and_ambiguity() -> Result<()> {
        let connection = populated_test_connection()?;

        for name in ["Preferred Alpha", "preferred alpha", "A. Alpha", "a. alpha"] {
            let query = GameListQuery {
                player: Some(name.to_owned()),
                colour: PlayerColour::Black,
                ..GameListQuery::default()
            };

            assert_eq!(
                list_games(&connection, &query)?
                    .iter()
                    .map(|game| game.game_id)
                    .collect::<Vec<_>>(),
                vec![1],
                "search name was {name:?}"
            );
        }

        let unresolved_query = GameListQuery {
            player: Some("gamma".to_owned()),
            colour: PlayerColour::Black,
            ..GameListQuery::default()
        };

        assert_eq!(
            list_games(&connection, &unresolved_query)?
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![2],
            "unlinked source-name search should ignore ASCII case"
        );

        /*
         * Shared Name is deliberately attached to both identities. Search
         * therefore finds either identity rather than arbitrarily choosing
         * one of them.
         */
        for colour in [PlayerColour::Black, PlayerColour::White] {
            let query = GameListQuery {
                player: Some("Shared Name".to_owned()),
                colour,
                ..GameListQuery::default()
            };

            assert_eq!(
                list_games(&connection, &query)?
                    .iter()
                    .map(|game| game.game_id)
                    .collect::<Vec<_>>(),
                vec![1]
            );
        }

        /*
         * Canonical game 2 has no identity links, so the historical exact
         * raw-spelling behaviour remains available.
         */
        let unresolved = GameListQuery {
            player: Some("Delta".to_owned()),
            colour: PlayerColour::Either,
            ..GameListQuery::default()
        };

        assert_eq!(
            list_games(&connection, &unresolved)?
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![2]
        );

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

    #[test]
    fn filters_by_opponent_with_player_of_either_colour() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            player: Some("Alpha".to_owned()),
            versus: Some("Beta".to_owned()),
            colour: PlayerColour::Either,
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![1]
        );

        let reversed_query = GameListQuery {
            player: Some("Beta".to_owned()),
            versus: Some("Alpha".to_owned()),
            colour: PlayerColour::Either,
            ..GameListQuery::default()
        };

        assert_eq!(
            list_games(&connection, &reversed_query)?
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![1]
        );

        Ok(())
    }

    #[test]
    fn filters_by_opponent_with_specific_colour() -> Result<()> {
        let connection = populated_test_connection()?;

        let black_query = GameListQuery {
            player: Some("Alpha".to_owned()),
            versus: Some("Beta".to_owned()),
            colour: PlayerColour::Black,
            ..GameListQuery::default()
        };

        assert_eq!(
            list_games(&connection, &black_query)?
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![1]
        );

        let wrong_colour_query = GameListQuery {
            player: Some("Alpha".to_owned()),
            versus: Some("Beta".to_owned()),
            colour: PlayerColour::White,
            ..GameListQuery::default()
        };

        assert!(list_games(&connection, &wrong_colour_query)?.is_empty());

        Ok(())
    }

    #[test]
    fn versus_without_player_matches_nothing() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            versus: Some("Beta".to_owned()),
            ..GameListQuery::default()
        };

        assert!(list_games(&connection, &query)?.is_empty());
        assert_eq!(count_games(&connection, &query)?, 0);

        Ok(())
    }

    #[test]
    fn filters_event_by_case_insensitive_substring() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            event: Some("SPRING".to_owned()),
            ..GameListQuery::default()
        };

        assert_eq!(
            list_games(&connection, &query)?
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![1]
        );

        let partial_query = GameListQuery {
            event: Some("tournament".to_owned()),
            ..GameListQuery::default()
        };

        assert_eq!(list_games(&connection, &partial_query)?.len(), 2);

        Ok(())
    }

    #[test]
    fn counts_games_matching_versus_and_event_filters() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            player: Some("Alpha".to_owned()),
            versus: Some("Beta".to_owned()),
            colour: PlayerColour::Either,
            event: Some("spring".to_owned()),
            ..GameListQuery::default()
        };

        assert_eq!(count_games(&connection, &query)?, 1);

        Ok(())
    }

    #[test]
    fn year_only_from_date_includes_the_whole_year() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            date_from: Some("2026".to_owned()),
            ..GameListQuery::default()
        };

        assert_eq!(
            list_games(&connection, &query)?
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![1]
        );

        Ok(())
    }

    #[test]
    fn year_only_to_date_includes_the_whole_year() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            date_to: Some("2025".to_owned()),
            ..GameListQuery::default()
        };

        assert_eq!(
            list_games(&connection, &query)?
                .iter()
                .map(|game| game.game_id)
                .collect::<Vec<_>>(),
            vec![2]
        );

        Ok(())
    }

    #[test]
    fn year_only_date_range_is_inclusive() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            date_from: Some("2025".to_owned()),
            date_to: Some("2026".to_owned()),
            ..GameListQuery::default()
        };

        assert_eq!(list_games(&connection, &query)?.len(), 2);
        assert_eq!(count_games(&connection, &query)?, 2);

        Ok(())
    }

    #[test]
    fn filters_from_date_inclusively() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            date_from: Some("2026-04-15".to_owned()),
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![1]
        );

        Ok(())
    }

    #[test]
    fn filters_to_date_inclusively() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            date_to: Some("2025-11-03".to_owned()),
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![2]
        );

        Ok(())
    }

    #[test]
    fn filters_within_date_range() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            date_from: Some("2025-12-01".to_owned()),
            date_to: Some("2026-12-31".to_owned()),
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![1]
        );

        Ok(())
    }

    #[test]
    fn filters_black_wins() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            result: GameResultFilter::BlackWin,
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![1]
        );

        Ok(())
    }

    #[test]
    fn filters_white_wins() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            result: GameResultFilter::WhiteWin,
            ..GameListQuery::default()
        };

        let games = list_games(&connection, &query)?;

        assert_eq!(
            games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
            vec![2]
        );

        Ok(())
    }

    #[test]
    fn filters_jigo_results() -> Result<()> {
        for stored_result in ["Jigo", "Jigo (B connects the ko)", "Draw", "0"] {
            let connection = populated_test_connection()?;

            connection.execute(
                "
            UPDATE game_metadata
            SET result = ?1
            WHERE game_source_id = 3
            ",
                [stored_result],
            )?;

            let query = GameListQuery {
                result: GameResultFilter::Jigo,
                ..GameListQuery::default()
            };

            let games = list_games(&connection, &query)?;

            assert_eq!(
                games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
                vec![2],
                "stored result was {stored_result:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn filters_void_results() -> Result<()> {
        for stored_result in ["Void", "Void (triple ko)", "Void game"] {
            let connection = populated_test_connection()?;

            connection.execute(
                "
            UPDATE game_metadata
            SET result = ?1
            WHERE game_source_id = 3
            ",
                [stored_result],
            )?;

            let query = GameListQuery {
                result: GameResultFilter::Void,
                ..GameListQuery::default()
            };

            let games = list_games(&connection, &query)?;

            assert_eq!(
                games.iter().map(|game| game.game_id).collect::<Vec<_>>(),
                vec![2],
                "stored result was {stored_result:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn counts_all_games_ignoring_pagination() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            offset: 1,
            limit: 1,
            ..GameListQuery::default()
        };

        assert_eq!(count_games(&connection, &query)?, 2);

        Ok(())
    }

    #[test]
    fn counts_games_matching_combined_filters() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            player: Some("Alpha".to_owned()),
            colour: PlayerColour::Black,
            date_from: Some("2026-01-01".to_owned()),
            date_to: Some("2026-12-31".to_owned()),
            result: GameResultFilter::BlackWin,
            offset: 100,
            limit: 0,
            ..GameListQuery::default()
        };

        assert_eq!(count_games(&connection, &query)?, 1);

        Ok(())
    }

    #[test]
    fn counts_zero_when_no_games_match() -> Result<()> {
        let connection = populated_test_connection()?;

        let query = GameListQuery {
            player: Some("Nobody".to_owned()),
            ..GameListQuery::default()
        };

        assert_eq!(count_games(&connection, &query)?, 0);

        Ok(())
    }

    #[test]
    fn gets_one_game_by_id() -> Result<()> {
        let connection = populated_test_connection()?;

        let game = get_game(&connection, 1)?;

        assert_eq!(game.game_id, 1);
        assert_eq!(game.black_player.as_deref(), Some("Alpha"));
        assert_eq!(game.white_player.as_deref(), Some("Beta"));
        assert_eq!(game.game_date.as_deref(), Some("2026-04-15"));
        assert_eq!(game.event.as_deref(), Some("Spring Tournament"));
        assert_eq!(game.result.as_deref(), Some("B+R"));
        assert_eq!(game.matched_move, None);
        assert_eq!(game.match_count, None);

        Ok(())
    }

    #[test]
    fn getting_unknown_game_reports_error() -> Result<()> {
        let connection = populated_test_connection()?;

        let error = get_game(&connection, 999).expect_err("unknown game should fail");

        assert!(error.to_string().contains("game 999 does not exist"));

        Ok(())
    }
}
