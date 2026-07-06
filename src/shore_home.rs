use std::path::PathBuf;

use crate::error::{Result, ShoreError};

/// Environment override for the user-level shore-home root, taken verbatim. This
/// is the hermetic-test seam: tests point it at a `tempfile` directory so neither
/// the keystore nor a family store touches the real user home.
pub(crate) const SHORE_HOME_ENV: &str = "SHORE_HOME";

/// Resolve the user-level shore-home root. The `keys/` and `stores/` subtrees
/// both hang off this one root, so they stay disjoint by construction. Env
/// wrapper over the pure [`resolve_shore_home_root`] seam.
pub(crate) fn shore_home_root() -> Result<PathBuf> {
    resolve_shore_home_root(
        std::env::var_os(SHORE_HOME_ENV).map(PathBuf::from),
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
        std::env::var_os("APPDATA").map(PathBuf::from),
    )
}

/// Pure resolution seam (kept env-free for testing). Precedence: explicit
/// override, then `$XDG_DATA_HOME/shore`, then the platform default
/// (`$HOME/.shore` on Unix, `%APPDATA%\shore` on Windows). A missing home with
/// no override is a typed error.
pub(crate) fn resolve_shore_home_root(
    shore_home: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
    app_data: Option<PathBuf>,
) -> Result<PathBuf> {
    if let Some(root) = shore_home {
        return Ok(root);
    }
    if let Some(xdg) = xdg_data_home {
        return Ok(xdg.join("shore"));
    }
    #[cfg(unix)]
    if let Some(home) = home {
        return Ok(home.join(".shore"));
    }
    #[cfg(windows)]
    if let Some(app_data) = app_data {
        return Ok(app_data.join("shore"));
    }
    // Keep both bindings live on every platform so neither triggers an
    // unused-variable warning on the leg that does not consume it.
    let _ = (home, app_data);
    Err(ShoreError::Message(
        "cannot resolve a user-level shore home: set SHORE_HOME or a platform home directory"
            .to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    // Pure seam: (shore_home, xdg_data_home, home, app_data) -> resolved root.
    // None models an unset variable; the seam never reads the process env.

    #[test]
    fn shore_home_override_is_used_verbatim() {
        let root = resolve_shore_home_root(
            Some(PathBuf::from("/tmp/hermetic-store")),
            Some(PathBuf::from("/xdg/data")),
            Some(PathBuf::from("/home/dev")),
            None,
        )
        .unwrap();
        assert_eq!(root, PathBuf::from("/tmp/hermetic-store"));
    }

    #[test]
    fn xdg_data_home_wins_over_home_when_no_override() {
        let root = resolve_shore_home_root(
            None,
            Some(PathBuf::from("/xdg/data")),
            Some(PathBuf::from("/home/dev")),
            None,
        )
        .unwrap();
        assert_eq!(root, PathBuf::from("/xdg/data").join("shore"));
    }

    #[cfg(unix)]
    #[test]
    fn home_dot_shore_is_the_unix_default() {
        let root =
            resolve_shore_home_root(None, None, Some(PathBuf::from("/home/dev")), None).unwrap();
        assert_eq!(root, PathBuf::from("/home/dev").join(".shore"));
    }

    #[cfg(windows)]
    #[test]
    fn app_data_shore_is_the_windows_default() {
        let root = resolve_shore_home_root(
            None,
            None,
            None,
            Some(PathBuf::from(r"C:\Users\dev\AppData\Roaming")),
        )
        .unwrap();
        assert_eq!(
            root,
            PathBuf::from(r"C:\Users\dev\AppData\Roaming").join("shore")
        );
    }

    #[test]
    fn missing_home_with_no_override_is_a_typed_error() {
        let result = resolve_shore_home_root(None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn shore_home_root_reads_the_env_override() {
        // The env wrapper reads SHORE_HOME and returns the root verbatim.
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: single-threaded test; nextest runs each test in its own process,
        // and SHORE_HOME is the documented hermetic seam (mirrors keys/home.rs).
        unsafe {
            std::env::set_var(SHORE_HOME_ENV, tmp.path());
        }
        let root = shore_home_root().unwrap();
        unsafe {
            std::env::remove_var(SHORE_HOME_ENV);
        }
        assert_eq!(root, tmp.path());
    }
}
