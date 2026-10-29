use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

fn app_info() -> AppInfo {
    AppInfo {
        name: "CSIR-NET Physics Tracker".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    app_info()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running the CSIR-NET Physics Tracker application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_exposes_the_product_identity() {
        let info = app_info();

        assert_eq!(info.name, "CSIR-NET Physics Tracker");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }
}
