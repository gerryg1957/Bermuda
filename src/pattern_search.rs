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

pub const NEXT_MOVE_DISPLAY_MARGIN: i16 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextMovePointCount {
    pub x: i16,
    pub y: i16,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextMoveDistribution {
    pub margin: i16,
    pub appearances: usize,
    pub matching_games: usize,
    pub points: Vec<NextMovePointCount>,
    pub outside_displayed_area: usize,
    pub passes: usize,
    pub game_ended: usize,
}

impl Default for NextMoveDistribution {
    fn default() -> Self {
        Self {
            margin: NEXT_MOVE_DISPLAY_MARGIN,
            appearances: 0,
            matching_games: 0,
            points: Vec::new(),
            outside_displayed_area: 0,
            passes: 0,
            game_ended: 0,
        }
    }
}

impl NextMoveDistribution {
    fn record_appearance(
        &mut self,
        point_counts: &mut HashMap<(i16, i16), usize>,
        pattern: &Pattern,
        found: &PatternMatch,
        board_size: u8,
        next_move: Option<crate::Move>,
    ) {
        self.appearances = self.appearances.saturating_add(1);

        let Some(next_move) = next_move else {
            self.game_ended = self.game_ended.saturating_add(1);
            return;
        };

        let Some(point) = next_move.point else {
            self.passes = self.passes.saturating_add(1);
            return;
        };

        let board_size = u16::from(board_size);

        debug_assert!(board_size > 0);

        let board_x =
            i16::try_from(point % board_size).expect("board x coordinate must fit in i16");

        let board_y =
            i16::try_from(point / board_size).expect("board y coordinate must fit in i16");

        let relative_x = board_x - i16::from(found.left);
        let relative_y = board_y - i16::from(found.bottom);

        let (normalised_x, normalised_y) = found.transformation.inverse_relative_point(
            relative_x,
            relative_y,
            pattern.width,
            pattern.height,
        );

        let inside_displayed_area = normalised_x >= -self.margin
            && normalised_y >= -self.margin
            && normalised_x < i16::from(pattern.width) + self.margin
            && normalised_y < i16::from(pattern.height) + self.margin;

        if inside_displayed_area {
            let count = point_counts
                .entry((normalised_x, normalised_y))
                .or_default();

            *count = count.saturating_add(1);
        } else {
            self.outside_displayed_area = self.outside_displayed_area.saturating_add(1);
        }
    }

    fn finish_points(&mut self, point_counts: HashMap<(i16, i16), usize>) {
        let mut points = point_counts
            .into_iter()
            .map(|((x, y), count)| NextMovePointCount { x, y, count })
            .collect::<Vec<_>>();

        points.sort_by_key(|point| (point.y, point.x));

        self.points = points;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternSearchSummaryReport {
    pub summaries: Vec<PatternGameSummary>,
    pub next_moves: NextMoveDistribution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternSearchSummaryReportOutcome {
    Completed(PatternSearchSummaryReport),
    Cancelled,
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
        let record = indexer.read_game_by_id(game_id)?;

        self.search_record_with_options(game_id, &record, pattern, options)
    }

    fn search_record_with_options(
        &self,
        game_id: i64,
        record: &crate::GameRecord,
        pattern: &Pattern,
        options: PatternSearchOptions,
    ) -> Result<Vec<PatternMatch>> {
        let states = crate::replay_positions(record)?;
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
        is_cancelled: C,
        on_progress: P,
    ) -> Result<PatternSearchSummaryOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        match self.search_database_summary_report_with_progress(
            indexer,
            pattern,
            options,
            is_cancelled,
            on_progress,
        )? {
            PatternSearchSummaryReportOutcome::Completed(report) => {
                Ok(PatternSearchSummaryOutcome::Completed(report.summaries))
            }

            PatternSearchSummaryReportOutcome::Cancelled => {
                Ok(PatternSearchSummaryOutcome::Cancelled)
            }
        }
    }

    pub fn search_database_summary_report(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        options: PatternSearchOptions,
    ) -> Result<PatternSearchSummaryReport> {
        match self.search_database_summary_report_with_progress(
            indexer,
            pattern,
            options,
            || false,
            |_| {},
        )? {
            PatternSearchSummaryReportOutcome::Completed(report) => Ok(report),

            PatternSearchSummaryReportOutcome::Cancelled => {
                unreachable!("an uncancellable summary report search was cancelled")
            }
        }
    }

    pub fn search_database_summary_report_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        options: PatternSearchOptions,
        mut is_cancelled: C,
        mut on_progress: P,
    ) -> Result<PatternSearchSummaryReportOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        let game_ids = indexer.game_ids()?;
        let total_games = game_ids.len();

        let mut summaries = Vec::new();
        let mut matching_games = 0_usize;
        let mut matches_found = 0_usize;

        let mut next_moves = NextMoveDistribution::default();
        let mut next_move_point_counts = HashMap::new();

        on_progress(PatternSearchProgress {
            games_examined: 0,
            total_games,
            matching_games,
            matches_found,
        });

        for (game_index, game_id) in game_ids.into_iter().enumerate() {
            if is_cancelled() {
                return Ok(PatternSearchSummaryReportOutcome::Cancelled);
            }

            /*
             * Load and replay the compact game record once. The same
             * record then supplies the move immediately following each
             * distinct appearance.
             */
            let record = indexer.read_game_by_id(game_id)?;

            let raw_matches =
                self.search_record_with_options(game_id, &record, pattern, options)?;

            let game_matches = Self::distinct_appearances(raw_matches);

            if let Some(first_match) = game_matches.first().cloned() {
                matching_games = matching_games.saturating_add(1);
                matches_found = matches_found.saturating_add(game_matches.len());

                next_moves.matching_games = next_moves.matching_games.saturating_add(1);

                for found in &game_matches {
                    /*
                     * A match at position N is followed by move N + 1,
                     * stored at zero-based record.moves[N].
                     */
                    let next_move = record.moves.get(found.move_number).copied();

                    next_moves.record_appearance(
                        &mut next_move_point_counts,
                        pattern,
                        found,
                        record.board_size,
                        next_move,
                    );
                }

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

        next_moves.finish_points(next_move_point_counts);

        Ok(PatternSearchSummaryReportOutcome::Completed(
            PatternSearchSummaryReport {
                summaries,
                next_moves,
            },
        ))
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
    #[test]
    fn next_move_distribution_normalises_and_classifies_moves() {
        let pattern = Pattern {
            width: 4,
            height: 5,
            cells: vec![crate::PatternCell::Empty; 20],
            edges: crate::BoardEdges {
                left: false,
                right: false,
                bottom: false,
                top: false,
            },
        };

        let identity_match = PatternMatch {
            game_id: 1,
            move_number: 10,
            side_to_move: Colour::Black,
            ko_point: None,
            left: 3,
            bottom: 4,
            transformation: PatternTransformation::Identity,
            colours_reversed: false,
        };

        let rotated_match = PatternMatch {
            game_id: 2,
            move_number: 20,
            side_to_move: Colour::White,
            ko_point: None,
            left: 7,
            bottom: 8,
            transformation: PatternTransformation::Rotate90Clockwise,
            colours_reversed: true,
        };

        let board_size = 19_u8;

        let point = |x: u8, y: u8| u16::from(y) * u16::from(board_size) + u16::from(x);

        let mut distribution = NextMoveDistribution::default();
        let mut point_counts = HashMap::new();

        distribution.matching_games = 2;

        /*
         * Identity match: absolute (4, 6) is relative (1, 2).
         */
        distribution.record_appearance(
            &mut point_counts,
            &pattern,
            &identity_match,
            board_size,
            Some(crate::Move {
                colour: Colour::Black,
                point: Some(point(4, 6)),
            }),
        );

        /*
         * For a 4 x 5 pattern, normalised (1, 2) becomes
         * transformed relative coordinate (2, 2) after a clockwise
         * quarter turn. At match origin (7, 8), that is (9, 10).
         */
        distribution.record_appearance(
            &mut point_counts,
            &pattern,
            &rotated_match,
            board_size,
            Some(crate::Move {
                colour: Colour::White,
                point: Some(point(9, 10)),
            }),
        );

        /*
         * x = 7 lies just beyond a width-four pattern plus the
         * three-intersection display margin.
         */
        distribution.record_appearance(
            &mut point_counts,
            &pattern,
            &identity_match,
            board_size,
            Some(crate::Move {
                colour: Colour::Black,
                point: Some(point(10, 6)),
            }),
        );

        distribution.record_appearance(
            &mut point_counts,
            &pattern,
            &identity_match,
            board_size,
            Some(crate::Move {
                colour: Colour::White,
                point: None,
            }),
        );

        distribution.record_appearance(
            &mut point_counts,
            &pattern,
            &identity_match,
            board_size,
            None,
        );

        distribution.finish_points(point_counts);

        assert_eq!(distribution.margin, 3);
        assert_eq!(distribution.appearances, 5);
        assert_eq!(distribution.matching_games, 2);

        assert_eq!(
            distribution.points,
            vec![NextMovePointCount {
                x: 1,
                y: 2,
                count: 2,
            }]
        );

        assert_eq!(distribution.outside_displayed_area, 1);
        assert_eq!(distribution.passes, 1);
        assert_eq!(distribution.game_ended, 1);

        let classified_total = distribution
            .points
            .iter()
            .map(|point| point.count)
            .sum::<usize>()
            + distribution.outside_displayed_area
            + distribution.passes
            + distribution.game_ended;

        assert_eq!(classified_total, distribution.appearances);
    }
}
