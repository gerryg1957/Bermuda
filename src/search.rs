use crate::{Color, PatternMatch, PatternSearchQuery, PatternSearcher, indexer::PositionIndexer};
use anyhow::Result;

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

pub struct SearchEngine<'a> {
    indexer: &'a PositionIndexer,
    pattern_searcher: PatternSearcher,
}

impl<'a> SearchEngine<'a> {
    #[must_use]
    pub fn new(indexer: &'a PositionIndexer) -> Self {
        Self {
            indexer,
            pattern_searcher: PatternSearcher::new(),
        }
    }

    pub fn search_pattern(&self, query: &PatternSearchQuery) -> Result<Vec<PatternMatch>> {
        self.pattern_searcher.search(self.indexer, query)
    }
}
