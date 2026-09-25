use bermuda::{Board, Colour, Move};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use directories::ProjectDirs;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    pin::Pin,
    time::Duration,
};

const OGS_POSITION_URL: &str = "https://online-go.com/oje/position?id=";
const JOSEKI_CACHE_DIRECTORY: &str = "joseki-cache";

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");

        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, loading)]
        #[qproperty(bool, using_cache)]
        #[qproperty(QString, error_message)]
        #[qproperty(QString, status_message)]
        #[qproperty(QString, node_id)]
        #[qproperty(QString, parent_node_id)]
        #[qproperty(QString, description)]
        #[qproperty(QString, category)]
        #[qproperty(QString, tags_text)]
        #[qproperty(QString, source_description)]
        #[qproperty(QString, source_url)]
        #[qproperty(QString, stones_json)]
        #[qproperty(QString, continuations_json)]
        #[qproperty(QString, tree_json)]
        #[qproperty(QString, markup_json)]
        #[qproperty(QString, study_sgf_path)]
        #[qproperty(i32, last_move_x)]
        #[qproperty(i32, last_move_y)]
        #[qproperty(i32, move_count)]
        type JosekiModel = super::JosekiModelRust;

        #[qinvokable]
        #[cxx_name = "loadPosition"]
        fn load_position(self: Pin<&mut JosekiModel>, node_id: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "followPoint"]
        fn follow_point(self: Pin<&mut JosekiModel>, x: i32, y: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "goBack"]
        fn go_back(self: Pin<&mut JosekiModel>) -> bool;

        #[qinvokable]
        fn refresh(self: Pin<&mut JosekiModel>) -> bool;
    }

    impl cxx_qt::Threading for JosekiModel {}
}

#[derive(Debug, Clone)]
struct JosekiContinuation {
    node_id: String,
    placement: String,
    category: String,
    label: String,
    x: i32,
    y: i32,
}

#[derive(Debug, Clone)]
struct JosekiTreeNode {
    node_id: String,
    parent_node_id: Option<String>,
    children: Vec<String>,
    placement: String,
    category: String,
    label: String,
    move_number: usize,
    description: String,
    tags_text: String,
    source_description: String,
    source_url: String,
    markup_json: String,
    loaded: bool,
}

#[derive(Debug, Default)]
struct JosekiTree {
    nodes: HashMap<String, JosekiTreeNode>,
}

pub struct JosekiModelRust {
    loading: bool,
    using_cache: bool,
    error_message: QString,
    status_message: QString,
    node_id: QString,
    parent_node_id: QString,
    description: QString,
    category: QString,
    tags_text: QString,
    source_description: QString,
    source_url: QString,
    stones_json: QString,
    continuations_json: QString,
    tree_json: QString,
    markup_json: QString,
    study_sgf_path: QString,
    last_move_x: i32,
    last_move_y: i32,
    move_count: i32,

    request_id: u64,
    continuations: Vec<JosekiContinuation>,
    tree: JosekiTree,
}

impl Default for JosekiModelRust {
    fn default() -> Self {
        Self {
            loading: false,
            using_cache: false,
            error_message: QString::default(),
            status_message: QString::default(),
            node_id: QString::default(),
            parent_node_id: QString::default(),
            description: QString::default(),
            category: QString::default(),
            tags_text: QString::default(),
            source_description: QString::default(),
            source_url: QString::default(),
            stones_json: QString::from("[]"),
            continuations_json: QString::from("[]"),
            tree_json: QString::from("[]"),
            markup_json: QString::from("[]"),
            study_sgf_path: QString::default(),
            last_move_x: -1,
            last_move_y: -1,
            move_count: 0,
            request_id: 0,
            continuations: Vec::new(),
            tree: JosekiTree::default(),
        }
    }
}

struct JosekiPresentation {
    node_id: String,
    parent_node_id: String,
    description: String,
    category: String,
    tags_text: String,
    source_description: String,
    source_url: String,
    stones_json: String,
    continuations_json: String,
    markup_json: String,
    study_sgf_path: String,
    last_move_x: i32,
    last_move_y: i32,
    move_count: i32,
    using_cache: bool,
    current_placement: String,
    continuations: Vec<JosekiContinuation>,
}

