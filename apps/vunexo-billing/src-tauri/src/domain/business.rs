//! The single business profile. database-schema.md §13 (`business`, id
//! fixed at 1) — application-architecture.md §3b.

use std::path::{Path, PathBuf};

/// Where an app-managed logo lives, **relative to the app's data
/// directory**. database-schema.md §9 requires assets to sit there so a
/// `.vbx` can carry them and a restore can put them back.
///
/// It has to be stored relative, not absolute: the data directory's real
/// path contains the user's account name, so an absolute path restored onto
/// a different machine — or a different user account — points at nothing.
pub const MANAGED_LOGO_DIRECTORY: &str = "assets";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Business {
    pub name: String,
    pub logo_path: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub gstin: Option<String>,
    pub bank_details: Option<String>,
    pub upi_id: Option<String>,
}

/// The name a newly imported logo is stored under. Fixed, so nothing has to
/// parse a user-chosen file name later; only the extension varies, because
/// the image decoder picks its format from the bytes but other tools (and
/// the OS) still expect a sensible suffix.
pub fn managed_logo_path(extension: &str) -> String {
    let extension = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    let extension = if extension.is_empty() {
        "png".to_string()
    } else {
        extension
    };
    format!("{MANAGED_LOGO_DIRECTORY}/business-logo.{extension}")
}

/// Whether `stored` is an app-managed logo — i.e. one that lives in the data
/// directory and therefore travels inside a backup.
pub fn is_managed_logo_path(stored: &str) -> bool {
    !Path::new(stored).is_absolute()
}

/// Turns whatever is in `business.logo_path` into a path that can actually be
/// opened.
///
/// Absolute values are passed through unchanged. Those are the *legacy*
/// shape — before logos were imported into the data directory, the picker
/// stored wherever the user's file happened to be — and they still work on
/// the machine that chose them, so they are honoured rather than broken.
pub fn resolve_logo_path(stored: &str, data_directory: &Path) -> PathBuf {
    let path = Path::new(stored);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    data_directory.join(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_managed_logo_is_stored_relative_so_it_survives_a_restore_elsewhere() {
        assert_eq!(managed_logo_path("png"), "assets/business-logo.png");
        assert!(is_managed_logo_path(&managed_logo_path("png")));
    }

    #[test]
    fn the_extension_is_normalised_and_defaults_to_png() {
        assert_eq!(managed_logo_path("JPG"), "assets/business-logo.jpg");
        assert_eq!(managed_logo_path(".jpeg"), "assets/business-logo.jpeg");
        assert_eq!(managed_logo_path(""), "assets/business-logo.png");
    }

    #[test]
    fn a_managed_path_resolves_against_the_data_directory() {
        assert_eq!(
            resolve_logo_path("assets/business-logo.png", Path::new("/data")),
            PathBuf::from("/data/assets/business-logo.png")
        );
    }

    #[test]
    fn a_legacy_absolute_path_still_opens_on_the_machine_that_chose_it() {
        // Pre-import logos pointed anywhere on disk. Those must keep working
        // rather than break the moment this changed.
        let legacy = "/Users/someone/Documents/logo.png";
        assert!(!is_managed_logo_path(legacy));
        assert_eq!(
            resolve_logo_path(legacy, Path::new("/data")),
            PathBuf::from(legacy)
        );
    }
}
