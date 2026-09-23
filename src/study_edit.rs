use crate::{Collection, Colour, GameTree, Move, Node, StudySourceLocation, StudyTree};

/// Result of asking Study to add one continuation.
///
/// If the requested move is already an immediate child, Bermuda should
/// navigate to that existing continuation rather than duplicate it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyMoveInsertion {
    pub source: StudySourceLocation,
    pub inserted: bool,
}

/// Extend the current line with one move.
///
/// If the requested move already exists as an immediate continuation, return
/// that continuation so the UI can navigate to it. If a different
/// continuation already exists, require the explicit Branch operation.
pub fn extend_study_move(
    collection: &mut Collection,
    study: &StudyTree,
    node_id: usize,
    mv: Move,
) -> Result<StudyMoveInsertion, String> {
    let node = study
        .nodes
        .get(node_id)
        .ok_or_else(|| format!("Study tree node {node_id} does not exist"))?;

    let existing = node.children.iter().any(|child_id| {
        study
            .nodes
            .get(*child_id)
            .is_some_and(|child| child.position.last_move == Some(mv))
    });

    if !existing && !node.children.is_empty() {
        return Err(
            "this position already has a continuation; use Branch to add an alternative".to_owned(),
        );
    }

    insert_study_move(collection, study, node_id, mv)
}

/// Add an alternative continuation from the current Study position.
pub fn branch_study_move(
    collection: &mut Collection,
    study: &StudyTree,
    node_id: usize,
    mv: Move,
) -> Result<StudyMoveInsertion, String> {
    study
        .nodes
        .get(node_id)
        .ok_or_else(|| format!("Study tree node {node_id} does not exist"))?;

    /*
     * Branch expresses the user's structural intention. At a leaf, the
     * first branch is necessarily the first continuation. Returning to that
     * node and branching again then creates a visible fork.
     */

    insert_study_move(collection, study, node_id, mv)
}

/// Insert one empty SGF node immediately after one literal SGF node.
///
/// Unlike `insert_study_node`, this operates on an exact SGF source address
/// rather than a move-oriented StudyTree node. It is the primitive used by
/// the structural SGF editor, where comment-only and empty nodes are visible
/// and independently selectable.
pub fn insert_study_node_after_source(
    collection: &mut Collection,
    source: &StudySourceLocation,
) -> Result<StudySourceLocation, String> {
    let tree = game_tree_at_mut(collection, &source.variation_path)?;

    if source.sequence_index >= tree.sequence.len() {
        return Err(format!(
            "SGF node {} at variation path {:?} does not exist",
            source.sequence_index, source.variation_path
        ));
    }

    let sequence_index = source.sequence_index + 1;
    tree.sequence.insert(sequence_index, Node::default());

    Ok(StudySourceLocation {
        variation_path: source.variation_path.clone(),
        sequence_index,
    })
}

/// Insert one empty SGF node immediately after a displayed Study position.
///
/// This is a structural SGF operation. The inserted node deliberately has no
/// move property yet; later editor operations may add a comment, markup,
/// setup properties or a move to it.
///
/// A displayed Study position can own trailing comment/setup-only SGF nodes.
/// The new node is inserted after those nodes but before the next move in the
/// same sequence. If the position is a branch point, it is inserted into the
/// common sequence before the child variations, so all existing variations
/// continue after the newly inserted node.
pub fn insert_study_node(
    collection: &mut Collection,
    study: &StudyTree,
    node_id: usize,
) -> Result<StudySourceLocation, String> {
    let node = study
        .nodes
        .get(node_id)
        .ok_or_else(|| format!("Study tree node {node_id} does not exist"))?;

    let branch_path = node.source.variation_path.clone();

    let next_same_sequence = node
        .children
        .iter()
        .filter_map(|child_id| study.nodes.get(*child_id))
        .filter(|child| {
            child.source.variation_path == branch_path
                && (node_id == 0 || child.source.sequence_index > node.source.sequence_index)
        })
        .map(|child| child.source.sequence_index)
        .min();

    let tree = game_tree_at_mut(collection, &branch_path)?;

    let insert_index = next_same_sequence.unwrap_or(tree.sequence.len());

    if insert_index > tree.sequence.len() {
        return Err(format!(
            "Study node insertion point {insert_index} lies beyond SGF sequence length {}",
            tree.sequence.len()
        ));
    }

    tree.sequence.insert(insert_index, Node::default());

    Ok(StudySourceLocation {
        variation_path: branch_path,
        sequence_index: insert_index,
    })
}