fn update_joseki_tree(tree: &mut JosekiTree, presentation: &JosekiPresentation) {
    let node_id = presentation.node_id.clone();
    let move_number = usize::try_from(presentation.move_count).unwrap_or(0);

    let parent_node_id = if presentation.parent_node_id.trim().is_empty() {
        None
    } else {
        Some(presentation.parent_node_id.clone())
    };

    if let Some(parent_id) = parent_node_id.as_ref() {
        let parent = tree
            .nodes
            .entry(parent_id.clone())
            .or_insert_with(|| JosekiTreeNode {
                node_id: parent_id.clone(),
                parent_node_id: None,
                children: Vec::new(),
                placement: String::new(),
                category: String::new(),
                label: String::new(),
                move_number: move_number.saturating_sub(1),
                description: String::new(),
                tags_text: String::new(),
                source_description: String::new(),
                source_url: String::new(),
                markup_json: "[]".to_owned(),
                loaded: false,
            });

        if !parent.children.iter().any(|child| child == &node_id) {
            parent.children.push(node_id.clone());
        }
    }

    let children = presentation
        .continuations
        .iter()
        .map(|continuation| continuation.node_id.clone())
        .collect::<Vec<_>>();

    {
        let current = tree
            .nodes
            .entry(node_id.clone())
            .or_insert_with(|| JosekiTreeNode {
                node_id: node_id.clone(),
                parent_node_id: parent_node_id.clone(),
                children: Vec::new(),
                placement: presentation.current_placement.clone(),
                category: presentation.category.clone(),
                label: String::new(),
                move_number,
                description: presentation.description.clone(),
                tags_text: presentation.tags_text.clone(),
                source_description: presentation.source_description.clone(),
                source_url: presentation.source_url.clone(),
                markup_json: presentation.markup_json.clone(),
                loaded: true,
            });

        current.parent_node_id = parent_node_id;
        current.children = children;
        current.placement = presentation.current_placement.clone();
        current.category = presentation.category.clone();
        current.move_number = move_number;
        current.description = presentation.description.clone();
        current.tags_text = presentation.tags_text.clone();
        current.source_description = presentation.source_description.clone();
        current.source_url = presentation.source_url.clone();
        current.markup_json = presentation.markup_json.clone();
        current.loaded = true;
    }

    for continuation in &presentation.continuations {
        let child_move_number = move_number.saturating_add(1);

        let child = tree
            .nodes
            .entry(continuation.node_id.clone())
            .or_insert_with(|| JosekiTreeNode {
                node_id: continuation.node_id.clone(),
                parent_node_id: Some(node_id.clone()),
                children: Vec::new(),
                placement: continuation.placement.clone(),
                category: continuation.category.clone(),
                label: continuation.label.clone(),
                move_number: child_move_number,
                description: String::new(),
                tags_text: String::new(),
                source_description: String::new(),
                source_url: String::new(),
                markup_json: "[]".to_owned(),
                loaded: false,
            });

        child.parent_node_id = Some(node_id.clone());
        child.placement = continuation.placement.clone();
        child.category = continuation.category.clone();
        child.label = continuation.label.clone();
        child.move_number = child_move_number;
    }
}

fn append_joseki_tree_nodes(
    tree: &JosekiTree,
    node_id: &str,
    lane: usize,
    next_lane: &mut usize,
    visited: &mut HashSet<String>,
    output: &mut Vec<Value>,
) {
    if !visited.insert(node_id.to_owned()) {
        return;
    }

    let Some(node) = tree.nodes.get(node_id) else {
        return;
    };

    let colour = if node.move_number == 0 {
        "root"
    } else if node.move_number % 2 == 1 {
        "black"
    } else {
        "white"
    };

    output.push(serde_json::json!({
        "id": node.node_id,
        "parent": node.parent_node_id,
        "row": node.move_number,
        "lane": lane,
        "colour": colour,
        "category": node.category,
        "label": node.label,
        "placement": node.placement,
        "loaded": node.loaded,
        "branchPoint": node.children.len() > 1,
    }));

    let visible_children = node
        .children
        .iter()
        .filter(|child| tree.nodes.contains_key(*child))
        .cloned()
        .collect::<Vec<_>>();

    let Some(first_child) = visible_children.first() else {
        return;
    };

    append_joseki_tree_nodes(tree, first_child, lane, next_lane, visited, output);

    for child in visible_children.iter().skip(1) {
        let child_lane = *next_lane;
        *next_lane = next_lane.saturating_add(1);

        append_joseki_tree_nodes(tree, child, child_lane, next_lane, visited, output);
    }
}

fn joseki_tree_json(tree: &JosekiTree) -> String {
    let root_id = if tree.nodes.contains_key("root") {
        Some("root".to_owned())
    } else {
        tree.nodes
            .values()
            .min_by_key(|node| node.move_number)
            .map(|node| node.node_id.clone())
    };

    let Some(root_id) = root_id else {
        return "[]".to_owned();
    };

    let mut output = Vec::new();
    let mut visited = HashSet::new();
    let mut next_lane = 1;

    append_joseki_tree_nodes(tree, &root_id, 0, &mut next_lane, &mut visited, &mut output);

    serde_json::to_string(&output).unwrap_or_else(|_| "[]".to_owned())
}

