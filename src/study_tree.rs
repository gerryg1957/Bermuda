use crate::{
    Board, Collection, Colour, GameTree, Move, Node, PositionOccurrence, PositionState,
    extract_main_variation,
    game::{GameError, coordinate, move_coordinate},
    position_fingerprint,
};

/// A replayable move node from an SGF game tree.
///
/// Bermuda's Study tree deliberately collapses SGF nodes that contain no
/// move into a neighbouring move position. This keeps the Study tree aligned
/// with the move slider while preserving the branch structure used for replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudyMarkupKind {
    Label(String),
    Triangle,
    Square,
    Circle,
    Cross,
    Selected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyMarkup {
    pub point: u16,
    pub kind: StudyMarkupKind,
}

#[derive(Debug, Clone)]
pub struct StudyTreeNode {
    pub id: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub row: usize,
    pub lane: usize,
    pub position: PositionState,
    pub comment: String,
    pub move_colour: Option<Colour>,
    pub markup: Vec<StudyMarkup>,
}

#[derive(Debug, Clone)]
pub struct StudyTree {
    pub nodes: Vec<StudyTreeNode>,
    pub main_path: Vec<usize>,
    pub active_path: Vec<usize>,
}

impl StudyTree {
    /// Return the root-to-leaf path that passes through `node_id`.
    ///
    /// Before the selected node this follows its actual ancestors. After it,
    /// the first child at each branch is used. The result therefore gives
    /// the move slider one coherent line through the selected variation.
    #[must_use]
    pub fn path_through(&self, node_id: usize) -> Option<Vec<usize>> {
        if node_id >= self.nodes.len() {
            return None;
        }

        let mut prefix = Vec::new();
        let mut current = Some(node_id);

        while let Some(id) = current {
            prefix.push(id);
            current = self.nodes.get(id)?.parent;
        }

        prefix.reverse();

        let mut path = prefix;
        let mut current = node_id;

        while let Some(&child) = self.nodes.get(current)?.children.first() {
            path.push(child);
            current = child;
        }

        Some(path)
    }
}

/// Build a move-oriented Study tree from the first SGF game tree.
///
/// The ordinary database representation remains a compact main variation.
/// This richer representation is used only while studying an external SGF.
pub fn build_study_tree(collection: &Collection) -> Result<StudyTree, GameError> {
    let record = extract_main_variation(collection)?;
    let game_tree = collection.trees.first().ok_or(GameError::EmptyCollection)?;

    let board = Board::new(record.board_size).map_err(|source| GameError::Replay {
        move_number: 0,
        source,
    })?;

    let root_position = make_position(&board, 0, Colour::Black, None);

    let mut study = StudyTree {
        nodes: vec![StudyTreeNode {
            id: 0,
            parent: None,
            children: Vec::new(),
            row: 0,
            lane: 0,
            position: root_position,
            comment: String::new(),
            move_colour: None,
            markup: Vec::new(),
        }],
        main_path: Vec::new(),
        active_path: Vec::new(),
    };

    let mut next_lane = 0usize;

    walk_tree(game_tree, &mut study, board, 0, 0, 0, &mut next_lane, true)?;

    /*
     * Side-to-move is determined by the next move on the first continuation
     * from each displayed node. This matches replay_positions() on the main
     * line and also copes with teaching SGFs that contain two moves of the
     * same colour in succession.
     */
    for id in 0..study.nodes.len() {
        let next_colour = study.nodes[id]
            .children
            .first()
            .and_then(|child| study.nodes.get(*child))
            .and_then(|child| child.move_colour);

        let side_to_move = next_colour.unwrap_or_else(|| {
            study.nodes[id]
                .position
                .last_move
                .map(|mv| mv.colour.opponent())
                .unwrap_or(Colour::Black)
        });

        let node = &mut study.nodes[id];
        node.position.occurrence.side_to_move = side_to_move;
        node.position.occurrence.ko_point = node.position.board.ko_point();
        node.position.occurrence.fingerprint =
            position_fingerprint(&node.position.board, side_to_move);
    }

    study.main_path = first_child_path(&study);
    study.active_path = study.main_path.clone();

    Ok(study)
}