/// Add one legal move after a displayed Study-tree position.
///
/// This is deliberately an SGF operation, not a StudyTree operation.
/// `StudyTree` tells us where the displayed position sits in the source
/// SGF; the `Collection` remains the editable source of truth.
///
/// SGF stores an unbranched run of moves in one `GameTree.sequence`.
/// Therefore adding an alternative before an existing later move cannot
/// be implemented as a simple Vec insertion. The existing suffix must
/// become the first child variation and the new move a sibling variation.
pub fn insert_study_move(
    collection: &mut Collection,
    study: &StudyTree,
    node_id: usize,
    mv: Move,
) -> Result<StudyMoveInsertion, String> {
    let node = study
        .nodes
        .get(node_id)
        .ok_or_else(|| format!("Study tree node {node_id} does not exist"))?;

    let expected_colour = node.position.occurrence.side_to_move;

    if mv.colour != expected_colour {
        return Err(format!(
            "Study continuation must be {:?}, not {:?}",
            expected_colour, mv.colour
        ));
    }

    let mut trial_board = node.position.board.clone();

    trial_board
        .play(mv)
        .map_err(|error| format!("illegal Study move: {error}"))?;

    /*
     * Clicking an already-existing continuation is navigation, not an
     * instruction to create a duplicate SGF branch.
     */
    for &child_id in &node.children {
        let child = study.nodes.get(child_id).ok_or_else(|| {
            format!("Study tree node {node_id} refers to missing child {child_id}")
        })?;

        if child.position.last_move == Some(mv) {
            return Ok(StudyMoveInsertion {
                source: child.source.clone(),
                inserted: false,
            });
        }
    }

    let branch_path = node.source.variation_path.clone();

    /*
     * A displayed position can own comment/setup-only SGF nodes after its
     * canonical move node. The next same-sequence move tells us where that
     * displayed position ends. If there is no such child, the position owns
     * the remainder of this GameTree sequence and any children are already
     * represented as SGF variations.
     */
    let next_same_sequence = node
        .children
        .iter()
        .filter_map(|child_id| study.nodes.get(*child_id))
        .filter(|child| {
            child.source.variation_path == branch_path
                && (node_id == 0 || child.source.sequence_index > node.source.sequence_index)
        })
        .map(|child| child.source.sequence_index)
        .min();

    let board_size = node.position.board.size();
    let move_node = sgf_move_node(mv, board_size)?;

    let tree = game_tree_at_mut(collection, &branch_path)?;

    let split_index = next_same_sequence.unwrap_or(tree.sequence.len());

    if split_index > tree.sequence.len() {
        return Err(format!(
            "Study continuation point {split_index} lies beyond SGF sequence length {}",
            tree.sequence.len()
        ));
    }

    /*
     * The normal SGF shape has at least one non-move root node before the
     * first move. Supporting a move property directly on the root node would
     * require splitting that node's root properties from its move property.
     * Do not silently rewrite such unusual source material.
     */
    if split_index == 0 {
        return Err(
            "cannot yet branch before an SGF root node that itself contains a move".to_owned(),
        );
    }

    if split_index < tree.sequence.len() {
        /*
         * We are branching in the middle of one SGF sequence:
         *
         *     ;A;B;C
         *
         * becomes:
         *
         *     ;A (;B;C) (;new)
         *
         * Any variations that originally followed C belong to the preserved
         * old continuation, so move them with the suffix.
         */
        let suffix = tree.sequence.split_off(split_index);
        let suffix_variations = std::mem::take(&mut tree.variations);

        tree.variations.push(GameTree {
            sequence: suffix,
            variations: suffix_variations,
        });

        let variation_index = tree.variations.len();

        tree.variations.push(GameTree {
            sequence: vec![move_node],
            variations: Vec::new(),
        });

        let mut path = branch_path;
        path.push(variation_index);

        return Ok(StudyMoveInsertion {
            source: StudySourceLocation {
                variation_path: path,
                sequence_index: 0,
            },
            inserted: true,
        });
    }

    if tree.variations.is_empty() {
        /*
         * A simple leaf extension stays in the same SGF sequence.
         */
        let sequence_index = tree.sequence.len();
        tree.sequence.push(move_node);

        return Ok(StudyMoveInsertion {
            source: StudySourceLocation {
                variation_path: branch_path,
                sequence_index,
            },
            inserted: true,
        });
    }

    /*
     * The current position already branches. Add one more sibling.
     */
    let variation_index = tree.variations.len();

    tree.variations.push(GameTree {
        sequence: vec![move_node],
        variations: Vec::new(),
    });

    let mut path = branch_path;
    path.push(variation_index);

    Ok(StudyMoveInsertion {
        source: StudySourceLocation {
            variation_path: path,
            sequence_index: 0,
        },
        inserted: true,
    })
}

