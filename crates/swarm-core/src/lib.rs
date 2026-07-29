//! swarm-core — public-safe inventory engine for self-hosted Buzz on macOS.

pub mod discover;
pub mod error;
pub mod fix;
pub mod manifest;
pub mod paths;
pub mod plan;
pub mod progress;
pub mod status;

pub use discover::{discover, discover_with_progress, Discovery};
pub use error::{Error, Result};
pub use fix::{apply_safe_fixes, FixReport};
pub use manifest::{
    adopt_from_discovery, load_manifest, save_manifest, Component, Manifest, RelayRole,
};
pub use paths::Paths;
pub use plan::{
    load_plan, merge_plan, parse_relay_role, plan_is_complete, save_plan, SetupPlan, PLAN_SCHEMA,
};
pub use progress::{
    append_history, CollectSink, HistorySink, NullSink, ProgressEvent, ProgressSink, ProgressStatus,
};
pub use status::{compute_status, CheckLevel, StatusReport};
