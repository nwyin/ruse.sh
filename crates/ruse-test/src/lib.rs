//! Testing harness for ruse TUI applications.
//!
//! Provides two levels of testing:
//!
//! - [`ModelTest`]: Synchronous, drives a `Model` directly without a runtime.
//!   No tokio, no terminal. Best for unit-testing model logic.
//!
//! - [`ProgramTest`]: Async, wraps a full `Program` in headless mode.
//!   Tests command execution, async behavior, and the full event loop.
//!
//! # Example (sync)
//! ```ignore
//! use ruse_test::ModelTest;
//! use ruse_runtime::KeyCode;
//!
//! let mut t = ModelTest::new(MyModel::new());
//! t.init().send_key(KeyCode::Up);
//! assert_eq!(t.view_plain(), "Count: 1");
//! ```
//!
//! # Example (async)
//! ```ignore
//! use ruse_test::ProgramTest;
//! use ruse_runtime::KeyCode;
//!
//! #[tokio::test]
//! async fn test_model() {
//!     let t = ProgramTest::new(MyModel::new()).await;
//!     t.send_key(KeyCode::Up).unwrap();
//!     t.settle().await;
//!     let model = t.quit().await.unwrap();
//!     assert_eq!(model.count, 1);
//! }
//! ```

pub mod helpers;
pub mod model_test;
pub mod program_test;

pub use helpers::*;
pub use model_test::ModelTest;
pub use program_test::ProgramTest;

/// Re-export `insta` for convenient snapshot testing.
pub use insta;

/// Strip ANSI escape sequences from view content for human-readable snapshots.
pub fn view_plain(view: &ruse_runtime::view::View) -> String {
    ruse_ansi::strip::strip_ansi(&view.content)
}
