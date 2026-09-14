use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::Colour;

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
pub struct AnalysisRequest {
    pub id: String,
    pub board_size: u8,
    pub rules: String,
    pub komi: f64,
    pub moves: Vec<AnalysisMove>,
    pub max_visits: u64,
}

pub struct KataGoProcess {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
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
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
        })
    }

    pub fn analyse(&mut self, request: &AnalysisRequest) -> Result<AnalysisResult> {
        let request_json = analysis_request_json(request)?;

        {
            let stdin = self
                .stdin
                .as_mut()
                .context("KataGo stdin is already closed")?;

            writeln!(stdin, "{request_json}").context("writing KataGo analysis request")?;

            stdin.flush().context("flushing KataGo analysis request")?;
        }

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
            return Ok(result);
        }
    }

    pub fn shutdown(mut self) -> Result<()> {
        // Closing stdin asks KataGo to finish any queued work and exit cleanly.
        self.stdin.take();

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
        self.stdin.take();

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
struct ProtocolQuery<'a> {
    id: &'a str,
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
    #[test]
    fn serialises_analysis_request_as_katago_json() {
        let request = AnalysisRequest {
            id: "request-1".to_owned(),
            board_size: 19,
            rules: "japanese".to_owned(),
            komi: 6.5,
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
        assert_eq!(value["moves"][0][0].as_str(), Some("B"));
        assert_eq!(value["moves"][0][1].as_str(), Some("Q16"));
        assert_eq!(value["moves"][1][0].as_str(), Some("W"));
        assert_eq!(value["moves"][1][1].as_str(), Some("pass"));
    }

    #[test]
    fn gtp_output_skips_i_column() {
        let vertex = AnalysisVertex::Point(AnalysisPoint { x: 8, y: 0 });

        assert_eq!(format_vertex(vertex, 19).unwrap(), "J1");
    }
}
