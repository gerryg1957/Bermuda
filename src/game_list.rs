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
            Self::BlackPlayer => "metadata.black_player",
            Self::WhitePlayer => "metadata.white_player",
            Self::Date => "metadata.game_date",
            Self::Result => "metadata.result",
            Self::Event => "metadata.event",
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
            "metadata.black_player"
        );
        assert_eq!(GameColumn::Date.sql_expression(), "metadata.game_date");
        assert_eq!(GameColumn::Result.sql_expression(), "metadata.result");
    }

    #[test]
    fn directions_map_to_sql_keywords() {
        assert_eq!(SortDirection::Ascending.sql_keyword(), "ASC");
        assert_eq!(SortDirection::Descending.sql_keyword(), "DESC");
    }
}
