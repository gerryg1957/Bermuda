use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    thread::{self, JoinHandle},
};

use anyhow::{Context, Result, anyhow, bail};

use crate::katago::{
    AnalysisOutcome, AnalysisRequest, KataGoConfiguration, KataGoControl, KataGoProcess,
};

/// Result produced by the background KataGo worker.
#[derive(Debug, Clone, PartialEq)]
pub enum KataGoWorkerEvent {
    Analysis {
        request_id: String,
        outcome: AnalysisOutcome,
    },
    Error {
        request_id: String,
        message: String,
    },
}

#[derive(Default)]
struct WorkerState {
    /// Only the newest not-yet-started request is retained.
    pending: Option<AnalysisRequest>,
    active_id: Option<String>,
    termination_requested: bool,
    stopping: bool,
}

impl WorkerState {
    fn queue_latest(&mut self, request: AnalysisRequest) -> Option<String> {
        self.pending = Some(request);
        self.request_active_termination()
    }

    fn request_active_termination(&mut self) -> Option<String> {
        if self.termination_requested {
            return None;
        }

        let active_id = self.active_id.clone()?;
        self.termination_requested = true;
        Some(active_id)
    }

    fn termination_failed(&mut self, request_id: &str) {
        if self.active_id.as_deref() == Some(request_id) {
            self.termination_requested = false;
        }
    }

    fn take_next(&mut self) -> Option<AnalysisRequest> {
        let request = self.pending.take()?;
        self.active_id = Some(request.id.clone());
        self.termination_requested = false;
        Some(request)
    }

    fn finish(&mut self, request_id: &str) {
        if self.active_id.as_deref() == Some(request_id) {
            self.active_id = None;
            self.termination_requested = false;
        }
    }
}

/// Owns one long-lived KataGo process on a background thread.
///
/// `submit_latest` implements supersession rather than an ordinary FIFO queue:
/// while one request is running, a newer request terminates it and replaces
/// any older request that has not yet started.
pub struct KataGoWorker {
    control: KataGoControl,
    state: Arc<(Mutex<WorkerState>, Condvar)>,
    events: Receiver<KataGoWorkerEvent>,
    next_action_id: AtomicU64,
    thread: Option<JoinHandle<Result<()>>>,
}

impl KataGoWorker {
    pub fn start(configuration: &KataGoConfiguration) -> Result<Self> {
        let process = KataGoProcess::start(configuration)?;
        let control = process.control();

        let state = Arc::new((Mutex::new(WorkerState::default()), Condvar::new()));
        let thread_state = Arc::clone(&state);

        let (event_sender, events) = mpsc::channel();

        let thread = thread::Builder::new()
            .name("bermuda-katago".to_owned())
            .spawn(move || run_worker(process, thread_state, event_sender))
            .context("starting Bermuda KataGo worker thread")?;

        Ok(Self {
            control,
            state,
            events,
            next_action_id: AtomicU64::new(1),
            thread: Some(thread),
        })
    }

    /// Queue an analysis request, superseding older work.
    ///
    /// If an analysis is currently running, a KataGo `terminate` action is
    /// sent immediately. Any pending request is replaced by this one.
    pub fn submit_latest(&self, request: AnalysisRequest) -> Result<()> {
        let active_id = {
            let (lock, condition) = &*self.state;
            let mut state = lock
                .lock()
                .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;

            if state.stopping {
                bail!("KataGo worker is shutting down");
            }

            let active_id = state.queue_latest(request);
            condition.notify_one();
            active_id
        };

        if let Some(active_id) = active_id {
            self.terminate_active_or_rearm(&active_id)?;
        }

        Ok(())
    }

    /// Cancel all outstanding analysis work.
    ///
    /// Any request that has not started is discarded. If an analysis is
    /// currently running, KataGo is asked to terminate it.
    pub fn cancel_all(&self) -> Result<bool> {
        let (active_id, had_pending) = {
            let (lock, _) = &*self.state;
            let mut state = lock
                .lock()
                .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;

            let had_pending = state.pending.take().is_some();
            let active_id = state.request_active_termination();

            (active_id, had_pending)
        };

        let Some(active_id) = active_id else {
            return Ok(had_pending);
        };

        self.terminate_active_or_rearm(&active_id)?;
        Ok(true)
    }

    /// Explicitly cancel the analysis currently running, if any.
    ///
    /// Returns `true` if a new termination request was sent.
    pub fn cancel_current(&self) -> Result<bool> {
        let active_id = {
            let (lock, _) = &*self.state;
            let mut state = lock
                .lock()
                .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;

            state.request_active_termination()
        };

        let Some(active_id) = active_id else {
            return Ok(false);
        };

        self.terminate_active_or_rearm(&active_id)?;
        Ok(true)
    }