fn explored_main_children(
    tree: &JosekiTree,
    current_node_id: &str,
) -> Result<HashMap<String, String>, String> {
    let current = tree
        .nodes
        .get(current_node_id)
        .ok_or_else(|| format!("OGS joseki node {current_node_id} is not in the explored tree"))?;

    let expected_depth = current.move_number;
    let mut preferred = HashMap::new();
    let mut visited = HashSet::new();
    let mut child_id = current_node_id.to_owned();
    let mut depth = 0usize;

    loop {
        if !visited.insert(child_id.clone()) {
            return Err("cycle in explored OGS joseki tree".to_owned());
        }

        if child_id == "root" {
            break;
        }

        let child = tree
            .nodes
            .get(&child_id)
            .ok_or_else(|| format!("OGS joseki node {child_id} is missing"))?;

        let parent_id = child
            .parent_node_id
            .as_ref()
            .ok_or_else(|| format!("OGS joseki node {child_id} has no parent"))?
            .clone();

        preferred.insert(parent_id.clone(), child_id);
        child_id = parent_id;
        depth = depth.saturating_add(1);
    }

    if depth != expected_depth {
        return Err(format!(
            "explored OGS joseki path is incomplete: move {expected_depth}, path depth {depth}"
        ));
    }

    Ok(preferred)
}

fn qml_point_to_sgf(x: i64, y: i64) -> Option<String> {
    let x = u8::try_from(x).ok()?;
    let y = u8::try_from(y).ok()?;

    if x >= 19 || y >= 19 {
        return None;
    }

    Some(format!("{}{}", char::from(b'a' + x), char::from(b'a' + y)))
}

fn append_joseki_markup(sgf: &mut String, markup_json: &str) {
    let Ok(Value::Array(marks)) = serde_json::from_str::<Value>(markup_json) else {
        return;
    };

    for mark in marks {
        if mark.get("type").and_then(Value::as_str) != Some("label") {
            continue;
        }

        let Some(x) = mark.get("x").and_then(Value::as_i64) else {
            continue;
        };
        let Some(y) = mark.get("y").and_then(Value::as_i64) else {
            continue;
        };
        let Some(text) = mark.get("text").and_then(Value::as_str) else {
            continue;
        };
        let Some(point) = qml_point_to_sgf(x, y) else {
            continue;
        };

        let label = sgf_escape(text).replace(':', "\\:");
        sgf.push_str(&format!("LB[{point}:{label}]"));
    }
}

fn joseki_tree_node_comment(node: &JosekiTreeNode) -> String {
    let mut details = Vec::new();

    if node.loaded {
        if !node.description.trim().is_empty() {
            details.push(node.description.trim().to_owned());
        }

        if !node.category.trim().is_empty() {
            details.push(format!("Last move: {}", node.category.trim()));
        }

        if !node.label.trim().is_empty() {
            details.push(format!("Variation: {}", node.label.trim()));
        }

        if !node.tags_text.trim().is_empty() {
            details.push(format!("Tags: {}", node.tags_text.trim()));
        }

        if !node.source_description.trim().is_empty() {
            details.push(format!("Source: {}", node.source_description.trim()));
        }

        if !node.source_url.trim().is_empty() {
            details.push(node.source_url.trim().to_owned());
        }
    } else {
        if !node.category.trim().is_empty() {
            details.push(node.category.trim().to_owned());
        }

        if !node.label.trim().is_empty() {
            details.push(format!("variation {}", node.label.trim()));
        }
    }

    details.push(format!("OGS Joseki Explorer position {}", node.node_id));
    details.join("\n\n")
}

fn append_explored_joseki_node(
    sgf: &mut String,
    node: &JosekiTreeNode,
    root: bool,
) -> Result<(), String> {
    if !root {
        let colour = if node.move_number % 2 == 1 { 'B' } else { 'W' };
        let point = ogs_coordinate_to_sgf(&node.placement)?;
        sgf.push_str(&format!(";{colour}[{point}]"));
    }

    let comment = joseki_tree_node_comment(node);
    if !comment.is_empty() {
        sgf.push_str(&format!("C[{}]", sgf_escape(&comment)));
    }

    append_joseki_markup(sgf, &node.markup_json);
    Ok(())
}

