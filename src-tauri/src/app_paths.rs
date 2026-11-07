use std::path::{Path, PathBuf};

use crate::error::AppError;

pub const V1_APP_IDENTIFIER: &str = "com.crimsonvisa.csirnetphysics";
pub const V2_APP_IDENTIFIER: &str = "com.crimsonvisa.physics-tracker";

/// Resolve the V1 and V2 storage directories from the platform data directory.
///
/// The paths are explicit and independent of Tauri's bundle identifier. This
/// prevents a future bundle-identifier cutover from redirecting V1 data into
/// the V2 directory or vice versa.
pub fn resolve_data_dirs(base_data_dir: &Path) -> Result<(PathBuf, PathBuf), AppError> {
    if !base_data_dir.is_absolute() {
        return Err(AppError::database_unavailable(
            "The platform data directory is not absolute, so V1 and V2 storage cannot be isolated reliably.",
        ));
    }
    let v1_data_dir = base_data_dir.join(V1_APP_IDENTIFIER);
    let v2_data_dir = base_data_dir.join(V2_APP_IDENTIFIER);
    ensure_isolated(&v1_data_dir, &v2_data_dir)?;
    Ok((v1_data_dir, v2_data_dir))
}

/// Resolve only the V2 storage directory for the normal V2 startup path.
/// This deliberately does not initialize, open, or create the V1 directory.
pub fn resolve_v2_data_dir(base_data_dir: &Path) -> Result<PathBuf, AppError> {
    resolve_data_dirs(base_data_dir).map(|(_, v2_data_dir)| v2_data_dir)
}

fn ensure_isolated(v1_data_dir: &Path, v2_data_dir: &Path) -> Result<(), AppError> {
    if v1_data_dir == v2_data_dir
        || v2_data_dir.starts_with(v1_data_dir)
        || v1_data_dir.starts_with(v2_data_dir)
    {
        return Err(AppError::database_unavailable(
            "The V1 and V2 application-data directories overlap. Initialization was stopped.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn explicit_paths_are_siblings_and_independent() {
        let root = tempfile::tempdir().expect("temporary directory");
        let (v1, v2) = resolve_data_dirs(root.path()).expect("paths");
        assert_eq!(v1, root.path().join(V1_APP_IDENTIFIER));
        assert_eq!(v2, root.path().join(V2_APP_IDENTIFIER));
        assert_ne!(v1, v2);
        assert!(!v2.starts_with(&v1));
        assert!(!v1.starts_with(&v2));
    }

    #[test]
    fn current_and_future_bundle_identifiers_do_not_change_paths() {
        let root = tempfile::tempdir().expect("temporary directory");
        fs::create_dir_all(root.path().join(V1_APP_IDENTIFIER)).expect("v1 dir");
        let current_app_data = root.path().join(V1_APP_IDENTIFIER);
        let future_app_data = root.path().join(V2_APP_IDENTIFIER);
        let (v1, v2) = resolve_data_dirs(root.path()).expect("paths");

        assert_eq!(v1, current_app_data);
        assert_eq!(v2, future_app_data);
        assert!(!v1.starts_with(&v2));
        assert!(!v2.starts_with(&v1));
    }

    #[test]
    fn v2_identity_is_separate_and_matches_tauri_configuration() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        assert_eq!(config["identifier"].as_str(), Some(V2_APP_IDENTIFIER));
        assert_eq!(config["productName"].as_str(), Some("Physics Tracker"));
        assert_eq!(
            config["bundle"]["windows"]["nsis"]["startMenuFolder"].as_str(),
            Some("Physics Tracker")
        );
        assert_eq!(
            config["bundle"]["windows"]["nsis"]["template"].as_str(),
            Some("nsis/installer.nsi")
        );
        assert_eq!(
            config["bundle"]["windows"]["allowDowngrades"].as_bool(),
            Some(false)
        );
        assert_ne!(V1_APP_IDENTIFIER, V2_APP_IDENTIFIER);
        assert!(!V1_APP_IDENTIFIER.contains(V2_APP_IDENTIFIER));
        assert!(!V2_APP_IDENTIFIER.contains(V1_APP_IDENTIFIER));
        let product = config["productName"].as_str().expect("product name");
        let uninstall_key =
            format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{product}");
        assert!(!uninstall_key.contains("CSIR-NET"));
        assert_eq!(
            format!("%LOCALAPPDATA%\\{product}"),
            "%LOCALAPPDATA%\\Physics Tracker"
        );
        assert_ne!(
            format!("%LOCALAPPDATA%\\{V2_APP_IDENTIFIER}"),
            format!("%LOCALAPPDATA%\\{V1_APP_IDENTIFIER}")
        );
    }

    #[test]
    fn v2_storage_path_remains_identity_based_and_separate_from_v1() {
        let root = tempfile::tempdir().expect("temporary directory");
        let v2_dir = resolve_v2_data_dir(root.path()).expect("v2 path");
        assert_eq!(v2_dir, root.path().join(V2_APP_IDENTIFIER));
        assert_eq!(
            v2_dir.file_name().and_then(|name| name.to_str()),
            Some(V2_APP_IDENTIFIER)
        );
        assert_ne!(v2_dir, root.path().join(V1_APP_IDENTIFIER));
        assert!(!v2_dir.starts_with(root.path().join(V1_APP_IDENTIFIER)));
        assert!(!root.path().join(V1_APP_IDENTIFIER).starts_with(&v2_dir));
    }

    #[test]
    fn authoritative_release_metadata_uses_version_two() {
        let expected = "2.0.0";
        assert_eq!(env!("CARGO_PKG_VERSION"), expected);
        let package: serde_json::Value =
            serde_json::from_str(include_str!("../../package.json")).expect("package metadata");
        let package_lock: serde_json::Value =
            serde_json::from_str(include_str!("../../package-lock.json")).expect("package lock");
        let tauri: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        assert_eq!(package["version"].as_str(), Some(expected));
        assert_eq!(package_lock["version"].as_str(), Some(expected));
        assert_eq!(
            package_lock["packages"][""]["version"].as_str(),
            Some(expected)
        );
        assert_eq!(tauri["version"].as_str(), Some(expected));
        let cargo_lock = include_str!("../Cargo.lock").replace("\r\n", "\n");
        assert!(cargo_lock.contains("name = \"csir-net-tracker\"\nversion = \"2.0.0\""));
    }

    #[test]
    fn v2_uninstaller_template_has_no_recursive_data_deletion() {
        let template = include_str!("../nsis/installer.nsi");
        assert!(!template.contains("DeleteAppDataCheckbox"));
        assert!(!template.contains("RmDir /r"));
        assert!(!template.contains("$APPDATA\\${BUNDLEID}"));
        assert!(!template.contains("$LOCALAPPDATA\\${BUNDLEID}"));
        assert!(!template.contains(V1_APP_IDENTIFIER));
        assert!(template.contains("Preserve application data"));
    }
}
