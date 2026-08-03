use std::collections::BTreeMap;

use anyhow::Result;

use crate::{
    Colour, PatternMatch, PatternSearchOutcome, PatternSearchProgress, PatternSearchQuery,
    PatternSearcher, game_catalogue::GameCatalogue, indexer::PositionIndexer, project::Project,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOccurrence {
    pub move_number: usize,

    pub side_to_move: Option<Colour>,
    pub ko_point: Option<u16>,

    pub left: Option<u8>,
    pub bottom: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub game_id: i64,

    pub black_player: Option<String>,
    pub white_player: Option<String>,

    pub game_date: Option<String>,
    pub result: Option<String>,
    pub event: Option<String>,
    pub komi: Option<f32>,

    pub occurrences: Vec<SearchOccurrence>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchPatternOutcome {
    Completed(Vec<SearchResult>),
    Cancelled,
}

pub struct SearchEngine {
    indexer: PositionIndexer,
    catalogue: GameCatalogue,
    pattern_searcher: PatternSearcher,
}

impl SearchEngine {
    pub fn new(project: &Project) -> Result<Self> {
        Ok(Self {
            indexer: project.position_indexer()?,
            catalogue: project.catalogue()?,
            pattern_searcher: PatternSearcher::new(),
        })
    }

    pub fn search_pattern(&self, query: &PatternSearchQuery) -> Result<Vec<SearchResult>> {
        match self.search_pattern_with_progress(query, || false, |_| {})? {
            SearchPatternOutcome::Completed(results) => Ok(results),

            SearchPatternOutcome::Cancelled => {
                unreachable!("an uncancellable search was cancelled")
            }
        }
    }

    pub fn search_pattern_with_progress<C, P>(
        &self,
        query: &PatternSearchQuery,
        is_cancelled: C,
        on_progress: P,
    ) -> Result<SearchPatternOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        match self.pattern_searcher.search_with_progress(
            &self.indexer,
            query,
            is_cancelled,
            on_progress,
        )? {
            PatternSearchOutcome::Completed(matches) => Ok(SearchPatternOutcome::Completed(
                self.results_from_matches(matches)?,
            )),

            PatternSearchOutcome::Cancelled => Ok(SearchPatternOutcome::Cancelled),
        }
    }

    fn results_from_matches(&self, matches: Vec<PatternMatch>) -> Result<Vec<SearchResult>> {
        let mut grouped_occurrences: BTreeMap<i64, Vec<SearchOccurrence>> = BTreeMap::new();

        for found in matches {
            grouped_occurrences
                .entry(found.game_id)
                .or_default()
                .push(SearchOccurrence {
                    move_number: found.move_number,
                    side_to_move: Some(found.side_to_move),
                    ko_point: found.ko_point,
                    left: Some(found.left),
                    bottom: Some(found.bottom),
                });
        }

        grouped_occurrences
            .into_iter()
            .map(|(game_id, occurrences)| {
                let game = self.catalogue.get(game_id)?;

                Ok(SearchResult {
                    game_id,
                    black_player: game.black_player,
                    white_player: game.white_player,
                    game_date: game.game_date,
                    result: game.result,
                    event: game.event,
                    komi: game.komi,
                    occurrences,
                })
            })
            .collect()
    }
}