    /// Non-blockingly receive one worker event.
    ///
    /// This is intended for GUI/event-loop polling.
    pub fn try_recv(&self) -> std::result::Result<KataGoWorkerEvent, TryRecvError> {
        self.events.try_recv()
    }

    pub fn shutdown(mut self) -> Result<()> {
        let stop_result = self.request_stop();
        let join_result = self.join_thread();

        match (stop_result, join_result) {
            (Err(error), _) => Err(error),
            (Ok(()), result) => result,
        }
    }

    fn terminate_active(&self, active_id: &str) -> Result<()> {
        let sequence = self.next_action_id.fetch_add(1, Ordering::Relaxed);
        let action_id = format!("bermuda-terminate-{sequence}");

        self.control.terminate(&action_id, active_id)
    }

    fn terminate_active_or_rearm(&self, active_id: &str) -> Result<()> {
        match self.terminate_active(active_id) {
            Ok(()) => Ok(()),
            Err(error) => {
                let (lock, _) = &*self.state;
                let mut state = lock
                    .lock()
                    .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;

                state.termination_failed(active_id);
                Err(error)
            }
        }
    }

    fn request_stop(&self) -> Result<()> {
        let active_id = {
            let (lock, condition) = &*self.state;
            let mut state = lock
                .lock()
                .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;

            state.stopping = true;
            state.pending = None;

            let active_id = state.request_active_termination();
            condition.notify_all();
            active_id
        };

        if let Some(active_id) = active_id {
            self.terminate_active_or_rearm(&active_id)?;
        }

        Ok(())
    }

    fn join_thread(&mut self) -> Result<()> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };

        thread
            .join()
            .map_err(|_| anyhow!("KataGo worker thread panicked"))?
    }
}

impl Drop for KataGoWorker {
    fn drop(&mut self) {
        /*
         * Do not block an event-loop thread in Drop. Ask the worker to stop;
         * explicit `shutdown()` remains available when the caller wants to
         * wait for clean process termination.
         */
        let _ = self.request_stop();

        /*
         * Dropping JoinHandle detaches the Rust thread. The thread still owns
         * KataGoProcess and will close it when the requested termination has
         * been observed.
         */
        self.thread.take();
    }
}

