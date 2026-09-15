use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{Colour, replay::PositionState};

/// Zero-based board coordinate used by Bermuda analysis results.
///
/// `x` increases from left to right and `y` from the lower edge upwards.
/// This type deliberately does not expose KataGo's textual GTP coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisPoint {
    pub x: u8,
    pub y: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisVertex {
    Point(AnalysisPoint),
    Pass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisCandidate {
    pub vertex: AnalysisVertex,
    pub visits: u64,

    /// Score lead from Black's perspective when produced by `KataGoProcess`.
    pub score_lead: f64,

    /// Win probability for Black when produced by `KataGoProcess`.
    pub win_rate: f64,

    pub principal_variation: Vec<AnalysisVertex>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisResult {
    pub id: String,
    pub turn_number: u32,
    pub current_player: Colour,

    /// Score lead from Black's perspective when produced by `KataGoProcess`.
    pub score_lead: f64,

    /// Win probability for Black when produced by `KataGoProcess`.
    pub win_rate: f64,

    pub visits: u64,
    pub candidates: Vec<AnalysisCandidate>,
    pub is_during_search: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KataGoConfiguration {
    pub executable: PathBuf,
    pub model: PathBuf,
    pub config: PathBuf,
    pub working_directory: PathBuf,
}

impl KataGoConfiguration {
    pub fn new(
        executable: impl Into<PathBuf>,
        model: impl Into<PathBuf>,
        config: impl Into<PathBuf>,
        working_directory: impl Into<PathBuf>,
    ) -> Self {
        Self {
            executable: executable.into(),
            model: model.into(),
            config: config.into(),
            working_directory: working_directory.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisMove {
    pub colour: Colour,
    pub vertex: AnalysisVertex,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisStone {
    pub colour: Colour,
    pub point: AnalysisPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisRequest {
    pub id: String,
    pub board_size: u8,
    pub rules: String,
    pub komi: f64,
    pub initial_stones: Vec<AnalysisStone>,
    pub initial_player: Colour,
    pub moves: Vec<AnalysisMove>,
    pub max_visits: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisPosition {
    pub board_size: u8,
    pub initial_stones: Vec<AnalysisStone>,
    pub initial_player: Colour,
    pub moves: Vec<AnalysisMove>,
    pub current_player: Colour,
}

/// Converts Bermuda replay states into the position/history representation
/// needed for KataGo analysis.
///
/// Position zero supplies the already-resolved initial board after all setup
/// edits. Subsequent states supply the real moves that reached the requested
/// position.
pub fn analysis_position_from_states(
    positions: &[PositionState],
    move_number: usize,
) -> Result<AnalysisPosition> {
    let initial = positions
        .first()
        .context("cannot analyse an empty Bermuda position sequence")?;

    if initial.occurrence.move_number != 0 {
        bail!(
            "Bermuda analysis position sequence starts at move {}, not move 0",
            initial.occurrence.move_number
        );
    }

    let current = positions.get(move_number).with_context(|| {
        format!(
            "requested analysis move {move_number}, but replay contains only {} moves",
            positions.len().saturating_sub(1)
        )
    })?;

    let board_size = initial.board.size();
    let point_count = u16::from(board_size) * u16::from(board_size);

    let mut initial_stones = Vec::new();

    for point in 0..point_count {
        if let Some(colour) = initial.board.colour_at(point) {
            initial_stones.push(AnalysisStone {
                colour,
                point: analysis_point_from_core(point, board_size)?,
            });
        }
    }

    let mut moves = Vec::with_capacity(move_number);

    for (index, state) in positions.iter().enumerate().skip(1).take(move_number) {
        if state.occurrence.move_number != index {
            bail!(
                "Bermuda analysis replay position {index} reports move number {}",
                state.occurrence.move_number
            );
        }

        if state.board.size() != board_size {
            bail!("Bermuda analysis replay changes board size at move {index}");
        }

        let mv = state.last_move.with_context(|| {
            format!("Bermuda analysis replay position {index} has no producing move")
        })?;

        let vertex = match mv.point {
            Some(point) => AnalysisVertex::Point(analysis_point_from_core(point, board_size)?),
            None => AnalysisVertex::Pass,
        };

        moves.push(AnalysisMove {
            colour: mv.colour,
            vertex,
        });
    }

    Ok(AnalysisPosition {
        board_size,
        initial_stones,
        initial_player: initial.occurrence.side_to_move,
        moves,
        current_player: current.occurrence.side_to_move,
    })
}

fn analysis_point_from_core(point: u16, board_size: u8) -> Result<AnalysisPoint> {
    let size = u16::from(board_size);
    let point_count = size * size;

    if point >= point_count {
        bail!("Bermuda point {point} lies outside a {board_size}x{board_size} board");
    }

    Ok(AnalysisPoint {
        x: (point % size) as u8,
        y: (point / size) as u8,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisOutcome {
    Complete(AnalysisResult),
    Terminated,
}

/// A lightweight handle that may write control messages while another thread
/// is blocked waiting for an analysis response.
///
/// The KataGo process and stdout reader remain owned by `KataGoProcess`.
#[derive(Clone)]
pub struct KataGoControl {
    stdin: Arc<Mutex<Option<ChildStdin>>>,
}

impl KataGoControl {
    pub fn terminate(&self, action_id: &str, terminate_id: &str) -> Result<()> {
        let request_json = termination_request_json(action_id, terminate_id)?;

        let mut guard = self
            .stdin
            .lock()
            .map_err(|_| anyhow::anyhow!("KataGo stdin mutex is poisoned"))?;

        let stdin = guard.as_mut().context("KataGo stdin is already closed")?;

        writeln!(stdin, "{request_json}").context("writing KataGo termination request")?;

        stdin
            .flush()
            .context("flushing KataGo termination request")?;

        Ok(())
    }
}

pub struct KataGoProcess {
    child: Option<Child>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    stdout: BufReader<ChildStdout>,
}

impl KataGoProcess {
    pub fn start(configuration: &KataGoConfiguration) -> Result<Self> {
        let mut child = Command::new(&configuration.executable)
            .current_dir(&configuration.working_directory)
            .arg("analysis")
            .arg("-model")
            .arg(&configuration.model)
            .arg("-config")
            .arg(&configuration.config)
            .arg("-override-config")
            .arg("reportAnalysisWinratesAs=BLACK")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| {
                format!(
                    "starting KataGo executable {}",
                    configuration.executable.display()
                )
            })?;

        let stdin = child
            .stdin
            .take()
            .expect("KataGo stdin was configured as piped");

        let stdout = child
            .stdout
            .take()
            .expect("KataGo stdout was configured as piped");

        Ok(Self {
            child: Some(child),
            stdin: Arc::new(Mutex::new(Some(stdin))),
            stdout: BufReader::new(stdout),
        })
    }

    pub fn control(&self) -> KataGoControl {
        KataGoControl {
            stdin: Arc::clone(&self.stdin),
        }
    }

    pub fn analyse(&mut self, request: &AnalysisRequest) -> Result<AnalysisResult> {
        match self.analyse_interruptible(request)? {
            AnalysisOutcome::Complete(result) => Ok(result),
            AnalysisOutcome::Terminated => {
                bail!("KataGo analysis was terminated before producing results")
            }
        }
    }

    pub fn analyse_interruptible(&mut self, request: &AnalysisRequest) -> Result<AnalysisOutcome> {
        self.send_analysis(request)?;
        self.wait_for_analysis(request)
    }

    pub fn send_analysis(&self, request: &AnalysisRequest) -> Result<()> {
        let request_json = analysis_request_json(request)?;

        let mut guard = self
            .stdin
            .lock()
            .map_err(|_| anyhow::anyhow!("KataGo stdin mutex is poisoned"))?;

        let stdin = guard.as_mut().context("KataGo stdin is already closed")?;

        writeln!(stdin, "{request_json}").context("writing KataGo analysis request")?;

        stdin.flush().context("flushing KataGo analysis request")?;

        Ok(())
    }

    pub fn wait_for_analysis(&mut self, request: &AnalysisRequest) -> Result<AnalysisOutcome> {
        let mut warnings = Vec::new();

        loop {
            let mut line = String::new();

            let bytes = self
                .stdout
                .read_line(&mut line)
                .context("reading KataGo analysis response")?;

            if bytes == 0 {
                bail!(
                    "KataGo closed stdout before returning analysis for {:?}",
                    request.id
                );
            }

            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let notice: ProtocolNotice =
                serde_json::from_str(line).context("parsing KataGo protocol message")?;

            /*
             * A terminate action has its own acknowledgement on stdout.
             * It is not an analysis response and may arrive while we are
             * waiting for the terminated analysis to emit its final reply.
             */
            if notice.action.as_deref() == Some("terminate") {
                continue;
            }

            if notice.error.is_some() || notice.warning.is_some() {
                if let Some(id) = notice.id.as_deref()
                    && id != request.id
                {
                    bail!(
                        "KataGo returned protocol notice for request {id:?}                          while waiting for {:?}",
                        request.id
                    );
                }
            }

            if let Some(error) = notice.error {
                match notice.field {
                    Some(field) => {
                        bail!("KataGo error for field {field:?}: {error}");
                    }
                    None => {
                        bail!("KataGo error: {error}");
                    }
                }
            }

            if let Some(warning) = notice.warning {
                let text = match notice.field {
                    Some(field) => {
                        format!("KataGo warning for field {field:?}: {warning}")
                    }
                    None => format!("KataGo warning: {warning}"),
                };

                warnings.push(text);
                continue;
            }

            if notice.no_results {
                if notice.id.as_deref() != Some(request.id.as_str()) {
                    bail!(
                        "KataGo returned terminated analysis for {:?} while waiting for {:?}",
                        notice.id,
                        request.id
                    );
                }

                return Ok(AnalysisOutcome::Terminated);
            }

            let mut result = parse_analysis_response(line, request.board_size)?;

            if result.id != request.id {
                bail!(
                    "KataGo returned analysis for request {:?} while waiting for {:?}",
                    result.id,
                    request.id
                );
            }

            if result.is_during_search {
                continue;
            }

            result.warnings = warnings;
            return Ok(AnalysisOutcome::Complete(result));
        }
    }

    pub fn shutdown(mut self) -> Result<()> {
        // Closing stdin asks KataGo to finish any queued work and exit cleanly.
        self.stdin
            .lock()
            .map_err(|_| anyhow::anyhow!("KataGo stdin mutex is poisoned"))?
            .take();

        let Some(mut child) = self.child.take() else {
            return Ok(());
        };

        let status = child.wait().context("waiting for KataGo to exit")?;

        if !status.success() {
            bail!("KataGo exited with status {status}");
        }

        Ok(())
    }
}

impl Drop for KataGoProcess {
    fn drop(&mut self) {
        if let Ok(mut stdin) = self.stdin.lock() {
            stdin.take();
        }

        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => {}
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminationQuery<'a> {
    id: &'a str,
    action: &'static str,
    terminate_id: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProtocolQuery<'a> {
    id: &'a str,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    initial_stones: Vec<[String; 2]>,

    initial_player: &'static str,
    moves: Vec<[String; 2]>,
    rules: &'a str,
    komi: f64,
    board_x_size: u8,
    board_y_size: u8,
    max_visits: u64,
}

#[derive(Debug, Deserialize)]
struct ProtocolNotice {
    id: Option<String>,
    error: Option<String>,
    warning: Option<String>,
    field: Option<String>,

    action: Option<String>,

    #[serde(rename = "noResults", default)]
    no_results: bool,
}

#[derive(Debug, Deserialize)]
struct ProtocolResponse {
    id: String,

    #[serde(rename = "isDuringSearch", default)]
    is_during_search: bool,

    #[serde(rename = "turnNumber")]
    turn_number: u32,

    #[serde(rename = "moveInfos")]
    move_infos: Vec<ProtocolMoveInfo>,

    #[serde(rename = "rootInfo")]
    root_info: ProtocolRootInfo,
}

#[derive(Debug, Deserialize)]
struct ProtocolMoveInfo {
    #[serde(rename = "move")]
    vertex: String,

    visits: u64,

    #[serde(rename = "scoreLead")]
    score_lead: f64,

    winrate: f64,

    #[serde(default)]
    pv: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProtocolRootInfo {
    #[serde(rename = "currentPlayer")]
    current_player: String,

    #[serde(rename = "scoreLead")]
    score_lead: f64,

    winrate: f64,
    visits: u64,
}

fn termination_request_json(action_id: &str, terminate_id: &str) -> Result<String> {
    if action_id.trim().is_empty() {
        bail!("KataGo termination action id must not be empty");
    }

    if terminate_id.trim().is_empty() {
        bail!("KataGo termination target id must not be empty");
    }

    let query = TerminationQuery {
        id: action_id,
        action: "terminate",
        terminate_id,
    };

    serde_json::to_string(&query).context("serialising KataGo termination request")
}

fn analysis_request_json(request: &AnalysisRequest) -> Result<String> {
    if !(1..=19).contains(&request.board_size) {
        bail!(
            "unsupported Bermuda analysis board size {}",
            request.board_size
        );
    }

    if request.max_visits == 0 {
        bail!("KataGo analysis max_visits must be greater than zero");
    }

    let initial_stones = request
        .initial_stones
        .iter()
        .map(|stone| {
            Ok([
                protocol_colour(stone.colour).to_owned(),
                format_vertex(AnalysisVertex::Point(stone.point), request.board_size)?,
            ])
        })
        .collect::<Result<Vec<_>>>()?;

    let moves = request
        .moves
        .iter()
        .map(|mv| {
            Ok([
                protocol_colour(mv.colour).to_owned(),
                format_vertex(mv.vertex, request.board_size)?,
            ])
        })
        .collect::<Result<Vec<_>>>()?;

    let query = ProtocolQuery {
        id: &request.id,
        initial_stones,
        initial_player: protocol_colour(request.initial_player),
        moves,
        rules: &request.rules,
        komi: request.komi,
        board_x_size: request.board_size,
        board_y_size: request.board_size,
        max_visits: request.max_visits,
    };

    serde_json::to_string(&query).context("serialising KataGo analysis request")
}

fn protocol_colour(colour: Colour) -> &'static str {
    match colour {
        Colour::Black => "B",
        Colour::White => "W",
    }
}

fn format_vertex(vertex: AnalysisVertex, board_size: u8) -> Result<String> {
    match vertex {
        AnalysisVertex::Pass => Ok("pass".to_owned()),

        AnalysisVertex::Point(point) => {
            if point.x >= board_size || point.y >= board_size {
                bail!(
                    "analysis point ({}, {}) is outside a {}x{} board",
                    point.x,
                    point.y,
                    board_size,
                    board_size
                );
            }

            let mut column = b'A' + point.x;

            // GTP coordinates omit I.
            if column >= b'I' {
                column += 1;
            }

            Ok(format!("{}{}", char::from(column), point.y + 1))
        }
    }
}

/// Parse one completed KataGo analysis response into Bermuda-owned types.
///
/// KataGo protocol details are confined to this module. Callers receive
/// board coordinates, colours and numerical analysis rather than JSON fields
/// or GTP coordinate strings.
pub fn parse_analysis_response(json: &str, board_size: u8) -> Result<AnalysisResult> {
    if !(1..=19).contains(&board_size) {
        bail!("unsupported Bermuda analysis board size {board_size}");
    }

    let response: ProtocolResponse =
        serde_json::from_str(json).context("parsing KataGo analysis response")?;

    let current_player = parse_colour(&response.root_info.current_player)?;

    let candidates = response
        .move_infos
        .into_iter()
        .map(|candidate| {
            let vertex = parse_vertex(&candidate.vertex, board_size)?;

            let principal_variation = candidate
                .pv
                .iter()
                .map(|vertex| parse_vertex(vertex, board_size))
                .collect::<Result<Vec<_>>>()?;

            Ok(AnalysisCandidate {
                vertex,
                visits: candidate.visits,
                score_lead: candidate.score_lead,
                win_rate: candidate.winrate,
                principal_variation,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(AnalysisResult {
        id: response.id,
        turn_number: response.turn_number,
        current_player,
        score_lead: response.root_info.score_lead,
        win_rate: response.root_info.winrate,
        visits: response.root_info.visits,
        candidates,
        is_during_search: response.is_during_search,
        warnings: Vec::new(),
    })
}

fn parse_colour(value: &str) -> Result<Colour> {
    match value {
        "B" => Ok(Colour::Black),
        "W" => Ok(Colour::White),
        _ => bail!("unexpected KataGo player colour {value:?}"),
    }
}

fn parse_vertex(value: &str, board_size: u8) -> Result<AnalysisVertex> {
    if value.eq_ignore_ascii_case("pass") {
        return Ok(AnalysisVertex::Pass);
    }

    let bytes = value.as_bytes();

    if bytes.len() < 2 {
        bail!("invalid KataGo coordinate {value:?}");
    }

    let column = bytes[0].to_ascii_uppercase();

    if !column.is_ascii_uppercase() || column == b'I' {
        bail!("invalid KataGo coordinate {value:?}");
    }

    let mut x = column - b'A';
    if column > b'I' {
        x -= 1;
    }

    let row_text = value
        .get(1..)
        .context("KataGo coordinate is not valid UTF-8 at row boundary")?;

    let row = row_text
        .parse::<u8>()
        .with_context(|| format!("invalid KataGo coordinate {value:?}"))?;

    if x >= board_size || row == 0 || row > board_size {
        bail!("KataGo coordinate {value:?} is outside a {board_size}x{board_size} board");
    }

    Ok(AnalysisVertex::Point(AnalysisPoint { x, y: row - 1 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"
    {
      "id": "bermuda-smoke",
      "moveInfos": [
        {
          "move": "Q16",
          "visits": 9,
          "scoreLead": -0.385083483,
          "winrate": 0.471208354,
          "pv": ["Q16", "D4", "pass"]
        }
      ],
      "rootInfo": {
        "currentPlayer": "B",
        "scoreLead": -0.37419635,
        "visits": 55,
        "winrate": 0.471942894
      },
      "turnNumber": 0
    }
    "#;

    #[test]
    fn parses_analysis_response_into_bermuda_types() {
        let result = parse_analysis_response(RESPONSE, 19).expect("parse analysis");

        assert_eq!(result.id, "bermuda-smoke");
        assert_eq!(result.turn_number, 0);
        assert_eq!(result.current_player, Colour::Black);
        assert_eq!(result.visits, 55);
        assert_eq!(result.candidates.len(), 1);

        let candidate = &result.candidates[0];

        assert_eq!(
            candidate.vertex,
            AnalysisVertex::Point(AnalysisPoint { x: 15, y: 15 })
        );
        assert_eq!(candidate.visits, 9);
        assert_eq!(
            candidate.principal_variation,
            vec![
                AnalysisVertex::Point(AnalysisPoint { x: 15, y: 15 }),
                AnalysisVertex::Point(AnalysisPoint { x: 3, y: 3 }),
                AnalysisVertex::Pass,
            ]
        );
    }

    #[test]
    fn rejects_invalid_gtp_coordinate() {
        let response = RESPONSE.replace("\"Q16\"", "\"I16\"");

        let error =
            parse_analysis_response(&response, 19).expect_err("I is not a GTP board column");

        assert!(error.to_string().contains("invalid KataGo coordinate"));
    }

    #[test]
    fn rejects_coordinate_outside_board() {
        let response = RESPONSE.replace("\"Q16\"", "\"T20\"");

        let error =
            parse_analysis_response(&response, 19).expect_err("coordinate must be on board");

        assert!(error.to_string().contains("outside a 19x19 board"));
    }
    fn position_from_sgf(sgf: &str, move_number: usize) -> AnalysisPosition {
        let collection = crate::parse_collection(sgf.as_bytes()).expect("parse SGF");

        let record = crate::extract_main_variation(&collection).expect("extract game");

        let positions = crate::replay_positions(&record).expect("replay game");

        analysis_position_from_states(&positions, move_number).expect("convert replay position")
    }

    #[test]
    fn converts_ordinary_move_history() {
        let position = position_from_sgf("(;FF[4]GM[1]SZ[19];B[dd];W[pq])", 2);

        assert_eq!(position.board_size, 19);
        assert!(position.initial_stones.is_empty());
        assert_eq!(position.initial_player, Colour::Black);
        assert_eq!(position.current_player, Colour::Black);

        assert_eq!(
            position.moves,
            vec![
                AnalysisMove {
                    colour: Colour::Black,
                    vertex: AnalysisVertex::Point(AnalysisPoint { x: 3, y: 3 }),
                },
                AnalysisMove {
                    colour: Colour::White,
                    vertex: AnalysisVertex::Point(AnalysisPoint { x: 15, y: 16 }),
                },
            ]
        );
    }

    #[test]
    fn converts_pass_in_real_move_history() {
        let position = position_from_sgf("(;FF[4]GM[1]SZ[19];B[];W[dd])", 1);

        assert_eq!(position.current_player, Colour::White);

        assert_eq!(
            position.moves,
            vec![AnalysisMove {
                colour: Colour::Black,
                vertex: AnalysisVertex::Pass,
            }]
        );
    }

    #[test]
    fn converts_resolved_setup_board_and_white_to_move() {
        let position = position_from_sgf("(;FF[4]GM[1]SZ[19]AB[dd][pd]AW[dp]AE[pd];W[qp])", 0);

        assert_eq!(position.initial_player, Colour::White);
        assert_eq!(position.current_player, Colour::White);
        assert!(position.moves.is_empty());

        assert_eq!(
            position.initial_stones,
            vec![
                AnalysisStone {
                    colour: Colour::Black,
                    point: AnalysisPoint { x: 3, y: 3 },
                },
                AnalysisStone {
                    colour: Colour::White,
                    point: AnalysisPoint { x: 3, y: 15 },
                },
            ]
        );
    }

    #[test]
    fn converts_only_moves_up_to_requested_position() {
        let position = position_from_sgf("(;FF[4]GM[1]SZ[19];B[dd];W[pq])", 1);

        assert_eq!(position.moves.len(), 1);
        assert_eq!(position.current_player, Colour::White);

        assert_eq!(
            position.moves[0],
            AnalysisMove {
                colour: Colour::Black,
                vertex: AnalysisVertex::Point(AnalysisPoint { x: 3, y: 3 }),
            }
        );
    }

    #[test]
    fn serialises_analysis_request_as_katago_json() {
        let request = AnalysisRequest {
            id: "request-1".to_owned(),
            board_size: 19,
            rules: "japanese".to_owned(),
            komi: 6.5,
            initial_stones: vec![
                AnalysisStone {
                    colour: Colour::Black,
                    point: AnalysisPoint { x: 3, y: 3 },
                },
                AnalysisStone {
                    colour: Colour::White,
                    point: AnalysisPoint { x: 15, y: 15 },
                },
            ],
            initial_player: Colour::White,
            moves: vec![
                AnalysisMove {
                    colour: Colour::Black,
                    vertex: AnalysisVertex::Point(AnalysisPoint { x: 15, y: 15 }),
                },
                AnalysisMove {
                    colour: Colour::White,
                    vertex: AnalysisVertex::Pass,
                },
            ],
            max_visits: 50,
        };

        let json = analysis_request_json(&request).expect("serialise request");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("request must be valid JSON");

        assert_eq!(value["id"].as_str(), Some("request-1"));
        assert_eq!(value["boardXSize"].as_u64(), Some(19));
        assert_eq!(value["boardYSize"].as_u64(), Some(19));
        assert_eq!(value["maxVisits"].as_u64(), Some(50));
        assert_eq!(value["initialPlayer"].as_str(), Some("W"));
        assert_eq!(value["initialStones"][0][0].as_str(), Some("B"));
        assert_eq!(value["initialStones"][0][1].as_str(), Some("D4"));
        assert_eq!(value["initialStones"][1][0].as_str(), Some("W"));
        assert_eq!(value["initialStones"][1][1].as_str(), Some("Q16"));
        assert_eq!(value["moves"][0][0].as_str(), Some("B"));
        assert_eq!(value["moves"][0][1].as_str(), Some("Q16"));
        assert_eq!(value["moves"][1][0].as_str(), Some("W"));
        assert_eq!(value["moves"][1][1].as_str(), Some("pass"));
    }

    #[test]
    fn serialises_termination_request_as_katago_json() {
        let json =
            termination_request_json("cancel-2", "request-1").expect("serialise termination");

        let value: serde_json::Value =
            serde_json::from_str(&json).expect("termination request must be valid JSON");

        assert_eq!(value["id"].as_str(), Some("cancel-2"));
        assert_eq!(value["action"].as_str(), Some("terminate"));
        assert_eq!(value["terminateId"].as_str(), Some("request-1"));
    }

    #[test]
    fn recognises_terminated_response_without_results() {
        let notice: ProtocolNotice = serde_json::from_str(
            r#"{"id":"request-1","isDuringSearch":false,"noResults":true,"turnNumber":3}"#,
        )
        .expect("parse terminated response");

        assert_eq!(notice.id.as_deref(), Some("request-1"));
        assert!(notice.no_results);
        assert!(notice.action.is_none());
    }

    #[test]
    fn gtp_output_skips_i_column() {
        let vertex = AnalysisVertex::Point(AnalysisPoint { x: 8, y: 0 });

        assert_eq!(format_vertex(vertex, 19).unwrap(), "J1");
    }
}