fn game_tree_at_mut<'a>(
    collection: &'a mut Collection,
    variation_path: &[usize],
) -> Result<&'a mut GameTree, String> {
    let mut tree = collection
        .trees
        .first_mut()
        .ok_or_else(|| "Study SGF has no game tree".to_owned())?;

    for &variation_index in variation_path {
        tree = tree.variations.get_mut(variation_index).ok_or_else(|| {
            format!("Study SGF variation path {variation_path:?} no longer exists")
        })?;
    }

    Ok(tree)
}

fn sgf_move_node(mv: Move, board_size: u8) -> Result<Node, String> {
    let property = match mv.colour {
        Colour::Black => "B",
        Colour::White => "W",
    };

    let value = match mv.point {
        None => String::new(),

        Some(point) => {
            let size = u16::from(board_size);
            let point_count = size * size;

            if point >= point_count {
                return Err(format!(
                    "Study move point {point} lies outside a {board_size}×{board_size} board"
                ));
            }

            let x = u8::try_from(point % size)
                .map_err(|_| "Study move x-coordinate is too large".to_owned())?;
            let y = u8::try_from(point / size)
                .map_err(|_| "Study move y-coordinate is too large".to_owned())?;

            let mut value = String::with_capacity(2);
            value.push((b'a' + x) as char);
            value.push((b'a' + y) as char);
            value
        }
    };

    let mut node = Node::default();
    node.properties.insert(property.to_owned(), vec![value]);

    Ok(node)
}

/// Make the nearest selected side variation the first continuation at its
/// branch point.
///
/// A selection may be several variation levels below the main line. In that
/// case the deepest non-main variation is promoted first. Repeating the
/// operation can therefore promote the selected route one branch point at a
/// time without changing or deleting any SGF nodes.
pub fn promote_study_variation(
    collection: &mut Collection,
    source: &StudySourceLocation,
) -> Result<StudySourceLocation, String> {
    let promote_depth = source
        .variation_path
        .iter()
        .rposition(|&variation_index| variation_index != 0)
        .ok_or_else(|| "the selected Study line is already the main variation".to_owned())?;

    {
        let root = collection
            .trees
            .first()
            .ok_or_else(|| "Study SGF has no game tree".to_owned())?;

        let mut selected = root;
        for (depth, &variation_index) in source.variation_path.iter().enumerate() {
            selected = selected.variations.get(variation_index).ok_or_else(|| {
                format!(
                    "Study variation {} does not exist at depth {}",
                    variation_index, depth
                )
            })?;
        }

        if source.sequence_index >= selected.sequence.len() {
            return Err(format!(
                "Study SGF node {} does not exist in the selected variation",
                source.sequence_index
            ));
        }
    }

    let root = collection
        .trees
        .first_mut()
        .ok_or_else(|| "Study SGF has no game tree".to_owned())?;

    let parent_path = &source.variation_path[..promote_depth];
    let variation_index = source.variation_path[promote_depth];

    let mut parent = root;
    for (depth, &index) in parent_path.iter().enumerate() {
        parent = parent.variations.get_mut(index).ok_or_else(|| {
            format!(
                "Study variation {} does not exist at depth {}",
                index, depth
            )
        })?;
    }

    if variation_index >= parent.variations.len() {
        return Err(format!(
            "Study variation {} does not exist at depth {}",
            variation_index, promote_depth
        ));
    }

    let promoted = parent.variations.remove(variation_index);
    parent.variations.insert(0, promoted);

    let mut selected_source = source.clone();
    selected_source.variation_path[promote_depth] = 0;

    Ok(selected_source)
}

