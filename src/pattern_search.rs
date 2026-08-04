use std::collections::HashMap;

use crate::{Colour, Pattern, indexer::PositionIndexer};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    pub game_id: i64,
    pub move_number: usize,
    pub side_to_move: Colour,
    pub ko_point: Option<u16>,
    pub left: u8,
    pub bottom: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternSearchProgress {
    pub games_examined: usize,
    pub total_games: usize,
    pub matching_games: usize,
    pub matches_found: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternSearchOutcome {
    Completed(Vec<PatternMatch>),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternGameSummary {
    pub game_id: i64,
    pub match_count: usize,
    pub first_match: PatternMatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternSearchSummaryOutcome {
    Completed(Vec<PatternGameSummary>),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternSearchScope {
    Game(i64),
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternSearchQuery {
    pub pattern: Pattern,
    pub scope: PatternSearchScope,
}

pub struct PatternSearcher;

impl PatternSearcher {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn search_position(
        game_id: i64,
        move_number: usize,
        side_to_move: Colour,
        ko_point: Option<u16>,
        board: &crate::Board,
        pattern: &Pattern,
    ) -> Result<Vec<PatternMatch>> {
        let mut matches = Vec::new();

        if pattern.width > board.size() || pattern.height > board.size() {
            return Ok(matches);
        }

        let max_left = board.size() - pattern.width;
        let max_bottom = board.size() - pattern.height;

        for bottom in 0..=max_bottom {
            for left in 0..=max_left {
                if pattern.matches_at(board, left, bottom)? {
                    matches.push(PatternMatch {
                        game_id,
                        move_number,
                        side_to_move,
                        ko_point,
                        left,
                        bottom,
                    });
                }
            }
        }

        Ok(matches)
    }

    pub fn search(
        &self,
        indexer: &PositionIndexer,
        query: &PatternSearchQuery,
    ) -> Result<Vec<PatternMatch>> {
        match self.search_with_progress(indexer, query, || false, |_| {})? {
            PatternSearchOutcome::Completed(matches) => Ok(matches),

            PatternSearchOutcome::Cancelled => {
                unreachable!("an uncancellable pattern search was cancelled")
            }
        }
    }

    pub fn search_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        query: &PatternSearchQuery,
        mut is_cancelled: C,
        mut on_progress: P,
    ) -> Result<PatternSearchOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        match query.scope {
            PatternSearchScope::Game(game_id) => {
                let initial = PatternSearchProgress {
                    games_examined: 0,
                    total_games: 1,
                    matching_games: 0,
                    matches_found: 0,
                };

                on_progress(initial);

                if is_cancelled() {
                    return Ok(PatternSearchOutcome::Cancelled);
                }

                let matches = self.search_game_appearances(indexer, game_id, &query.pattern)?;

                let progress = PatternSearchProgress {
                    games_examined: 1,
                    total_games: 1,
                    matching_games: usize::from(!matches.is_empty()),
                    matches_found: matches.len(),
                };

                on_progress(progress);

                Ok(PatternSearchOutcome::Completed(matches))
            }

            PatternSearchScope::Project => self.search_database_with_progress(
                indexer,
                &query.pattern,
                is_cancelled,
                on_progress,
            ),
        }
    }

    pub fn search_summaries_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        query: &PatternSearchQuery,
        mut is_cancelled: C,
        mut on_progress: P,
    ) -> Result<PatternSearchSummaryOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        match query.scope {
            PatternSearchScope::Game(game_id) => {
                let initial = PatternSearchProgress {
                    games_examined: 0,
                    total_games: 1,
                    matching_games: 0,
                    matches_found: 0,
                };

                on_progress(initial);

                if is_cancelled() {
                    return Ok(PatternSearchSummaryOutcome::Cancelled);
                }

                let matches = self.search_game_appearances(indexer, game_id, &query.pattern)?;

                let progress = PatternSearchProgress {
                    games_examined: 1,
                    total_games: 1,
                    matching_games: usize::from(!matches.is_empty()),
                    matches_found: matches.len(),
                };

                on_progress(progress);

                let summaries = matches
                    .first()
                    .cloned()
                    .map(|first_match| {
                        vec![PatternGameSummary {
                            game_id,
                            match_count: matches.len(),
                            first_match,
                        }]
                    })
                    .unwrap_or_default();

                Ok(PatternSearchSummaryOutcome::Completed(summaries))
            }

            PatternSearchScope::Project => self.search_database_summaries_with_progress(
                indexer,
                &query.pattern,
                is_cancelled,
                on_progress,
            ),
        }
    }

    pub fn search_game(
        &self,
        indexer: &PositionIndexer,
        game_id: i64,
        pattern: &Pattern,
    ) -> Result<Vec<PatternMatch>> {
        let states = indexer.replay_game_states_by_id(game_id)?;

        let mut matches = Vec::new();

        for state in states {
            matches.extend(Self::search_position(
                game_id,
                state.occurrence.move_number,
                state.occurrence.side_to_move,
                state.occurrence.ko_point,
                &state.board,
                pattern,
            )?);
        }

        Ok(matches)
    }

    /// Collapse chronologically ordered raw matches into distinct continuous
    /// appearances.
    ///
    /// A match continues an existing appearance when the same pattern
    /// location was also present at the preceding board state. A later match
    /// after one or more non-matching states starts a new appearance.
    ///
    /// The current exact-pattern identity is:
    ///
    /// - game ID;
    /// - left coordinate;
    /// - bottom coordinate.
    ///
    /// Future transformed searches will extend this identity with the
    /// matching transformation and colour assignment.
    #[must_use]
    pub fn distinct_appearances(matches: Vec<PatternMatch>) -> Vec<PatternMatch> {
        let mut last_seen_move = HashMap::new();
        let mut appearances = Vec::new();

        for found in matches {
            let key = (found.game_id, found.left, found.bottom);

            let continues_existing =
                last_seen_move
                    .get(&key)
                    .is_some_and(|previous_move: &usize| {
                        found.move_number == *previous_move
                            || found.move_number == previous_move.saturating_add(1)
                    });

            last_seen_move.insert(key, found.move_number);

            if !continues_existing {
                appearances.push(found);
            }
        }

        appearances
    }

    pub fn search_game_appearances(
        &self,
        indexer: &PositionIndexer,
        game_id: i64,
        pattern: &Pattern,
    ) -> Result<Vec<PatternMatch>> {
        let raw_matches = self.search_game(indexer, game_id, pattern)?;

        Ok(Self::distinct_appearances(raw_matches))
    }

    pub fn search_database_summaries(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
    ) -> Result<Vec<PatternGameSummary>> {
        match self.search_database_summaries_with_progress(indexer, pattern, || false, |_| {})? {
            PatternSearchSummaryOutcome::Completed(summaries) => Ok(summaries),

            PatternSearchSummaryOutcome::Cancelled => {
                unreachable!("an uncancellable summary search was cancelled")
            }
        }
    }

    pub fn search_database_summaries_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        mut is_cancelled: C,
        mut on_progress: P,
    ) -> Result<PatternSearchSummaryOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        let game_ids = indexer.game_ids()?;
        let total_games = game_ids.len();

        let mut summaries = Vec::new();
        let mut matching_games = 0_usize;
        let mut matches_found = 0_usize;

        on_progress(PatternSearchProgress {
            games_examined: 0,
            total_games,
            matching_games,
            matches_found,
        });

        for (game_index, game_id) in game_ids.into_iter().enumerate() {
            if is_cancelled() {
                return Ok(PatternSearchSummaryOutcome::Cancelled);
            }

            /*
             * Matches for only this game are retained temporarily.
             * Once the count and first match have been recorded, the
             * complete per-game vector is released.
             */
            let game_matches = self.search_game_appearances(indexer, game_id, pattern)?;

            if let Some(first_match) = game_matches.first().cloned() {
                matching_games = matching_games.saturating_add(1);

                matches_found = matches_found.saturating_add(game_matches.len());

                summaries.push(PatternGameSummary {
                    game_id,
                    match_count: game_matches.len(),
                    first_match,
                });
            }

            on_progress(PatternSearchProgress {
                games_examined: game_index + 1,
                total_games,
                matching_games,
                matches_found,
            });
        }

        Ok(PatternSearchSummaryOutcome::Completed(summaries))
    }

    pub fn search_database(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
    ) -> Result<Vec<PatternMatch>> {
        match self.search_database_with_progress(indexer, pattern, || false, |_| {})? {
            PatternSearchOutcome::Completed(matches) => Ok(matches),

            PatternSearchOutcome::Cancelled => {
                unreachable!("an uncancellable database search was cancelled")
            }
        }
    }

    pub fn search_database_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        mut is_cancelled: C,
        mut on_progress: P,
    ) -> Result<PatternSearchOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        let game_ids = indexer.game_ids()?;
        let total_games = game_ids.len();

        let mut matches = Vec::new();
        let mut matching_games = 0_usize;
        let mut matches_found = 0_usize;

        on_progress(PatternSearchProgress {
            games_examined: 0,
            total_games,
            matching_games,
            matches_found,
        });

        for (game_index, game_id) in game_ids.into_iter().enumerate() {
            if is_cancelled() {
                return Ok(PatternSearchOutcome::Cancelled);
            }

            let game_matches = self.search_game_appearances(indexer, game_id, pattern)?;

            if !game_matches.is_empty() {
                matching_games = matching_games.saturating_add(1);
            }

            matches_found = matches_found.saturating_add(game_matches.len());

            matches.extend(game_matches);

            on_progress(PatternSearchProgress {
                games_examined: game_index + 1,
                total_games,
                matching_games,
                matches_found,
            });
        }

        Ok(PatternSearchOutcome::Completed(matches))
    }
}

impl Default for PatternSearcher {
    fn default() -> Self {
        Self::new()
    }
}
