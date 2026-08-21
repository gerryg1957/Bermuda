use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};

use crate::{
    Colour, NextMoveDistribution, PatternGameSummary, PatternMatch, PatternSearchOutcome,
    PatternSearchProgress, PatternSearchQuery, PatternSearchScope, PatternSearchSummaryOutcome,
    PatternSearchSummaryReportOutcome, PatternSearcher, PatternTransformation,
    game_catalogue::GameCatalogue, game_list::GameListQuery, indexer::PositionIndexer,
    project::Project,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOccurrence {
    /// First board position at which this appearance exists.
    pub move_number: usize,

    /// Last consecutive board position at which the same appearance exists.
    pub last_move_number: usize,

    pub side_to_move: Option<Colour>,
    pub ko_point: Option<u16>,

    pub left: Option<u8>,
    pub bottom: Option<u8>,

    pub transformation: Option<PatternTransformation>,
    pub colours_reversed: Option<bool>,
}

impl SearchOccurrence {
    /// Number of moves for which this continuous appearance persists.
    ///
    /// An appearance present at only one board position has duration zero.
    #[must_use]
    pub fn duration_moves(&self) -> usize {
        self.last_move_number.saturating_sub(self.move_number)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub game_id: i64,

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

    pub occurrences: Vec<SearchOccurrence>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchPatternOutcome {
    Completed(Vec<SearchResult>),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchSummaryResult {
    pub game_id: i64,

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

    pub match_count: usize,
    pub first_occurrence: SearchOccurrence,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchPatternSummaryOutcome {
    Completed(Vec<SearchSummaryResult>),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchSummaryReport {
    pub results: Vec<SearchSummaryResult>,
    pub next_moves: NextMoveDistribution,
    pub continuation_game_ids: HashMap<(i16, i16), Vec<i64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchPatternSummaryReportOutcome {
    Completed(SearchSummaryReport),
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

    pub fn search_pattern_summaries(
        &self,
        query: &PatternSearchQuery,
    ) -> Result<Vec<SearchSummaryResult>> {
        match self.search_pattern_summaries_with_progress(query, || false, |_| {})? {
            SearchPatternSummaryOutcome::Completed(results) => Ok(results),

            SearchPatternSummaryOutcome::Cancelled => {
                unreachable!("an uncancellable summary search was cancelled")
            }
        }
    }

    pub fn search_pattern_summaries_with_progress<C, P>(
        &self,
        query: &PatternSearchQuery,
        mut is_cancelled: C,
        on_progress: P,
    ) -> Result<SearchPatternSummaryOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        let outcome = self.pattern_searcher.search_summaries_with_progress(
            &self.indexer,
            query,
            &mut is_cancelled,
            on_progress,
        )?;

        let PatternSearchSummaryOutcome::Completed(summaries) = outcome else {
            return Ok(SearchPatternSummaryOutcome::Cancelled);
        };

        if is_cancelled() {
            return Ok(SearchPatternSummaryOutcome::Cancelled);
        }

        let results = self.results_from_summaries(summaries)?;

        if is_cancelled() {
            return Ok(SearchPatternSummaryOutcome::Cancelled);
        }

        Ok(SearchPatternSummaryOutcome::Completed(results))
    }

    pub fn search_pattern_summary_report_with_progress<C, P>(
        &self,
        query: &PatternSearchQuery,
        mut is_cancelled: C,
        on_progress: P,
    ) -> Result<SearchPatternSummaryReportOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        anyhow::ensure!(
            matches!(query.scope, PatternSearchScope::Project),
            "pattern summary reports currently require project scope"
        );

        let outcome = self
            .pattern_searcher
            .search_database_summary_report_with_progress(
                &self.indexer,
                &query.pattern,
                query.options,
                &mut is_cancelled,
                on_progress,
            )?;

        let PatternSearchSummaryReportOutcome::Completed(report) = outcome else {
            return Ok(SearchPatternSummaryReportOutcome::Cancelled);
        };

        if is_cancelled() {
            return Ok(SearchPatternSummaryReportOutcome::Cancelled);
        }

        let results = self.results_from_summaries(report.summaries)?;

        if is_cancelled() {
            return Ok(SearchPatternSummaryReportOutcome::Cancelled);
        }

        Ok(SearchPatternSummaryReportOutcome::Completed(
            SearchSummaryReport {
                results,
                next_moves: report.next_moves,
                continuation_game_ids: report.continuation_game_ids,
            },
        ))
    }

    fn results_from_summaries(
        &self,
        summaries: Vec<PatternGameSummary>,
    ) -> Result<Vec<SearchSummaryResult>> {
        /*
         * Read preferred metadata in one catalogue operation rather
         * than issuing a separate query for every matching game.
         */
        let catalogue_rows = self.catalogue.list(&GameListQuery {
            sort_fields: Vec::new(),
            limit: u32::MAX,
            ..GameListQuery::default()
        })?;

        let mut games_by_id = catalogue_rows
            .into_iter()
            .map(|game| (game.game_id, game))
            .collect::<HashMap<_, _>>();

        summaries
            .into_iter()
            .map(|summary| {
                let game = games_by_id
                    .remove(&summary.game_id)
                    .with_context(|| format!("game {} does not exist", summary.game_id))?;

                let found = summary.first_match;

                Ok(SearchSummaryResult {
                    game_id: summary.game_id,
                    black_player: game.black_player,
                    white_player: game.white_player,
                    black_player_id: game.black_player_id,
                    white_player_id: game.white_player_id,
                    black_player_display: game.black_player_display,
                    white_player_display: game.white_player_display,
                    game_date: game.game_date,
                    result: game.result,
                    event: game.event,
                    komi: game.komi,
                    match_count: summary.match_count,
                    first_occurrence: SearchOccurrence {
                        move_number: found.move_number,
                        last_move_number: found.last_move_number,
                        side_to_move: Some(found.side_to_move),
                        ko_point: found.ko_point,
                        left: Some(found.left),
                        bottom: Some(found.bottom),
                        transformation: Some(found.transformation),
                        colours_reversed: Some(found.colours_reversed),
                    },
                })
            })
            .collect()
    }

    fn results_from_matches(&self, matches: Vec<PatternMatch>) -> Result<Vec<SearchResult>> {
        let mut grouped_occurrences: BTreeMap<i64, Vec<SearchOccurrence>> = BTreeMap::new();

        for found in matches {
            grouped_occurrences
                .entry(found.game_id)
                .or_default()
                .push(SearchOccurrence {
                    move_number: found.move_number,
                    last_move_number: found.last_move_number,
                    side_to_move: Some(found.side_to_move),
                    ko_point: found.ko_point,
                    left: Some(found.left),
                    bottom: Some(found.bottom),
                    transformation: Some(found.transformation),
                    colours_reversed: Some(found.colours_reversed),
                });
        }

        /*
         * Read preferred metadata in one catalogue operation rather
         * than issuing a separate query for every matching game.
         */
        let catalogue_rows = self.catalogue.list(&GameListQuery {
            sort_fields: Vec::new(),
            limit: u32::MAX,
            ..GameListQuery::default()
        })?;

        let mut games_by_id = catalogue_rows
            .into_iter()
            .map(|game| (game.game_id, game))
            .collect::<HashMap<_, _>>();

        grouped_occurrences
            .into_iter()
            .map(|(game_id, occurrences)| {
                let game = games_by_id
                    .remove(&game_id)
                    .with_context(|| format!("game {game_id} does not exist"))?;

                Ok(SearchResult {
                    game_id,
                    black_player: game.black_player,
                    white_player: game.white_player,
                    black_player_id: game.black_player_id,
                    white_player_id: game.white_player_id,
                    black_player_display: game.black_player_display,
                    white_player_display: game.white_player_display,
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