#[allow(clippy::too_many_arguments)]
fn walk_tree(
    tree: &GameTree,
    study: &mut StudyTree,
    mut board: Board,
    mut parent: usize,
    mut move_number: usize,
    lane: usize,
    next_lane: &mut usize,
    root_tree: bool,
) -> Result<(), GameError> {
    let mut created_move = false;
    let mut pending_comment = String::new();
    let mut pending_markup = Vec::new();

    for node in &tree.sequence {
        let setup_changed = apply_setup_properties(&mut board, node, move_number)?;

        let node_markup = node_markup(node, board.size())?;
        let mv = node_move(node, board.size())?;

        if let Some(mv) = mv {
            /*
             * Root setup belongs to position zero. Keep it visible before
             * the first move even when SGF stores setup and move properties
             * close together.
             */
            if root_tree && !created_move && setup_changed {
                refresh_position(&mut study.nodes[parent], &board);
            }

            board
                .play_archival(mv)
                .map_err(|source| GameError::Replay {
                    move_number: move_number + 1,
                    source,
                })?;

            move_number += 1;

            let mut comment = std::mem::take(&mut pending_comment);
            append_comment(&mut comment, node.first("C"));

            let mut markup = std::mem::take(&mut pending_markup);
            markup.extend(node_markup);

            let id = study.nodes.len();
            let position = make_position(&board, move_number, mv.colour.opponent(), Some(mv));

            study.nodes.push(StudyTreeNode {
                id,
                parent: Some(parent),
                children: Vec::new(),
                row: move_number,
                lane,
                position,
                comment,
                move_colour: Some(mv.colour),
                markup,
            });

            study.nodes[parent].children.push(id);
            parent = id;
            created_move = true;
        } else if root_tree || created_move {
            /*
             * A no-move SGF node on an established line belongs to the
             * currently displayed position.
             */
            if setup_changed {
                refresh_position(&mut study.nodes[parent], &board);
            }

            append_comment(&mut study.nodes[parent].comment, node.first("C"));
            study.nodes[parent].markup.extend(node_markup);
        } else {
            /*
             * A variation can begin with a comment before its first move.
             * Do not attach that text to the shared branch point; carry it
             * forward to the first move belonging to this variation.
             */
            append_comment(&mut pending_comment, node.first("C"));
            pending_markup.extend(node_markup);
        }
    }

    for (index, variation) in tree.variations.iter().enumerate() {
        let variation_lane = if index == 0 {
            lane
        } else {
            *next_lane += 1;
            *next_lane
        };

        walk_tree(
            variation,
            study,
            board.clone(),
            parent,
            move_number,
            variation_lane,
            next_lane,
            false,
        )?;
    }

    Ok(())
}

fn apply_setup_properties(
    board: &mut Board,
    node: &Node,
    move_number: usize,
) -> Result<bool, GameError> {
    let size = board.size();
    let mut changed = false;

    for value in node.values("AB") {
        let point = coordinate(value, size)?.ok_or_else(|| GameError::InvalidCoordinate {
            value: value.clone(),
            size,
        })?;

        board
            .set_setup(Colour::Black, point)
            .map_err(|source| GameError::Replay {
                move_number,
                source,
            })?;

        changed = true;
    }

    for value in node.values("AW") {
        let point = coordinate(value, size)?.ok_or_else(|| GameError::InvalidCoordinate {
            value: value.clone(),
            size,
        })?;

        board
            .set_setup(Colour::White, point)
            .map_err(|source| GameError::Replay {
                move_number,
                source,
            })?;

        changed = true;
    }

    for value in node.values("AE") {
        let point = coordinate(value, size)?.ok_or_else(|| GameError::InvalidCoordinate {
            value: value.clone(),
            size,
        })?;

        board
            .clear_setup(point)
            .map_err(|source| GameError::Replay {
                move_number,
                source,
            })?;

        changed = true;
    }

    Ok(changed)
}

fn node_markup(node: &Node, size: u8) -> Result<Vec<StudyMarkup>, GameError> {
    let mut markup = Vec::new();

    for value in node.values("LB") {
        let Some((point_text, label)) = value.split_once(':') else {
            continue;
        };

        let point = coordinate(point_text, size)?.ok_or_else(|| GameError::InvalidCoordinate {
            value: point_text.to_owned(),
            size,
        })?;

        markup.push(StudyMarkup {
            point,
            kind: StudyMarkupKind::Label(label.to_owned()),
        });
    }

    append_point_markup(&mut markup, node, "TR", size, StudyMarkupKind::Triangle)?;
    append_point_markup(&mut markup, node, "SQ", size, StudyMarkupKind::Square)?;
    append_point_markup(&mut markup, node, "CR", size, StudyMarkupKind::Circle)?;
    append_point_markup(&mut markup, node, "MA", size, StudyMarkupKind::Cross)?;
    append_point_markup(&mut markup, node, "SL", size, StudyMarkupKind::Selected)?;

    Ok(markup)
}

