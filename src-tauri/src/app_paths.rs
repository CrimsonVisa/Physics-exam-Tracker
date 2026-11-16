use std::path::{Path, PathBuf};

use crate::error::AppError;

pub const V1_APP_IDENTIFIER: &str = "com.crimsonvisa.csirnetphysics";
pub const PRODUCTION_V2_APP_IDENTIFIER: &str = "com.crimsonvisa.physics-tracker";
pub const DEVELOPMENT_V2_APP_IDENTIFIER: &str = "com.crimsonvisa.physics-tracker.dev";

/// Only the variant for the active build channel is constructed by production
/// code; tests exercise both variants.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildChannel {
    Development,
    Production,
}

impl BuildChannel {
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Development => DEVELOPMENT_V2_APP_IDENTIFIER,
            Self::Production => PRODUCTION_V2_APP_IDENTIFIER,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Production => "release",
        }
    }

    pub const fn is_development(self) -> bool {
        matches!(self, Self::Development)
    }
}

#[cfg(debug_assertions)]
pub const BUILD_CHANNEL: BuildChannel = BuildChannel::Development;
#[cfg(not(debug_assertions))]
pub const BUILD_CHANNEL: BuildChannel = BuildChannel::Production;

pub const fn build_channel() -> BuildChannel {
    BUILD_CHANNEL
}

pub const fn active_v2_identifier() -> &'static str {
    BUILD_CHANNEL.identifier()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2StoragePaths {
    pub channel: BuildChannel,
    pub identifier: &'static str,
    pub data_dir: PathBuf,
    pub webview_data_dir: PathBuf,
    pub production_data_dir: PathBuf,
    pub production_webview_data_dir: PathBuf,
}

/// Resolve the V1 and active V2 storage directories from the platform data
/// directory.
///
/// V2 debug builds use a separate development identifier. Release builds keep
/// the historical production identifier and data location exactly as before.
#[cfg(test)]
pub fn resolve_data_dirs(base_data_dir: &Path) -> Result<(PathBuf, PathBuf), AppError> {
    resolve_data_dirs_for(build_channel(), base_data_dir)
}

#[cfg(test)]
pub fn resolve_data_dirs_for(
    channel: BuildChannel,
    base_data_dir: &Path,
) -> Result<(PathBuf, PathBuf), AppError> {
    ensure_absolute("platform data directory", base_data_dir)?;
    let v1_data_dir = base_data_dir.join(V1_APP_IDENTIFIER);
    let v2_data_dir = base_data_dir.join(channel.identifier());
    ensure_disjoint(
        &v1_data_dir,
        &v2_data_dir,
        "The V1 and V2 application-data directories overlap",
    )?;
    if channel.is_development() {
        ensure_disjoint(
            &v2_data_dir,
            &production_v2_data_dir(base_data_dir),
            "The development V2 data directory overlaps the production V2 data directory",
        )?;
    }
    Ok((v1_data_dir, v2_data_dir))
}

/// Resolve only the active V2 storage directory for the normal startup path.
/// This deliberately does not initialize, open, or create the V1 directory.
#[cfg(test)]
pub fn resolve_v2_data_dir(base_data_dir: &Path) -> Result<PathBuf, AppError> {
    resolve_v2_data_dir_for(build_channel(), base_data_dir)
}

#[cfg(test)]
pub fn resolve_v2_data_dir_for(
    channel: BuildChannel,
    base_data_dir: &Path,
) -> Result<PathBuf, AppError> {
    resolve_v2_data_dir_with_identifier(channel, channel.identifier(), base_data_dir)
}

pub fn production_v2_data_dir(base_data_dir: &Path) -> PathBuf {
    base_data_dir.join(PRODUCTION_V2_APP_IDENTIFIER)
}

#[cfg(test)]
pub fn development_v2_data_dir(base_data_dir: &Path) -> PathBuf {
    base_data_dir.join(DEVELOPMENT_V2_APP_IDENTIFIER)
}

pub fn production_v2_webview_dir(base_local_data_dir: &Path) -> PathBuf {
    base_local_data_dir.join(PRODUCTION_V2_APP_IDENTIFIER)
}

#[cfg(test)]
pub fn development_v2_webview_dir(base_local_data_dir: &Path) -> PathBuf {
    base_local_data_dir.join(DEVELOPMENT_V2_APP_IDENTIFIER)
}

/// Resolve every V2 filesystem location that must be isolated between a
/// development run and an installed production application.
pub fn resolve_v2_storage_paths(
    base_data_dir: &Path,
    base_local_data_dir: &Path,
) -> Result<V2StoragePaths, AppError> {
    resolve_v2_storage_paths_with_identifier(
        BUILD_CHANNEL,
        BUILD_CHANNEL.identifier(),
        base_data_dir,
        base_local_data_dir,
    )
}

