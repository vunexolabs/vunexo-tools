//! `ReceiptStore` over `std::fs` — copy-in-on-attach, delete-on-remove, same
//! pattern as Billing's business-logo management
//! (`application::business::import_logo_if_chosen`), generalized to however
//! many receipt files a user has attached across all their expenses.

use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::receipt_store::ReceiptStore;
use crate::domain::receipt::{
    is_managed_receipt_path, managed_receipt_path, resolve_receipt_path, RECEIPTS_DIRECTORY,
};

pub struct FsReceiptStore {
    data_directory: PathBuf,
}

impl FsReceiptStore {
    pub fn new(data_directory: PathBuf) -> Self {
        Self { data_directory }
    }
}

fn io(context: &str, err: impl std::fmt::Display) -> InfrastructureError {
    InfrastructureError::Io(format!("{context}: {err}"))
}

impl ReceiptStore for FsReceiptStore {
    fn attach(&self, source: &Path) -> Result<String, InfrastructureError> {
        let extension = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_ascii_lowercase();
        let file_name = format!("{}.{extension}", Uuid::new_v4());
        let managed = managed_receipt_path(&file_name);
        let destination = self.data_directory.join(&managed);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| io(&format!("could not create {}", parent.display()), err))?;
        }
        std::fs::copy(source, &destination).map_err(|err| {
            io(
                &format!(
                    "could not copy {} to {}",
                    source.display(),
                    destination.display()
                ),
                err,
            )
        })?;
        Ok(managed)
    }

    fn remove(&self, relative_path: &str) -> Result<(), InfrastructureError> {
        if !is_managed_receipt_path(relative_path) {
            // Defensive: every `receipt_path` this app itself ever writes is
            // managed/relative (there is no legacy-absolute case in a brand
            // new product). A value that isn't shaped that way reaching here
            // would mean something upstream went wrong — refusing silently
            // is safer than deleting an arbitrary file elsewhere on disk.
            return Ok(());
        }
        let resolved = resolve_receipt_path(relative_path, &self.data_directory);
        match std::fs::remove_file(&resolved) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(io(&format!("could not remove {}", resolved.display()), err)),
        }
    }

    fn list_all(&self) -> Result<Vec<(String, PathBuf)>, InfrastructureError> {
        let receipts_directory = self.data_directory.join(RECEIPTS_DIRECTORY);
        if !receipts_directory.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&receipts_directory).map_err(|err| {
            io(
                &format!("could not read {}", receipts_directory.display()),
                err,
            )
        })?;
        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| io("could not read a directory entry", err))?;
            let path = entry.path();
            if path.is_file() {
                let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                files.push((file_name.to_string(), path));
            }
        }
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "expense_manager_receipts_{tag}_{}_{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn attach_copies_the_file_under_a_generated_uuid_name_preserving_extension() {
        let dir = unique_dir("attach");
        let source = dir.join("my-receipt.JPG");
        std::fs::write(&source, b"bytes").unwrap();

        let store = FsReceiptStore::new(dir.clone());
        let managed = store.attach(&source).expect("attach");
        assert!(managed.starts_with("receipts/"));
        assert!(managed.ends_with(".jpg"));
        assert!(dir.join(&managed).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_is_a_no_op_when_the_file_is_already_gone() {
        let dir = unique_dir("remove_missing");
        let store = FsReceiptStore::new(dir.clone());
        assert!(store.remove("receipts/does-not-exist.jpg").is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_all_returns_every_file_in_the_receipts_directory() {
        let dir = unique_dir("list_all");
        let store = FsReceiptStore::new(dir.clone());
        assert!(store.list_all().unwrap().is_empty());

        let source = dir.join("a.png");
        std::fs::write(&source, b"bytes").unwrap();
        store.attach(&source).unwrap();
        let source2 = dir.join("b.png");
        std::fs::write(&source2, b"more bytes").unwrap();
        store.attach(&source2).unwrap();

        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
