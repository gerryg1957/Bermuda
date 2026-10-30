use std::{path::Path, pin::Pin, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, report_json)]
        #[qproperty(QString, error_message)]
        #[qproperty(bool, loading)]
        #[qproperty(bool, batch_busy)]
        #[qproperty(QString, batch_status)]
        #[qproperty(QString, queue_json)]
        #[qproperty(i32, completed_games)]
        #[qproperty(i32, total_games)]
        #[qproperty(i32, completed_positions)]
        #[qproperty(i32, total_positions)]
        type PersonalReportModel = super::PersonalReportModelRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut PersonalReportModel>, project_path: &QString);
        #[qinvokable]
        #[cxx_name = "startBatch"]
        fn start_batch(self: Pin<&mut PersonalReportModel>, project_path: &QString, resume: bool,
            visits: i32, rules: &QString, fallback_komi: &QString,
            executable: &QString, network: &QString, configuration: &QString);
        #[qinvokable]
        #[cxx_name = "pauseBatch"]
        fn pause_batch(self: Pin<&mut PersonalReportModel>);
        #[qinvokable]
        #[cxx_name = "clearQueue"]
        fn clear_queue(self: Pin<&mut PersonalReportModel>, project_path: &QString);
    }
    impl cxx_qt::Threading for PersonalReportModel {}
}

#[derive(Default)]
pub struct PersonalReportModelRust {
    report_json: QString,
    error_message: QString,
    loading: bool,
    batch_busy: bool,
    batch_status: QString,
    queue_json: QString,
    completed_games: i32,
    total_games: i32,
    completed_positions: i32,
    total_positions: i32,
    stop: Option<Arc<AtomicBool>>,
}

impl ffi::PersonalReportModel {
    fn refresh(mut self: Pin<&mut Self>, project_path: &QString) {
        if self.rust().loading || self.rust().batch_busy { return; }
        self.as_mut().set_loading(true);
        self.as_mut().set_report_json(QString::from("{}"));
        self.as_mut().set_error_message(QString::default());
        let path = project_path.to_string();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<_> {
                let project = bermuda::project_manager::ProjectManager::new().open(Path::new(&path))?;
                let analysis = crate::app::katago_working_directory().map_err(anyhow::Error::msg)?
                    .join("game-analysis.sqlite3");
                Ok((bermuda::personal_report::build(&project, &analysis)?, bermuda::personal_batch::summary(&project)?))
            })();
            let _ = thread.queue(move |mut model| {
                model.as_mut().set_loading(false);
                match result {
                    Ok((report, queue)) => {
                        model.as_mut().set_report_json(QString::from(report.to_string()));
                        model.as_mut().set_queue_json(QString::from(queue.to_string()));
                    }
                    Err(error) => model.as_mut().set_error_message(QString::from(format!("{error:#}"))),
                }
            });
        });
    }
}

impl Drop for PersonalReportModelRust {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop { stop.store(true, Ordering::Relaxed); }
    }
}

impl ffi::PersonalReportModel {
    fn pause_batch(mut self: Pin<&mut Self>) {
        if let Some(stop) = &self.rust().stop { stop.store(true, Ordering::Relaxed); }
        self.as_mut().set_batch_status(QString::from("Pausing; saved positions will be kept…"));
    }

    fn clear_queue(mut self: Pin<&mut Self>, project_path: &QString) {
        if self.rust().batch_busy || self.rust().loading { return; }
        let result = bermuda::project_manager::ProjectManager::new().open(project_path.to_string())
            .and_then(|p| bermuda::personal_batch::clear(&p));
        match result {
            Ok(()) => {
                self.as_mut().set_batch_status(QString::from("Queue cleared; saved analysis retained"));
                self.refresh(project_path);
            }
            Err(error) => self.as_mut().set_error_message(QString::from(error.to_string())),
        }
    }

