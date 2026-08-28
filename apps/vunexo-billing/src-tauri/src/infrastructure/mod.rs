//! Concrete implementations of ports defined in `crate::application`:
//! SQLite persistence, filesystem I/O, PDF rendering.

pub mod database;
pub mod filesystem;
pub mod pdf;
