use crate::{Board, Colour};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternCell {
    Empty,
    Black,
    White,
}

impl From<Option<Colour>> for PatternCell {
    fn from(value: Option<Colour>) -> Self {
        match value {
            Some(Colour::Black) => Self::Black,
            Some(Colour::White) => Self::White,
            None => Self::Empty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRect {
    pub left: u8,
    pub bottom: u8,
    pub width: u8,
    pub height: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardEdges {
    pub left: bool,
    pub right: bool,
    pub bottom: bool,
    pub top: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternTransformation {
    Identity,
    Rotate90Clockwise,
    Rotate180,
    Rotate270Clockwise,
    MirrorLeftRight,
    MirrorTopBottom,
    MirrorMainDiagonal,
    MirrorAntiDiagonal,
}

impl PatternTransformation {
    #[must_use]
    pub fn swaps_dimensions(self) -> bool {
        matches!(
            self,
            Self::Rotate90Clockwise
                | Self::Rotate270Clockwise
                | Self::MirrorMainDiagonal
                | Self::MirrorAntiDiagonal
        )
    }

    #[must_use]
    pub fn transformed_dimensions(self, width: u8, height: u8) -> (u8, u8) {
        if self.swaps_dimensions() {
            (height, width)
        } else {
            (width, height)
        }
    }

    /// Transform coordinates relative to the pattern's bottom-left corner.
    ///
    /// Coordinates are signed deliberately: continuation heat maps may
    /// include moves lying outside the pattern rectangle.
    #[must_use]
    pub fn transform_relative_point(self, x: i16, y: i16, width: u8, height: u8) -> (i16, i16) {
        let width = i16::from(width);
        let height = i16::from(height);

        match self {
            Self::Identity => (x, y),

            Self::Rotate90Clockwise => (y, width - 1 - x),

            Self::Rotate180 => (width - 1 - x, height - 1 - y),

            Self::Rotate270Clockwise => (height - 1 - y, x),

            Self::MirrorLeftRight => (width - 1 - x, y),

            Self::MirrorTopBottom => (x, height - 1 - y),

            Self::MirrorMainDiagonal => (y, x),

            Self::MirrorAntiDiagonal => (height - 1 - y, width - 1 - x),
        }
    }

    #[must_use]
    pub fn inverse(self) -> Self {
        match self {
            Self::Rotate90Clockwise => Self::Rotate270Clockwise,
            Self::Rotate270Clockwise => Self::Rotate90Clockwise,
            other => other,
        }
    }

    /// Convert a coordinate from a transformed match back into the
    /// orientation of the original query pattern.
    #[must_use]
    pub fn inverse_relative_point(
        self,
        x: i16,
        y: i16,
        original_width: u8,
        original_height: u8,
    ) -> (i16, i16) {
        let (transformed_width, transformed_height) =
            self.transformed_dimensions(original_width, original_height);

        self.inverse()
            .transform_relative_point(x, y, transformed_width, transformed_height)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<PatternCell>,
    pub edges: BoardEdges,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PatternError {
    #[error("pattern rectangle must have non-zero width and height")]
    EmptyRectangle,

    #[error("pattern rectangle lies outside the board")]
    RectangleOutsideBoard,
}

impl Pattern {
    pub fn extract(board: &Board, rect: PatternRect) -> Result<Self, PatternError> {
        if rect.width == 0 || rect.height == 0 {
            return Err(PatternError::EmptyRectangle);
        }

        let right = rect
            .left
            .checked_add(rect.width)
            .ok_or(PatternError::RectangleOutsideBoard)?;

        let top = rect
            .bottom
            .checked_add(rect.height)
            .ok_or(PatternError::RectangleOutsideBoard)?;

        if right > board.size() || top > board.size() {
            return Err(PatternError::RectangleOutsideBoard);
        }

        let mut cells = Vec::with_capacity(usize::from(rect.width) * usize::from(rect.height));

        for y in rect.bottom..top {
            for x in rect.left..right {
                let point = board
                    .point(x, y)
                    .expect("validated pattern coordinates must lie on board");

                cells.push(PatternCell::from(board.colour_at(point)));
            }
        }

        Ok(Self {
            width: rect.width,
            height: rect.height,
            cells,
            edges: BoardEdges {
                left: rect.left == 0,
                right: right == board.size(),
                bottom: rect.bottom == 0,
                top: top == board.size(),
            },
        })
    }

    #[must_use]
    pub fn transformed(&self, transformation: PatternTransformation) -> Self {
        let swaps_dimensions = matches!(
            transformation,
            PatternTransformation::Rotate90Clockwise
                | PatternTransformation::Rotate270Clockwise
                | PatternTransformation::MirrorMainDiagonal
                | PatternTransformation::MirrorAntiDiagonal
        );

        let (width, height) = if swaps_dimensions {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };

        let mut cells = vec![PatternCell::Empty; usize::from(width) * usize::from(height)];

        for source_y in 0..self.height {
            for source_x in 0..self.width {
                let source_index =
                    usize::from(source_y) * usize::from(self.width) + usize::from(source_x);

                let (target_x, target_y) = match transformation {
                    PatternTransformation::Identity => (source_x, source_y),

                    PatternTransformation::Rotate90Clockwise => {
                        (source_y, self.width - 1 - source_x)
                    }

                    PatternTransformation::Rotate180 => {
                        (self.width - 1 - source_x, self.height - 1 - source_y)
                    }

                    PatternTransformation::Rotate270Clockwise => {
                        (self.height - 1 - source_y, source_x)
                    }

                    PatternTransformation::MirrorLeftRight => (self.width - 1 - source_x, source_y),

                    PatternTransformation::MirrorTopBottom => {
                        (source_x, self.height - 1 - source_y)
                    }

                    PatternTransformation::MirrorMainDiagonal => (source_y, source_x),

                    PatternTransformation::MirrorAntiDiagonal => {
                        (self.height - 1 - source_y, self.width - 1 - source_x)
                    }
                };

                let target_index =
                    usize::from(target_y) * usize::from(width) + usize::from(target_x);

                cells[target_index] = self.cells[source_index];
            }
        }

        let edges = match transformation {
            PatternTransformation::Identity => self.edges,

            PatternTransformation::Rotate90Clockwise => BoardEdges {
                left: self.edges.bottom,
                right: self.edges.top,
                bottom: self.edges.right,
                top: self.edges.left,
            },

            PatternTransformation::Rotate180 => BoardEdges {
                left: self.edges.right,
                right: self.edges.left,
                bottom: self.edges.top,
                top: self.edges.bottom,
            },

            PatternTransformation::Rotate270Clockwise => BoardEdges {
                left: self.edges.top,
                right: self.edges.bottom,
                bottom: self.edges.left,
                top: self.edges.right,
            },

            PatternTransformation::MirrorLeftRight => BoardEdges {
                left: self.edges.right,
                right: self.edges.left,
                bottom: self.edges.bottom,
                top: self.edges.top,
            },

            PatternTransformation::MirrorTopBottom => BoardEdges {
                left: self.edges.left,
                right: self.edges.right,
                bottom: self.edges.top,
                top: self.edges.bottom,
            },

            PatternTransformation::MirrorMainDiagonal => BoardEdges {
                left: self.edges.bottom,
                right: self.edges.top,
                bottom: self.edges.left,
                top: self.edges.right,
            },

            PatternTransformation::MirrorAntiDiagonal => BoardEdges {
                left: self.edges.top,
                right: self.edges.bottom,
                bottom: self.edges.right,
                top: self.edges.left,
            },
        };

        Self {
            width,
            height,
            cells,
            edges,
        }
    }

    #[must_use]
    pub fn reversed_colours(&self) -> Self {
        let cells = self
            .cells
            .iter()
            .map(|cell| match cell {
                PatternCell::Empty => PatternCell::Empty,
                PatternCell::Black => PatternCell::White,
                PatternCell::White => PatternCell::Black,
            })
            .collect();

        Self {
            width: self.width,
            height: self.height,
            cells,
            edges: self.edges,
        }
    }

    pub fn matches_at(&self, board: &Board, left: u8, bottom: u8) -> Result<bool, PatternError> {
        if self.width == 0 || self.height == 0 {
            return Err(PatternError::EmptyRectangle);
        }

        let right = left
            .checked_add(self.width)
            .ok_or(PatternError::RectangleOutsideBoard)?;

        let top = bottom
            .checked_add(self.height)
            .ok_or(PatternError::RectangleOutsideBoard)?;

        if right > board.size() || top > board.size() {
            return Err(PatternError::RectangleOutsideBoard);
        }

        let expected_cells = usize::from(self.width) * usize::from(self.height);
        if self.cells.len() != expected_cells {
            return Ok(false);
        }

        let candidate_edges = BoardEdges {
            left: left == 0,
            right: right == board.size(),
            bottom: bottom == 0,
            top: top == board.size(),
        };

        if self.edges != candidate_edges {
            return Ok(false);
        }

        for relative_y in 0..self.height {
            for relative_x in 0..self.width {
                let index =
                    usize::from(relative_y) * usize::from(self.width) + usize::from(relative_x);

                let point = board
                    .point(left + relative_x, bottom + relative_y)
                    .expect("validated pattern coordinates must lie on board");

                if self.cells[index] != PatternCell::from(board.colour_at(point)) {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Board, Colour};

    #[test]
    fn transforms_relative_coordinates() {
        let width = 4;
        let height = 5;
        let point = (1, 2);

        assert_eq!(
            PatternTransformation::Identity
                .transform_relative_point(point.0, point.1, width, height),
            (1, 2)
        );

        assert_eq!(
            PatternTransformation::Rotate90Clockwise
                .transform_relative_point(point.0, point.1, width, height),
            (2, 2)
        );

        assert_eq!(
            PatternTransformation::Rotate180
                .transform_relative_point(point.0, point.1, width, height),
            (2, 2)
        );

        assert_eq!(
            PatternTransformation::Rotate270Clockwise
                .transform_relative_point(point.0, point.1, width, height),
            (2, 1)
        );

        assert_eq!(
            PatternTransformation::MirrorLeftRight
                .transform_relative_point(point.0, point.1, width, height),
            (2, 2)
        );

        assert_eq!(
            PatternTransformation::MirrorTopBottom
                .transform_relative_point(point.0, point.1, width, height),
            (1, 2)
        );

        assert_eq!(
            PatternTransformation::MirrorMainDiagonal
                .transform_relative_point(point.0, point.1, width, height),
            (2, 1)
        );

        assert_eq!(
            PatternTransformation::MirrorAntiDiagonal
                .transform_relative_point(point.0, point.1, width, height),
            (2, 2)
        );
    }

    #[test]
    fn inverse_coordinate_transform_restores_inside_and_outside_points() {
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

        let points = [(-3, -2), (0, 0), (1, 2), (3, 4), (6, 7)];

        for transformation in transformations {
            for point in points {
                let transformed = transformation.transform_relative_point(point.0, point.1, 4, 5);

                let restored =
                    transformation.inverse_relative_point(transformed.0, transformed.1, 4, 5);

                assert_eq!(restored, point, "{transformation:?} failed for {point:?}");
            }
        }
    }

    #[test]
    fn reports_transformed_dimensions() {
        assert_eq!(
            PatternTransformation::Identity.transformed_dimensions(4, 5),
            (4, 5)
        );

        assert_eq!(
            PatternTransformation::Rotate90Clockwise.transformed_dimensions(4, 5),
            (5, 4)
        );

        assert_eq!(
            PatternTransformation::MirrorMainDiagonal.transformed_dimensions(4, 5),
            (5, 4)
        );
    }

    #[test]
    fn extracts_centre_pattern() {
        let mut board = Board::new(19).unwrap();

        board
            .set_setup(Colour::Black, board.point(9, 9).unwrap())
            .unwrap();
        board
            .set_setup(Colour::White, board.point(10, 9).unwrap())
            .unwrap();

        let pattern = Pattern::extract(
            &board,
            PatternRect {
                left: 8,
                bottom: 8,
                width: 3,
                height: 3,
            },
        )
        .unwrap();

        assert_eq!(pattern.width, 3);
        assert_eq!(pattern.height, 3);

        assert_eq!(
            pattern.edges,
            BoardEdges {
                left: false,
                right: false,
                bottom: false,
                top: false,
            }
        );
    }

    #[test]
    fn extracts_corner_pattern() {
        let board = Board::new(19).unwrap();

        let pattern = Pattern::extract(
            &board,
            PatternRect {
                left: 0,
                bottom: 0,
                width: 4,
                height: 4,
            },
        )
        .unwrap();

        assert!(pattern.edges.left);
        assert!(pattern.edges.bottom);
        assert!(!pattern.edges.right);
        assert!(!pattern.edges.top);
    }

    #[test]
    fn extracts_side_pattern() {
        let board = Board::new(19).unwrap();

        let pattern = Pattern::extract(
            &board,
            PatternRect {
                left: 0,
                bottom: 5,
                width: 4,
                height: 4,
            },
        )
        .unwrap();

        assert!(pattern.edges.left);
        assert!(!pattern.edges.right);
        assert!(!pattern.edges.bottom);
        assert!(!pattern.edges.top);
    }

    #[test]
    fn edge_pattern_does_not_match_interior_location() {
        let mut board = Board::new(19).unwrap();

        board
            .set_setup(Colour::Black, board.point(0, 0).unwrap())
            .unwrap();

        board
            .set_setup(Colour::Black, board.point(5, 5).unwrap())
            .unwrap();

        let pattern = Pattern::extract(
            &board,
            PatternRect {
                left: 0,
                bottom: 0,
                width: 1,
                height: 1,
            },
        )
        .unwrap();

        assert!(pattern.matches_at(&board, 0, 0).unwrap());
        assert!(!pattern.matches_at(&board, 5, 5).unwrap());
    }

    #[test]
    fn interior_pattern_does_not_match_edge_location() {
        let mut board = Board::new(19).unwrap();

        board
            .set_setup(Colour::Black, board.point(0, 0).unwrap())
            .unwrap();

        board
            .set_setup(Colour::Black, board.point(5, 5).unwrap())
            .unwrap();

        let pattern = Pattern::extract(
            &board,
            PatternRect {
                left: 5,
                bottom: 5,
                width: 1,
                height: 1,
            },
        )
        .unwrap();

        assert!(pattern.matches_at(&board, 5, 5).unwrap());
        assert!(!pattern.matches_at(&board, 0, 0).unwrap());
    }

    #[test]
    fn top_right_corner_pattern_only_matches_top_right_corner() {
        let mut board = Board::new(19).unwrap();

        board
            .set_setup(Colour::Black, board.point(18, 18).unwrap())
            .unwrap();

        board
            .set_setup(Colour::Black, board.point(0, 18).unwrap())
            .unwrap();

        board
            .set_setup(Colour::Black, board.point(18, 0).unwrap())
            .unwrap();

        let pattern = Pattern::extract(
            &board,
            PatternRect {
                left: 18,
                bottom: 18,
                width: 1,
                height: 1,
            },
        )
        .unwrap();

        assert!(pattern.matches_at(&board, 18, 18).unwrap());
        assert!(!pattern.matches_at(&board, 0, 18).unwrap());
        assert!(!pattern.matches_at(&board, 18, 0).unwrap());
    }

    #[test]
    fn rejects_empty_rectangle() {
        let board = Board::new(19).unwrap();

        assert_eq!(
            Pattern::extract(
                &board,
                PatternRect {
                    left: 0,
                    bottom: 0,
                    width: 0,
                    height: 1,
                }
            ),
            Err(PatternError::EmptyRectangle)
        );
    }

    #[test]
    fn rejects_rectangle_outside_board() {
        let board = Board::new(19).unwrap();

        assert_eq!(
            Pattern::extract(
                &board,
                PatternRect {
                    left: 18,
                    bottom: 18,
                    width: 2,
                    height: 2,
                }
            ),
            Err(PatternError::RectangleOutsideBoard)
        );
    }
}

#[test]
fn matches_identical_pattern() {
    let mut board = Board::new(19).unwrap();

    board
        .set_setup(Colour::Black, board.point(5, 5).unwrap())
        .unwrap();
    board
        .set_setup(Colour::White, board.point(6, 5).unwrap())
        .unwrap();

    let pattern = Pattern::extract(
        &board,
        PatternRect {
            left: 5,
            bottom: 5,
            width: 2,
            height: 1,
        },
    )
    .unwrap();

    assert!(pattern.matches_at(&board, 5, 5).unwrap());
}

#[test]
fn does_not_match_different_location() {
    let mut board = Board::new(19).unwrap();

    board
        .set_setup(Colour::Black, board.point(5, 5).unwrap())
        .unwrap();

    let pattern = Pattern::extract(
        &board,
        PatternRect {
            left: 5,
            bottom: 5,
            width: 1,
            height: 1,
        },
    )
    .unwrap();

    assert!(!pattern.matches_at(&board, 6, 6).unwrap());
}

#[cfg(test)]
mod transformation_tests {
    use super::*;

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
    fn rotates_rectangular_pattern_clockwise() {
        let transformed =
            asymmetric_pattern().transformed(PatternTransformation::Rotate90Clockwise);

        assert_eq!(
            transformed,
            Pattern {
                width: 3,
                height: 2,
                cells: vec![
                    PatternCell::White,
                    PatternCell::Black,
                    PatternCell::Empty,
                    PatternCell::Black,
                    PatternCell::Empty,
                    PatternCell::White,
                ],
                edges: BoardEdges {
                    left: false,
                    right: true,
                    bottom: false,
                    top: true,
                },
            }
        );
    }

    #[test]
    fn four_clockwise_quarter_turns_restore_pattern() {
        let original = asymmetric_pattern();

        let transformed = (0..4).fold(original.clone(), |pattern, _| {
            pattern.transformed(PatternTransformation::Rotate90Clockwise)
        });

        assert_eq!(transformed, original);
    }

    #[test]
    fn mirroring_twice_restores_pattern() {
        let original = asymmetric_pattern();

        let transformed = original
            .transformed(PatternTransformation::MirrorLeftRight)
            .transformed(PatternTransformation::MirrorLeftRight);

        assert_eq!(transformed, original);
    }

    #[test]
    fn diagonal_mirror_swaps_dimensions_and_edges() {
        let transformed =
            asymmetric_pattern().transformed(PatternTransformation::MirrorMainDiagonal);

        assert_eq!(transformed.width, 3);
        assert_eq!(transformed.height, 2);
        assert_eq!(
            transformed.edges,
            BoardEdges {
                left: false,
                right: true,
                bottom: true,
                top: false,
            }
        );
    }

    #[test]
    fn colour_reversal_preserves_geometry_and_edges() {
        let original = asymmetric_pattern();
        let reversed = original.reversed_colours();

        assert_eq!(reversed.width, original.width);
        assert_eq!(reversed.height, original.height);
        assert_eq!(reversed.edges, original.edges);
        assert_eq!(
            reversed.cells,
            vec![
                PatternCell::White,
                PatternCell::Black,
                PatternCell::Empty,
                PatternCell::White,
                PatternCell::Black,
                PatternCell::Empty,
            ]
        );
    }

    #[test]
    fn reversing_colours_twice_restores_pattern() {
        let original = asymmetric_pattern();

        assert_eq!(original.reversed_colours().reversed_colours(), original);
    }
}
