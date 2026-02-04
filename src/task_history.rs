use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};

use log::warn;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
enum HistoryElem {
    Taskset(String),
    Taskline(String),
    TasklineEntry(usize),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct History {
    history: Vec<HistoryElem>,
}

impl History {
    pub fn new() -> Self {
        Self { history: Vec::new() }
    }

    pub fn push_taskset<S: AsRef<str>>(&mut self, name: S) {
        self.history.push(HistoryElem::Taskset(name.as_ref().to_string()));
    }

    pub fn push_taskline<S: AsRef<str>>(&mut self, name: S) {
        self.history.push(HistoryElem::Taskline(name.as_ref().to_string()));
    }

    pub fn push_taskline_entry(&mut self, number: usize) {
        self.history.push(HistoryElem::TasklineEntry(number));
    }

    pub fn write(&self, file: &Option<Arc<Mutex<File>>>) {
        if let Some(file) = file {
            let mut file = file.lock().unwrap();
            if let Ok(json_history) = serde_json::to_string(&self.history) {
                if file.write_all(format!("{}\n", json_history).as_bytes()).is_err() {
                    warn!("Failed to write task history: {}", &json_history);
                }
            } else {
                warn!("Failed to json encode task history: {:?}", &self.history);
            }
        }
    }
}