#[cfg(test)]
pub fn resolve_v2_storage_paths_for(
    channel: BuildChannel,
    base_data_dir: &Path,
    base_local_data_dir: &Path,
) -> Result<V2StoragePaths, AppError> {
    resolve_v2_storage_paths_with_identifier(
        channel,
        channel.identifier(),
        base_data_dir,
        base_local_data_dir,
    )
}

/// Verify that the Tauri configuration identifier matches the compiled build
/// channel. This check is performed before the database is opened so a debug
/// build can never silently fall back to production storage.
pub fn ensure_tauri_identity(
    channel: BuildChannel,
    configured_identifier: &str,
) -> Result<(), AppError> {
    let expected = channel.identifier();
    if configured_identifier != expected {
        return Err(AppError::database_unavailable(format!(
            "The {} build is configured with application identifier '{}', but isolated storage \
             requires '{}'. Start it with the matching configuration; no database was opened.",
            channel.label(),
            configured_identifier,
            expected
        )));
    }
    Ok(())
}

fn resolve_v2_data_dir_with_identifier(
    channel: BuildChannel,
    identifier: &'static str,
    base_data_dir: &Path,
) -> Result<PathBuf, AppError> {
    ensure_absolute("platform data directory", base_data_dir)?;
    let v2_data_dir = base_data_dir.join(identifier);
    let production_data_dir = production_v2_data_dir(base_data_dir);
    ensure_disjoint(
        &v2_data_dir,
        &base_data_dir.join(V1_APP_IDENTIFIER),
        "The V1 and V2 application-data directories overlap",
    )?;
    if channel.is_development() {
        ensure_disjoint(
            &v2_data_dir,
            &production_data_dir,
            "The development V2 data directory overlaps the production V2 data directory",
        )?;
    }
    if !channel.is_development() && identifier != PRODUCTION_V2_APP_IDENTIFIER {
        return Err(AppError::database_unavailable(
            "The release build V2 data directory is not the production identifier. \
             Initialization was stopped.",
        ));
    }
    Ok(v2_data_dir)
}

fn resolve_v2_storage_paths_with_identifier(
    channel: BuildChannel,
    identifier: &'static str,
    base_data_dir: &Path,
    base_local_data_dir: &Path,
) -> Result<V2StoragePaths, AppError> {
    ensure_absolute("platform local-data directory", base_local_data_dir)?;
    let data_dir = resolve_v2_data_dir_with_identifier(channel, identifier, base_data_dir)?;
    let webview_data_dir = base_local_data_dir.join(identifier);
    let production_data_dir = production_v2_data_dir(base_data_dir);
    let production_webview_data_dir = production_v2_webview_dir(base_local_data_dir);
    if channel.is_development() {
        ensure_disjoint(
            &webview_data_dir,
            &production_webview_data_dir,
            "The development WebView data directory overlaps the production WebView data directory",
        )?;
    }
    Ok(V2StoragePaths {
        channel,
        identifier,
        data_dir,
        webview_data_dir,
        production_data_dir,
        production_webview_data_dir,
    })
}

fn ensure_absolute(label: &str, path: &Path) -> Result<(), AppError> {
    if !path.is_absolute() {
        return Err(AppError::database_unavailable(format!(
            "The {label} is not absolute, so storage cannot be isolated reliably: {}",
            path.display()
        )));
    }
    Ok(())
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = path_key(left);
    let right = path_key(right);
    left == right
        || left.starts_with(&format!("{right}\\"))
        || right.starts_with(&format!("{left}\\"))
}