    fn start_batch(mut self: Pin<&mut Self>, project_path: &QString, resume: bool,
        visits: i32, rules: &QString, fallback_komi: &QString,
        executable: &QString, network: &QString, configuration: &QString) {
        if self.rust().batch_busy || self.rust().loading { return; }
        let input = (|| -> anyhow::Result<_> {
            let fallback: f64 = if resume { 6.5 } else { fallback_komi.to_string().trim().parse()? };
            anyhow::ensure!(resume || (fallback.is_finite() && (10..=100000).contains(&visits)), "Invalid analysis settings");
            let config = if resume { None } else {
                Some(crate::app::review_configuration(executable, network, configuration).map_err(anyhow::Error::msg)?)
            };
            let working = crate::app::katago_working_directory().map_err(anyhow::Error::msg)?;
            Ok((fallback, config, working))
        })();
        let (fallback, config, working) = match input {
            Ok(v) => v,
            Err(error) => { self.as_mut().set_batch_status(QString::from(format!("Cannot start: {error:#}"))); return; }
        };
        let path = project_path.to_string();
        let rules = rules.to_string();
        let stop = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().stop = Some(Arc::clone(&stop));
        self.as_mut().set_completed_games(0);
        self.as_mut().set_total_games(0);
        self.as_mut().set_completed_positions(0);
        self.as_mut().set_total_positions(0);
        self.as_mut().set_batch_status(QString::from("Preparing queue and identifying engine files…"));
        self.as_mut().set_batch_busy(true);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<_> {
                let project = bermuda::project_manager::ProjectManager::new().open(&path)?;
                let analysis = working.join("game-analysis.sqlite3");
                let batch = if resume {
                    bermuda::personal_batch::load(&project)?.ok_or_else(|| anyhow::anyhow!("No saved queue"))?
                } else {
                    bermuda::personal_batch::create(&project, &analysis, config.as_ref().unwrap(),
                        visits as u64, &rules, fallback, &stop)?
                };
                let config = bermuda::personal_batch::configuration(&batch, &working)?;
                let mut timing = None;
                bermuda::personal_batch::run(&project, batch, &analysis, &config, &stop, |index,total,run| {
                    let now = std::time::Instant::now();
                    let (old_index, initial, started) = timing.get_or_insert((index, run.results.len(), now));
                    if *old_index != index { *old_index=index; *initial=run.results.len(); *started=now; }
                    let seconds = started.elapsed().as_secs_f64();
                    let rate = (run.results.len().saturating_sub(*initial)) as f64 / seconds.max(0.001);
                    let eta = if rate > 0.0 { format!(" · {:.1} positions/min · about {:.0} min left in this game",
                        rate * 60.0, (run.total()-run.results.len()) as f64 / rate / 60.0) } else { String::new() };
                    let status = format!("Game {} of {} · {} · {}/{} positions saved{}",
                        index+1, total, run.title, run.results.len(), run.total(), eta);
                    let saved = run.results.len() as i32;
                    let positions = run.total() as i32;
                    thread.queue(move |mut model| {
                        model.as_mut().set_completed_games(index as i32);
                        model.as_mut().set_total_games(total as i32);
                        model.as_mut().set_completed_positions(saved);
                        model.as_mut().set_total_positions(positions);
                        // Progress already queued before Pause must not replace
                        // the acknowledgement with another running message.
                        let pausing = model.rust().stop.as_ref()
                            .map(|stop| stop.load(Ordering::Relaxed)).unwrap_or(false);
                        if !pausing { model.as_mut().set_batch_status(QString::from(status)); }
                    }).is_ok()
                })
            })();
            let _ = thread.queue(move |mut model| {
                match result {
                    Ok(batch) => {
                        model.as_mut().set_completed_games(batch.completed as i32);
                        model.as_mut().set_total_games(batch.jobs.len() as i32);
                        let status = if batch.completed == batch.jobs.len() { "Batch complete — analysis saved" }
                            else { "Paused — resume this queue here, including after restarting Bermuda" };
                        model.as_mut().set_batch_status(QString::from(status));
                    }
                    Err(error) => model.as_mut().set_batch_status(QString::from(format!("Batch stopped: {error:#}. Saved positions retained."))),
                }
                model.as_mut().rust_mut().stop = None;
                model.as_mut().set_batch_busy(false);
            });
        });
    }
}
