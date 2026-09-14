use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use bermuda::{AnalysisRequest, Colour, KataGoConfiguration, KataGoProcess};
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
