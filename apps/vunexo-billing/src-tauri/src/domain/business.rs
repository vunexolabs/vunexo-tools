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

/// Whether `stored` looks like an absolute path under *any* desktop OS's
/// convention (Unix `/...`, Windows drive `C:\...`/`C:/...`, or a UNC
/// `\\server\...`) — deliberately not `Path::is_absolute()`, which means
/// "absolute for the OS this binary happens to be compiled for". A `.vbx`
/// backup can be restored onto a different machine *and a different OS*
/// (database-schema.md §9's whole reason for existing); a legacy Unix path
/// restored onto Windows would fail `Path::is_absolute()` there and get
/// misjudged as a *managed*, relative one — silently joined onto the data
/// directory instead of opened as-is. Found via CI's Windows runner: two
/// tests using a Unix-style legacy path failed only on that target.
fn looks_absolute(stored: &str) -> bool {
    let bytes = stored.as_bytes();
    stored.starts_with('/')
        || stored.starts_with('\\')
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

/// Whether `stored` is an app-managed logo — i.e. one that lives in the data
/// directory and therefore travels inside a backup.
pub fn is_managed_logo_path(stored: &str) -> bool {
    !looks_absolute(stored)
}

/// Turns whatever is in `business.logo_path` into a path that can actually be
/// opened.
///
/// Absolute values are passed through unchanged. Those are the *legacy*
/// shape — before logos were imported into the data directory, the picker
/// stored wherever the user's file happened to be — and they still work on
/// the machine that chose them, so they are honoured rather than broken.
pub fn resolve_logo_path(stored: &str, data_directory: &Path) -> PathBuf {
    if looks_absolute(stored) {
        return PathBuf::from(stored);
    }
    data_directory.join(stored)
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

    /// A legacy absolute path is recognised by its *shape*, not by
    /// `Path::is_absolute()` against whatever OS this test happens to run
    /// on: a `.vbx` restored onto a different machine can mean a different
    /// OS too. CI's Windows runner caught the regression this guards
    /// (`Path::new("/Users/...").is_absolute()` is `false` on Windows, and
    /// `Path::new("C:\\...").is_absolute()` is `false` on Unix) — both must
    /// be treated as absolute regardless of which OS is asking.
    #[test]
    fn a_legacy_path_is_recognised_as_absolute_regardless_of_which_os_wrote_it() {
        for legacy in [
            "/Users/someone/Documents/logo.png",    // Unix
            r"C:\Users\someone\Documents\logo.png", // Windows drive letter
            "C:/Users/someone/Documents/logo.png",  // Windows, forward slashes
            r"\\server\share\logo.png",             // Windows UNC
        ] {
            assert!(
                !is_managed_logo_path(legacy),
                "expected {legacy:?} to be absolute"
            );
            assert_eq!(
                resolve_logo_path(legacy, Path::new("/data")),
                PathBuf::from(legacy)
            );
        }
    }
}
