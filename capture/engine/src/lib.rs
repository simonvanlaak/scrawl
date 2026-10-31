//! Staged Engine package; not yet consumed by Desktop, timed recording or Agent.
#![forbid(unsafe_op_in_unsafe_fn)]
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("capture-engine supports Linux and macOS only");
mod identity;
mod locking;
mod receipt;
mod secure;
mod session;
pub use identity::{new_id, new_owner_token};
pub use receipt::{Receipt, Status};
pub use session::{Admission, Control, Runtime, Session};
