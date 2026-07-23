use crate::{Board, Color};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternCell {
    Empty,
    Black,
    White,
}

impl From<Option<Color>> for PatternCell {
    fn from(value: Option<Color>) -> Self {
        match value {
            Some(Color::Black) => Self::Black,
            Some(Color::White) => Self::White,
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

                cells.push(PatternCell::from(board.color_at(point)));
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

    pub fn matches_at(&self, board: &Board, left: u8, bottom: u8) -> Result<bool, PatternError> {
        let rect = PatternRect {
            left,
            bottom,
            width: self.width,
            height: self.height,
        };

        let candidate = Pattern::extract(board, rect)?;

        Ok(self == &candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Board, Color};

    #[test]
    fn extracts_centre_pattern() {
        let mut board = Board::new(19).unwrap();

        board
            .set_setup(Color::Black, board.point(9, 9).unwrap())
            .unwrap();
        board
            .set_setup(Color::White, board.point(10, 9).unwrap())
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
        .set_setup(Color::Black, board.point(5, 5).unwrap())
        .unwrap();
    board
        .set_setup(Color::White, board.point(6, 5).unwrap())
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
        .set_setup(Color::Black, board.point(5, 5).unwrap())
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