fn run_worker(
    mut process: KataGoProcess,
    shared: Arc<(Mutex<WorkerState>, Condvar)>,
    event_sender: Sender<KataGoWorkerEvent>,
) -> Result<()> {
    loop {
        let request = {
            let (lock, condition) = &*shared;
            let mut state = lock
                .lock()
                .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;

            while state.pending.is_none() && !state.stopping {
                state = condition
                    .wait(state)
                    .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;
            }

            if state.stopping {
                break;
            }

            state
                .take_next()
                .expect("pending request disappeared while worker held the mutex")
        };

        let request_id = request.id.clone();

        let event = match process.analyse_interruptible(&request) {
            Ok(outcome) => KataGoWorkerEvent::Analysis {
                request_id: request_id.clone(),
                outcome,
            },
            Err(error) => KataGoWorkerEvent::Error {
                request_id: request_id.clone(),
                message: format!("{error:#}"),
            },
        };

        {
            let (lock, _) = &*shared;
            let mut state = lock
                .lock()
                .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;

            state.finish(&request_id);
        }

        if event_sender.send(event).is_err() {
            break;
        }
    }

    process.shutdown()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Colour;

    fn request(id: &str) -> AnalysisRequest {
        AnalysisRequest {
            id: id.to_owned(),
            board_size: 19,
            rules: "japanese".to_owned(),
            komi: 6.5,
            initial_stones: Vec::new(),
            initial_player: Colour::Black,
            moves: Vec::new(),
            max_visits: 50,
        }
    }

    #[test]
    fn pending_analysis_keeps_only_latest_request() {
        let mut state = WorkerState::default();

        assert_eq!(state.queue_latest(request("A")), None);

        let first = state.take_next().expect("first request");
        assert_eq!(first.id, "A");
        assert_eq!(state.active_id.as_deref(), Some("A"));

        assert_eq!(state.queue_latest(request("B")).as_deref(), Some("A"));

        // A has already been asked to terminate. C supersedes B, but must not
        // send KataGo another terminate action for A.
        assert_eq!(state.queue_latest(request("C")), None);

        state.finish("A");

        let next = state.take_next().expect("latest pending request");
        assert_eq!(next.id, "C");
        assert_eq!(state.active_id.as_deref(), Some("C"));
        assert!(!state.termination_requested);
    }

    #[test]
    fn finishing_old_request_does_not_clear_new_active_request() {
        let mut state = WorkerState::default();

        state.active_id = Some("new".to_owned());
        state.finish("old");

        assert_eq!(state.active_id.as_deref(), Some("new"));
    }

    #[test]
    fn cancelling_all_discards_pending_work() {
        let mut state = WorkerState::default();

        state.active_id = Some("A".to_owned());
        state.pending = Some(request("B"));

        let had_pending = state.pending.take().is_some();
        let active_id = state.request_active_termination();

        assert!(had_pending);
        assert!(state.pending.is_none());
        assert_eq!(active_id.as_deref(), Some("A"));
        assert!(state.termination_requested);
    }

    #[test]
    fn failed_termination_can_be_retried() {
        let mut state = WorkerState::default();
        state.active_id = Some("A".to_owned());

        assert_eq!(state.request_active_termination().as_deref(), Some("A"));
        assert_eq!(state.request_active_termination(), None);

        state.termination_failed("A");

        assert_eq!(state.request_active_termination().as_deref(), Some("A"));
    }

    #[test]
    #[ignore = "requires an explicitly configured local KataGo installation"]
    fn supersedes_running_analysis_with_latest_request_using_real_katago() -> Result<()> {
        use std::{
            env,
            path::PathBuf,
            time::{Duration, Instant},
        };

        fn required_path(name: &str) -> Result<PathBuf> {
            env::var_os(name)
                .map(PathBuf::from)
                .ok_or_else(|| anyhow!("{name} is not set"))
        }

        let working_directory = tempfile::tempdir()?;

        let configuration = KataGoConfiguration::new(
            required_path("BERMUDA_KATAGO_EXECUTABLE")?,
            required_path("BERMUDA_KATAGO_MODEL")?,
            required_path("BERMUDA_KATAGO_CONFIG")?,
            working_directory.path(),
        );

        let worker = KataGoWorker::start(&configuration)?;

        let mut first = request("A");
        first.max_visits = 100_000;
        worker.submit_latest(first)?;

        // Wait until the worker has definitely taken A from the pending slot
        // and entered the active-analysis state.
        let active_deadline = Instant::now() + Duration::from_secs(5);

        loop {
            let active = {
                let (lock, _) = &*worker.state;
                let state = lock
                    .lock()
                    .map_err(|_| anyhow!("KataGo worker state mutex is poisoned"))?;

                state.active_id.clone()
            };

            if active.as_deref() == Some("A") {
                break;
            }

            if Instant::now() >= active_deadline {
                bail!("KataGo worker did not make request A active");
            }

            std::thread::sleep(Duration::from_millis(10));
        }

        // B asks A to terminate. C immediately replaces B while A's
        // termination is already in flight.
        worker.submit_latest(request("B"))?;
        worker.submit_latest(request("C"))?;

        let deadline = Instant::now() + Duration::from_secs(30);
        let mut saw_a_terminated = false;
        let mut saw_b = false;
        let mut saw_c_complete = false;

        while Instant::now() < deadline && !saw_c_complete {
            match worker.try_recv() {
                Ok(KataGoWorkerEvent::Analysis {
                    request_id,
                    outcome,
                }) => match request_id.as_str() {
                    "A" => {
                        assert_eq!(outcome, AnalysisOutcome::Terminated);
                        saw_a_terminated = true;
                    }
                    "B" => {
                        saw_b = true;
                    }
                    "C" => match outcome {
                        AnalysisOutcome::Complete(result) => {
                            assert_eq!(result.id, "C");
                            assert!(result.visits > 0);
                            assert!(!result.candidates.is_empty());
                            saw_c_complete = true;
                        }
                        AnalysisOutcome::Terminated => {
                            bail!("latest request C was unexpectedly terminated");
                        }
                    },
                    other => bail!("unexpected KataGo worker request id {other:?}"),
                },

                Ok(KataGoWorkerEvent::Error {
                    request_id,
                    message,
                }) => {
                    bail!("KataGo worker error for {request_id:?}: {message}");
                }

                Err(TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(10));
                }

                Err(TryRecvError::Disconnected) => {
                    bail!("KataGo worker event channel disconnected");
                }
            }
        }

        assert!(saw_a_terminated, "request A was not reported terminated");
        assert!(!saw_b, "superseded request B unexpectedly ran");
        assert!(saw_c_complete, "latest request C did not complete");

        worker.shutdown()?;

        Ok(())
    }
}
