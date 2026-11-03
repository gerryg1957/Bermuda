use std::{path::PathBuf, pin::Pin, sync::{Arc, atomic::{AtomicBool, Ordering}}};
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
        #[qproperty(bool, busy)]
        #[qproperty(i32, processed)]
        #[qproperty(i32, total)]
        #[qproperty(QString, result_json)]
        #[qproperty(QString, error_message)]
        type PersonalImportModel = super::PersonalImportModelRust;
        #[qinvokable]
        #[cxx_name = "importFiles"]
        fn import_files(self: Pin<&mut PersonalImportModel>, paths_json: &QString);
        #[qinvokable]
        fn stop(self: Pin<&mut PersonalImportModel>);
    }
    impl cxx_qt::Threading for PersonalImportModel {}
}

#[derive(Default)]
pub struct PersonalImportModelRust {
    busy: bool,
    processed: i32,
    total: i32,
    result_json: QString,
    error_message: QString,
    stop: Option<Arc<AtomicBool>>,
}
impl ffi::PersonalImportModel {
    fn import_files(mut self: Pin<&mut Self>, paths_json: &QString) {
        if self.rust().busy { return; }
        self.as_mut().set_error_message(QString::default());
        self.as_mut().set_result_json(QString::default());
        let paths: Vec<PathBuf> = match serde_json::from_str(&paths_json.to_string()) {
            Ok(paths) => paths,
            Err(error) => { self.as_mut().set_error_message(QString::from(error.to_string())); return; }
        };
        if paths.is_empty() { return; }
        self.as_mut().set_processed(0);
        self.as_mut().set_total(paths.len().min(i32::MAX as usize) as i32);
        let stop = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().stop = Some(stop.clone());
        self.as_mut().set_busy(true);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<_> {
                let project = crate::app::open_or_create_personal_project().map_err(anyhow::Error::msg)?;
                bermuda::personal_import::import_files(&project, &paths, &stop, |count| {
                    let _ = thread.queue(move |mut model| model.as_mut().set_processed(count.min(i32::MAX as usize) as i32));
                })
            })();
            let _ = thread.queue(move |mut model| {
                match result {
                    Ok(result) => match serde_json::to_string(&result) {
                        Ok(json) => model.as_mut().set_result_json(QString::from(json)),
                        Err(error) => model.as_mut().set_error_message(QString::from(error.to_string())),
                    },
                    Err(error) => model.as_mut().set_error_message(QString::from(format!("{error:#}"))),
                }
                model.as_mut().set_busy(false);
            });
        });
    }
    fn stop(self: Pin<&mut Self>) {
        if let Some(stop) = &self.rust().stop { stop.store(true, Ordering::Relaxed); }
    }
}
impl Drop for PersonalImportModelRust {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop { stop.store(true, Ordering::Relaxed); }
    }
}
