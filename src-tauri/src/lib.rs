use sha2::{Digest, Sha512};
use std::{
    fs::{self, remove_dir_all},
    io::Cursor,
    path::PathBuf,
    process::Command,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_os::platform;
use zip::ZipArchive;

fn unzip_to_dir(bytes: &[u8], target: &PathBuf) -> std::io::Result<()> {
    let reader = Cursor::new(bytes);
    let mut zip = ZipArchive::new(reader).unwrap();

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let outpath = target.join(file.mangled_name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

fn get_sha512_hash(data: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(data);
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

#[tauri::command]
async fn download(app: AppHandle, url: String, hash: String) -> String {
    let client = reqwest::Client::new();
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return "-1".to_string(),
    };
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(_) => return "-1".to_string(),
    };

    let download_hash = get_sha512_hash(&bytes);
    if hash != download_hash {
        return "-2".to_string();
    }

    let bin_path = app.path().app_local_data_dir().unwrap().join("bin");
    let _ = tokio::fs::create_dir_all(&bin_path).await;
    if let Err(_) = unzip_to_dir(&bytes, &bin_path) {
        return "-3".to_string();
    }

    drop(bytes);

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::{fs, os::unix::fs::PermissionsExt};

        let executable_path = if cfg!(target_os = "linux") {
            bin_path.join("lncvrt-games-launcher")
        } else {
            bin_path
                .join("Lncvrt Games Launcher.app")
                .join("Contents")
                .join("MacOS")
                .join("lncvrt-games-launcher")
        };

        let mut perms = fs::metadata(&executable_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&executable_path, perms).unwrap();
    }

    return "1".to_string();
}

#[allow(unused_variables)]
#[tauri::command]
fn load(app: AppHandle) {
    let bin_path = app.path().app_local_data_dir().unwrap().join("bin");
    if !bin_path.exists() {
        return;
    }

    if platform() == "macos" {
        Command::new("open")
            .arg("Lncvrt Games Launcher.app")
            .current_dir(&bin_path)
            .spawn()
            .unwrap();
    } else if platform() == "linux" {
        Command::new("./lncvrt-games-launcher")
            .current_dir(&bin_path)
            .spawn()
            .unwrap();
    } else if platform() == "windows" {
        Command::new(&bin_path.join("lncvrt-games-launcher.exe"))
            .current_dir(&bin_path)
            .spawn()
            .unwrap();
    }

    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_local_data_dir = app.path().app_local_data_dir().unwrap();
            let downloads_dir = app_local_data_dir.join("downloads");
            let updates_dir = app_local_data_dir.join("updates");
            let bin_dir = app_local_data_dir.join("bin");
            let version_file = app_local_data_dir.join(".version");

            if downloads_dir.exists() {
                let _ = remove_dir_all(downloads_dir);
            }
            if updates_dir.exists() {
                let _ = remove_dir_all(&updates_dir);
            }
            if bin_dir.exists() && bin_dir.is_file() {
                let _ = remove_dir_all(&bin_dir);
            }
            if version_file.exists() && !bin_dir.is_file() {
                let _ = remove_dir_all(&version_file);
            }

            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![download, load])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