fn ensure_disjoint(left: &Path, right: &Path, message: &str) -> Result<(), AppError> {
    if paths_overlap(left, right) {
        return Err(AppError::database_unavailable(format!(
            "{message}: {} and {}. Initialization was stopped.",
            left.display(),
            right.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn active_identifier_matches_the_compile_time_channel() {
        let expected = if cfg!(debug_assertions) {
            BuildChannel::Development
        } else {
            BuildChannel::Production
        };
        assert_eq!(build_channel(), expected);
        assert_eq!(active_v2_identifier(), expected.identifier());
        if cfg!(debug_assertions) {
            assert_eq!(active_v2_identifier(), DEVELOPMENT_V2_APP_IDENTIFIER);
        } else {
            assert_eq!(active_v2_identifier(), PRODUCTION_V2_APP_IDENTIFIER);
        }
    }

    #[test]
    fn explicit_paths_are_siblings_and_independent() {
        let root = tempfile::tempdir().expect("temporary directory");
        let (v1, v2) = resolve_data_dirs(root.path()).expect("paths");
        assert_eq!(v1, root.path().join(V1_APP_IDENTIFIER));
        assert_eq!(v2, root.path().join(active_v2_identifier()));
        assert_ne!(v1, v2);
        assert!(!v2.starts_with(&v1));
        assert!(!v1.starts_with(&v2));
    }

    #[test]
    fn current_and_future_bundle_identifiers_do_not_change_paths() {
        let root = tempfile::tempdir().expect("temporary directory");
        fs::create_dir_all(root.path().join(V1_APP_IDENTIFIER)).expect("v1 dir");
        let current_app_data = root.path().join(V1_APP_IDENTIFIER);
        let future_app_data = root.path().join(active_v2_identifier());
        let (v1, v2) = resolve_data_dirs(root.path()).expect("paths");

        assert_eq!(v1, current_app_data);
        assert_eq!(v2, future_app_data);
        assert!(!v1.starts_with(&v2));
        assert!(!v2.starts_with(&v1));
    }

    #[test]
    fn development_paths_are_isolated_from_production() {
        let root = tempfile::tempdir().expect("temporary directory");
        let local = tempfile::tempdir().expect("temporary local directory");
        let paths =
            resolve_v2_storage_paths_for(BuildChannel::Development, root.path(), local.path())
                .expect("development paths");

        assert_eq!(paths.data_dir, development_v2_data_dir(root.path()));
        assert_eq!(
            paths.production_data_dir,
            production_v2_data_dir(root.path())
        );
        assert_eq!(
            paths.webview_data_dir,
            development_v2_webview_dir(local.path())
        );
        assert_eq!(
            paths.production_webview_data_dir,
            production_v2_webview_dir(local.path())
        );
        assert!(!paths_overlap(&paths.data_dir, &paths.production_data_dir));
        assert!(!paths_overlap(
            &paths.webview_data_dir,
            &paths.production_webview_data_dir
        ));
    }

    #[test]
    fn production_paths_preserve_the_existing_layout() {
        let root = tempfile::tempdir().expect("temporary directory");
        let local = tempfile::tempdir().expect("temporary local directory");
        let paths =
            resolve_v2_storage_paths_for(BuildChannel::Production, root.path(), local.path())
                .expect("production paths");

        assert_eq!(paths.data_dir, production_v2_data_dir(root.path()));
        assert_eq!(
            paths.production_data_dir,
            production_v2_data_dir(root.path())
        );
        assert_eq!(
            paths.webview_data_dir,
            production_v2_webview_dir(local.path())
        );
        assert_eq!(paths.identifier, PRODUCTION_V2_APP_IDENTIFIER);
    }

    #[test]
    fn development_resolution_rejects_the_production_identifier() {
        let root = tempfile::tempdir().expect("temporary directory");
        let local = tempfile::tempdir().expect("temporary local directory");
        let error = resolve_v2_storage_paths_with_identifier(
            BuildChannel::Development,
            PRODUCTION_V2_APP_IDENTIFIER,
            root.path(),
            local.path(),
        )
        .expect_err("production identifier must not be used for development");
        assert_eq!(error.code, "database_unavailable");
    }

    #[test]
    fn tauri_identity_must_match_the_build_channel() {
        assert!(
            ensure_tauri_identity(BuildChannel::Development, DEVELOPMENT_V2_APP_IDENTIFIER).is_ok()
        );
        assert!(
            ensure_tauri_identity(BuildChannel::Production, PRODUCTION_V2_APP_IDENTIFIER).is_ok()
        );
        assert!(
            ensure_tauri_identity(BuildChannel::Development, PRODUCTION_V2_APP_IDENTIFIER).is_err()
        );
        assert!(
            ensure_tauri_identity(BuildChannel::Production, DEVELOPMENT_V2_APP_IDENTIFIER).is_err()
        );
    }

    #[test]
    fn overlapping_directories_are_rejected() {
        let root = tempfile::tempdir().expect("temporary directory");
        let base = root.path();
        assert!(ensure_disjoint(base, base, "same directory").is_err());
        assert!(ensure_disjoint(&base.join("child"), base, "nested directory").is_err());
        assert!(ensure_disjoint(base, &base.join("child"), "nested directory").is_err());
        assert!(ensure_disjoint(&base.join("one"), &base.join("two"), "siblings").is_ok());
    }

    #[test]
    fn main_window_starts_maximized_with_native_window_controls() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        let windows = config["app"]["windows"].as_array().expect("app windows");
        assert_eq!(windows.len(), 1, "the app must keep one main window");
        let window = &windows[0];

        assert_eq!(window["title"].as_str(), Some("Physics Tracker"));
        assert!(window["label"].is_null() || window["label"].as_str() == Some("main"));
        assert_eq!(window["maximized"].as_bool(), Some(true));
        assert_eq!(window["fullscreen"].as_bool(), Some(false));
        assert_eq!(window["decorations"].as_bool(), Some(true));
        assert_eq!(window["resizable"].as_bool(), Some(true));
        assert_eq!(window["maximizable"].as_bool(), Some(true));
        assert_eq!(window["minimizable"].as_bool(), Some(true));
        assert_eq!(window["closable"].as_bool(), Some(true));
        assert_eq!(window["create"].as_bool(), Some(false));
        assert_eq!(window["width"].as_u64(), Some(1100));
        assert_eq!(window["height"].as_u64(), Some(760));
        assert_eq!(window["minWidth"].as_u64(), Some(800));
        assert_eq!(window["minHeight"].as_u64(), Some(600));
    }

    #[test]
    fn content_security_policy_allows_embedded_fonts_without_weakening_other_restrictions() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        let csp = config["app"]["security"]["csp"]
            .as_str()
            .expect("production csp");
        let dev_csp = config["app"]["security"]["devCsp"]
            .as_str()
            .expect("development csp");

        for policy in [csp, dev_csp] {
            assert!(
                policy.contains("font-src 'self' data:"),
                "embedded fonts must be allowed"
            );
            assert!(policy.contains("default-src 'self'"));
            assert!(policy.contains("script-src 'self'"));
            assert!(policy.contains("style-src 'self' 'unsafe-inline'"));
            assert!(policy.contains("img-src 'self' asset: http://asset.localhost data:"));
            assert!(
                !policy
                    .split(';')
                    .any(|directive| directive.trim() == "default-src *")
            );
            assert!(
                !policy
                    .split(';')
                    .any(|directive| directive.trim() == "font-src *")
            );
        }
        assert!(csp.contains("connect-src ipc: http://ipc.localhost"));
        assert!(csp.contains("script-src 'self'"));
        assert!(!csp.contains("script-src 'unsafe-inline'"));
    }

    #[test]
    fn v2_identity_is_separate_and_matches_tauri_configurations() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        let dev_config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.dev.conf.json")).expect("dev config");
        assert_eq!(
            config["identifier"].as_str(),
            Some(PRODUCTION_V2_APP_IDENTIFIER)
        );
        assert_eq!(
            dev_config["identifier"].as_str(),
            Some(DEVELOPMENT_V2_APP_IDENTIFIER)
        );
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
        assert_ne!(V1_APP_IDENTIFIER, PRODUCTION_V2_APP_IDENTIFIER);
        assert_ne!(PRODUCTION_V2_APP_IDENTIFIER, DEVELOPMENT_V2_APP_IDENTIFIER);
        assert!(!V1_APP_IDENTIFIER.contains(PRODUCTION_V2_APP_IDENTIFIER));
        assert!(!PRODUCTION_V2_APP_IDENTIFIER.contains(V1_APP_IDENTIFIER));
        let product = config["productName"].as_str().expect("product name");
        let uninstall_key =
            format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{product}");
        assert!(!uninstall_key.contains("CSIR-NET"));
        assert_eq!(
            format!("%LOCALAPPDATA%\\{product}"),
            "%LOCALAPPDATA%\\Physics Tracker"
        );
        assert_ne!(
            format!("%LOCALAPPDATA%\\{PRODUCTION_V2_APP_IDENTIFIER}"),
            format!("%LOCALAPPDATA%\\{V1_APP_IDENTIFIER}")
        );
    }

    #[test]
    fn development_script_passes_the_dev_configuration_by_relative_path() {
        let package: serde_json::Value =
            serde_json::from_str(include_str!("../../package.json")).expect("package metadata");
        let script = package["scripts"]["tauri:dev"]
            .as_str()
            .expect("tauri:dev script");
        assert_eq!(script, "tauri dev --config src-tauri/tauri.dev.conf.json");
        let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.dev.conf.json");
        assert!(config.is_file(), "development configuration must exist");
    }

    #[test]
    fn v2_storage_path_remains_identity_based_and_separate_from_v1() {
        let root = tempfile::tempdir().expect("temporary directory");
        let v2_dir = resolve_v2_data_dir(root.path()).expect("v2 path");
        assert_eq!(v2_dir, root.path().join(active_v2_identifier()));
        assert_eq!(
            v2_dir.file_name().and_then(|name| name.to_str()),
            Some(active_v2_identifier())
        );
        assert_ne!(v2_dir, root.path().join(V1_APP_IDENTIFIER));
        assert!(!v2_dir.starts_with(root.path().join(V1_APP_IDENTIFIER)));
        assert!(!root.path().join(V1_APP_IDENTIFIER).starts_with(&v2_dir));
    }

    #[test]
    fn authoritative_release_metadata_uses_version_two() {
        let expected = "2.1.0";
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
        assert!(cargo_lock.contains("name = \"csir-net-tracker\"\nversion = \"2.1.0\""));
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