fn append_point_markup(
    markup: &mut Vec<StudyMarkup>,
    node: &Node,
    property: &str,
    size: u8,
    kind: StudyMarkupKind,
) -> Result<(), GameError> {
    for value in node.values(property) {
        let point = coordinate(value, size)?.ok_or_else(|| GameError::InvalidCoordinate {
            value: value.clone(),
            size,
        })?;

        markup.push(StudyMarkup {
            point,
            kind: kind.clone(),
        });
    }

    Ok(())
}

fn node_move(node: &Node, size: u8) -> Result<Option<Move>, GameError> {
    let black = node.first("B");
    let white = node.first("W");

    if black.is_some() && white.is_some() {
        return Err(GameError::TwoMovesInNode);
    }

    if let Some(value) = black {
        return Ok(Some(Move {
            colour: Colour::Black,
            point: move_coordinate(value, size)?,
        }));
    }

    if let Some(value) = white {
        return Ok(Some(Move {
            colour: Colour::White,
            point: move_coordinate(value, size)?,
        }));
    }

    Ok(None)
}

fn make_position(
    board: &Board,
    move_number: usize,
    side_to_move: Colour,
    last_move: Option<Move>,
) -> PositionState {
    PositionState {
        board: board.clone(),
        occurrence: PositionOccurrence {
            move_number,
            side_to_move,
            ko_point: board.ko_point(),
            fingerprint: position_fingerprint(board, side_to_move),
        },
        last_move,
    }
}

fn refresh_position(node: &mut StudyTreeNode, board: &Board) {
    let side_to_move = node.position.occurrence.side_to_move;

    node.position.board = board.clone();
    node.position.occurrence.ko_point = board.ko_point();
    node.position.occurrence.fingerprint = position_fingerprint(board, side_to_move);
}

fn append_comment(target: &mut String, comment: Option<&str>) {
    let Some(comment) = comment else {
        return;
    };

    if comment.is_empty() {
        return;
    }

    if !target.is_empty() {
        target.push_str("\n\n");
    }

    target.push_str(comment);
}

fn first_child_path(tree: &StudyTree) -> Vec<usize> {
    let mut path = vec![0];
    let mut current = 0usize;

    while let Some(&child) = tree.nodes[current].children.first() {
        path.push(child);
        current = child;
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_collection;

    #[test]
    fn preserves_branch_order_and_builds_main_path() {
        let collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd](;W[pp];B[qq])(;W[dp]))")
            .expect("parse SGF");

        let tree = build_study_tree(&collection).expect("build Study tree");

        assert_eq!(tree.nodes.len(), 5);
        assert_eq!(tree.main_path, vec![0, 1, 2, 3]);
        assert_eq!(tree.nodes[1].children, vec![2, 4]);
        assert_eq!(tree.path_through(4), Some(vec![0, 1, 4]));
    }

    #[test]
    fn keeps_comments_with_their_variation_moves() {
        let collection = parse_collection(
            b"(;FF[4]GM[1]SZ[19]C[root];B[dd]C[first](;W[pp]C[main])(;W[dp]C[other]))",
        )
        .expect("parse SGF");

        let tree = build_study_tree(&collection).expect("build Study tree");

        assert_eq!(tree.nodes[0].comment, "root");
        assert_eq!(tree.nodes[1].comment, "first");
        assert_eq!(tree.nodes[2].comment, "main");
        assert_eq!(tree.nodes[3].comment, "other");
    }

    #[test]
    fn carries_leading_variation_comment_to_first_variation_move() {
        let collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd](;C[note];W[pp]))").expect("parse SGF");

        let tree = build_study_tree(&collection).expect("build Study tree");

        assert_eq!(tree.nodes[2].comment, "note");
    }

    #[test]
    fn retains_labels_and_standard_board_marks() {
        let collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd]LB[qd:A][qi:B]TR[pd]SQ[dp]CR[pp]MA[jj])")
                .expect("parse SGF");

        let tree = build_study_tree(&collection).expect("build Study tree");
        let markup = &tree.nodes[1].markup;

        assert_eq!(markup.len(), 6);

        assert!(
            markup
                .iter()
                .any(|mark| { matches!(&mark.kind, StudyMarkupKind::Label(text) if text == "A") })
        );
        assert!(
            markup
                .iter()
                .any(|mark| { matches!(&mark.kind, StudyMarkupKind::Label(text) if text == "B") })
        );
        assert!(
            markup
                .iter()
                .any(|mark| { matches!(mark.kind, StudyMarkupKind::Triangle) })
        );
    }
}
