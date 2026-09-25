//! Optional, per-user CPU installation. All executable downloads are pinned and
//! verified before extraction. Existing installations/settings are never changed.
use sha2::{Digest, Sha256};
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}, process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering}, time::{Duration, Instant}};

const RELEASE: &str = "https://github.com/lightvector/KataGo/releases/download/v1.18.1/";
const MODEL_URL: &str = "https://github.com/lightvector/KataGo/releases/download/v1.17.0/b10c384h6nbttflrs.bin.gz";
const MODEL_HASH: &str = "0ba27eced5180b3e3d0b898b280c541112989765e789d1eb6cd0d31b2b2c1229";
const MODEL_SIZE: u64 = 38_245_488;

#[derive(Clone)]
pub struct Plan {
    pub archive: &'static str,
    pub digest: &'static str,
    pub bytes: u64,
    pub workers: usize,
}

pub fn plan(faster: bool) -> Result<Plan, String> {
    let avx = {
        #[cfg(target_arch = "x86_64")]
        { std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") }
        #[cfg(not(target_arch = "x86_64"))]
        { false }
    };
    let (archive, digest, bytes) = package(std::env::consts::OS, std::env::consts::ARCH, avx)?;
    let logical = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
    let workers = if faster { (logical / 2).clamp(1, 6) } else { (logical / 4).clamp(1, 2) };
    Ok(Plan { archive, digest, bytes, workers })
}

fn package(os: &str, arch: &str, avx: bool) -> Result<(&'static str, &'static str, u64), String> {
    if arch != "x86_64" {
        return Err("Automatic installation currently supports Intel/AMD 64-bit Linux and Windows. Use your own installation on this system.".into());
    }
    match (os, avx) {
        ("linux", true) => Ok(("katago-v1.18.1-eigenavx2-linux-x64.zip", "33e79780dbe3bf6ee859e16f64952cdfc90f7210c8f71ad978ffcba85ad20d79", 41_821_245)),
        ("linux", false) => Ok(("katago-v1.18.1-eigen-linux-x64.zip", "993b642601e806037003d11e43775e7b4fc65281aed9b9469b7122f18fc16811", 41_780_528)),
        ("windows", true) => Ok(("katago-v1.18.1-eigenavx2-windows-x64.zip", "0d62ffa41ee04dd89dd1b80fe45e306c837231cb1e2dccb4f5780d0ca7c313db", 5_899_607)),
        ("windows", false) => Ok(("katago-v1.18.1-eigen-windows-x64.zip", "074485cf150c38aa3bb14ac9f54f2952ffefbceb44673709bbb8a83650bf95d6", 5_903_072)),
        _ => Err("Automatic installation is not yet available on this operating system. Use your own installation instead.".into()),
    }
}

pub fn summary(faster: bool) -> String {
    match plan(faster) {
        Ok(p) => serde_json::json!({"available": true,
            "description": format!("Compatible CPU setup selected for your computer: KataGo 1.18.1 with a compact 10-block network. About {:.0} MB to download; allow 300 MB of free space. No graphics driver setup is required. Bermuda will store and configure everything automatically.", (p.bytes + MODEL_SIZE) as f64 / 1_000_000.0)
        }).to_string(),
        Err(message) => serde_json::json!({"available": false, "description": message}).to_string(),
    }
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) { Err("Installation cancelled. Your saved setup has not changed.".into()) } else { Ok(()) }
}

