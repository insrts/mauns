//! Mauns SDK — embed Mauns as a library in your own project.
//!
//! Minimal integration:
//! ```rust,no_run
//! use mauns_sdk::Mauns;
//!
//! #[tokio::main]
//! async fn main() {
//!     let report = Mauns::default()
//!         .run_task("write a hello world Rust function")
//!         .await
//!         .unwrap();
//!     println!("{}", report.execution.summary);
//! }
//! ```

pub mod builder;

pub use builder::Mauns;
