use sha2::{Digest, Sha512};
use std::{
    fs::{File, create_dir_all},
    io::{BufReader, copy},
    path::PathBuf,
    process::Command,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_os::platform;
use zip::ZipArchive;

#[cfg(target_os = "linux")]
use std::{fs, os::unix::fs::PermissionsExt};

async fn unzip_to_dir(zip_path: PathBuf, out_dir: PathBuf) -> String {
    let res = tauri::async_runtime::spawn_blocking(move || {
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(BufReader::new(file))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = out_dir.join(file.name());

            if file.is_dir() {
                create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    create_dir_all(parent)?;
                }
                let mut outfile = File::create(&outpath)?;
                copy(&mut file, &mut outfile)?;
            }
        }

        Ok::<(), zip::result::ZipError>(())
    })
    .await;

    match res {
        Ok(Ok(())) => "1".into(),
        _ => "-1".into(),
    }
}

fn get_sha512_hash(data: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(data);
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

#[tauri::command]
async fn check_latest_ver(app: AppHandle, version: String) -> String {
    let updates_path = app.path().app_local_data_dir().unwrap().join("updates");
    if updates_path.exists()
        && updates_path.is_dir()
        && updates_path.join(&version).exists()
        && updates_path.join(&version).is_dir()
    {
        return "1".to_string();
    }
    return "-1".to_string();
}

#[tauri::command]
async fn download(app: AppHandle, url: String, name: String, hash: String) -> String {
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

    let downloads_path = app.path().app_local_data_dir().unwrap().join("downloads");
    let updates_path = app.path().app_local_data_dir().unwrap().join("updates");

    let download_part_path = downloads_path.join(format!("{}.part", name));
    let download_zip_path = downloads_path.join(format!("{}.zip", name));

    let _ = tokio::fs::create_dir_all(&downloads_path).await;
    if let Ok(true) = tokio::fs::try_exists(&updates_path.join(name.clone())).await {
        let _ = tokio::fs::remove_dir_all(&updates_path.join(name.clone())).await;
    }
    if updates_path.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&updates_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let _ = tokio::fs::remove_dir_all(entry.path()).await;
            }
        }
        let _ = tokio::fs::create_dir_all(updates_path.join(&name)).await;
    }
    if download_part_path.exists() {
        let _ = tokio::fs::remove_file(&download_part_path).await;
    }

    if tokio::fs::write(&download_part_path, bytes).await.is_err() {
        return "-1".to_string();
    }

    if tokio::fs::rename(&download_part_path, &download_zip_path)
        .await
        .is_err()
    {
        return "-1".to_string();
    }

    let unzip_res = unzip_to_dir(download_zip_path.clone(), updates_path.join(&name)).await;
    tokio::fs::remove_file(download_zip_path.clone())
        .await
        .unwrap();
    if unzip_res == "-1" {
        return "-1".to_string();
    }

    #[cfg(target_os = "linux")]
    {
        let executable_path = updates_path.join(&name).join("lncvrt-games-launcher");
        let mut perms = fs::metadata(&executable_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(executable_path, perms).unwrap();
    }
    #[cfg(target_os = "macos")]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let macos_app_path = updates_path
            .join(&name)
            .join("Lncvrt Games Launcher.app")
            .join("Contents")
            .join("MacOS")
            .join("lncvrt-games-launcher");

        let mut perms = fs::metadata(&macos_app_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&macos_app_path, perms).unwrap();
    }
    return "1".to_string();
}

#[allow(unused_variables)]
#[tauri::command]
fn load(app: AppHandle, name: String) {
    let update_path = app
        .path()
        .app_local_data_dir()
        .unwrap()
        .join("updates")
        .join(&name);
    if !update_path.exists() {
        return;
    }
    if platform() == "macos" {
        Command::new("open")
            .arg("Lncvrt Games Launcher.app")
            .current_dir(&update_path)
            .spawn()
            .unwrap();
    } else if platform() == "linux" {
        Command::new("./lncvrt-games-launcher")
            .current_dir(&update_path)
            .spawn()
            .unwrap();
    } else if platform() == "windows" {
        Command::new(&update_path.join("lncvrt-games-launcher.exe"))
            .current_dir(&update_path)
            .spawn()
            .unwrap();
    }
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![check_latest_ver, download, load])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