/// Delete the selected literal SGF node and every descendant below it.
///
/// If the selected node starts a variation, the whole variation is removed
/// while its siblings are preserved. If it occurs later in a sequence, that
/// sequence is truncated immediately before the selected node and any child
/// variations below the deleted suffix are discarded.
///
/// The root SGF node cannot be deleted because it owns the game record.
pub fn delete_study_from_source(
    collection: &mut Collection,
    source: &StudySourceLocation,
) -> Result<StudySourceLocation, String> {
    let root = collection
        .trees
        .first_mut()
        .ok_or_else(|| "Study SGF has no game tree".to_owned())?;

    if source.variation_path.is_empty() {
        if source.sequence_index >= root.sequence.len() {
            return Err(format!(
                "Study SGF node {} does not exist",
                source.sequence_index
            ));
        }

        if source.sequence_index == 0 {
            return Err("the root SGF node cannot be deleted".to_owned());
        }

        root.sequence.truncate(source.sequence_index);
        root.variations.clear();

        return Ok(StudySourceLocation {
            variation_path: Vec::new(),
            sequence_index: source.sequence_index - 1,
        });
    }

    let parent_path = &source.variation_path[..source.variation_path.len() - 1];
    let variation_index = *source
        .variation_path
        .last()
        .expect("non-empty variation path checked above");

    let mut parent = root;
    for (depth, &index) in parent_path.iter().enumerate() {
        parent = parent.variations.get_mut(index).ok_or_else(|| {
            format!(
                "Study variation {} does not exist at depth {}",
                index, depth
            )
        })?;
    }

    if variation_index >= parent.variations.len() {
        return Err(format!(
            "Study variation {} does not exist",
            variation_index
        ));
    }

    let child_len = parent.variations[variation_index].sequence.len();
    if source.sequence_index >= child_len {
        return Err(format!(
            "Study SGF node {} does not exist in variation {}",
            source.sequence_index, variation_index
        ));
    }

    if source.sequence_index == 0 {
        if parent.sequence.is_empty() {
            return Err("cannot return to the parent of an empty Study variation".to_owned());
        }

        parent.variations.remove(variation_index);

        return Ok(StudySourceLocation {
            variation_path: parent_path.to_vec(),
            sequence_index: parent.sequence.len() - 1,
        });
    }

    let child = &mut parent.variations[variation_index];
    child.sequence.truncate(source.sequence_index);
    child.variations.clear();

    Ok(StudySourceLocation {
        variation_path: source.variation_path.clone(),
        sequence_index: source.sequence_index - 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_study_tree, parse_collection};

    #[test]
    fn promotes_nearest_side_variation_without_losing_siblings() {
        let mut collection =
            parse_collection(b"(;GM[1]FF[4]SZ[19];B[aa](;W[bb];B[cc])(;W[dd](;B[ee])(;B[ff])))")
                .expect("parse Study SGF");

        let selected = StudySourceLocation {
            variation_path: vec![1, 1],
            sequence_index: 0,
        };

        let selected =
            promote_study_variation(&mut collection, &selected).expect("promote inner variation");

        assert_eq!(
            selected,
            StudySourceLocation {
                variation_path: vec![1, 0],
                sequence_index: 0,
            }
        );

        let outer = &collection.trees[0].variations[1];

        assert_eq!(
            outer.variations[0].sequence[0]
                .properties
                .get("B")
                .and_then(|values| values.first())
                .map(String::as_str),
            Some("ff")
        );
        assert_eq!(
            outer.variations[1].sequence[0]
                .properties
                .get("B")
                .and_then(|values| values.first())
                .map(String::as_str),
            Some("ee")
        );

        let selected =
            promote_study_variation(&mut collection, &selected).expect("promote outer variation");

        assert_eq!(
            selected,
            StudySourceLocation {
                variation_path: vec![0, 0],
                sequence_index: 0,
            }
        );

        assert_eq!(
            collection.trees[0].variations[0].sequence[0]
                .properties
                .get("W")
                .and_then(|values| values.first())
                .map(String::as_str),
            Some("dd")
        );
        assert_eq!(
            collection.trees[0].variations[1].sequence[0]
                .properties
                .get("W")
                .and_then(|values| values.first())
                .map(String::as_str),
            Some("bb")
        );
    }

    #[test]
    fn main_variation_cannot_be_promoted_again() {
        let mut collection = parse_collection(b"(;GM[1]FF[4]SZ[19];B[aa](;W[bb])(;W[cc]))")
            .expect("parse Study SGF");

        let error = promote_study_variation(
            &mut collection,
            &StudySourceLocation {
                variation_path: vec![0],
                sequence_index: 0,
            },
        )
        .expect_err("main variation should not be promotable");

        assert_eq!(
            error,
            "the selected Study line is already the main variation"
        );
    }

    fn point(study: &StudyTree, node_id: usize, x: u8, y: u8) -> u16 {
        study.nodes[node_id]
            .position
            .board
            .point(x, y)
            .expect("board point")
    }

    #[test]
    fn inserts_literal_node_after_non_move_source() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];C[note];W[pp])").expect("parse SGF");

        let inserted = insert_study_node_after_source(
            &mut collection,
            &StudySourceLocation {
                variation_path: vec![],
                sequence_index: 2,
            },
        )
        .expect("insert after comment node");

        assert_eq!(inserted.sequence_index, 3);
        assert_eq!(collection.trees[0].sequence[2].first("C"), Some("note"));
        assert!(collection.trees[0].sequence[3].properties.is_empty());
        assert_eq!(collection.trees[0].sequence[4].first("W"), Some("pp"));
    }

    #[test]
    fn literal_insert_at_branch_point_stays_common_to_variations() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd](;W[pp])(;W[dp]))").expect("parse SGF");

        let inserted = insert_study_node_after_source(
            &mut collection,
            &StudySourceLocation {
                variation_path: vec![],
                sequence_index: 1,
            },
        )
        .expect("insert at branch point");

        assert_eq!(inserted.sequence_index, 2);
        assert!(collection.trees[0].sequence[2].properties.is_empty());
        assert_eq!(collection.trees[0].variations.len(), 2);
    }

    #[test]
    fn inserts_empty_node_at_leaf() {
        let mut collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let source = insert_study_node(&mut collection, &study, 1).expect("insert node");

        assert_eq!(
            source,
            StudySourceLocation {
                variation_path: vec![],
                sequence_index: 2,
            }
        );
        assert_eq!(collection.trees[0].sequence.len(), 3);
        assert!(collection.trees[0].sequence[2].properties.is_empty());
    }

    #[test]
    fn inserts_before_next_move_in_same_sequence() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];W[pp];B[qq])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let source = insert_study_node(&mut collection, &study, 1).expect("insert node");

        assert_eq!(source.sequence_index, 2);
        assert!(collection.trees[0].sequence[2].properties.is_empty());
        assert_eq!(collection.trees[0].sequence[3].first("W"), Some("pp"));
        assert_eq!(collection.trees[0].sequence[4].first("B"), Some("qq"));
    }

    #[test]
    fn inserts_after_non_move_nodes_owned_by_position() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];C[note]TR[pp];W[pp])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let source = insert_study_node(&mut collection, &study, 1).expect("insert node");

        assert_eq!(source.sequence_index, 3);
        assert_eq!(collection.trees[0].sequence[2].first("C"), Some("note"));
        assert_eq!(collection.trees[0].sequence[2].first("TR"), Some("pp"));
        assert!(collection.trees[0].sequence[3].properties.is_empty());
        assert_eq!(collection.trees[0].sequence[4].first("W"), Some("pp"));
    }

    #[test]
    fn inserts_common_node_before_existing_variations() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd](;W[pp])(;W[dp]))").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let source = insert_study_node(&mut collection, &study, 1).expect("insert node");

        assert_eq!(source.sequence_index, 2);
        assert_eq!(collection.trees[0].sequence.len(), 3);
        assert!(collection.trees[0].sequence[2].properties.is_empty());

        assert_eq!(collection.trees[0].variations.len(), 2);
        assert_eq!(
            collection.trees[0].variations[0].sequence[0].first("W"),
            Some("pp")
        );
        assert_eq!(
            collection.trees[0].variations[1].sequence[0].first("W"),
            Some("dp")
        );
    }

    #[test]
    fn inserts_inside_nested_variation() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd](;W[pp];B[qq]))").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let w_node = study.nodes[1].children[0];

        let source =
            insert_study_node(&mut collection, &study, w_node).expect("insert nested node");

        assert_eq!(
            source,
            StudySourceLocation {
                variation_path: vec![0],
                sequence_index: 1,
            }
        );

        assert_eq!(
            collection.trees[0].variations[0].sequence[0].first("W"),
            Some("pp")
        );
        assert!(
            collection.trees[0].variations[0].sequence[1]
                .properties
                .is_empty()
        );
        assert_eq!(
            collection.trees[0].variations[0].sequence[2].first("B"),
            Some("qq")
        );
    }

    #[test]
    fn empty_inserted_node_round_trips_through_writer_and_parser() {
        let mut collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        insert_study_node(&mut collection, &study, 1).expect("insert node");

        let sgf = crate::write_collection_sgf(&collection);
        let reparsed = parse_collection(sgf.as_bytes()).expect("reparse written SGF");

        assert_eq!(reparsed, collection);
    }

    #[test]
    fn extends_a_leaf_in_the_same_sequence() {
        let mut collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let result = insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 15, 15)),
            },
        )
        .expect("insert continuation");

        assert!(result.inserted);
        assert_eq!(result.source.variation_path, Vec::<usize>::new());
        assert_eq!(result.source.sequence_index, 2);

        assert_eq!(collection.trees[0].sequence[2].first("W"), Some("pp"));
        assert!(collection.trees[0].variations.is_empty());
    }

    #[test]
    fn splits_an_existing_sequence_into_old_and_new_variations() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];W[pp];B[qq])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let result = insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 3, 15)),
            },
        )
        .expect("insert alternative");

        assert!(result.inserted);
        assert_eq!(collection.trees[0].sequence.len(), 2);
        assert_eq!(collection.trees[0].variations.len(), 2);

        assert_eq!(
            collection.trees[0].variations[0].sequence[0].first("W"),
            Some("pp")
        );
        assert_eq!(
            collection.trees[0].variations[0].sequence[1].first("B"),
            Some("qq")
        );
        assert_eq!(
            collection.trees[0].variations[1].sequence[0].first("W"),
            Some("dp")
        );

        assert_eq!(result.source.variation_path, vec![1]);
        assert_eq!(result.source.sequence_index, 0);
    }

    #[test]
    fn preserves_nested_variations_with_the_old_continuation() {
        let mut collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];W[pp](;B[qq])(;B[qp]))")
            .expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 3, 15)),
            },
        )
        .expect("insert alternative");

        let old = &collection.trees[0].variations[0];

        assert_eq!(old.sequence[0].first("W"), Some("pp"));
        assert_eq!(old.variations.len(), 2);
        assert_eq!(old.variations[0].sequence[0].first("B"), Some("qq"));
        assert_eq!(old.variations[1].sequence[0].first("B"), Some("qp"));
    }

    #[test]
    fn adds_a_sibling_to_an_existing_branch_point() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd](;W[pp])(;W[dp]))").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let result = insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 16, 16)),
            },
        )
        .expect("insert third variation");

        assert!(result.inserted);
        assert_eq!(collection.trees[0].variations.len(), 3);
        assert_eq!(
            collection.trees[0].variations[2].sequence[0].first("W"),
            Some("qq")
        );
        assert_eq!(result.source.variation_path, vec![2]);
    }

    #[test]
    fn existing_immediate_continuation_is_reused() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];W[pp])").expect("parse SGF");

        let original = collection.clone();

        let study = build_study_tree(&collection).expect("build Study tree");

        let result = insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 15, 15)),
            },
        )
        .expect("reuse continuation");

        assert!(!result.inserted);
        assert_eq!(collection, original);
        assert_eq!(result.source, study.nodes[2].source);
    }

    #[test]
    fn branches_after_comment_only_nodes_belonging_to_the_position() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];C[note]TR[pp];W[pp])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 3, 15)),
            },
        )
        .expect("insert alternative");

        /*
         * Root, B[dd], and the comment/markup-only node remain together
         * before the branch. The existing W[pp] becomes the first child.
         */
        assert_eq!(collection.trees[0].sequence.len(), 3);
        assert_eq!(collection.trees[0].sequence[2].first("C"), Some("note"));
        assert_eq!(collection.trees[0].sequence[2].first("TR"), Some("pp"));
        assert_eq!(
            collection.trees[0].variations[0].sequence[0].first("W"),
            Some("pp")
        );
        assert_eq!(
            collection.trees[0].variations[1].sequence[0].first("W"),
            Some("dp")
        );
    }

    #[test]
    fn extend_requires_explicit_branch_for_new_alternative() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];W[pp])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let error = extend_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 3, 15)),
            },
        )
        .expect_err("different continuation must require Branch");

        assert!(error.contains("use Branch"));
    }

    #[test]
    fn branch_from_leaf_creates_first_continuation() {
        let mut collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let result = branch_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 15, 15)),
            },
        )
        .expect("branch from leaf");

        assert!(result.inserted);
        assert_eq!(collection.trees[0].sequence[2].first("W"), Some("pp"));
    }

    #[test]
    fn can_branch_again_inside_a_new_variation() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];W[pp];B[qq])").expect("parse SGF");

        /*
         * First make an alternative W move after B[dd].
         */
        let first_tree = build_study_tree(&collection).expect("build first Study tree");

        let first_branch = insert_study_move(
            &mut collection,
            &first_tree,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&first_tree, 1, 3, 15)),
            },
        )
        .expect("create first branch");

        assert!(first_branch.inserted);

        /*
         * Rebuild exactly as the Qt editor does, locate the newly-created
         * W move, and extend it with a Black continuation.
         */
        let second_tree = build_study_tree(&collection).expect("rebuild after first branch");

        let branch_w = second_tree
            .node_id_for_source(&first_branch.source)
            .expect("find first branch");

        let first_black = insert_study_move(
            &mut collection,
            &second_tree,
            branch_w,
            Move {
                colour: Colour::Black,
                point: Some(point(&second_tree, branch_w, 16, 3)),
            },
        )
        .expect("extend first branch");

        assert!(first_black.inserted);

        /*
         * Rebuild again, return to the W branch point, then add a different
         * Black move. This is the exact "branch again" operation that must
         * work after the first user-created variation.
         */
        let third_tree = build_study_tree(&collection).expect("rebuild after extension");

        let branch_w = third_tree
            .node_id_for_source(&first_branch.source)
            .expect("find branch point again");

        let second_black = branch_study_move(
            &mut collection,
            &third_tree,
            branch_w,
            Move {
                colour: Colour::Black,
                point: Some(point(&third_tree, branch_w, 15, 3)),
            },
        )
        .expect("branch again");

        assert!(second_black.inserted);

        let final_tree = build_study_tree(&collection).expect("final rebuild");

        let branch_w = final_tree
            .node_id_for_source(&first_branch.source)
            .expect("find final branch point");

        assert_eq!(final_tree.nodes[branch_w].children.len(), 2);

        let child_points = final_tree.nodes[branch_w]
            .children
            .iter()
            .map(|child_id| {
                final_tree.nodes[*child_id]
                    .position
                    .last_move
                    .expect("child move")
                    .point
                    .expect("non-pass child")
            })
            .collect::<Vec<_>>();

        assert!(child_points.contains(&point(&final_tree, branch_w, 16, 3)));
        assert!(child_points.contains(&point(&final_tree, branch_w, 15, 3)));

        /*
         * The original main continuation remains separate as well.
         */
        assert_eq!(collection.trees[0].variations.len(), 2);
        assert_eq!(
            collection.trees[0].variations[0].sequence[0].first("W"),
            Some("pp")
        );

        let user_branch = &collection.trees[0].variations[1];
        assert_eq!(user_branch.sequence[0].first("W"), Some("dp"));
        assert_eq!(user_branch.variations.len(), 2);
    }

    #[test]
    fn supports_pass_as_an_inserted_move() {
        let mut collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: None,
            },
        )
        .expect("insert pass");

        assert_eq!(collection.trees[0].sequence[2].first("W"), Some(""));
    }

    #[test]
    fn rejects_wrong_colour_and_illegal_move() {
        let mut collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd])").expect("parse SGF");

        let study = build_study_tree(&collection).expect("build Study tree");

        let wrong_colour = insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::Black,
                point: Some(point(&study, 1, 15, 15)),
            },
        )
        .expect_err("wrong colour");

        assert!(wrong_colour.contains("must be White"));

        let occupied = insert_study_move(
            &mut collection,
            &study,
            1,
            Move {
                colour: Colour::White,
                point: Some(point(&study, 1, 3, 3)),
            },
        )
        .expect_err("occupied point");

        assert!(occupied.contains("occupied"));
    }
}

