pub mod breakpoint;
pub mod session;
pub mod watchpoint;

pub mod command;
mod pipeline;
pub mod repl;

pub use pipeline::run_pipeline_repl;

pub use breakpoint::Breakpoints;
pub use session::DebuggerSession;

pub use watchpoint::{MemoryWidth, WatchTarget, WatchpointHit, Watchpoints};

pub use command::{Command, parse_command};
pub use repl::run_repl;
