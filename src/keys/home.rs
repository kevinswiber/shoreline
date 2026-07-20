use std::path::PathBuf;

use crate::error::{Result, ShoreError};

/// Resolve and create the user-level keystore's `keys/` directory, returning its
/// path. The directory tree is created if absent; on Unix it is created `0700`.
pub(crate) fn keys_dir() -> Result<PathBuf> {
    let keys = crate::paths::UserHomePaths::resolve()?.keys_dir();
    create_private_dir(&keys)?;
    Ok(keys)
}

/// Create `dir` (and parents) if absent. On Unix the leaf is set to mode `0700`
/// so private keys beneath it are not world-readable; on other platforms the
/// directory inherits the default ACL (documented caveat).
fn create_private_dir(dir: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|error| {
        ShoreError::Message(format!("create key home {}: {error}", dir.display()))
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|error| {
            ShoreError::Message(format!("set 0700 on {}: {error}", dir.display()))
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_dir_under_override_creates_keys_subtree_deterministically() {
        let tmp = tempfile::tempdir().unwrap();
        let (first, second) =
            crate::environment::test_support::with_home_override(tmp.path(), || {
                (keys_dir().unwrap(), keys_dir().unwrap())
            });

        assert_eq!(first, second, "resolution is deterministic under override");
        assert_eq!(first, tmp.path().join("keys"));
        assert!(first.is_dir(), "the keys/ subtree is created");
    }

    #[cfg(unix)]
    #[test]
    fn created_keys_directory_is_0700_on_unix() {
        use std::os::unix::fs::PermissionsExt as _;

        let tmp = tempfile::tempdir().unwrap();
        let dir = crate::environment::test_support::with_home_override(tmp.path(), || {
            keys_dir().unwrap()
        });

        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "keystore dir must be private");
    }
}
