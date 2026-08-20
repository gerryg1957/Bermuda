use std::collections::HashMap;

use crate::{Colour, Pattern, PatternTransformation, indexer::PositionIndexer, read_move_file};
use anyhow::{Context, Result};
use rayon::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    pub game_id: i64,

    /// First board position at which this match/appearance exists.
    pub move_number: usize,

    /// Last consecutive board position at which the same appearance exists.
    pub last_move_number: usize,

    pub side_to_move: Colour,
    pub ko_point: Option<u16>,
    pub left: u8,
    pub bottom: u8,
    pub transformation: PatternTransformation,
    pub colours_reversed: bool,
}

impl PatternMatch {
    /// Number of moves for which a continuous appearance persists.
    ///
    /// A match present at only one board position has duration zero.
    #[must_use]
    pub fn duration_moves(&self) -> usize {
        self.last_move_number.saturating_sub(self.move_number)
    }
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
    ) -> Option<(i16, i16)> {
        self.appearances = self.appearances.saturating_add(1);

        let Some(next_move) = next_move else {
            self.game_ended = self.game_ended.saturating_add(1);
            return None;
        };

        let Some(point) = next_move.point else {
            self.passes = self.passes.saturating_add(1);
            return None;
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
            Some((normalised_x, normalised_y))
        } else {
            self.outside_displayed_area = self.outside_displayed_area.saturating_add(1);
            None
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
    pub continuation_game_ids: HashMap<(i16, i16), Vec<i64>>,
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
    pub include_handicap_games: bool,
    pub long_axis_edge_band: Option<u8>,
    pub max_match_move: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternBoardContext {
    pub left: u8,
    pub right: u8,
    pub bottom: u8,
    pub top: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternSearchQuery {
    pub pattern: Pattern,
    pub board_context: Option<PatternBoardContext>,
    pub scope: PatternSearchScope,
    pub options: PatternSearchOptions,
}

impl PatternBoardContext {
    #[must_use]
    pub fn transformed(self, transformation: PatternTransformation) -> Self {
        match transformation {
            PatternTransformation::Identity => self,

            PatternTransformation::Rotate90Clockwise => Self {
                left: self.bottom,
                right: self.top,
                bottom: self.right,
                top: self.left,
            },

            PatternTransformation::Rotate180 => Self {
                left: self.right,
                right: self.left,
                bottom: self.top,
                top: self.bottom,
            },

            PatternTransformation::Rotate270Clockwise => Self {
                left: self.top,
                right: self.bottom,
                bottom: self.left,
                top: self.right,
            },

            PatternTransformation::MirrorLeftRight => Self {
                left: self.right,
                right: self.left,
                bottom: self.bottom,
                top: self.top,
            },

            PatternTransformation::MirrorTopBottom => Self {
                left: self.left,
                right: self.right,
                bottom: self.top,
                top: self.bottom,
            },

            PatternTransformation::MirrorMainDiagonal => Self {
                left: self.bottom,
                right: self.top,
                bottom: self.left,
                top: self.right,
            },

            PatternTransformation::MirrorAntiDiagonal => Self {
                left: self.top,
                right: self.bottom,
                bottom: self.right,
                top: self.left,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PatternVariant {
    pattern: Pattern,
    board_context: Option<PatternBoardContext>,
    transformation: PatternTransformation,
    colours_reversed: bool,
    long_axis_edge_band: Option<u8>,
    black_rows: Vec<u64>,
    white_rows: Vec<u64>,
    any_rows: Vec<u64>,
    row_mask: u64,
}

impl PatternVariant {
    fn new(
        pattern: Pattern,
        board_context: Option<PatternBoardContext>,
        transformation: PatternTransformation,
        colours_reversed: bool,
    ) -> Self {
        let width = usize::from(pattern.width);
        let height = usize::from(pattern.height);

        let mut black_rows = vec![0u64; height];
        let mut white_rows = vec![0u64; height];
        let mut any_rows = vec![0u64; height];

        for y in 0..height {
            for x in 0..width {
                let bit = 1u64 << x;

                match pattern.cells[y * width + x] {
                    crate::pattern::PatternCell::Black => black_rows[y] |= bit,
                    crate::pattern::PatternCell::White => white_rows[y] |= bit,
                    crate::pattern::PatternCell::Any => any_rows[y] |= bit,
                    crate::pattern::PatternCell::Empty => {}
                }
            }
        }

        let row_mask = (1u64 << width) - 1;

        Self {
            pattern,
            board_context,
            transformation,
            colours_reversed,
            long_axis_edge_band: None,
            black_rows,
            white_rows,
            any_rows,
            row_mask,
        }
    }

    fn board_row_bits(words: &[u64], start: usize, width: usize, row_mask: u64) -> u64 {
        let word_index = start / 64;
        let shift = start % 64;

        let mut bits = words[word_index] >> shift;

        if shift + width > 64 {
            bits |= words[word_index + 1] << (64 - shift);
        }

        bits & row_mask
    }

    fn matching_lefts_at_bottom_words(
        &self,
        board_size: u8,
        black_words: &[u64],
        white_words: &[u64],
        bottom: u8,
        first_left: u8,
        last_left: u8,
    ) -> u64 {
        /*
         * Each bit represents one possible horizontal origin.  Pattern
         * constraints progressively clear origins that cannot match.
         *
         * For a pattern cell at horizontal offset x, board >> x places
         * the board cell at left + x into candidate bit "left".  We can
         * therefore test every horizontal placement with one bitwise AND.
         */
        let first_left = usize::from(first_left);
        let last_left = usize::from(last_left);
        let candidate_count = last_left - first_left + 1;

        let legal_lefts = ((1u64 << candidate_count) - 1) << first_left;

        let mut candidates = legal_lefts;

        let board_size = usize::from(board_size);
        let board_row_mask = (1u64 << board_size) - 1;

        for y in 0..usize::from(self.pattern.height) {
            let start = (usize::from(bottom) + y) * board_size;

            let black = Self::board_row_bits(black_words, start, board_size, board_row_mask);

            let white = Self::board_row_bits(white_words, start, board_size, board_row_mask);

            /*
             * Test occupied pattern points first.  These usually reject
             * most possible origins very quickly.
             */
            let mut required_black = self.black_rows[y];

            while required_black != 0 {
                let x = required_black.trailing_zeros();
                candidates &= black >> x;

                if candidates == 0 {
                    return 0;
                }

                required_black &= required_black - 1;
            }

            let mut required_white = self.white_rows[y];

            while required_white != 0 {
                let x = required_white.trailing_zeros();
                candidates &= white >> x;

                if candidates == 0 {
                    return 0;
                }

                required_white &= required_white - 1;
            }

            /*
             * Empty intersections are part of an exact pattern too.
             */
            let board_empty = (!(black | white)) & board_row_mask;

            let mut required_empty =
                self.row_mask & !(self.black_rows[y] | self.white_rows[y] | self.any_rows[y]);

            while required_empty != 0 {
                let x = required_empty.trailing_zeros();
                candidates &= board_empty >> x;

                if candidates == 0 {
                    return 0;
                }

                required_empty &= required_empty - 1;
            }
        }

        candidates & legal_lefts
    }

    fn matching_lefts_at_bottom(
        &self,
        board: &crate::Board,
        bottom: u8,
        first_left: u8,
        last_left: u8,
    ) -> u64 {
        self.matching_lefts_at_bottom_words(
            board.size(),
            board.black_words(),
            board.white_words(),
            bottom,
            first_left,
            last_left,
        )
    }

    #[cfg(test)]
    fn matching_lefts_at_bottom_indexed(
        &self,
        board_size: u8,
        position: &crate::pattern_index::PatternIndexedPosition,
        bottom: u8,
        first_left: u8,
        last_left: u8,
    ) -> u64 {
        self.matching_lefts_at_bottom_words(
            board_size,
            &position.black,
            &position.white,
            bottom,
            first_left,
            last_left,
        )
    }
}

pub struct PatternSearcher;

fn parallel_game_batch_size() -> usize {
    rayon::current_num_threads().saturating_mul(4).max(32)
}

fn exact_origin_range(
    max_origin: u8,
    touches_low_edge: bool,
    touches_high_edge: bool,
) -> Option<(u8, u8)> {
    match (touches_low_edge, touches_high_edge) {
        /*
         * Both edges can be touched only when the pattern spans the
         * complete board in this dimension.
         */
        (true, true) => (max_origin == 0).then_some((0, 0)),

        /*
         * Touch the low edge but explicitly not the high edge.
         */
        (true, false) => (max_origin > 0).then_some((0, 0)),

        /*
         * Touch the high edge but explicitly not the low edge.
         */
        (false, true) => (max_origin > 0).then_some((max_origin, max_origin)),

        /*
         * An interior pattern must remain clear of both board edges.
         */
        (false, false) => (max_origin >= 2).then_some((1, max_origin - 1)),
    }
}

fn long_axis_near_edge(
    board_size: u8,
    pattern_width: u8,
    pattern_height: u8,
    left: u8,
    bottom: u8,
    edge_band: Option<u8>,
) -> bool {
    let Some(edge_band) = edge_band else {
        return true;
    };

    let edge_band = edge_band.min(board_size);

    let short_axis_centre_near_edge = |origin: u8, size: u8| {
        if board_size == 0 || size == 0 || edge_band == 0 {
            return false;
        }

        /*
         * Work in doubled coordinates so even-width patterns have an
         * exact half-intersection centre without arbitrary rounding.
         *
         * For a 19 x 19 board and a five-line edge band, the centre of
         * the short axis must lie on or beyond line 5 or line 15.
         */
        let centre_twice = 2 * u16::from(origin) + u16::from(size) - 1;
        let low_edge_limit = 2 * (u16::from(edge_band) - 1);
        let high_edge_limit = 2 * u16::from(board_size - edge_band);

        centre_twice <= low_edge_limit || centre_twice >= high_edge_limit
    };

    if pattern_width > pattern_height {
        short_axis_centre_near_edge(bottom, pattern_height)
    } else if pattern_height > pattern_width {
        short_axis_centre_near_edge(left, pattern_width)
    } else {
        true
    }
}

fn is_bermuda_shape(pattern_width: u8, pattern_height: u8) -> bool {
    let long_side = u16::from(pattern_width.max(pattern_height));
    let short_side = u16::from(pattern_width.min(pattern_height));

    /*
     * Bermuda is deliberately a conservative geometric heuristic.
     *
     * Its purpose is to suppress obviously displaced matches when a long,
     * shallow selection beside a board edge would otherwise be translated
     * into a substantially different geometrical situation elsewhere on the
     * board. It is not a model of Go strategy, fuseki, move quality, or the
     * historical purpose of the stones in the selected position.
     *
     * These thresholds are pragmatic rather than theoretically exact. Compact
     * and borderline rectangles remain unrestricted: Bermuda is a precedent
     * finder, not a strategic evaluator. If practical use reveals a useful
     * class of precedents that this heuristic hides, the boundary should be
     * reconsidered from that evidence.
     */
    short_side > 0 && long_side >= 10 && long_side >= 2 * short_side + 2
}

#[must_use]
pub fn source_long_axis_edge_band(
    board_size: u8,
    pattern_width: u8,
    pattern_height: u8,
    left: u8,
    bottom: u8,
    edge_band: u8,
) -> Option<u8> {
    if !is_bermuda_shape(pattern_width, pattern_height) {
        return None;
    }

    let edge_band = edge_band.min(board_size);

    long_axis_near_edge(
        board_size,
        pattern_width,
        pattern_height,
        left,
        bottom,
        Some(edge_band),
    )
    .then_some(edge_band)
}

impl PatternSearcher {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn push_variant(
        variants: &mut Vec<PatternVariant>,
        pattern: Pattern,
        board_context: Option<PatternBoardContext>,
        transformation: PatternTransformation,
        colours_reversed: bool,
        long_axis_edge_band: Option<u8>,
    ) {
        if variants
            .iter()
            .any(|existing| existing.pattern == pattern && existing.board_context == board_context)
        {
            return;
        }

        let mut variant =
            PatternVariant::new(pattern, board_context, transformation, colours_reversed);
        variant.long_axis_edge_band = long_axis_edge_band;
        variants.push(variant);
    }

    fn search_variants(
        pattern: &Pattern,
        board_context: Option<PatternBoardContext>,
        options: PatternSearchOptions,
    ) -> Vec<PatternVariant> {
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

            let transformed_context =
                board_context.map(|context| context.transformed(transformation));

            Self::push_variant(
                &mut variants,
                transformed.clone(),
                transformed_context,
                transformation,
                false,
                options.long_axis_edge_band,
            );

            if options.include_reversed_colours {
                Self::push_variant(
                    &mut variants,
                    transformed.reversed_colours(),
                    transformed_context,
                    transformation,
                    true,
                    options.long_axis_edge_band,
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

        let horizontal_range = if let Some(context) = variant.board_context {
            let expected_left = context.left;

            let expected_right = max_left.saturating_sub(expected_left);

            if expected_left > max_left || expected_right != context.right {
                return Ok(matches);
            }

            Some((expected_left, expected_left))
        } else {
            exact_origin_range(
                max_left,
                variant.pattern.edges.left,
                variant.pattern.edges.right,
            )
        };

        let Some((first_left, last_left)) = horizontal_range else {
            return Ok(matches);
        };

        let vertical_range = if let Some(context) = variant.board_context {
            let expected_bottom = context.bottom;

            let expected_top = max_bottom.saturating_sub(expected_bottom);

            if expected_bottom > max_bottom || expected_top != context.top {
                return Ok(matches);
            }

            Some((expected_bottom, expected_bottom))
        } else {
            exact_origin_range(
                max_bottom,
                variant.pattern.edges.bottom,
                variant.pattern.edges.top,
            )
        };

        let Some((first_bottom, last_bottom)) = vertical_range else {
            return Ok(matches);
        };

        for bottom in first_bottom..=last_bottom {
            let mut matching_lefts =
                variant.matching_lefts_at_bottom(board, bottom, first_left, last_left);

            /*
             * trailing_zeros visits surviving origins from left to right,
             * preserving the result ordering of the old nested loop.
             */
            while matching_lefts != 0 {
                let left = matching_lefts.trailing_zeros() as u8;

                if long_axis_near_edge(
                    board.size(),
                    variant.pattern.width,
                    variant.pattern.height,
                    left,
                    bottom,
                    variant.long_axis_edge_band,
                ) {
                    matches.push(PatternMatch {
                        game_id,
                        move_number,
                        last_move_number: move_number,
                        side_to_move,
                        ko_point,
                        left,
                        bottom,
                        transformation: variant.transformation,
                        colours_reversed: variant.colours_reversed,
                    });
                }

                matching_lefts &= matching_lefts - 1;
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

                let matches = self.search_game_appearances_with_context(
                    indexer,
                    game_id,
                    &query.pattern,
                    query.board_context,
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

            PatternSearchScope::Project => self.search_database_with_context_with_progress(
                indexer,
                &query.pattern,
                query.board_context,
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

                let matches = self.search_game_appearances_with_context(
                    indexer,
                    game_id,
                    &query.pattern,
                    query.board_context,
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
                .search_database_summaries_with_context_with_progress(
                    indexer,
                    &query.pattern,
                    query.board_context,
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
        let variants = Self::search_variants(pattern, None, options);

        self.search_record_with_variants(game_id, record, &variants, options.max_match_move)
    }

    fn search_record_with_variants(
        &self,
        game_id: i64,
        record: &crate::GameRecord,
        variants: &[PatternVariant],
        max_match_move: Option<usize>,
    ) -> Result<Vec<PatternMatch>> {
        /*
         * Pattern searching needs only the current board.  It does not need
         * the PositionState fingerprint or a stored clone of every position,
         * so replay directly onto one live board.
         *
         * Replay continues beyond max_match_move so later illegal moves are
         * still detected, preserving the behaviour of replay_positions().
         */
        let mut board = crate::Board::new(record.board_size).context("creating replay board")?;

        for setup in &record.setup {
            match *setup {
                crate::SetupStone::Add { colour, point } => {
                    board.set_setup(colour, point)?;
                }

                crate::SetupStone::Remove { point } => {
                    board.clear_setup(point)?;
                }
            }
        }

        let initial_side = record
            .moves
            .first()
            .map(|mv| mv.colour)
            .unwrap_or(Colour::Black);

        let mut matches = Vec::new();

        /*
         * Position zero is part of the normal search stream.
         */
        for variant in variants {
            matches.extend(Self::search_position(
                game_id,
                0,
                initial_side,
                board.ko_point(),
                &board,
                variant,
            )?);
        }

        for (index, &mv) in record.moves.iter().enumerate() {
            board
                .play_archival(mv)
                .with_context(|| format!("replaying move {}", index + 1))?;

            let move_number = index + 1;

            if let Some(max_match_move) = max_match_move
                && move_number > max_match_move
            {
                continue;
            }

            let side_to_move = record
                .moves
                .get(index + 1)
                .map(|next| next.colour)
                .unwrap_or_else(|| mv.colour.opponent());

            for variant in variants {
                matches.extend(Self::search_position(
                    game_id,
                    move_number,
                    side_to_move,
                    board.ko_point(),
                    &board,
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
        /*
         * For each exact appearance identity, remember both the most
         * recently matched position and the index of the retained
         * appearance. This preserves the complete first..last span.
         */
        type AppearanceKey = (i64, u8, u8, PatternTransformation, bool);
        type AppearanceState = (usize, usize);

        let mut last_seen: HashMap<AppearanceKey, AppearanceState> = HashMap::new();
        let mut appearances: Vec<PatternMatch> = Vec::new();

        for found in matches {
            let key = (
                found.game_id,
                found.left,
                found.bottom,
                found.transformation,
                found.colours_reversed,
            );

            if let Some((previous_move, appearance_index)) = last_seen.get(&key).copied() {
                let continues_existing = found.move_number == previous_move
                    || found.move_number == previous_move.saturating_add(1);

                if continues_existing {
                    appearances[appearance_index].last_move_number = appearances[appearance_index]
                        .last_move_number
                        .max(found.move_number);

                    last_seen.insert(key, (found.move_number, appearance_index));
                    continue;
                }
            }

            let appearance_index = appearances.len();
            let move_number = found.move_number;

            appearances.push(found);
            last_seen.insert(key, (move_number, appearance_index));
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

    fn search_game_appearances_with_context(
        &self,
        indexer: &PositionIndexer,
        game_id: i64,
        pattern: &Pattern,
        board_context: Option<PatternBoardContext>,
        options: PatternSearchOptions,
    ) -> Result<Vec<PatternMatch>> {
        let record = indexer.read_game_by_id(game_id)?;
        let variants = Self::search_variants(pattern, board_context, options);

        let raw_matches =
            self.search_record_with_variants(game_id, &record, &variants, options.max_match_move)?;

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
        self.search_database_summaries_with_context_with_progress(
            indexer,
            pattern,
            None,
            PatternSearchOptions::default(),
            is_cancelled,
            on_progress,
        )
    }

    fn search_database_summaries_with_context_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        board_context: Option<PatternBoardContext>,
        options: PatternSearchOptions,
        is_cancelled: C,
        on_progress: P,
    ) -> Result<PatternSearchSummaryOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        match self.search_database_summary_report_with_context_with_progress(
            indexer,
            pattern,
            board_context,
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
        is_cancelled: C,
        on_progress: P,
    ) -> Result<PatternSearchSummaryReportOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        self.search_database_summary_report_with_context_with_progress(
            indexer,
            pattern,
            None,
            options,
            is_cancelled,
            on_progress,
        )
    }

    fn search_database_summary_report_with_context_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        board_context: Option<PatternBoardContext>,
        options: PatternSearchOptions,
        mut is_cancelled: C,
        mut on_progress: P,
    ) -> Result<PatternSearchSummaryReportOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        let games = indexer.games_for_pattern_search(options.include_handicap_games)?;
        let total_games = games.len();

        let mut summaries = Vec::new();
        let mut matching_games = 0_usize;
        let mut matches_found = 0_usize;

        let mut next_moves = NextMoveDistribution::default();
        let mut next_move_point_counts = HashMap::new();
        let mut continuation_game_ids = HashMap::<(i16, i16), Vec<i64>>::new();

        on_progress(PatternSearchProgress {
            games_examined: 0,
            total_games,
            matching_games,
            matches_found,
        });

        let variants = Self::search_variants(pattern, board_context, options);
        let batch_size = parallel_game_batch_size();
        let mut games_examined = 0_usize;

        for game_batch in games.chunks(batch_size) {
            if is_cancelled() {
                return Ok(PatternSearchSummaryReportOutcome::Cancelled);
            }

            /*
             * SQLite has already supplied all move-file paths in one query.
             * Each Rayon worker now loads and searches its own game, allowing
             * file I/O, replay and pattern matching to overlap.
             */
            let searched: Vec<Result<_>> = game_batch
                .par_iter()
                .map(|game| {
                    let record = read_move_file(&game.move_file)?;

                    let raw_matches = self.search_record_with_variants(
                        game.game_id,
                        &record,
                        &variants,
                        options.max_match_move,
                    )?;

                    Ok((record, Self::distinct_appearances(raw_matches)))
                })
                .collect();

            /*
             * Consume worker results in original game order so progress,
             * continuation statistics and visible result ordering remain
             * deterministic.
             */
            for (game, searched_game) in game_batch.iter().zip(searched) {
                if is_cancelled() {
                    return Ok(PatternSearchSummaryReportOutcome::Cancelled);
                }

                let game_id = game.game_id;
                let (record, game_matches) = searched_game?;

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

                        if let Some(point) = next_moves.record_appearance(
                            &mut next_move_point_counts,
                            pattern,
                            found,
                            record.board_size,
                            next_move,
                        ) {
                            let game_ids = continuation_game_ids.entry(point).or_default();
                            if !game_ids.contains(&game_id) {
                                game_ids.push(game_id);
                            }
                        }
                    }

                    summaries.push(PatternGameSummary {
                        game_id,
                        match_count: game_matches.len(),
                        first_match,
                    });
                }

                games_examined = games_examined.saturating_add(1);

                on_progress(PatternSearchProgress {
                    games_examined,
                    total_games,
                    matching_games,
                    matches_found,
                });
            }
        }

        next_moves.finish_points(next_move_point_counts);

        for game_ids in continuation_game_ids.values_mut() {
            game_ids.sort_unstable();
            game_ids.dedup();
        }

        Ok(PatternSearchSummaryReportOutcome::Completed(
            PatternSearchSummaryReport {
                summaries,
                next_moves,
                continuation_game_ids,
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
        self.search_database_with_context_with_progress(
            indexer,
            pattern,
            None,
            PatternSearchOptions::default(),
            is_cancelled,
            on_progress,
        )
    }

    fn search_database_with_context_with_progress<C, P>(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
        board_context: Option<PatternBoardContext>,
        options: PatternSearchOptions,
        mut is_cancelled: C,
        mut on_progress: P,
    ) -> Result<PatternSearchOutcome>
    where
        C: FnMut() -> bool,
        P: FnMut(PatternSearchProgress),
    {
        let games = indexer.games_for_pattern_search(options.include_handicap_games)?;
        let total_games = games.len();

        let mut matches = Vec::new();
        let mut matching_games = 0_usize;
        let mut matches_found = 0_usize;

        on_progress(PatternSearchProgress {
            games_examined: 0,
            total_games,
            matching_games,
            matches_found,
        });

        let variants = Self::search_variants(pattern, board_context, options);
        let batch_size = parallel_game_batch_size();
        let mut games_examined = 0_usize;

        for game_batch in games.chunks(batch_size) {
            if is_cancelled() {
                return Ok(PatternSearchOutcome::Cancelled);
            }

            let searched: Vec<Result<Vec<PatternMatch>>> = game_batch
                .par_iter()
                .map(|game| {
                    let record = read_move_file(&game.move_file)?;

                    self.search_record_with_variants(
                        game.game_id,
                        &record,
                        &variants,
                        options.max_match_move,
                    )
                    .map(Self::distinct_appearances)
                })
                .collect();

            for game_matches in searched {
                if is_cancelled() {
                    return Ok(PatternSearchOutcome::Cancelled);
                }

                let game_matches = game_matches?;

                if !game_matches.is_empty() {
                    matching_games = matching_games.saturating_add(1);
                }

                matches_found = matches_found.saturating_add(game_matches.len());
                matches.extend(game_matches);

                games_examined = games_examined.saturating_add(1);

                on_progress(PatternSearchProgress {
                    games_examined,
                    total_games,
                    matching_games,
                    matches_found,
                });
            }
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
    fn fast_matcher_treats_any_as_wildcard_but_empty_as_empty() {
        let mut board = crate::Board::new(19).unwrap();

        let black_point = board.point(6, 5).unwrap();
        board.set_setup(crate::Colour::Black, black_point).unwrap();

        let white_point = board.point(7, 5).unwrap();
        board.set_setup(crate::Colour::White, white_point).unwrap();

        let pattern = |cell| Pattern {
            width: 1,
            height: 1,
            cells: vec![cell],
            edges: BoardEdges {
                left: false,
                right: false,
                bottom: false,
                top: false,
            },
        };

        let any_variant = PatternVariant::new(
            pattern(PatternCell::Any),
            None,
            PatternTransformation::Identity,
            false,
        );

        let any_matches = any_variant.matching_lefts_at_bottom(&board, 5, 5, 7);

        assert_eq!(any_matches, (1u64 << 5) | (1u64 << 6) | (1u64 << 7));

        let empty_variant = PatternVariant::new(
            pattern(PatternCell::Empty),
            None,
            PatternTransformation::Identity,
            false,
        );

        let empty_matches = empty_variant.matching_lefts_at_bottom(&board, 5, 5, 7);

        assert_eq!(empty_matches, 1u64 << 5);
    }

    #[test]
    fn packed_indexed_position_matches_like_replayed_board() {
        use crate::pattern_index::pattern_positions_from_record;
        use crate::replay_positions;

        let collection = crate::parse_collection(
            b"(;FF[4]GM[1]SZ[19]
                ;B[pd]
                ;W[dd]
                ;B[qp]
                ;W[dp])",
        )
        .unwrap();

        let game = crate::extract_main_variation(&collection).unwrap();

        let replayed = replay_positions(&game).unwrap();
        let indexed = pattern_positions_from_record(42, &game).unwrap();

        let variant = PatternVariant::new(
            asymmetric_pattern(),
            None,
            PatternTransformation::Identity,
            false,
        );

        assert_eq!(replayed.len(), indexed.len());

        for (state, packed) in replayed.iter().zip(&indexed) {
            for bottom in 0..=19 - variant.pattern.height {
                let last_left = 19 - variant.pattern.width;

                let board_matches =
                    variant.matching_lefts_at_bottom(&state.board, bottom, 0, last_left);

                let packed_matches =
                    variant.matching_lefts_at_bottom_indexed(19, packed, bottom, 0, last_left);

                assert_eq!(
                    packed_matches, board_matches,
                    "packed matcher differs at move {} bottom {}",
                    state.occurrence.move_number, bottom
                );
            }
        }
    }

    #[test]
    fn bermuda_shape_only_accepts_clearly_elongated_patterns() {
        assert!(!super::is_bermuda_shape(4, 4));
        assert!(!super::is_bermuda_shape(5, 6));
        assert!(!super::is_bermuda_shape(6, 5));
        assert!(!super::is_bermuda_shape(8, 4));
        assert!(!super::is_bermuda_shape(4, 8));
        assert!(!super::is_bermuda_shape(8, 3));
        assert!(!super::is_bermuda_shape(3, 8));
        assert!(!super::is_bermuda_shape(9, 4));
        assert!(!super::is_bermuda_shape(4, 9));

        assert!(super::is_bermuda_shape(10, 4));
        assert!(super::is_bermuda_shape(4, 10));
        assert!(super::is_bermuda_shape(10, 3));
        assert!(super::is_bermuda_shape(3, 10));
        assert!(super::is_bermuda_shape(16, 3));
        assert!(super::is_bermuda_shape(3, 16));
    }

    #[test]
    fn source_long_axis_edge_band_ignores_square_patterns() {
        assert_eq!(super::source_long_axis_edge_band(19, 4, 4, 0, 0, 5), None);
    }

    #[test]
    fn source_long_axis_edge_band_detects_horizontal_edge_pattern() {
        assert_eq!(
            super::source_long_axis_edge_band(19, 10, 3, 3, 0, 5),
            Some(5)
        );

        assert_eq!(super::source_long_axis_edge_band(19, 10, 3, 3, 8, 5), None);
    }

    #[test]
    fn source_long_axis_edge_band_detects_vertical_edge_pattern() {
        assert_eq!(
            super::source_long_axis_edge_band(19, 3, 10, 0, 3, 5),
            Some(5)
        );

        assert_eq!(super::source_long_axis_edge_band(19, 3, 10, 8, 3, 5), None);
    }

    #[test]
    fn long_axis_edge_band_uses_horizontal_shape_centre() {
        assert!(super::long_axis_near_edge(19, 16, 3, 1, 3, None));
        assert!(super::long_axis_near_edge(19, 16, 3, 1, 3, Some(5)));
        assert!(!super::long_axis_near_edge(19, 16, 3, 1, 4, Some(5)));
        assert!(!super::long_axis_near_edge(19, 16, 3, 1, 12, Some(5)));
        assert!(super::long_axis_near_edge(19, 16, 3, 1, 13, Some(5)));
    }

    #[test]
    fn long_axis_edge_band_uses_vertical_shape_centre() {
        assert!(super::long_axis_near_edge(19, 3, 16, 3, 1, Some(5)));
        assert!(!super::long_axis_near_edge(19, 3, 16, 4, 1, Some(5)));
        assert!(!super::long_axis_near_edge(19, 3, 16, 12, 1, Some(5)));
        assert!(super::long_axis_near_edge(19, 3, 16, 13, 1, Some(5)));
    }

    #[test]
    fn long_axis_edge_band_handles_even_short_dimension_symmetrically() {
        assert!(super::long_axis_near_edge(19, 2, 16, 3, 1, Some(5)));
        assert!(!super::long_axis_near_edge(19, 2, 16, 4, 1, Some(5)));
        assert!(!super::long_axis_near_edge(19, 2, 16, 13, 1, Some(5)));
        assert!(super::long_axis_near_edge(19, 2, 16, 14, 1, Some(5)));
    }

    #[test]
    fn long_axis_edge_band_does_not_restrict_square_patterns() {
        assert!(super::long_axis_near_edge(19, 5, 5, 7, 7, Some(5)));
    }

    #[test]
    fn search_variants_keep_long_axis_edge_band() {
        let variants = PatternSearcher::search_variants(
            &asymmetric_pattern(),
            None,
            PatternSearchOptions {
                long_axis_edge_band: Some(5),
                ..PatternSearchOptions::default()
            },
        );

        assert!(!variants.is_empty());
        assert!(
            variants
                .iter()
                .all(|variant| variant.long_axis_edge_band == Some(5))
        );
    }

    #[test]
    fn default_options_generate_only_the_exact_pattern() {
        let variants = PatternSearcher::search_variants(
            &asymmetric_pattern(),
            None,
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
            None,
            PatternSearchOptions {
                include_rotations: true,
                include_reflections: true,
                include_reversed_colours: true,
                include_handicap_games: false,
                long_axis_edge_band: None,
                max_match_move: None,
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
            None,
            PatternSearchOptions {
                include_rotations: true,
                include_reflections: true,
                include_reversed_colours: true,
                include_handicap_games: false,
                long_axis_edge_band: None,
                max_match_move: None,
            },
        );

        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn transformed_matches_have_separate_appearance_identity() {
        let base = PatternMatch {
            game_id: 1,
            move_number: 10,
            last_move_number: 10,
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
            last_move_number: 10,
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
            last_move_number: 20,
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
