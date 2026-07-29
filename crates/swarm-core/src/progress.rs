use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressStatus {
    Pending,
    Running,
    Ok,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub ts: DateTime<Utc>,
    pub step: String,
    pub status: ProgressStatus,
    pub pct: u8,
    pub msg: String,
}

impl ProgressEvent {
    pub fn new(
        step: impl Into<String>,
        status: ProgressStatus,
        pct: u8,
        msg: impl Into<String>,
    ) -> Self {
        Self {
            ts: Utc::now(),
            step: step.into(),
            status,
            pct: pct.min(100),
            msg: msg.into(),
        }
    }

    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

pub trait ProgressSink: Send {
    fn emit(&mut self, event: ProgressEvent);
}

pub struct NullSink;
impl ProgressSink for NullSink {
    fn emit(&mut self, _event: ProgressEvent) {}
}

pub struct HistorySink {
    path: std::path::PathBuf,
    inner: Option<Box<dyn ProgressSink>>,
}

impl HistorySink {
    pub fn new(path: impl Into<std::path::PathBuf>, inner: Option<Box<dyn ProgressSink>>) -> Self {
        Self {
            path: path.into(),
            inner,
        }
    }
}

impl ProgressSink for HistorySink {
    fn emit(&mut self, event: ProgressEvent) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(f, "{}", event.to_json_line());
        }
        if let Some(inner) = self.inner.as_mut() {
            inner.emit(event);
        }
    }
}

#[derive(Default, Clone)]
pub struct CollectSink {
    events: Arc<Mutex<Vec<ProgressEvent>>>,
}

impl CollectSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<ProgressEvent> {
        self.events.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl ProgressSink for CollectSink {
    fn emit(&mut self, event: ProgressEvent) {
        if let Ok(mut g) = self.events.lock() {
            g.push(event);
        }
    }
}

pub fn append_history(path: &Path, event: &ProgressEvent) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", event.to_json_line());
    }
}
