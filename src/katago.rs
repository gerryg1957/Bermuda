use anyhow::{Context, Result, bail};
use serde::Deserialize;

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
    pub score_lead: f64,
    pub win_rate: f64,
    pub principal_variation: Vec<AnalysisVertex>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisResult {
    pub id: String,
    pub turn_number: u32,
    pub current_player: Colour,
    pub score_lead: f64,
    pub win_rate: f64,
    pub visits: u64,
    pub candidates: Vec<AnalysisCandidate>,
}

#[derive(Debug, Deserialize)]
struct ProtocolResponse {
    id: String,

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
}
