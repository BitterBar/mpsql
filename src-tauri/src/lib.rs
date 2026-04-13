use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::process::Output;
use tauri::{AppHandle, Emitter, Manager};

// ─── Data structures ─────────────────────────────────────────────────────────

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
        Self {
            connections: Vec::new(),
        }
    }
}

// ─── Config / connection storage ─────────────────────────────────────────────

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
                match std::net::TcpStream::connect_timeout(
                    &addr,
                    std::time::Duration::from_secs(5),
                ) {
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
                Err(format!(
                    "无法连接到 {}:{} - {}",
                    connection.host, port, last_err
                ))
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

// ─── Micromamba / env discovery ───────────────────────────────────────────────

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

    if let Some(path) =
        find_micromamba_in_dir(&app_data.parent().unwrap_or(&app_data).to_path_buf())
    {
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

fn env_executable_dirs(env_path: &Path) -> Vec<PathBuf> {
    if cfg!(target_os = "windows") {
        // Matches the order that `conda activate` uses on Windows.
        // Library/mingw-w64/bin and Library/usr/bin must be in PATH so that
        // DLLs depended on by GDAL (including libpq) are found at load time.
        vec![
            env_path.to_path_buf(),
            env_path.join("Library").join("mingw-w64").join("bin"),
            env_path.join("Library").join("usr").join("bin"),
            env_path.join("Library").join("bin"),
            env_path.join("Scripts"),
            env_path.join("bin"),
        ]
    } else {
        vec![env_path.join("bin")]
    }
}

fn executable_candidates(name: &str) -> Vec<String> {
    if cfg!(target_os = "windows") {
        let mut candidates = vec![format!("{}.exe", name)];
        if name.ends_with(".exe") {
            candidates.push(name.to_string());
        } else {
            candidates.push(name.to_string());
        }
        candidates
    } else {
        vec![name.to_string()]
    }
}

fn find_env_executable(env_path: &Path, name: &str) -> Option<PathBuf> {
    let dirs = env_executable_dirs(env_path);
    let candidates = executable_candidates(name);

    for dir in dirs {
        for candidate in &candidates {
            let path = dir.join(candidate);
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

// ─── Process execution helpers ────────────────────────────────────────────────

/// Build the environment variables needed to run a tool from the conda env
/// directly (without `micromamba run`). Prepends the env's bin directories to
/// PATH and sets GDAL_DATA / PROJ_DATA so GDAL can locate its support files.
fn get_conda_env_vars(env_path: &Path) -> Vec<(OsString, OsString)> {
    let mut vars: Vec<(OsString, OsString)> = Vec::new();

    // ── PATH ──────────────────────────────────────────────────────────────────
    let env_dirs = env_executable_dirs(env_path);
    let current_path = std::env::var_os("PATH").unwrap_or_default();

    let mut path_parts: Vec<PathBuf> = env_dirs.clone();
    for part in std::env::split_paths(&current_path) {
        path_parts.push(part);
    }

    if let Ok(new_path) = std::env::join_paths(path_parts) {
        vars.push((OsString::from("PATH"), new_path));
    }

    // ── GDAL_DATA ─────────────────────────────────────────────────────────────
    let gdal_data = if cfg!(target_os = "windows") {
        env_path.join("Library").join("share").join("gdal")
    } else {
        env_path.join("share").join("gdal")
    };
    if gdal_data.exists() {
        vars.push((OsString::from("GDAL_DATA"), gdal_data.into_os_string()));
    }

    // ── PROJ_DATA / PROJ_LIB ──────────────────────────────────────────────────
    let proj_data = if cfg!(target_os = "windows") {
        env_path.join("Library").join("share").join("proj")
    } else {
        env_path.join("share").join("proj")
    };
    if proj_data.exists() {
        vars.push((
            OsString::from("PROJ_DATA"),
            proj_data.clone().into_os_string(),
        ));
        vars.push((OsString::from("PROJ_LIB"), proj_data.into_os_string()));
    }

    // ── GDAL_DRIVER_PATH ──────────────────────────────────────────────────────
    // Required so GDAL can find plugin drivers (e.g. the PostgreSQL OGR driver)
    // that are compiled as shared libraries rather than built into gdal.dll.
    let gdal_plugins = if cfg!(target_os = "windows") {
        env_path.join("Library").join("lib").join("gdalplugins")
    } else {
        env_path.join("lib").join("gdalplugins")
    };
    if gdal_plugins.exists() {
        vars.push((
            OsString::from("GDAL_DRIVER_PATH"),
            gdal_plugins.into_os_string(),
        ));
    }

    vars
}

/// Apply CREATE_NO_WINDOW flag on Windows so child processes don't flash a
/// console. This is a no-op on other platforms.
#[cfg(target_os = "windows")]
fn no_window(cmd: &mut tokio::process::Command) {
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
}

#[cfg(not(target_os = "windows"))]
fn no_window(_cmd: &mut tokio::process::Command) {}

/// Run a program from the conda env by path, with env vars injected.
async fn spawn_direct(program: &Path, args: &[OsString], env_vars: &[(OsString, OsString)]) -> std::io::Result<Output> {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args);
    cmd.envs(env_vars.iter().map(|(k, v)| (k.as_os_str(), v.as_os_str())));
    no_window(&mut cmd);
    cmd.output().await
}

/// Run a micromamba management command (create / env remove / …).
/// These operate on the micromamba binary itself, not inside the conda env.
async fn run_micromamba(micromamba: &Path, args: &[OsString]) -> Result<Output, String> {
    let mut cmd = tokio::process::Command::new(micromamba);
    cmd.args(args);
    no_window(&mut cmd);
    cmd.output().await.map_err(|e| e.to_string())
}

/// Run `program_name` inside the conda env at `env_path`.
///
/// Strategy:
/// 1. Look up the binary directly via `find_env_executable` and run it with
///    the appropriate env vars — no shell involved on any platform.
/// 2. If the binary is not found on disk (env not yet fully populated), fall
///    back to `micromamba run -p <env_path> <program> <args>`.
async fn run_in_env(
    env_path: &Path,
    micromamba: &Path,
    program_name: &str,
    args: &[OsString],
) -> Result<Output, String> {
    if let Some(program_path) = find_env_executable(env_path, program_name) {
        let env_vars = get_conda_env_vars(env_path);
        return spawn_direct(&program_path, args, &env_vars)
            .await
            .map_err(|e| format!("Failed to run {}: {}", program_name, e));
    }

    // Fallback: micromamba run
    let mut full_args: Vec<OsString> = vec![
        OsString::from("run"),
        OsString::from("-p"),
        env_path.as_os_str().to_owned(),
        OsString::from(program_name),
    ];
    full_args.extend_from_slice(args);

    let mut cmd = tokio::process::Command::new(micromamba);
    cmd.args(&full_args);
    no_window(&mut cmd);
    cmd.output()
        .await
        .map_err(|e| format!("Failed to run {} via micromamba: {}", program_name, e))
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

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
        let remove_output = run_micromamba(
            &micromamba,
            &[
                OsString::from("env"),
                OsString::from("remove"),
                OsString::from("-p"),
                env_path.as_os_str().to_owned(),
                OsString::from("-y"),
            ],
        )
        .await?;

        if !remove_output.status.success() {
            let stderr = String::from_utf8_lossy(&remove_output.stderr).to_string();
            return Err(format!("删除旧环境失败:\n{}", stderr));
        }
    }

    let mut create_args: Vec<OsString> = vec![
        OsString::from("create"),
        OsString::from("-p"),
        env_path.as_os_str().to_owned(),
        OsString::from("-c"),
        OsString::from("conda-forge"),
    ];
    for pkg in &packages {
        create_args.push(OsString::from(pkg));
    }
    create_args.push(OsString::from("-y"));

    let output = run_micromamba(&micromamba, &create_args).await?;

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

    let output = run_in_env(
        &env_path,
        &micromamba,
        "gdalinfo",
        &[OsString::from("--version")],
    )
    .await;

    match output {
        Ok(out) => Ok(out.status.success()),
        Err(e) => Err(format!("Failed to run gdalinfo: {}", e)),
    }
}

#[derive(Debug, Deserialize)]
pub struct OgrConvertOptions {
    pub input_path: String,
    pub output_connection: DbConnection,
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

    let files_to_import: Vec<PathBuf> = if is_directory {
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

    let pg_info = pg_conn_info_from_connection(&options.output_connection)?;

    for (index, file_path) in files_to_import.iter().enumerate() {
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported");

        let _ = app.emit(
            "gdal-progress",
            GdalProgress {
                message: format!("Importing {}/{}: {}...", index + 1, total_files, file_name),
                percent: Some(((index as f64 / total_files as f64) * 100.0) as f32),
            },
        );

        match import_single_file(&env_path, &micromamba, &options, file_path, &pg_info).await {
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
    env_path: &PathBuf,
    micromamba: &PathBuf,
    options: &OgrConvertOptions,
    input_file: &PathBuf,
    pg_info: &PgConnInfo,
) -> Result<String, String> {
    let final_conn = build_libpq_conn_string(pg_info);

    // Create schema if needed
    if let Some(ref schema_name) = options.schema {
        let create_schema_sql = format!(
            "CREATE SCHEMA IF NOT EXISTS {};",
            quote_sql_identifier(schema_name)
        );

        let psql_args: Vec<OsString> = vec![
            OsString::from(&final_conn),
            OsString::from("-c"),
            OsString::from(&create_schema_sql),
        ];

        let create_output = run_in_env(env_path, micromamba, "psql", &psql_args)
            .await
            .map_err(|e| e.to_string())?;

        if !create_output.status.success() {
            let stderr = String::from_utf8_lossy(&create_output.stderr);
            if !stderr.contains("already exists") && !stderr.contains("duplicate schema") {
                return Err(format!("Failed to create schema: {}", stderr));
            }
        }
    }

    let target_table =
        if let (Some(ref schema_name), Some(ref layer)) = (&options.schema, &options.layer_name) {
            Some(format!("{}.{}", schema_name, layer))
        } else if let Some(ref schema_name) = options.schema {
            let base_name = input_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("imported_data")
                .to_string();
            Some(format!("{}.{}", schema_name, base_name))
        } else {
            options.layer_name.clone()
        };

    let mut args: Vec<OsString> = vec![
        OsString::from("-f"),
        OsString::from("PostgreSQL"),
        OsString::from(format!("PG:{}", final_conn)),
    ];

    if let Some(ref table) = target_table {
        args.push(OsString::from("-nln"));
        args.push(OsString::from(table));
    }

    if options.promote_to_multi {
        args.push(OsString::from("-nlt"));
        args.push(OsString::from("PROMOTE_TO_MULTI"));
    }

    if let Some(ref geom_name) = options.geometry_name {
        args.push(OsString::from("-lco"));
        args.push(OsString::from(format!("GEOMETRY_NAME={}", geom_name)));
    }

    if let Some(ref fid) = options.fid_column {
        args.push(OsString::from("-lco"));
        args.push(OsString::from(format!("FID={}", fid)));
    }

    if options.overwrite {
        args.push(OsString::from("-overwrite"));
    }

    if options.skip_failures {
        args.push(OsString::from("-skipfailures"));
    }

    if let Some(ref srs) = options.srs {
        args.push(OsString::from("-s_srs"));
        args.push(OsString::from(srs));
    }

    if let Some(ref srs) = options.target_srs {
        args.push(OsString::from("-t_srs"));
        args.push(OsString::from(srs));
    }

    if let Some(ref fields) = options.select_fields {
        if !fields.is_empty() {
            args.push(OsString::from("-select"));
            args.push(OsString::from(fields));
        }
    }

    if options.use_copy {
        args.push(OsString::from("--config"));
        args.push(OsString::from("PG_USE_COPY"));
        args.push(OsString::from("YES"));
    }

    if let Some(ref enc) = options.encoding {
        args.push(OsString::from("--config"));
        args.push(OsString::from("SHAPE_ENCODING"));
        args.push(OsString::from(enc));
    }

    // Pass the input file path as OsString to preserve non-ASCII characters
    args.push(input_file.as_os_str().to_owned());

    let output = run_in_env(env_path, micromamba, "ogr2ogr", &args).await?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let file_name = input_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("data");

    Ok(format!("{} imported ({})", file_name, stdout.trim()))
}

#[derive(Debug, Deserialize)]
pub struct OptimizeOptions {
    pub connection: DbConnection,
    pub schema: Option<String>,
    pub table: Option<String>,
    pub create_geometry_index: bool,
}

struct PgConnInfo {
    host: String,
    port: String,
    user: String,
    db_name: String,
    password: String,
}

fn pg_conn_info_from_connection(connection: &DbConnection) -> Result<PgConnInfo, String> {
    let host = connection.host.trim();
    let port = connection.port.trim();
    let user = connection.user.trim();
    let db_name = connection.database.trim();

    if host.is_empty() {
        return Err("Database host is required".to_string());
    }

    if db_name.is_empty() {
        return Err("Database name is required".to_string());
    }

    Ok(PgConnInfo {
        host: host.to_string(),
        port: if port.is_empty() {
            "5432".to_string()
        } else {
            port.to_string()
        },
        user: if user.is_empty() {
            "postgres".to_string()
        } else {
            user.to_string()
        },
        db_name: db_name.to_string(),
        password: connection.password.clone(),
    })
}

fn quote_libpq_value(value: &str) -> String {
    let escaped = value.replace('\\', r"\\").replace('\'', r"\'");
    format!("'{}'", escaped)
}

fn build_libpq_conn_string(info: &PgConnInfo) -> String {
    let mut parts = vec![
        format!("host={}", quote_libpq_value(&info.host)),
        format!("port={}", quote_libpq_value(&info.port)),
        format!("dbname={}", quote_libpq_value(&info.db_name)),
        format!("user={}", quote_libpq_value(&info.user)),
    ];

    if !info.password.is_empty() {
        parts.push(format!("password={}", quote_libpq_value(&info.password)));
    }

    parts.join(" ")
}

fn quote_sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn quote_sql_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn build_analyze_sql(schema: &str, table: Option<&str>) -> String {
    let schema_ident = quote_sql_identifier(schema);
    match table {
        Some(table_name) => format!(
            "SET search_path TO {schema}; ANALYZE VERBOSE {table};",
            schema = schema_ident,
            table = quote_sql_identifier(table_name)
        ),
        None => format!(
            "SET search_path TO {schema}; ANALYZE VERBOSE;",
            schema = schema_ident
        ),
    }
}

fn build_geometry_index_sql(schema: &str, table: Option<&str>) -> String {
    let schema_ident = quote_sql_identifier(schema);
    let schema_literal = quote_sql_literal(schema);
    let table_filter = match table {
        Some(table_name) => format!("AND c.table_name = {}", quote_sql_literal(table_name)),
        None => String::new(),
    };

    format!(
        r#"SET search_path TO {schema_ident};
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT c.table_name, c.column_name
        FROM information_schema.columns c
        JOIN information_schema.tables t ON c.table_name = t.table_name AND c.table_schema = t.table_schema
        WHERE c.table_schema = {schema_literal}
        AND t.table_type = 'BASE TABLE'
        AND c.data_type = 'USER-DEFINED'
        AND c.udt_name = 'geometry'
        {table_filter}
    LOOP
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.%I USING GIST (%I)',
            'idx_' || r.table_name || '_' || r.column_name,
            {schema_literal},
            r.table_name,
            r.column_name
        );
    END LOOP;
END
$$;"#,
        schema_ident = schema_ident,
        schema_literal = schema_literal,
        table_filter = table_filter
    )
}

fn build_vacuum_sql(schema: &str, table: Option<&str>) -> String {
    let schema_ident = quote_sql_identifier(schema);
    match table {
        Some(table_name) => format!(
            "VACUUM (ANALYZE, VERBOSE) {schema}.{table};",
            schema = schema_ident,
            table = quote_sql_identifier(table_name)
        ),
        None => format!(
            "SET search_path TO {schema}; VACUUM (ANALYZE, VERBOSE);",
            schema = schema_ident
        ),
    }
}

#[tauri::command]
async fn optimize_postgres(
    app: tauri::AppHandle,
    options: OptimizeOptions,
) -> Result<String, String> {
    let env_path = get_env_path(&app)?;

    if !env_path.exists() {
        return Err("Environment does not exist".to_string());
    }

    let micromamba = get_micromamba_path(&app)?;

    let pg_info = pg_conn_info_from_connection(&options.connection)?;
    let conn_str = build_libpq_conn_string(&pg_info);

    let schema = options.schema.unwrap_or_else(|| "public".to_string());
    let table = options.table.as_deref();
    let mut results = Vec::new();

    let _ = app.emit(
        "gdal-progress",
        GdalProgress {
            message: "Starting PostgreSQL optimization...".to_string(),
            percent: Some(0.0),
        },
    );

    // ── ANALYZE ───────────────────────────────────────────────────────────────
    let analyze_sql = build_analyze_sql(&schema, table);
    let analyze_args: Vec<OsString> = vec![
        OsString::from(&conn_str),
        OsString::from("-c"),
        OsString::from(&analyze_sql),
    ];

    let analyze_output = run_in_env(&env_path, &micromamba, "psql", &analyze_args).await?;
    let analyze_stderr = String::from_utf8_lossy(&analyze_output.stderr).to_string();
    results.push(format!(
        "ANALYZE: {}{}",
        if analyze_output.status.success() { "SUCCESS" } else { "FAILED" },
        if !analyze_stderr.is_empty() {
            format!("\n  Error: {}", analyze_stderr.trim())
        } else {
            String::new()
        }
    ));

    let _ = app.emit(
        "gdal-progress",
        GdalProgress {
            message: "Running ANALYZE...".to_string(),
            percent: Some(30.0),
        },
    );

    // ── Geometry indexes ──────────────────────────────────────────────────────
    if options.create_geometry_index {
        let geometry_index_sql = build_geometry_index_sql(&schema, table);
        let geom_args: Vec<OsString> = vec![
            OsString::from(&conn_str),
            OsString::from("-c"),
            OsString::from(&geometry_index_sql),
        ];

        let geom_output = run_in_env(&env_path, &micromamba, "psql", &geom_args).await?;
        let geom_stderr = String::from_utf8_lossy(&geom_output.stderr).to_string();
        results.push(format!(
            "Geometry Index Creation: {}{}",
            if geom_output.status.success() { "SUCCESS" } else { "FAILED" },
            if !geom_stderr.is_empty() {
                format!("\n  Error: {}", geom_stderr.trim())
            } else {
                String::new()
            }
        ));

        let _ = app.emit(
            "gdal-progress",
            GdalProgress {
                message: "Creating geometry indexes...".to_string(),
                percent: Some(60.0),
            },
        );
    }

    // ── VACUUM ────────────────────────────────────────────────────────────────
    let vacuum_sql = build_vacuum_sql(&schema, table);
    let vacuum_args: Vec<OsString> = vec![
        OsString::from(&conn_str),
        OsString::from("-c"),
        OsString::from(&vacuum_sql),
    ];

    let vacuum_output = run_in_env(&env_path, &micromamba, "psql", &vacuum_args).await?;
    let vacuum_stderr = String::from_utf8_lossy(&vacuum_output.stderr).to_string();
    results.push(format!(
        "VACUUM ANALYZE: {}{}",
        if vacuum_output.status.success() { "SUCCESS" } else { "FAILED" },
        if !vacuum_stderr.is_empty() {
            format!("\n  Error: {}", vacuum_stderr.trim())
        } else {
            String::new()
        }
    ));

    let _ = app.emit(
        "gdal-progress",
        GdalProgress {
            message: "Optimization complete!".to_string(),
            percent: Some(100.0),
        },
    );

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

// ─── App entry point ──────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            check_env_status,
            create_env,
            check_gdal,
            ogr_convert,
            optimize_postgres,
            load_connections,
            save_connection,
            test_connection,
            delete_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
