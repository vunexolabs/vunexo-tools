//! Where an app-managed receipt image lives, **relative to the app's data
//! directory** — mirrors `domain::business::resolve_logo_path`/`looks_absolute`
//! in Vunexo Billing exactly (database-schema.md §7, application-architecture.md
//! `domain/receipt.rs`).
//!
//! Every receipt this app itself writes is app-managed (there is no V1-era
//! "legacy absolute path" the way Billing's logo has, since this is a new
//! product with no upgrade history) — but the resolver still recognises an
//! absolute path *by shape*, not by `Path::is_absolute()`, for the same
//! reason Billing's does: `Path::is_absolute()` means "absolute for the OS
//! this binary happens to be compiled for", and a `.vex` backup can be
//! restored onto a different machine *and a different OS*. Keeping the same
//! shape-based check here — rather than assuming "we only ever write
//! relative paths so this doesn't matter" — is what the locked instruction
//! ("same portable-path discipline: never use `Path::is_absolute()`") is
//! asking for, and it costs nothing to keep it correct from day one.

use std::path::{Path, PathBuf};

/// Where app-managed receipts live, relative to the data directory.
pub const RECEIPTS_DIRECTORY: &str = "receipts";

/// The name a newly attached receipt is stored under: a fresh UUID plus the
/// original extension, so nothing has to parse (or collide with) a
/// user-chosen file name.
pub fn managed_receipt_path(file_name: &str) -> String {
    format!("{RECEIPTS_DIRECTORY}/{file_name}")
}

/// Whether `stored` looks like an absolute path under *any* desktop OS's
/// convention (Unix `/...`, Windows drive `C:\...`/`C:/...`, or a UNC
/// `\\server\...`) — deliberately not `Path::is_absolute()`. See the module
/// doc comment above and `vunexo-billing`'s `domain::business::looks_absolute`,
/// which this mirrors field-for-field.
fn looks_absolute(stored: &str) -> bool {
    let bytes = stored.as_bytes();
    stored.starts_with('/')
        || stored.starts_with('\\')
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

/// Whether `stored` is an app-managed receipt — i.e. one that lives in the
/// data directory and therefore travels inside a `.vex` backup.
pub fn is_managed_receipt_path(stored: &str) -> bool {
    !looks_absolute(stored)
}

/// Turns whatever is in `expenses.receipt_path` into a path that can
/// actually be opened.
pub fn resolve_receipt_path(stored: &str, data_directory: &Path) -> PathBuf {
    if looks_absolute(stored) {
        return PathBuf::from(stored);
    }
    data_directory.join(stored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_managed_receipt_is_stored_relative_so_it_survives_a_restore_elsewhere() {
        assert_eq!(
            managed_receipt_path("11111111-1111-1111-1111-111111111111.jpg"),
            "receipts/11111111-1111-1111-1111-111111111111.jpg"
        );
        assert!(is_managed_receipt_path(&managed_receipt_path("a.png")));
    }

    #[test]
    fn a_managed_path_resolves_against_the_data_directory() {
        assert_eq!(
            resolve_receipt_path("receipts/a.jpg", Path::new("/data")),
            PathBuf::from("/data/receipts/a.jpg")
        );
    }

    #[test]
    fn a_path_that_looks_absolute_is_recognised_regardless_of_which_os_wrote_it() {
        for absolute in [
            "/Users/someone/Documents/receipt.jpg",    // Unix
            r"C:\Users\someone\Documents\receipt.jpg", // Windows drive letter
            "C:/Users/someone/Documents/receipt.jpg",  // Windows, forward slashes
            r"\\server\share\receipt.jpg",             // Windows UNC
        ] {
            assert!(
                !is_managed_receipt_path(absolute),
                "expected {absolute:?} to be absolute"
            );
            assert_eq!(
                resolve_receipt_path(absolute, Path::new("/data")),
                PathBuf::from(absolute)
            );
        }
    }
}