#[cfg(test)]
mod delete_from_here_tests {
    use super::delete_study_from_source;
    use crate::sgf::{Collection, GameTree, Node};
    use crate::study_tree::StudySourceLocation;

    fn node() -> Node {
        Node::default()
    }

    #[test]
    fn delete_from_root_sequence_truncates_every_descendant() {
        let mut collection = Collection {
            trees: vec![GameTree {
                sequence: vec![node(), node(), node()],
                variations: vec![GameTree {
                    sequence: vec![node()],
                    variations: Vec::new(),
                }],
            }],
        };

        let selected = delete_study_from_source(
            &mut collection,
            &StudySourceLocation {
                variation_path: Vec::new(),
                sequence_index: 1,
            },
        )
        .expect("delete from root sequence");

        assert_eq!(collection.trees[0].sequence.len(), 1);
        assert!(collection.trees[0].variations.is_empty());
        assert_eq!(
            selected,
            StudySourceLocation {
                variation_path: Vec::new(),
                sequence_index: 0,
            }
        );
    }

    #[test]
    fn delete_first_node_of_variation_removes_only_that_variation() {
        let mut collection = Collection {
            trees: vec![GameTree {
                sequence: vec![node(), node()],
                variations: vec![
                    GameTree {
                        sequence: vec![node(), node()],
                        variations: Vec::new(),
                    },
                    GameTree {
                        sequence: vec![node()],
                        variations: Vec::new(),
                    },
                ],
            }],
        };

        let selected = delete_study_from_source(
            &mut collection,
            &StudySourceLocation {
                variation_path: vec![0],
                sequence_index: 0,
            },
        )
        .expect("delete whole variation");

        assert_eq!(collection.trees[0].variations.len(), 1);
        assert_eq!(collection.trees[0].variations[0].sequence.len(), 1);
        assert_eq!(
            selected,
            StudySourceLocation {
                variation_path: Vec::new(),
                sequence_index: 1,
            }
        );
    }

