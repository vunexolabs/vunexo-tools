//! Thin Tauri command handlers: deserialize input, call into
//! `crate::application`, serialize output. No business logic here.
//! See docs/vunexo-billing/architecture.md.

/// Round 1 technical spike: proves the React -> Tauri -> Rust round trip.
/// Replaced by real commands (create_invoice, record_payment, ...) in later rounds.
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}! Vunexo Billing is wired up: React -> Tauri -> Rust.")
}