fn append_explored_joseki_descendants(
    sgf: &mut String,
    tree: &JosekiTree,
    parent_id: &str,
    preferred: &HashMap<String, String>,
    ancestry: &mut HashSet<String>,
) -> Result<(), String> {
    let parent = tree
        .nodes
        .get(parent_id)
        .ok_or_else(|| format!("OGS joseki node {parent_id} is missing"))?;

    let mut children = parent
        .children
        .iter()
        .filter(|child| tree.nodes.contains_key(*child))
        .cloned()
        .collect::<Vec<_>>();

    if let Some(preferred_child) = preferred.get(parent_id)
        && let Some(index) = children.iter().position(|child| child == preferred_child)
    {
        children.swap(0, index);
    }

    if children.len() == 1 {
        let child_id = &children[0];

        if !ancestry.insert(child_id.clone()) {
            return Err("cycle in explored OGS joseki tree".to_owned());
        }

        let child = tree
            .nodes
            .get(child_id)
            .ok_or_else(|| format!("OGS joseki node {child_id} is missing"))?;

        append_explored_joseki_node(sgf, child, false)?;
        append_explored_joseki_descendants(sgf, tree, child_id, preferred, ancestry)?;
        ancestry.remove(child_id);
        return Ok(());
    }

    for child_id in children {
        if !ancestry.insert(child_id.clone()) {
            return Err("cycle in explored OGS joseki tree".to_owned());
        }

        let child = tree
            .nodes
            .get(&child_id)
            .ok_or_else(|| format!("OGS joseki node {child_id} is missing"))?;

        sgf.push('(');
        append_explored_joseki_node(sgf, child, false)?;
        append_explored_joseki_descendants(sgf, tree, &child_id, preferred, ancestry)?;
        sgf.push(')');

        ancestry.remove(&child_id);
    }

    Ok(())
}

fn build_explored_study_sgf(tree: &JosekiTree, current_node_id: &str) -> Result<String, String> {
    let preferred = explored_main_children(tree, current_node_id)?;

    let root = tree
        .nodes
        .get("root")
        .ok_or_else(|| "the explored OGS joseki tree has no root".to_owned())?;

    let mut sgf = format!(
        "(;GM[1]FF[4]CA[UTF-8]SZ[19]GN[{}]",
        sgf_escape(&format!("OGS Joseki Explorer position {current_node_id}"))
    );

    append_explored_joseki_node(&mut sgf, root, true)?;

    let mut ancestry = HashSet::new();
    ancestry.insert("root".to_owned());

    append_explored_joseki_descendants(&mut sgf, tree, "root", &preferred, &mut ancestry)?;

    sgf.push(')');
    Ok(sgf)
}

impl ffi::JosekiModel {
    fn load_position(self: Pin<&mut Self>, node_id: &QString) -> bool {
        start_load(self, node_id.to_string())
    }

    fn follow_point(self: Pin<&mut Self>, x: i32, y: i32) -> bool {
        let node_id = {
            let self_ref = self.as_ref();
            self_ref
                .rust()
                .continuations
                .iter()
                .find(|continuation| continuation.x == x && continuation.y == y)
                .map(|continuation| continuation.node_id.clone())
        };

        let Some(node_id) = node_id else {
            return false;
        };

        start_load(self, node_id)
    }

    fn go_back(self: Pin<&mut Self>) -> bool {
        let node_id = self.as_ref().rust().parent_node_id.to_string();

        if node_id.trim().is_empty() {
            return false;
        }

        start_load(self, node_id)
    }

    fn refresh(self: Pin<&mut Self>) -> bool {
        let node_id = {
            let current = self.as_ref().rust().node_id.to_string();

            if current.trim().is_empty() {
                "root".to_owned()
            } else {
                current
            }
        };

        start_load(self, node_id)
    }
}

fn start_load(mut model: Pin<&mut ffi::JosekiModel>, requested_node: String) -> bool {
    let node_id = match normalize_node_id(&requested_node) {
        Ok(node_id) => node_id,

        Err(error) => {
            model.as_mut().set_error_message(QString::from(error));
            return false;
        }
    };

    let request_id = {
        let mut rust = model.as_mut().rust_mut();
        rust.request_id = rust.request_id.wrapping_add(1);
        rust.request_id
    };

    model.as_mut().set_loading(true);
    model.as_mut().set_using_cache(false);
    model.as_mut().set_error_message(QString::default());
    model
        .as_mut()
        .set_status_message(QString::from("Loading OGS Joseki Explorer…"));

    let qt_thread = model.qt_thread();

    std::thread::spawn(move || {
        let result = load_position_data(&node_id);

        qt_thread
            .queue(move |model| {
                finish_load(model, request_id, result);
            })
            .ok();
    });

    true
}

