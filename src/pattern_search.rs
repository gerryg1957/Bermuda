use std::collections::HashMap;

use crate::{Colour, Pattern, PatternTransformation, indexer::PositionIndexer};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    pub game_id: i64,
    pub move_number: usize,
    pub side_to_move: Colour,
    pub ko_point: Option<u16>,
    pub left: u8,
    pub bottom: u8,
    pub transformation: PatternTransformation,
    pub colours_reversed: bool,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PatternSearchOptions {
    pub include_rotations: bool,
    pub include_reflections: bool,
    pub include_reversed_colours: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternSearchQuery {
    pub pattern: Pattern,
    pub scope: PatternSearchScope,
    pub options: PatternSearchOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PatternVariant {
    pattern: Pattern,
    transformation: PatternTransformation,
    colours_reversed: bool,
}

pub struct PatternSearcher;

impl PatternSearcher {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn push_variant(
        variants: &mut Vec<PatternVariant>,
        pattern: Pattern,
        transformation: PatternTransformation,
        colours_reversed: bool,
    ) {
        if variants.iter().any(|existing| existing.pattern == pattern) {
            return;
        }

        variants.push(PatternVariant {
            pattern,
            transformation,
            colours_reversed,
        });
    }

    fn search_variants(pattern: &Pattern, options: PatternSearchOptions) -> Vec<PatternVariant> {
        let transformations = [
            PatternTransformation::Identity,
            PatternTransformation::Rotate90Clockwise,
            PatternTransformation::Rotate180,
            PatternTransformation::Rotate270Clockwise,
            PatternTransformation::MirrorLeftRight,
            PatternTransformation::MirrorTopBottom,
            PatternTransformation::MirrorMainDiagonal,
            PatternTransformation::MirrorAntiDiagonal,
        ];

        let mut variants = Vec::new();

        for transformation in transformations {
            let enabled = match transformation {
                PatternTransformation::Identity => true,

                PatternTransformation::Rotate90Clockwise
                | PatternTransformation::Rotate180
                | PatternTransformation::Rotate270Clockwise => options.include_rotations,

                PatternTransformation::MirrorLeftRight
                | PatternTransformation::MirrorTopBottom
                | PatternTransformation::MirrorMainDiagonal
                | PatternTransformation::MirrorAntiDiagonal => options.include_reflections,
            };

            if !enabled {
                continue;
            }

            let transformed = pattern.transformed(transformation);

            Self::push_variant(&mut variants, transformed.clone(), transformation, false);

            if options.include_reversed_colours {
                Self::push_variant(
                    &mut variants,
                    transformed.reversed_colours(),
                    transformation,
                    true,
                );
            }
        }

        variants
    }

    fn search_position(
        game_id: i64,
        move_number: usize,
        side_to_move: Colour,
        ko_point: Option<u16>,
        board: &crate::Board,
        variant: &PatternVariant,
    ) -> Result<Vec<PatternMatch>> {
        let mut matches = Vec::new();

        if variant.pattern.width > board.size() || variant.pattern.height > board.size() {
            return Ok(matches);
        }

        let max_left = board.size() - variant.pattern.width;
        let max_bottom = board.size() - variant.pattern.height;

        for bottom in 0..=max_bottom {
            for left in 0..=max_left {
                if variant.pattern.matches_at(board, left, bottom)? {
                    matches.push(PatternMatch {
                        game_id,
                        move_number,
                        side_to_move,
                        ko_point,
                        left,
                        bottom,
                        transformation: variant.transformation,
                        colours_reversed: variant.colours_reversed,
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

                let matches = self.search_game_appearances_with_options(
                    indexer,
                    game_id,
                    &query.pattern,
                    query.options,
                )?;

                let progress = PatternSearchProgress {
                    games_examined: 1,
                    total_games: 1,
                    matching_games: usize::from(!matches.is_empty()),
                    matches_found: matches.len(),
                };

                on_progress(progress);

                Ok(PatternSearchOutcome::Completed(matches))
            }

            PatternSearchScope::Project => self.search_database_with_options_with_progress(
                indexer,
                &query.pattern,
                query.options,
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

                let matches = self.search_game_appearances_with_options(
                    indexer,
                    game_id,
                    &query.pattern,
                    query.options,
                )?;

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

            PatternSearchScope::Project => self
                .search_database_summaries_with_options_with_progress(
                    indexer,
                    &query.pattern,
                    query.options,
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
        self.search_game_with_options(indexer, game_id, pattern, PatternSearchOptions::default())
    }

    pub fn search_game_with_options(
        &self,
        indexer: &PositionIndexer,
        game_id: i64,
        pattern: &Pattern,
        options: PatternSearchOptions,
    ) -> Result<Vec<PatternMatch>> {
        let states = indexer.replay_game_states_by_id(game_id)?;
        let variants = Self::search_variants(pattern, options);

        let mut matches = Vec::new();

        for state in states {
            for variant in &variants {
                matches.extend(Self::search_position(
                    game_id,
                    state.occurrence.move_number,
                    state.occurrence.side_to_move,
                    state.occurrence.ko_point,
                    &state.board,
                    variant,
                )?);
            }
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
    /// - bottom coordinate;
    /// - matching transformation;
    /// - colour assignment.
    #[must_use]
    pub fn distinct_appearances(matches: Vec<PatternMatch>) -> Vec<PatternMatch> {
        let mut last_seen_move = HashMap::new();
        let mut appearances = Vec::new();

        for found in matches {
            let key = (
                found.game_id,
                found.left,
                found.bottom,
                found.transformation,
                found.colours_reversed,
            );

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
        self.search_game_appearances_with_options(
            indexer,
            game_id,
            pattern,
            PatternSearchOptions::default(),
        )
    }

    pub fn search_game_appearances_with_options(
        &self,
        indexer: &PositionIndexer,
        game_id: i64,
        pattern: &Pattern,
        options: PatternSearchOptions,
    ) -> Result<Vec<PatternMatch>> {
        let raw_matches = self.search_game_with_options(indexer, game_id, pattern, options)?;

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
        is_cancelled: C,
        on_progress: P,
    ) -> Result<PatternSearchSummaryOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        self.search_database_summaries_with_options_with_progress(
            indexer,
            pattern,
            PatternSearchOptions::default(),
            is_cancelled,
            on_progress,
        )
    }

    fn search_database_summaries_with_options_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        options: PatternSearchOptions,
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
            let game_matches =
                self.search_game_appearances_with_options(indexer, game_id, pattern, options)?;

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
        is_cancelled: C,
        on_progress: P,
    ) -> Result<PatternSearchOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        self.search_database_with_options_with_progress(
            indexer,
            pattern,
            PatternSearchOptions::default(),
            is_cancelled,
            on_progress,
        )
    }

    fn search_database_with_options_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        options: PatternSearchOptions,
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

            let game_matches =
                self.search_game_appearances_with_options(indexer, game_id, pattern, options)?;

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

#[cfg(test)]
mod option_tests {
    use super::*;
    use crate::{BoardEdges, PatternCell};

    fn asymmetric_pattern() -> Pattern {
        Pattern {
            width: 2,
            height: 3,
            cells: vec![
                PatternCell::Black,
                PatternCell::White,
                PatternCell::Empty,
                PatternCell::Black,
                PatternCell::White,
                PatternCell::Empty,
            ],
            edges: BoardEdges {
                left: true,
                right: false,
                bottom: false,
                top: true,
            },
        }
    }

    #[test]
    fn default_options_generate_only_the_exact_pattern() {
        let variants = PatternSearcher::search_variants(
            &asymmetric_pattern(),
            PatternSearchOptions::default(),
        );

        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].transformation, PatternTransformation::Identity);
        assert!(!variants[0].colours_reversed);
    }

    #[test]
    fn enabled_options_generate_transformed_and_reversed_variants() {
        let variants = PatternSearcher::search_variants(
            &asymmetric_pattern(),
            PatternSearchOptions {
                include_rotations: true,
                include_reflections: true,
                include_reversed_colours: true,
            },
        );

        assert!(variants.iter().any(|variant| {
            variant.transformation == PatternTransformation::Rotate90Clockwise
                && !variant.colours_reversed
        }));

        assert!(variants.iter().any(|variant| {
            variant.transformation == PatternTransformation::MirrorMainDiagonal
                && !variant.colours_reversed
        }));

        assert!(variants.iter().any(|variant| {
            variant.transformation == PatternTransformation::Identity && variant.colours_reversed
        }));
    }

    #[test]
    fn equivalent_variants_are_deduplicated() {
        let pattern = Pattern {
            width: 2,
            height: 2,
            cells: vec![PatternCell::Empty; 4],
            edges: BoardEdges {
                left: false,
                right: false,
                bottom: false,
                top: false,
            },
        };

        let variants = PatternSearcher::search_variants(
            &pattern,
            PatternSearchOptions {
                include_rotations: true,
                include_reflections: true,
                include_reversed_colours: true,
            },
        );

        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn transformed_matches_have_separate_appearance_identity() {
        let base = PatternMatch {
            game_id: 1,
            move_number: 10,
            side_to_move: Colour::Black,
            ko_point: None,
            left: 3,
            bottom: 4,
            transformation: PatternTransformation::Identity,
            colours_reversed: false,
        };

        let mut rotated = base.clone();
        rotated.transformation = PatternTransformation::Rotate90Clockwise;

        let mut reversed = base.clone();
        reversed.colours_reversed = true;

        assert_eq!(
            PatternSearcher::distinct_appearances(vec![
                base.clone(),
                rotated.clone(),
                reversed.clone(),
            ]),
            vec![base, rotated, reversed]
        );
    }
}