fn download_error(error: &reqwest::Error) -> String {
    use std::error::Error;
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

fn download(client: &reqwest::blocking::Client, url: &str, path: &Path, expected: &str,
    size: u64, label: &str, cancel: &AtomicBool, progress: &impl Fn(String)) -> Result<(), String> {
    check_cancel(cancel)?;
    let mut response = client.get(url).send().and_then(|r| r.error_for_status())
        .map_err(|e| format!("Downloading {label}: {}. Check your connection and retry.", download_error(&e)))?;
    let mut output = fs::File::create(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    let mut last_percent = 101;
    loop {
        check_cancel(cancel)?;
        let n = response.read(&mut buffer).map_err(|e| format!("Downloading {label}: {e}"))?;
        if n == 0 { break; }
        total += n as u64;
        if total > size { return Err(format!("Unexpected download size for {label}")); }
        output.write_all(&buffer[..n]).map_err(|e| format!("Saving {label}: {e}"))?;
        hash.update(&buffer[..n]);
        let percent = total * 100 / size;
        if percent != last_percent {
            progress(format!("Downloading {label}: {percent}% ({:.1} / {:.1} MB)", total as f64 / 1e6, size as f64 / 1e6));
            last_percent = percent;
        }
    }
    if total != size || format!("{:x}", hash.finalize()) != expected {
        return Err(format!("Verification failed for {label}. Nothing has been activated. Please retry."));
    }
    output.sync_all().map_err(|e| e.to_string())?;
    Ok(())
}

fn unpack(zip: &Path, directory: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let file = fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut expanded = 0u64;
    if archive.len() > 2048 { return Err("Engine archive contains too many files".into()); }
    for i in 0..archive.len() {
        check_cancel(cancel)?;
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let relative = entry.enclosed_name().ok_or("Unsafe path in engine archive")?.to_path_buf();
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("Unexpected symbolic link in engine archive".into());
        }
        expanded = expanded.checked_add(entry.size()).ok_or("Archive too large")?;
        if expanded > 256 * 1024 * 1024 { return Err("Engine archive is too large".into()); }
        let target = directory.join(relative);
        if entry.is_dir() { fs::create_dir_all(&target).map_err(|e| e.to_string())?; continue; }
        fs::create_dir_all(target.parent().ok_or("Invalid archive path")?).map_err(|e| e.to_string())?;
        let mut out = fs::OpenOptions::new().write(true).create_new(true).open(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(entry.unix_mode().unwrap_or(0o644) & 0o777)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn extract_appimage(engine: &Path, directory: &Path, cancel: &AtomicBool) -> Result<PathBuf, String> {
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(engine, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    let mut child = Command::new(engine).arg("--appimage-extract").current_dir(directory)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn()
        .map_err(|e| format!("Preparing the downloaded engine: {e}"))?;
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(120) {
            let _ = child.kill(); let _ = child.wait();
            check_cancel(cancel)?;
            return Err("Engine preparation took too long. Please retry.".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() { return Err(format!("The downloaded engine could not be prepared ({status}). Your Linux system may be incompatible with this package.")); }
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => { let _ = child.kill(); let _ = child.wait(); return Err(e.to_string()); }
        }
    }
    let executable = directory.join("squashfs-root/AppRun");
    if !executable.is_file() { return Err("Downloaded engine has no application entry point".into()); }
    Ok(executable)
}

fn configuration(workers: usize) -> String {
    format!("# Managed by Bermuda. Custom configurations are kept separately.\nlogToStderr = true\nreportAnalysisWinratesAs = BLACK\nnumAnalysisThreads = 1\nnumSearchThreadsPerAnalysisThread = {workers}\nnumEigenThreadsPerModel = {workers}\nnnMaxBatchSize = {workers}\nnnCacheSizePowerOfTwo = 18\nnnMutexPoolSizePowerOfTwo = 14\n")
}

pub fn install(root: &Path, faster: bool, cancel: &AtomicBool, progress: impl Fn(String)) -> Result<String, String> {
    let p = plan(faster)?;
    fs::create_dir_all(root).map_err(|e| format!("Creating KataGo data directory: {e}"))?;
    let staging = tempfile::Builder::new().prefix("install-").tempdir_in(root).map_err(|e| e.to_string())?;
    let dir = staging.path();
    let client = reqwest::blocking::Client::builder().user_agent("Bermuda-KataGo-Setup/1")
        .https_only(true).connect_timeout(Duration::from_secs(20)).timeout(Duration::from_secs(600))
        .build().map_err(|e| e.to_string())?;
    let zip = dir.join("engine.zip");
    download(&client, &format!("{RELEASE}{}", p.archive), &zip, p.digest, p.bytes, "KataGo", cancel, &progress)?;
    progress("Preparing KataGo…".into());
    let engine_dir = dir.join("engine");
    unpack(&zip, &engine_dir, cancel)?;
    let executable = if cfg!(target_os = "linux") {
        extract_appimage(&engine_dir.join("katago"), &engine_dir, cancel)?
    } else { engine_dir.join("katago.exe") };
    if !executable.is_file() { return Err("Downloaded package has no KataGo executable".into()); }
    let model = dir.join("b10c384h6nbttflrs.bin.gz");
    download(&client, MODEL_URL, &model, MODEL_HASH, MODEL_SIZE, "network", cancel, &progress)?;
    let config = dir.join("analysis.cfg");
    fs::write(&config, configuration(p.workers)).map_err(|e| format!("Creating analysis configuration: {e}"))?;
    fs::remove_file(zip).map_err(|e| e.to_string())?;
    check_cancel(cancel)?;
    let paths = serde_json::json!({"executable": executable, "model": model, "config": config}).to_string();
    // Retain only complete, verified installs. TempDir removes partial downloads on failure/cancel.
    let _ = staging.keep();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packages_follow_platform_and_cpu_capabilities() {
        assert!(package("linux", "x86_64", true).unwrap().0.contains("eigenavx2-linux"));
        assert!(package("windows", "x86_64", false).unwrap().0.contains("eigen-windows"));
        assert!(package("linux", "aarch64", true).is_err());
        assert!(package("macos", "x86_64", true).is_err());
    }
    #[test]
    fn responsive_config_limits_both_worker_pools() {
        let c = configuration(2);
        assert!(c.contains("numAnalysisThreads = 1\n"));
        assert!(c.contains("numSearchThreadsPerAnalysisThread = 2\n"));
        assert!(c.contains("numEigenThreadsPerModel = 2\n"));
    }
    #[test]
    fn extraction_rejects_parent_paths() {
        let root = tempfile::tempdir().unwrap();
        let archive_path = root.path().join("bad.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&archive_path).unwrap());
        writer.start_file("../escape", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"bad").unwrap();
        writer.finish().unwrap();
        let result = unpack(&archive_path, &root.path().join("engine"), &AtomicBool::new(false));
        assert!(result.is_err());
        assert!(!root.path().join("escape").exists());
    }
    #[test]
    fn cancelled_install_does_not_download_or_leave_files() {
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(true);
        assert!(install(root.path(), false, &cancel, |_| {}).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
