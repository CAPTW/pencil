//! An explicit separate profile never falls back to the user's existing data.
use std::path::PathBuf;

pub(crate) fn directory() -> Result<Option<PathBuf>, String> {
    std::env::var_os("GRAMMAR_PROFILE_DIRECTORY")
        .map(|value| validate(PathBuf::from(value)))
        .transpose()
}

fn validate(path: PathBuf) -> Result<PathBuf, String> {
    if !path.is_absolute() || !path.is_dir() {
        return Err("Explicit Grammar profile must be an existing absolute directory".into());
    }
    // Reject links at every ancestor, including Windows junctions.
    for ancestor in path.ancestors() {
        let meta = std::fs::symlink_metadata(ancestor).map_err(|_| "Profile metadata unavailable")?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Profile cannot use reparse points".into());
            }
        }
        if meta.file_type().is_symlink() { return Err("Profile cannot use links".into()); }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_profile_rejects_missing_or_relative_paths() {
        assert!(validate(PathBuf::from("relative")).is_err());
        assert!(validate(std::env::temp_dir().join("grammar-nonexistent-profile-0c608ce3")).is_err());
    }
    #[test]
    fn explicit_profile_preserves_existing_absolute_directory() {
        let path = std::env::temp_dir().canonicalize().unwrap();
        assert_eq!(validate(path.clone()).unwrap(), path);
    }
}
