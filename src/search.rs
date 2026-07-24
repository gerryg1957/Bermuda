use crate::Color;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOccurrence {
    pub move_number: usize,

    pub side_to_move: Option<Color>,
    pub ko_point: Option<u16>,

    pub left: Option<u8>,
    pub bottom: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub game_id: i64,

    pub black_player: Option<String>,
    pub white_player: Option<String>,

    pub date: Option<String>,
    pub event: Option<String>,
    pub result: Option<String>,

    pub occurrences: Vec<SearchOccurrence>,
}