fn finish_load(
    mut model: Pin<&mut ffi::JosekiModel>,
    request_id: u64,
    result: Result<JosekiPresentation, String>,
) {
    if model.as_ref().rust().request_id != request_id {
        return;
    }

    match result {
        Ok(presentation) => {
            let (tree_json, explored_study_sgf) = {
                let mut rust = model.as_mut().rust_mut();

                update_joseki_tree(&mut rust.tree, &presentation);

                (
                    joseki_tree_json(&rust.tree),
                    build_explored_study_sgf(&rust.tree, &presentation.node_id).ok(),
                )
            };

            /*
             * parse_position() has already written a single-position Study
             * snapshot. Replace it with the complete explored tree when the
             * accumulated path back to root is available. If it is not (for
             * example after opening a deep OGS node directly), the snapshot
             * remains a valid fallback.
             */
            if let Some(sgf) = explored_study_sgf {
                let _ = fs::write(&presentation.study_sgf_path, sgf);
            }

            let status = if presentation.using_cache {
                format!(
                    "Cached OGS data · position {} · offline fallback",
                    presentation.node_id
                )
            } else {
                format!("Live OGS data · position {}", presentation.node_id)
            };

            model.as_mut().set_using_cache(presentation.using_cache);
            model.as_mut().set_error_message(QString::default());
            model.as_mut().set_status_message(QString::from(status));
            model
                .as_mut()
                .set_node_id(QString::from(presentation.node_id));
            model
                .as_mut()
                .set_parent_node_id(QString::from(presentation.parent_node_id));
            model
                .as_mut()
                .set_description(QString::from(presentation.description));
            model
                .as_mut()
                .set_category(QString::from(presentation.category));
            model
                .as_mut()
                .set_tags_text(QString::from(presentation.tags_text));
            model
                .as_mut()
                .set_source_description(QString::from(presentation.source_description));
            model
                .as_mut()
                .set_source_url(QString::from(presentation.source_url));
            model
                .as_mut()
                .set_stones_json(QString::from(presentation.stones_json));
            model
                .as_mut()
                .set_continuations_json(QString::from(presentation.continuations_json));
            model.as_mut().set_tree_json(QString::from(tree_json));
            model
                .as_mut()
                .set_markup_json(QString::from(presentation.markup_json));
            model
                .as_mut()
                .set_study_sgf_path(QString::from(presentation.study_sgf_path));
            model.as_mut().set_last_move_x(presentation.last_move_x);
            model.as_mut().set_last_move_y(presentation.last_move_y);
            model.as_mut().set_move_count(presentation.move_count);

            model.as_mut().rust_mut().continuations = presentation.continuations;
        }

        Err(error) => {
            model
                .as_mut()
                .set_status_message(QString::from("Could not load OGS joseki"));
            model.as_mut().set_error_message(QString::from(error));
        }
    }

    /*
     * QML treats loading=false as the completion signal. Publish the
     * complete OGS position first so the first Joseki Library load cannot
     * observe the model's initial empty board state.
     */
    model.as_mut().set_loading(false);
}

fn normalize_node_id(value: &str) -> Result<String, String> {
    let value = value.trim();

    if value.eq_ignore_ascii_case("root") {
        return Ok("root".to_owned());
    }

    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(value.to_owned());
    }

    Err(format!("invalid OGS joseki position identifier {value:?}"))
}

fn load_position_data(requested_node: &str) -> Result<JosekiPresentation, String> {
    let network_result = fetch_position(requested_node);

    match network_result {
        Ok(body) => match parse_position(requested_node, &body, false) {
            Ok(presentation) => {
                let _ = write_cached_response(requested_node, &body);
                Ok(presentation)
            }

            Err(network_parse_error) => {
                load_cached_position(requested_node).map_err(|cache_error| {
                    format!(
                        "OGS returned unusable data ({network_parse_error}); \
                             cached fallback also failed: {cache_error}"
                    )
                })
            }
        },

        Err(network_error) => load_cached_position(requested_node).map_err(|cache_error| {
            format!(
                "contacting OGS: {network_error}; \
                         no usable cached copy is available: {cache_error}"
            )
        }),
    }
}

fn fetch_position(node_id: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!(
            "Bermuda/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/gerryg1957/Bermuda)"
        ))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| format!("creating HTTP client: {error}"))?;

    let response = client
        .get(format!("{OGS_POSITION_URL}{node_id}"))
        .send()
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;

    response
        .text()
        .map_err(|error| format!("reading OGS response: {error}"))
}

fn joseki_cache_root() -> Result<PathBuf, String> {
    let project_dirs = ProjectDirs::from("org", "Bermuda", "Bermuda")
        .ok_or_else(|| "could not determine the per-user Bermuda data directory".to_owned())?;

    Ok(project_dirs.data_local_dir().join(JOSEKI_CACHE_DIRECTORY))
}

fn cached_position_path(node_id: &str) -> Result<PathBuf, String> {
    Ok(joseki_cache_root()?
        .join("positions")
        .join(format!("{node_id}.json")))
}

fn study_sgf_path(node_id: &str) -> Result<PathBuf, String> {
    Ok(joseki_cache_root()?
        .join("studies")
        .join(format!("ogs-joseki-{node_id}.sgf")))
}

fn write_cached_response(node_id: &str, body: &str) -> Result<(), String> {
    let path = cached_position_path(node_id)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("creating joseki cache {}: {error}", parent.display()))?;
    }

    fs::write(&path, body)
        .map_err(|error| format!("writing joseki cache {}: {error}", path.display()))
}

fn load_cached_position(node_id: &str) -> Result<JosekiPresentation, String> {
    let path = cached_position_path(node_id)?;

    let body = fs::read_to_string(&path)
        .map_err(|error| format!("reading joseki cache {}: {error}", path.display()))?;

    parse_position(node_id, &body, true)
}