    #[test]
    fn delete_inside_variation_keeps_earlier_nodes_and_siblings() {
        let mut collection = Collection {
            trees: vec![GameTree {
                sequence: vec![node(), node()],
                variations: vec![
                    GameTree {
                        sequence: vec![node(), node(), node()],
                        variations: vec![GameTree {
                            sequence: vec![node()],
                            variations: Vec::new(),
                        }],
                    },
                    GameTree {
                        sequence: vec![node()],
                        variations: Vec::new(),
                    },
                ],
            }],
        };

        let selected = delete_study_from_source(
            &mut collection,
            &StudySourceLocation {
                variation_path: vec![0],
                sequence_index: 1,
            },
        )
        .expect("delete inside variation");

        assert_eq!(collection.trees[0].variations.len(), 2);
        assert_eq!(collection.trees[0].variations[0].sequence.len(), 1);
        assert!(collection.trees[0].variations[0].variations.is_empty());
        assert_eq!(collection.trees[0].variations[1].sequence.len(), 1);
        assert_eq!(
            selected,
            StudySourceLocation {
                variation_path: vec![0],
                sequence_index: 0,
            }
        );
    }

    #[test]
    fn root_node_is_protected() {
        let mut collection = Collection {
            trees: vec![GameTree {
                sequence: vec![node(), node()],
                variations: Vec::new(),
            }],
        };

        let error = delete_study_from_source(
            &mut collection,
            &StudySourceLocation {
                variation_path: Vec::new(),
                sequence_index: 0,
            },
        )
        .expect_err("root node must be protected");

        assert!(error.contains("root SGF node"));
        assert_eq!(collection.trees[0].sequence.len(), 2);
    }
}
