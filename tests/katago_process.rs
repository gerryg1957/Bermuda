use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use bermuda::{
    AnalysisRequest, Colour, KataGoConfiguration, KataGoProcess, analysis_position_from_states,
    extract_main_variation, parse_collection, replay_positions,
};
use tempfile::tempdir;

fn required_path(name: &str) -> Result<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .with_context(|| format!("{name} is not set"))
}

#[test]
#[ignore = "requires an explicitly configured local KataGo installation"]
fn analyses_empty_board_with_real_katago() -> Result<()> {
    let working_directory = tempdir()?;

    let configuration = KataGoConfiguration::new(
        required_path("BERMUDA_KATAGO_EXECUTABLE")?,
        required_path("BERMUDA_KATAGO_MODEL")?,
        required_path("BERMUDA_KATAGO_CONFIG")?,
        working_directory.path(),
    );

    let mut katago = KataGoProcess::start(&configuration)?;

    let request = AnalysisRequest {
        id: "bermuda-rust-smoke".to_owned(),
        board_size: 19,
        rules: "japanese".to_owned(),
        komi: 6.5,
        initial_stones: Vec::new(),
        initial_player: Colour::Black,
        moves: Vec::new(),
        max_visits: 50,
    };

    let result = katago.analyse(&request)?;

    katago.shutdown()?;

    assert_eq!(result.id, "bermuda-rust-smoke");
    assert_eq!(result.turn_number, 0);
    assert_eq!(result.current_player, Colour::Black);
    assert!(!result.is_during_search);
    assert!(result.visits > 0);
    assert!(!result.candidates.is_empty());

    println!(
        "KataGo returned {} candidates after {} visits",
        result.candidates.len(),
        result.visits
    );

    if let Some(candidate) = result.candidates.first() {
        println!(
            "Leading candidate: {:?}, score lead {:.3}, win rate {:.3}",
            candidate.vertex, candidate.score_lead, candidate.win_rate
        );
    }

    Ok(())
}

#[test]
#[ignore = "requires an explicitly configured local KataGo installation"]
fn analyses_setup_position_from_bermuda_replay_with_real_katago() -> Result<()> {
    let collection = parse_collection(b"(;FF[4]GM[1]SZ[19]KM[6.5]AB[dd][pd]AW[dp];W[qp])")?;

    let record = extract_main_variation(&collection)?;
    let positions = replay_positions(&record)?;
    let position = analysis_position_from_states(&positions, 0)?;

    assert!(!position.initial_stones.is_empty());
    assert_eq!(position.initial_player, Colour::White);
    assert_eq!(position.current_player, Colour::White);
    assert!(position.moves.is_empty());

    let working_directory = tempdir()?;

    let configuration = KataGoConfiguration::new(
        required_path("BERMUDA_KATAGO_EXECUTABLE")?,
        required_path("BERMUDA_KATAGO_MODEL")?,
        required_path("BERMUDA_KATAGO_CONFIG")?,
        working_directory.path(),
    );

    let mut katago = KataGoProcess::start(&configuration)?;

    let request = AnalysisRequest {
        id: "bermuda-rust-setup-smoke".to_owned(),
        board_size: position.board_size,
        rules: "japanese".to_owned(),
        komi: 6.5,
        initial_stones: position.initial_stones,
        initial_player: position.initial_player,
        moves: position.moves,
        max_visits: 50,
    };

    let result = katago.analyse(&request)?;

    katago.shutdown()?;

    assert_eq!(result.id, "bermuda-rust-setup-smoke");
    assert_eq!(result.turn_number, 0);
    assert_eq!(result.current_player, position.current_player);
    assert!(!result.is_during_search);
    assert!(result.visits > 0);
    assert!(!result.candidates.is_empty());

    println!(
        "KataGo analysed Bermuda setup position: {} candidates after {} visits",
        result.candidates.len(),
        result.visits
    );

    if let Some(candidate) = result.candidates.first() {
        println!(
            "Leading candidate: {:?}, score lead {:.3}, win rate {:.3}",
            candidate.vertex, candidate.score_lead, candidate.win_rate
        );
    }

    Ok(())
}