fn parse_position(
    requested_node: &str,
    body: &str,
    using_cache: bool,
) -> Result<JosekiPresentation, String> {
    let value: Value =
        serde_json::from_str(body).map_err(|error| format!("decoding OGS joseki JSON: {error}"))?;

    let node_id = scalar_string(value.get("node_id"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| requested_node.to_owned());

    let description =
        present_ogs_markdown(&scalar_string(value.get("description")).unwrap_or_default());

    let category = scalar_string(value.get("category")).unwrap_or_default();

    let play = scalar_string(value.get("play")).unwrap_or_else(|| ".root.".to_owned());

    let moves = parse_play_sequence(&play);

    let current_placement = moves.last().cloned().unwrap_or_else(|| "root".to_owned());

    let (stones_json, last_move_x, last_move_y) = replay_ogs_moves(&moves)?;

    let parent_node_id = value
        .get("parent")
        .and_then(|parent| {
            if parent.is_object() {
                scalar_string(parent.get("node_id").or_else(|| parent.get("id")))
            } else {
                scalar_string(Some(parent))
            }
        })
        .unwrap_or_default();

    let continuations = parse_continuations(value.get("next_moves"));

    let continuations_json = serde_json::to_string(
        &continuations
            .iter()
            .map(|continuation| {
                serde_json::json!({
                    "nodeId": continuation.node_id,
                    "placement": continuation.placement,
                    "category": continuation.category,
                    "label": continuation.label,
                    "x": continuation.x,
                    "y": continuation.y,
                })
            })
            .collect::<Vec<_>>(),
    )
    .map_err(|error| format!("encoding OGS continuations: {error}"))?;

    let markup_json = parse_marks(value.get("marks"))?;

    let tags_text = value
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| scalar_string(tag.get("description")))
                .filter(|text| !text.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default();

    let source = value.get("joseki_source");

    let source_description = source
        .and_then(|source| scalar_string(source.get("description")))
        .unwrap_or_default();

    let source_url = source
        .and_then(|source| scalar_string(source.get("url")))
        .unwrap_or_default();

    let sgf = build_study_sgf(
        &node_id,
        &moves,
        &description,
        &category,
        &tags_text,
        &source_description,
        &source_url,
        &markup_json,
        &continuations,
    )?;

    let sgf_path = study_sgf_path(&node_id)?;

    if let Some(parent) = sgf_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("creating joseki Study cache {}: {error}", parent.display())
        })?;
    }

    fs::write(&sgf_path, sgf)
        .map_err(|error| format!("writing joseki Study cache {}: {error}", sgf_path.display()))?;

    Ok(JosekiPresentation {
        node_id,
        parent_node_id,
        description,
        category,
        tags_text,
        source_description,
        source_url,
        stones_json,
        continuations_json,
        markup_json,
        study_sgf_path: sgf_path.to_string_lossy().into_owned(),
        last_move_x,
        last_move_y,
        move_count: i32::try_from(moves.len()).unwrap_or(i32::MAX),
        using_cache,
        current_placement,
        continuations,
    })
}

