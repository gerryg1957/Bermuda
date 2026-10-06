//! Explicit CI check of the same guided installer used by the interface.
//! Never invoked by normal startup; downloads are confined to the supplied directory.
use std::{path::PathBuf, sync::atomic::AtomicBool};
use bermuda::{AnalysisRequest, Colour, KataGoConfiguration, KataGoProcess};

pub fn run_if_requested() -> Option<Result<(), String>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).and_then(|s| s.to_str()) != Some("--check-katago-install") {
        return None;
    }
    Some((|| {
        if args.len() != 3 {
            return Err("Usage: Bermuda.exe --check-katago-install DIRECTORY".into());
        }
        let root = PathBuf::from(&args[2]);
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let installed = crate::katago_install::install(
            &root, false, &AtomicBool::new(false), |message| eprintln!("{message}"),
        )?;
        let paths: serde_json::Value = serde_json::from_str(&installed).map_err(|e| e.to_string())?;
        let path = |key: &str| -> Result<PathBuf, String> {
            paths[key].as_str().map(PathBuf::from)
                .ok_or_else(|| format!("Installer returned no {key}"))
        };
        let config = KataGoConfiguration::new(path("executable")?, path("model")?, path("config")?, &root);
        let mut engine = KataGoProcess::start(&config).map_err(|e| e.to_string())?;
        let result = engine.analyse(&AnalysisRequest {
            id: "windows-guided-install-check".into(), board_size: 19,
            rules: "japanese".into(), komi: 6.5,
            initial_stones: vec![], initial_player: Colour::Black, moves: vec![], max_visits: 4,
        }).map_err(|e| e.to_string())?;
        engine.shutdown().map_err(|e| e.to_string())?;
        if result.visits == 0 || result.candidates.is_empty() {
            return Err("KataGo returned no analysis candidates".into());
        }
        let report = serde_json::json!({
            "success": true, "visits": result.visits,
            "candidates": result.candidates.len(),
            "archive": crate::katago_install::plan(false)?.archive,
            "version": env!("CARGO_PKG_VERSION")
        });
        std::fs::write(root.join("result.json"), report.to_string()).map_err(|e| e.to_string())?;
        Ok(())
    })())
}
