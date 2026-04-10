use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufReader;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::ShellExt;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvInfo {
    pub path: String,
    pub exists: bool,
    pub packages: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DbConnection {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DbConnectionList {
    pub connections: Vec<DbConnection>,
}

impl Default for DbConnectionList {
    fn default() -> Self {
        Self { connections: Vec::new() }
    }
}

fn get_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("config.json"))
        .map_err(|e| format!("Failed to get config path: {}", e))
}

#[tauri::command]
fn load_connections(app: tauri::AppHandle) -> Result<Vec<DbConnection>, String> {
    let config_path = get_config_path(&app)?;
    if config_path.exists() {
        let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        let list: DbConnectionList = serde_json::from_str(&content).unwrap_or_default();
        Ok(list.connections)
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn save_connection(app: tauri::AppHandle, connection: DbConnection) -> Result<(), String> {
    let config_path = get_config_path(&app)?;
    let mut list = if config_path.exists() {
        let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        serde_json::from_str::<DbConnectionList>(&content).unwrap_or_default()
    } else {
        DbConnectionList::default()
    };
    
    if let Some(existing) = list.connections.iter_mut().find(|c| c.id == connection.id) {
        *existing = connection;
    } else {
        list.connections.push(connection);
    }
    
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    let content = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
fn test_connection(connection: DbConnection) -> Result<String, String> {
    let port: u16 = connection.port.parse().unwrap_or(5432);
    
    let addr = (connection.host.as_str(), port);
    
    match addr.to_socket_addrs() {
        Ok(addrs) => {
            let mut success = false;
            let mut last_err = String::new();
            for addr in addrs {
                match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5)) {
                    Ok(_) => {
                        success = true;
                        break;
                    }
                    Err(e) => {
                        last_err = e.to_string();
                    }
                }
            }
            if success {
                Ok(format!("已连接到 {}:{}", connection.host, port))
            } else {
                Err(format!("无法连接到 {}:{} - {}", connection.host, port, last_err))
            }
        }
        Err(e) => Err(format!("地址解析失败: {}", e)),
    }
}

#[tauri::command]
fn delete_connection(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let config_path = get_config_path(&app)?;
    if config_path.exists() {
        let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        let mut list: DbConnectionList = serde_json::from_str(&content).unwrap_or_default();
        list.connections.retain(|c| c.id != id);
        let new_content = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
        fs::write(&config_path, new_content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn get_micromamba_binary_names() -> Vec<&'static str> {
    let target_os = std::env::consts::OS;
    let target_arch = std::env::consts::ARCH;
    
    let mut names = Vec::new();
    
    match (target_os, target_arch) {
        ("macos", "aarch64") => {
            names.push("micromamba");
            names.push("micromamba_osx_arm64");
        }
        ("macos", "x86_64") => {
            names.push("micromamba");
            names.push("micromamba_osx_64");
        }
        ("linux", "x86_64") => {
            names.push("micromamba");
            names.push("micromamba_linux_64");
        }
        ("linux", "aarch64") => {
            names.push("micromamba");
            names.push("micromamba_linux_64");
        }
        ("windows", _) => {
            names.push("micromamba.exe");
            names.push("micromamba_win_64.exe");
        }
        _ => {
            names.push("micromamba");
        }
    }
    
    names
}

fn find_micromamba_in_dir(dir: &PathBuf) -> Option<PathBuf> {
    let names = get_micromamba_binary_names();
    for name in &names {
        let path = dir.join("binaries").join(*name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn get_micromamba_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;
    
    if let Some(path) = find_micromamba_in_dir(&resource_dir) {
        return Ok(path);
    }
    
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    if let Some(path) = find_micromamba_in_dir(&app_data.parent().unwrap_or(&app_data).to_path_buf()) {
        return Ok(path);
    }
    
    let names = get_micromamba_binary_names();
    Err(format!("micromamba binary not found. Tried: {:?}", names))
}

fn get_env_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("gis_env"))
        .map_err(|e| format!("Failed to get env path: {}", e))
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn test_spawn(app: tauri::AppHandle) -> Result<String, String> {
    let shell = app.shell();
    
    let output = shell
        .command("echo")
        .args(["hello from shell plugin"])
        .output()
        .await
        .map_err(|e: tauri_plugin_shell::Error| e.to_string())?;
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
async fn check_env_status(app: tauri::AppHandle) -> Result<EnvInfo, String> {
    let env_path = get_env_path(&app)?;
    let _micromamba = get_micromamba_path(&app)?;
    
    let exists = env_path.exists();
    let mut packages = Vec::new();
    
    if exists {
        let pkg_dir = env_path.join("conda-meta");
        if pkg_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&pkg_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".json") {
                        packages.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }
    }
    
    Ok(EnvInfo {
        path: env_path.to_string_lossy().to_string(),
        exists,
        packages,
    })
}

#[tauri::command]
async fn create_env(app: tauri::AppHandle, packages: Vec<String>) -> Result<String, String> {
    let env_path = get_env_path(&app)?;
    let micromamba = get_micromamba_path(&app)?;
    
    if env_path.exists() {
        std::fs::remove_dir_all(&env_path).map_err(|e| e.to_string())?;
    }
    
    let shell = app.shell();
    
    let env_path_str = env_path.to_string_lossy().to_string();
    let mut all_args: Vec<&str> = vec![
        "create",
        "-p",
        &env_path_str,
        "-c", "conda-forge",
    ];
    for pkg in &packages {
        all_args.push(pkg);
    }
    all_args.push("-y");
    
    let output = shell
        .command(micromamba.to_string_lossy().as_ref())
        .args(&all_args)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    if !output.status.success() {
        return Err(format!(
            "Failed to create environment:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        ));
    }
    
    Ok(format!(
        "Environment created at {} with packages: {:?}\n{}",
        env_path.display(),
        packages,
        stdout
    ))
}

#[tauri::command]
async fn run_in_env(app: tauri::AppHandle, command: Vec<String>) -> Result<String, String> {
    let env_path = get_env_path(&app)?;
    let micromamba = get_micromamba_path(&app)?;
    
    if !env_path.exists() {
        return Err("Environment does not exist. Please create it first.".to_string());
    }
    
    let shell = app.shell();
    let mut args = vec!["run".to_string(), "-p".to_string(), env_path.to_string_lossy().to_string()];
    args.extend(command);
    
    let output = shell
        .command(micromamba.to_string_lossy().as_ref())
        .args(args.iter().map(|s: &String| s.as_str()).collect::<Vec<_>>())
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
async fn get_micromamba_version(app: tauri::AppHandle) -> Result<String, String> {
    let micromamba = get_micromamba_path(&app)?;
    let shell = app.shell();
    
    let output = shell
        .command(micromamba.to_string_lossy().as_ref())
        .args(["--version"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
async fn export_env(app: tauri::AppHandle, output_path: String) -> Result<String, String> {
    let env_path = get_env_path(&app)?;
    
    if !env_path.exists() {
        return Err("Environment does not exist. Nothing to export.".to_string());
    }
    
    let output_file = File::create(&output_path).map_err(|e| e.to_string())?;
    let encoder = GzEncoder::new(output_file, Compression::default());
    let mut tar = tar::Builder::new(encoder);
    
    for entry in WalkDir::new(&env_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let relative_path = path.strip_prefix(&env_path)
            .map_err(|e| e.to_string())?;
        
        if path.is_file() {
            tar.append_path_with_name(path, relative_path)
                .map_err(|e| format!("Failed to add file {:?}: {}", relative_path, e))?;
        } else if path.is_dir() && relative_path.as_os_str().len() > 0 {
            tar.append_dir(relative_path, path)
                .map_err(|e| format!("Failed to add dir {:?}: {}", relative_path, e))?;
        }
    }
    
    tar.finish().map_err(|e| e.to_string())?;
    
    Ok(format!("Environment exported to {}", output_path))
}

#[tauri::command]
async fn import_env(app: tauri::AppHandle, archive_path: String) -> Result<String, String> {
    let env_path = get_env_path(&app)?;
    
    if env_path.exists() {
        return Err("Environment already exists. Please remove it first.".to_string());
    }
    
    std::fs::create_dir_all(&env_path).map_err(|e| e.to_string())?;
    
    let file = File::open(&archive_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let decoder = GzDecoder::new(reader);
    let mut tar = tar::Archive::new(decoder);
    
    tar.unpack(&env_path).map_err(|e| e.to_string())?;
    
    Ok(format!("Environment imported from {} to {}", archive_path, env_path.display()))
}

#[tauri::command]
async fn extract_offline_package(app: tauri::AppHandle) -> Result<String, String> {
    let env_path = get_env_path(&app)?;
    
    if env_path.exists() {
        return Ok(format!("Environment already exists at {}", env_path.display()));
    }
    
    let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    let package_path = resource_dir.join("gis_env.tar.gz");
    
    if !package_path.exists() {
        return Err(format!("Offline package not found at {:?}", package_path));
    }
    
    std::fs::create_dir_all(&env_path).map_err(|e| e.to_string())?;
    
    let file = File::open(&package_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let decoder = GzDecoder::new(reader);
    let mut tar = tar::Archive::new(decoder);
    
    tar.unpack(&env_path).map_err(|e| e.to_string())?;
    
    Ok(format!("Offline environment extracted to {}", env_path.display()))
}

#[derive(Debug, Clone, Serialize)]
pub struct GdalProgress {
    pub message: String,
    pub percent: Option<f32>,
}

#[tauri::command]
async fn check_gdal(app: tauri::AppHandle) -> Result<bool, String> {
    let env_path = get_env_path(&app)?;
    let micromamba = match get_micromamba_path(&app) {
        Ok(p) => p,
        Err(e) => return Err(format!("micromamba path error: {}", e)),
    };
    
    if !env_path.exists() {
        return Ok(false);
    }
    
    let gdalinfo_path = env_path.join("bin").join("gdalinfo");
    if gdalinfo_path.exists() {
        return Ok(true);
    }
    
    let shell = app.shell();
    
    if let Ok(output) = shell
        .command(gdalinfo_path.to_string_lossy().as_ref())
        .args(["--version"])
        .output()
        .await
    {
        if output.status.success() {
            return Ok(true);
        }
    }
    
    let output = shell
        .command(micromamba.to_string_lossy().as_ref())
        .args(["run", "-p", &env_path.to_string_lossy(), "gdalinfo", "--version"])
        .output()
        .await;
    
    match output {
        Ok(out) => Ok(out.status.success()),
        Err(e) => Err(format!("Failed to run gdalinfo: {}", e)),
    }
}

#[tauri::command]
async fn list_env_bins(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let env_path = get_env_path(&app)?;
    
    if !env_path.exists() {
        return Err("Environment does not exist".to_string());
    }
    
    let bins_path = env_path.join("bin");
    
    if !bins_path.exists() {
        return Err("No bin directory found".to_string());
    }
    
    let mut bins = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&bins_path) {
        for entry in entries.flatten() {
            bins.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    
    Ok(bins)
}

#[derive(Debug, Deserialize)]
pub struct OgrConvertOptions {
    pub input_path: String,
    pub output_connection: String,
    pub layer_name: Option<String>,
    pub srs: Option<String>,
    pub target_srs: Option<String>,
    pub schema: Option<String>,
    pub geometry_name: Option<String>,
    pub fid_column: Option<String>,
    pub overwrite: bool,
    pub promote_to_multi: bool,
    pub select_fields: Option<String>,
    pub skip_failures: bool,
    pub encoding: Option<String>,
    pub use_copy: bool,
}

#[tauri::command]
async fn ogr_convert(app: tauri::AppHandle, options: OgrConvertOptions) -> Result<String, String> {
    let env_path = get_env_path(&app)?;
    let micromamba = get_micromamba_path(&app)?;
    
    if !env_path.exists() {
        return Err("Environment does not exist. Please create it first.".to_string());
    }
    
    let input_path = std::path::Path::new(&options.input_path);
    let is_directory = input_path.is_dir();
    
    let files_to_import: Vec<std::path::PathBuf> = if is_directory {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(input_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext.to_string_lossy().to_lowercase() == "shp" {
                            files.push(path);
                        }
                    }
                }
            }
        }
        if files.is_empty() {
            return Err("No .shp files found in the selected directory".to_string());
        }
        files.sort();
        files
    } else {
        vec![input_path.to_path_buf()]
    };
    
    let total_files = files_to_import.len();
    let mut results = Vec::new();
    let mut success_count = 0;
    
    let mut pg_host = "localhost".to_string();
    let mut pg_port = "5432".to_string();
    let mut pg_db = String::new();
    let mut pg_user = "postgres".to_string();
    let mut pg_pass = String::new();
    
    let conn_str = if options.output_connection.starts_with("PG:") {
        options.output_connection[3..].to_string()
    } else {
        options.output_connection.clone()
    };
    
    for part in conn_str.split_whitespace() {
        if let Some((k, v)) = part.split_once('=') {
            match k {
                "host" => pg_host = v.to_string(),
                "port" => pg_port = v.to_string(),
                "dbname" => pg_db = v.to_string(),
                "user" => pg_user = v.to_string(),
                "password" => pg_pass = v.to_string(),
                _ => {}
            }
        }
    }
    
    if pg_db.is_empty() {
        return Err("Database name not found".to_string());
    }
    
    for (index, file_path) in files_to_import.iter().enumerate() {
        let file_name = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported");
        
        let _ = app.emit("gdal-progress", GdalProgress {
            message: format!("Importing {}/{}: {}...", index + 1, total_files, file_name),
            percent: Some(((index as f64 / total_files as f64) * 100.0) as f32),
        });
        
        match import_single_file(&app, &env_path, &micromamba, &options, file_path, &pg_host, &pg_port, &pg_db, &pg_user, &pg_pass).await {
            Ok(msg) => {
                results.push(format!("✓ {}: {}", file_name, msg));
                success_count += 1;
            }
            Err(e) => {
                let error_msg = format!("✗ {}: {}", file_name, e);
                results.push(error_msg);
                if !options.skip_failures {
                    return Err(format!("Import failed for {}: {}", file_name, e));
                }
            }
        }
    }
    
    Ok(format!(
        "Import completed: {}/{} files successful\n{}",
        success_count,
        total_files,
        results.join("\n")
    ))
}

async fn import_single_file(
    app: &AppHandle,
    env_path: &PathBuf,
    micromamba: &PathBuf,
    options: &OgrConvertOptions,
    input_file: &PathBuf,
    pg_host: &str,
    pg_port: &str,
    pg_db: &str,
    pg_user: &str,
    pg_pass: &str,
) -> Result<String, String> {
    let mut final_conn = format!("host={} port={} dbname={} user={}", pg_host, pg_port, pg_db, pg_user);
    if !pg_pass.is_empty() {
        final_conn.push_str(&format!(" password={}", pg_pass));
    }
    
    let schema = options.schema.as_ref();
    if let Some(ref schema_name) = schema {
        let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", schema_name);
        
        let shell = app.shell();
        let mut create_args = vec![
            "run".to_string(),
            "-p".to_string(),
            env_path.to_string_lossy().to_string(),
        ];
        
        if !pg_pass.is_empty() {
            create_args.push("env".to_string());
            create_args.push(format!("PGPASSWORD={}", pg_pass));
        }
        
        create_args.extend(["psql".to_string(), "-h".to_string(), pg_host.to_string()]);
        create_args.extend(["-p".to_string(), pg_port.to_string()]);
        create_args.extend(["-U".to_string(), pg_user.to_string()]);
        create_args.extend(["-d".to_string(), pg_db.to_string()]);
        create_args.extend(["-c".to_string(), create_schema_sql]);
        
        let create_output = shell
            .command(micromamba.to_string_lossy().as_ref())
            .args(create_args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        
        if !create_output.status.success() {
            let stderr = String::from_utf8_lossy(&create_output.stderr);
            if !stderr.contains("already exists") && !stderr.contains("duplicate schema") {
                return Err(format!("Failed to create schema: {}", stderr));
            }
        }
    }
    
    let target_table = if let (Some(ref schema_name), Some(ref layer)) = (&options.schema, &options.layer_name) {
        Some(format!("{}.{}", schema_name, layer))
    } else if let Some(ref schema_name) = options.schema {
        let base_name = input_file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported_data")
            .to_string();
        Some(format!("{}.{}", schema_name, base_name))
    } else {
        options.layer_name.clone()
    };
    
    let mut args = vec![
        "run".to_string(),
        "-p".to_string(),
        env_path.to_string_lossy().to_string(),
        "ogr2ogr".to_string(),
        "-f".to_string(),
        "PostgreSQL".to_string(),
        format!("PG:{}", final_conn),
    ];
    
    if let Some(ref table) = target_table {
        args.push("-nln".to_string());
        args.push(table.clone());
    }
    
    if options.promote_to_multi {
        args.push("-nlt".to_string());
        args.push("PROMOTE_TO_MULTI".to_string());
    }
    
    if let Some(ref geom_name) = options.geometry_name {
        args.push("-lco".to_string());
        args.push(format!("GEOMETRY_NAME={}", geom_name));
    }
    
    if let Some(ref fid) = options.fid_column {
        args.push("-lco".to_string());
        args.push(format!("FID={}", fid));
    }
    
    if options.overwrite {
        args.push("-overwrite".to_string());
    }
    
    if options.skip_failures {
        args.push("-skipfailures".to_string());
    }
    
    if let Some(ref srs) = options.srs {
        args.push("-s_srs".to_string());
        args.push(srs.clone());
    }
    
    if let Some(ref srs) = options.target_srs {
        args.push("-t_srs".to_string());
        args.push(srs.clone());
    }
    
    if let Some(ref fields) = options.select_fields {
        if !fields.is_empty() {
            args.push("-select".to_string());
            args.push(fields.clone());
        }
    }
    
    if options.use_copy {
        args.push("--config".to_string());
        args.push("PG_USE_COPY".to_string());
        args.push("YES".to_string());
    }
    
    if let Some(ref enc) = options.encoding {
        args.push("--config".to_string());
        args.push("SHAPE_ENCODING".to_string());
        args.push(enc.clone());
    }
    
    args.push(input_file.to_string_lossy().to_string());
    
    let shell = app.shell();
    let output = shell
        .command(micromamba.to_string_lossy().as_ref())
        .args(args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let file_name = input_file.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("data");
    
    Ok(format!("{} imported ({})", file_name, stdout.trim()))
}

#[tauri::command]
async fn check_gdal_drivers(app: tauri::AppHandle) -> Result<String, String> {
    let env_path = get_env_path(&app)?;
    let micromamba = get_micromamba_path(&app)?;
    
    if !env_path.exists() {
        return Err("Environment does not exist".to_string());
    }
    
    let shell = app.shell();
    let output = shell
        .command(micromamba.to_string_lossy().as_ref())
        .args(["run", "-p", &env_path.to_string_lossy(), "ogrinfo", "--formats"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[derive(Debug, Deserialize)]
pub struct OptimizeOptions {
    pub connection: String,
    pub schema: Option<String>,
    pub table: Option<String>,
    pub create_geometry_index: bool,
}

fn parse_pg_connection(conn: &str) -> Result<(String, Option<String>, Vec<String>), String> {
    let conn = conn.trim();
    let mut db_name = String::new();
    let mut password: Option<String> = None;
    let mut psql_args = Vec::new();
    
    let parts: Vec<&str> = if conn.starts_with("PG:") {
        conn[3..].split_whitespace().collect()
    } else {
        conn.split_whitespace().collect()
    };
    
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if let Some((key, value)) = part.split_once('=') {
            match key.to_lowercase().as_str() {
                "host" => {
                    psql_args.push("-h".to_string());
                    psql_args.push(value.to_string());
                }
                "port" => {
                    psql_args.push("-p".to_string());
                    psql_args.push(value.to_string());
                }
                "dbname" | "database" => db_name = value.to_string(),
                "user" | "username" => {
                    psql_args.push("-U".to_string());
                    psql_args.push(value.to_string());
                }
                "password" => {
                    password = Some(value.to_string());
                }
                _ => {}
            }
        } else if !part.starts_with('-') && part != "PG:" {
            if db_name.is_empty() {
                db_name = part.to_string();
            }
        }
        i += 1;
    }
    
    if db_name.is_empty() {
        return Err("Database name not found in connection string".to_string());
    }
    
    Ok((db_name, password, psql_args))
}

#[tauri::command]
async fn optimize_postgres(app: tauri::AppHandle, options: OptimizeOptions) -> Result<String, String> {
    let env_path = get_env_path(&app)?;
    
    if !env_path.exists() {
        return Err("Environment does not exist".to_string());
    }
    
    let micromamba = get_micromamba_path(&app)?;
    let shell = app.shell();
    
    let (db_name, password, psql_args) = parse_pg_connection(&options.connection)?;
    
    let schema = options.schema.unwrap_or_else(|| "public".to_string());
    let table = options.table.as_deref();
    let mut results = Vec::new();
    
    let _ = app.emit("gdal-progress", GdalProgress {
        message: "Starting PostgreSQL optimization...".to_string(),
        percent: Some(0.0),
    });
    
    let analyze_sql = format!(
        "SET search_path TO {}; ANALYZE VERBOSE {};",
        schema,
        match table {
            Some(t) => format!("\"{}\"", t),
            None => String::new(),
        }
    );
    
    let mut analyze_args = vec!["run".to_string(), "-p".to_string(), env_path.to_string_lossy().to_string()];
    if let Some(pwd) = &password {
        analyze_args.push("env".to_string());
        analyze_args.push(format!("PGPASSWORD={}", pwd));
    }
    analyze_args.push("psql".to_string());
    analyze_args.extend(psql_args.iter().cloned());
    analyze_args.push("-d".to_string());
    analyze_args.push(db_name.clone());
    analyze_args.push("-c".to_string());
    analyze_args.push(analyze_sql);
    
    let analyze_output = shell
        .command(micromamba.to_string_lossy().as_ref())
        .args(analyze_args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    let analyze_stderr = String::from_utf8_lossy(&analyze_output.stderr).to_string();
    results.push(format!(
        "ANALYZE: {}{}",
        if analyze_output.status.success() { "SUCCESS" } else { "FAILED" },
        if !analyze_stderr.is_empty() { format!("\n  Error: {}", analyze_stderr.trim()) } else { String::new() }
    ));
    
    let _ = app.emit("gdal-progress", GdalProgress {
        message: "Running ANALYZE...".to_string(),
        percent: Some(30.0),
    });
    
    if options.create_geometry_index {
        let table_filter = match table {
            Some(t) => format!("AND c.table_name = '{}'", t),
            None => String::new(),
        };
        
        let geometry_index_sql = format!(
            r#"SET search_path TO "{schema}";
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN 
        SELECT c.table_name, c.column_name
        FROM information_schema.columns c
        JOIN information_schema.tables t ON c.table_name = t.table_name AND c.table_schema = t.table_schema
        WHERE c.table_schema = '{schema}'
        AND t.table_type = 'BASE TABLE'
        AND c.data_type = 'USER-DEFINED'
        AND c.udt_name = 'geometry'
        {table_filter}
    LOOP
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_' || r.table_name || '_' || r.column_name || ' ON "{schema}".' || r.table_name || ' USING GIST (' || r.column_name || ')', '');
    END LOOP;
END
$$;"#,
            schema = schema, table_filter = table_filter
        );
        
        let mut geom_args = vec!["run".to_string(), "-p".to_string(), env_path.to_string_lossy().to_string()];
        if let Some(pwd) = &password {
            geom_args.push("env".to_string());
            geom_args.push(format!("PGPASSWORD={}", pwd));
        }
        geom_args.push("psql".to_string());
        geom_args.extend(psql_args.iter().cloned());
        geom_args.push("-d".to_string());
        geom_args.push(db_name.clone());
        geom_args.push("-c".to_string());
        geom_args.push(geometry_index_sql);
        
        let geom_output = shell
            .command(micromamba.to_string_lossy().as_ref())
            .args(geom_args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .output()
            .await
            .map_err(|e| e.to_string())?;
        
        let geom_stderr = String::from_utf8_lossy(&geom_output.stderr).to_string();
        results.push(format!(
            "Geometry Index Creation: {}{}",
            if geom_output.status.success() { "SUCCESS" } else { "FAILED" },
            if !geom_stderr.is_empty() { format!("\n  Error: {}", geom_stderr.trim()) } else { String::new() }
        ));
        
        let _ = app.emit("gdal-progress", GdalProgress {
            message: "Creating geometry indexes...".to_string(),
            percent: Some(60.0),
        });
    }
    
    let vacuum_table_spec = match table {
        Some(t) => format!("\"{}\".\"{}\"", schema, t),
        None => format!("\"{}\".*", schema),
    };
    
    let vacuum_sql = format!("VACUUM (ANALYZE, VERBOSE) {};", vacuum_table_spec);
    
    let mut vacuum_args = vec!["run".to_string(), "-p".to_string(), env_path.to_string_lossy().to_string()];
    if let Some(pwd) = &password {
        vacuum_args.push("env".to_string());
        vacuum_args.push(format!("PGPASSWORD={}", pwd));
    }
    vacuum_args.push("psql".to_string());
    vacuum_args.extend(psql_args.iter().cloned());
    vacuum_args.push("-d".to_string());
    vacuum_args.push(db_name);
    vacuum_args.push("-c".to_string());
    vacuum_args.push(vacuum_sql);
    
    let vacuum_output = shell
        .command(micromamba.to_string_lossy().as_ref())
        .args(vacuum_args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    let vacuum_stderr = String::from_utf8_lossy(&vacuum_output.stderr).to_string();
    results.push(format!(
        "VACUUM ANALYZE: {}{}",
        if vacuum_output.status.success() { "SUCCESS" } else { "FAILED" },
        if !vacuum_stderr.is_empty() { format!("\n  Error: {}", vacuum_stderr.trim()) } else { String::new() }
    ));
    
    let _ = app.emit("gdal-progress", GdalProgress {
        message: "Optimization complete!".to_string(),
        percent: Some(100.0),
    });
    
    let output_details = String::from_utf8_lossy(&vacuum_output.stdout).to_string();
    let combined_output = if !vacuum_stderr.is_empty() {
        format!("{}\n\nStderr:\n{}", output_details, vacuum_stderr)
    } else {
        output_details
    };
    
    Ok(format!(
        "PostgreSQL Optimization Results:\n{}\n\nDetails:\n{}",
        results.join("\n"),
        combined_output
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            test_spawn,
            check_env_status,
            create_env,
            run_in_env,
            get_micromamba_version,
            export_env,
            import_env,
            extract_offline_package,
            check_gdal,
            ogr_convert,
            list_env_bins,
            check_gdal_drivers,
            optimize_postgres,
            load_connections,
            save_connection,
            test_connection,
            delete_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