fn present_ogs_markdown(source: &str) -> String {
    let source = source
        .lines()
        .map(|line| {
            let hashes = line.chars().take_while(|ch| *ch == '#').count();

            if (1..=6).contains(&hashes)
                && line
                    .chars()
                    .nth(hashes)
                    .is_some_and(|ch| !ch.is_whitespace())
            {
                format!("{} {}", &line[..hashes], &line[hashes..])
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut output = String::with_capacity(source.len());
    let mut rest = source.as_str();

    while let Some(start) = rest.find('<') {
        output.push_str(&rest[..start]);

        let after_open = &rest[start + 1..];
        let Some(relative_end) = after_open.find('>') else {
            output.push_str(&rest[start..]);
            rest = "";
            break;
        };

        let tag = &after_open[..relative_end];
        let replacement = if let Some(position) = tag.strip_prefix("position:") {
            let position = position.trim();

            if !position.is_empty() && position.bytes().all(|byte| byte.is_ascii_digit()) {
                Some(format!(
                    "[Position {position}](https://online-go.com/joseki/{position})"
                ))
            } else {
                None
            }
        } else if let Some((label, coordinate)) = tag.split_once(':') {
            let valid_label =
                label.len() == 1 && label.bytes().all(|byte| byte.is_ascii_uppercase());

            let valid_coordinate = parse_ogs_coordinate(coordinate).is_ok();

            if valid_label && valid_coordinate {
                Some(format!("**{label}**"))
            } else {
                None
            }
        } else {
            None
        };

        match replacement {
            Some(replacement) => output.push_str(&replacement),
            None => {
                output.push('<');
                output.push_str(tag);
                output.push('>');
            }
        }

        rest = &after_open[relative_end + 1..];
    }

    output.push_str(rest);
    output
}

fn scalar_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn parse_play_sequence(play: &str) -> Vec<String> {
    play.split(['.', ','])
        .map(str::trim)
        .filter(|token| !token.is_empty() && !token.eq_ignore_ascii_case("root"))
        .map(str::to_owned)
        .collect()
}

fn parse_ogs_coordinate(coordinate: &str) -> Result<Option<(u8, u8, i32, i32)>, String> {
    let coordinate = coordinate.trim();

    if coordinate.eq_ignore_ascii_case("pass") {
        return Ok(None);
    }

    let mut chars = coordinate.chars();

    let column = chars
        .next()
        .ok_or_else(|| "empty OGS coordinate".to_owned())?
        .to_ascii_uppercase();

    let columns = "ABCDEFGHJKLMNOPQRST";

    let x = columns
        .find(column)
        .ok_or_else(|| format!("invalid OGS coordinate {coordinate:?}"))?;

    let row_text = chars.collect::<String>();

    let row = row_text
        .parse::<u8>()
        .map_err(|_| format!("invalid OGS coordinate {coordinate:?}"))?;

    if !(1..=19).contains(&row) {
        return Err(format!("invalid OGS coordinate {coordinate:?}"));
    }

    let x = u8::try_from(x).map_err(|_| "OGS coordinate overflow".to_owned())?;

    let core_y = 19 - row;
    let qml_y = i32::from(19 - row);

    Ok(Some((x, core_y, i32::from(x), qml_y)))
}

fn replay_ogs_moves(moves: &[String]) -> Result<(String, i32, i32), String> {
    let mut board = Board::new(19).map_err(|error| error.to_string())?;

    let mut colour = Colour::Black;
    let mut last_x = -1;
    let mut last_y = -1;

    for coordinate in moves {
        let parsed = parse_ogs_coordinate(coordinate)?;

        let point = match parsed {
            Some((x, core_y, qml_x, qml_y)) => {
                last_x = qml_x;
                last_y = qml_y;

                Some(board.point(x, core_y).map_err(|error| error.to_string())?)
            }

            None => {
                last_x = -1;
                last_y = -1;
                None
            }
        };

        board
            .play_archival(Move { colour, point })
            .map_err(|error| format!("replaying OGS joseki move {coordinate}: {error}"))?;

        colour = colour.opponent();
    }

    Ok((board_stones_json(&board), last_x, last_y))
}

fn board_stones_json(board: &Board) -> String {
    let size = u16::from(board.size());
    let point_count = size * size;
    let mut stones = Vec::new();

    for point in 0..point_count {
        let Some(colour) = board.colour_at(point) else {
            continue;
        };

        let x = point % size;
        let core_y = point / size;
        let qml_y = core_y;

        stones.push(serde_json::json!({
            "x": x,
            "y": qml_y,
            "color": match colour {
                Colour::Black => "black",
                Colour::White => "white",
            },
        }));
    }

    serde_json::to_string(&stones).unwrap_or_else(|_| "[]".to_owned())
}

fn parse_continuations(value: Option<&Value>) -> Vec<JosekiContinuation> {
    let Some(next_moves) = value.and_then(Value::as_array) else {
        return Vec::new();
    };

    next_moves
        .iter()
        .filter_map(|item| {
            let node_id = scalar_string(item.get("node_id"))?;

            let placement = scalar_string(item.get("placement"))?;

            let category = scalar_string(item.get("category")).unwrap_or_default();

            let label = scalar_string(item.get("variation_label")).unwrap_or_default();

            let (x, y) = match parse_ogs_coordinate(&placement) {
                Ok(Some((_, _, qml_x, qml_y))) => (qml_x, qml_y),

                Ok(None) => (-1, -1),
                Err(_) => (-1, -1),
            };

            Some(JosekiContinuation {
                node_id,
                placement,
                category,
                label,
                x,
                y,
            })
        })
        .collect()
}

fn parse_marks(value: Option<&Value>) -> Result<String, String> {
    let marks_value = match value {
        Some(Value::String(text)) if !text.trim().is_empty() => serde_json::from_str::<Value>(text)
            .map_err(|error| format!("decoding OGS board marks: {error}"))?,

        Some(Value::Array(_)) => value.cloned().unwrap_or(Value::Array(Vec::new())),

        _ => Value::Array(Vec::new()),
    };

    let marks = marks_value
        .as_array()
        .map(|marks| {
            marks
                .iter()
                .filter_map(|mark| {
                    let label = scalar_string(mark.get("label"))?;

                    let position = scalar_string(mark.get("position"))?;

                    let (_, _, x, y) = parse_ogs_coordinate(&position).ok().flatten()?;

                    Some(serde_json::json!({
                        "type": "label",
                        "x": x,
                        "y": y,
                        "text": label,
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    serde_json::to_string(&marks).map_err(|error| format!("encoding OGS board marks: {error}"))
}

fn build_study_sgf(
    node_id: &str,
    moves: &[String],
    description: &str,
    category: &str,
    tags: &str,
    source_description: &str,
    source_url: &str,
    markup_json: &str,
    continuations: &[JosekiContinuation],
) -> Result<String, String> {
    let mut details = Vec::new();

    if !description.trim().is_empty() {
        details.push(description.trim().to_owned());
    }

    if !category.trim().is_empty() {
        details.push(format!("Last move: {category}"));
    }

    if !tags.trim().is_empty() {
        details.push(format!("Tags: {tags}"));
    }

    if !source_description.trim().is_empty() {
        details.push(format!("Source: {source_description}"));
    }

    if !source_url.trim().is_empty() {
        details.push(source_url.trim().to_owned());
    }

    details.push(format!("OGS Joseki Explorer position {node_id}"));

    let current_comment = sgf_escape(&details.join("\n\n"));

    let mut sgf = format!(
        "(;GM[1]FF[4]CA[UTF-8]SZ[19]GN[{}]",
        sgf_escape(&format!("OGS Joseki Explorer position {node_id}"))
    );

    if moves.is_empty() {
        sgf.push_str(&format!("C[{current_comment}]"));
        append_joseki_markup(&mut sgf, markup_json);
    }

    for (index, coordinate) in moves.iter().enumerate() {
        let colour = if index % 2 == 0 { 'B' } else { 'W' };
        let point = ogs_coordinate_to_sgf(coordinate)?;

        sgf.push_str(&format!(";{colour}[{point}]"));

        if index + 1 == moves.len() {
            sgf.push_str(&format!("C[{current_comment}]"));
            append_joseki_markup(&mut sgf, markup_json);
        }
    }

    let next_colour = if moves.len() % 2 == 0 { 'B' } else { 'W' };

    for continuation in continuations {
        let point = ogs_coordinate_to_sgf(&continuation.placement)?;

        let mut comment_parts = Vec::new();

        if !continuation.category.trim().is_empty() {
            comment_parts.push(continuation.category.clone());
        }

        if !continuation.label.trim().is_empty() {
            comment_parts.push(format!("variation {}", continuation.label));
        }

        comment_parts.push(format!("OGS position {}", continuation.node_id));

        let comment = sgf_escape(&comment_parts.join(" · "));

        sgf.push_str(&format!("(;{next_colour}[{point}]C[{comment}])"));
    }

    sgf.push(')');
    Ok(sgf)
}

fn ogs_coordinate_to_sgf(coordinate: &str) -> Result<String, String> {
    let Some((x, _, _, qml_y)) = parse_ogs_coordinate(coordinate)? else {
        return Ok(String::new());
    };

    let x = char::from(b'a' + x);

    let qml_y =
        u8::try_from(qml_y).map_err(|_| format!("invalid OGS coordinate {coordinate:?}"))?;

    let y = char::from(b'a' + qml_y);

    Ok(format!("{x}{y}"))
}

fn sgf_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(']', "\\]")
        .replace('\r', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ogs_coordinates() {
        assert_eq!(parse_ogs_coordinate("Q16").unwrap(), Some((15, 3, 15, 3)));

        assert_eq!(parse_ogs_coordinate("D4").unwrap(), Some((3, 15, 3, 15)));

        assert_eq!(parse_ogs_coordinate("pass").unwrap(), None);
    }

    #[test]
    #[ignore = "writes a joseki cache file in the platform data directory"]
    fn parses_position_payload() {
        let json = r#"{
            "node_id": 42,
            "description": "A test joseki",
            "category": "IDEAL",
            "play": ".root.Q16.D4.",
            "parent": {"node_id": 41},
            "next_moves": [
                {
                    "node_id": 43,
                    "placement": "R17",
                    "category": "GOOD",
                    "variation_label": "A"
                }
            ],
            "marks": "[{\\"label\\":\\"A\\",\\"position\\":\\"Q16\\"}]",
            "tags": [
                {"description": "Current"}
            ],
            "joseki_source": {
                "description": "Test source",
                "url": "https://example.invalid/"
            }
        }"#;

        let presentation = parse_position("42", json, false).unwrap();

        assert_eq!(presentation.node_id, "42");
        assert_eq!(presentation.parent_node_id, "41");
        assert_eq!(presentation.move_count, 2);
        assert_eq!(presentation.last_move_x, 3);
        assert_eq!(presentation.last_move_y, 15);
        assert_eq!(presentation.continuations.len(), 1);
        assert_eq!(presentation.continuations[0].x, 16);
        assert_eq!(presentation.continuations[0].y, 2);
        assert!(presentation.tags_text.contains("Current"));
        assert!(presentation.study_sgf_path.contains("ogs-joseki-42.sgf"));
    }
}
