use crate::{Collection, Colour, GameTree, Node, StudySourceLocation};

/// One literal SGF node as shown by the structural Study tree.
///
/// Unlike `StudyTreeNode`, this representation does not collapse nodes that
/// contain no move. It is therefore suitable for an SGF editor while the
/// existing `StudyTree` remains the replay/move-slider model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyStructureNode {
    pub id: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub row: usize,
    pub lane: usize,
    pub source: StudySourceLocation,
    pub move_colour: Option<Colour>,
    pub move_number: usize,
    pub has_comment: bool,
    pub is_empty: bool,
}

/// Literal SGF topology for the first game tree in a collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyStructureTree {
    pub nodes: Vec<StudyStructureNode>,
    pub main_path: Vec<usize>,
}

impl StudyStructureTree {
    #[must_use]
    pub fn node_id_for_source(&self, source: &StudySourceLocation) -> Option<usize> {
        self.nodes
            .iter()
            .find(|node| &node.source == source)
            .map(|node| node.id)
    }
}

/// Build a tree that preserves every SGF node.
///
/// This is intentionally separate from `build_study_tree()`: the latter is
/// move-oriented and drives replay, while this tree is structure-oriented
/// and will drive SGF editing.
pub fn build_study_structure_tree(collection: &Collection) -> Result<StudyStructureTree, String> {
    let game_tree = collection
        .trees
        .first()
        .ok_or_else(|| "SGF collection contains no game tree".to_owned())?;

    let mut result = StudyStructureTree {
        nodes: Vec::new(),
        main_path: Vec::new(),
    };

    let mut next_lane = 0usize;

    walk_tree(game_tree, &mut result, None, 0, 0, 0, &mut next_lane, &[])?;

    result.main_path = first_child_path(&result);

    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn walk_tree(
    tree: &GameTree,
    result: &mut StudyStructureTree,
    mut parent: Option<usize>,
    mut row: usize,
    mut move_number: usize,
    lane: usize,
    next_lane: &mut usize,
    tree_path: &[usize],
) -> Result<(), String> {
    for (sequence_index, node) in tree.sequence.iter().enumerate() {
        let move_colour = node_move_colour(node)?;

        if move_colour.is_some() {
            move_number += 1;
        }

        let id = result.nodes.len();

        result.nodes.push(StudyStructureNode {
            id,
            parent,
            children: Vec::new(),
            row,
            lane,
            source: StudySourceLocation {
                variation_path: tree_path.to_vec(),
                sequence_index,
            },
            move_colour,
            move_number,
            has_comment: node.first("C").is_some_and(|comment| !comment.is_empty()),
            is_empty: node.properties.is_empty(),
        });

        if let Some(parent_id) = parent {
            result.nodes[parent_id].children.push(id);
        }

        parent = Some(id);
        row += 1;
    }

    let Some(parent_id) = parent else {
        return Err(format!(
            "SGF game tree at variation path {tree_path:?} contains no nodes"
        ));
    };

    for (index, variation) in tree.variations.iter().enumerate() {
        let variation_lane = if index == 0 {
            lane
        } else {
            *next_lane += 1;
            *next_lane
        };

        let mut variation_path = tree_path.to_vec();
        variation_path.push(index);

        walk_tree(
            variation,
            result,
            Some(parent_id),
            row,
            move_number,
            variation_lane,
            next_lane,
            &variation_path,
        )?;
    }

    Ok(())
}

fn node_move_colour(node: &Node) -> Result<Option<Colour>, String> {
    let black = node.first("B").is_some();
    let white = node.first("W").is_some();

    match (black, white) {
        (true, true) => Err("SGF node contains both Black and White move properties".to_owned()),
        (true, false) => Ok(Some(Colour::Black)),
        (false, true) => Ok(Some(Colour::White)),
        (false, false) => Ok(None),
    }
}

fn first_child_path(tree: &StudyStructureTree) -> Vec<usize> {
    let Some(first) = tree.nodes.first() else {
        return Vec::new();
    };

    let mut path = vec![first.id];
    let mut current = first.id;

    while let Some(&child) = tree.nodes[current].children.first() {
        path.push(child);
        current = child;
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_study_tree, insert_study_node, parse_collection};

    #[test]
    fn preserves_non_move_nodes_in_sequence() {
        let collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];C[note]TR[pp];W[qq])").expect("parse SGF");

        let tree = build_study_structure_tree(&collection).expect("build structure");

        assert_eq!(tree.nodes.len(), 4);
        assert_eq!(tree.main_path, vec![0, 1, 2, 3]);

        assert_eq!(tree.nodes[0].move_colour, None);
        assert_eq!(tree.nodes[0].move_number, 0);

        assert_eq!(tree.nodes[1].move_colour, Some(Colour::Black));
        assert_eq!(tree.nodes[1].move_number, 1);

        assert_eq!(tree.nodes[2].move_colour, None);
        assert_eq!(tree.nodes[2].move_number, 1);
        assert!(tree.nodes[2].has_comment);
        assert!(!tree.nodes[2].is_empty);

        assert_eq!(tree.nodes[3].move_colour, Some(Colour::White));
        assert_eq!(tree.nodes[3].move_number, 2);
    }

    #[test]
    fn preserves_variation_topology_and_source_addresses() {
        let collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd](;C[a];W[pp])(;W[dp]))").expect("parse SGF");

        let tree = build_study_structure_tree(&collection).expect("build structure");

        assert_eq!(tree.nodes.len(), 5);
        assert_eq!(tree.nodes[1].children, vec![2, 4]);

        assert_eq!(
            tree.nodes[2].source,
            StudySourceLocation {
                variation_path: vec![0],
                sequence_index: 0,
            }
        );
        assert_eq!(
            tree.nodes[3].source,
            StudySourceLocation {
                variation_path: vec![0],
                sequence_index: 1,
            }
        );
        assert_eq!(
            tree.nodes[4].source,
            StudySourceLocation {
                variation_path: vec![1],
                sequence_index: 0,
            }
        );

        assert_eq!(tree.nodes[2].move_number, 1);
        assert_eq!(tree.nodes[3].move_number, 2);
        assert_eq!(tree.nodes[4].move_number, 2);
    }

    #[test]
    fn inserted_empty_node_becomes_visible_structure() {
        let mut collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];W[pp])").expect("parse SGF");

        let replay_tree = build_study_tree(&collection).expect("build replay tree");

        let inserted =
            insert_study_node(&mut collection, &replay_tree, 1).expect("insert empty node");

        let structure = build_study_structure_tree(&collection).expect("build structure");

        let inserted_id = structure
            .node_id_for_source(&inserted)
            .expect("find inserted node");

        let node = &structure.nodes[inserted_id];

        assert!(node.is_empty);
        assert_eq!(node.move_colour, None);
        assert_eq!(node.move_number, 1);

        let child = node.children.first().copied().expect("next move");
        assert_eq!(structure.nodes[child].move_colour, Some(Colour::White));
        assert_eq!(structure.nodes[child].move_number, 2);
    }

    #[test]
    fn structural_rows_advance_even_when_move_number_does_not() {
        let collection =
            parse_collection(b"(;FF[4]GM[1]SZ[19];B[dd];C[a];C[b];W[pp])").expect("parse SGF");

        let tree = build_study_structure_tree(&collection).expect("build structure");

        let rows = tree.nodes.iter().map(|node| node.row).collect::<Vec<_>>();
        let moves = tree
            .nodes
            .iter()
            .map(|node| node.move_number)
            .collect::<Vec<_>>();

        assert_eq!(rows, vec![0, 1, 2, 3, 4]);
        assert_eq!(moves, vec![0, 1, 1, 1, 2]);
    }
}
