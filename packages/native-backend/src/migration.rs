use fs4::tokio::AsyncFileExt;
use sqlx::{Connection, Row, sqlite::SqliteConnectOptions};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::{fs, fs::File};

const UPSTREAM_APP_IDENTIFIER: &str = "ModrinthApp";
const MIGRATION_RESERVE_BYTES: u64 = 256 * 1024 * 1024;
const FRESH_START_MARKER: &str = ".northstar-start-fresh-v1";

/// Persist an explicit choice to keep the new launcher's library separate.
/// The marker is written only beneath the validated target app directory.
pub async fn remember_fresh_start_choice(
    target_app_identifier: &str,
) -> Result<(), String> {
    if !crate::valid_app_identifier(target_app_identifier) {
        return Err("invalid target application identifier".into());
    }
    let custom_settings_dir =
        std::env::var_os("THESEUS_CONFIG_DIR").map(PathBuf::from);
    let data_dir = dirs::data_dir();
    let target_settings_dir = custom_settings_dir
        .clone()
        .or_else(|| {
            data_dir
                .as_ref()
                .map(|data_dir| data_dir.join(target_app_identifier))
        })
        .ok_or_else(|| {
            "could not locate the application data directory".to_owned()
        })?;
    fs::create_dir_all(&target_settings_dir)
        .await
        .map_err(|error| error.to_string())?;
    let canonical_target = fs::canonicalize(&target_settings_dir)
        .await
        .map_err(|error| error.to_string())?;
    if custom_settings_dir.is_none() {
        let data_dir = data_dir.ok_or_else(|| {
            "could not locate the application data directory".to_owned()
        })?;
        let canonical_data_dir = fs::canonicalize(&data_dir)
            .await
            .map_err(|error| error.to_string())?;
        if !canonical_target.starts_with(canonical_data_dir) {
            return Err(
                "target profile resolves outside the application data directory"
                    .into(),
            );
        }
    }

    let marker = canonical_target.join(FRESH_START_MARKER);
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker)
        .await
    {
        Ok(mut file) => {
            file.write_all(b"fresh-v1\n")
                .await
                .map_err(|error| error.to_string())?;
            file.sync_all().await.map_err(|error| error.to_string())?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let contents = fs::read_to_string(&marker)
                .await
                .map_err(|error| error.to_string())?;
            if contents == "fresh-v1\n" {
                Ok(())
            } else {
                Err("fresh-start marker exists but is invalid".into())
            }
        }
        Err(error) => Err(error.to_string()),
    }
}

#[derive(Clone, Debug)]
pub struct ExistingModrinthData {
    pub instance_count: u64,
    pub copy_bytes: u64,
    pub available_bytes: u64,
    source_settings_dir: PathBuf,
    source_config_dir: PathBuf,
    target_settings_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ExistingModrinthMigration {
    pub instance_count: u64,
    pub backup_dir: Option<PathBuf>,
}

/// Find the current Modrinth installation without opening it for writes.
///
/// The plan is only a preview. `copy_existing_modrinth_data` rechecks the
/// database, obtains the Theseus store locks, and checks free space again.
pub async fn inspect_existing_modrinth_data(
    target_app_identifier: &str,
) -> Result<Option<ExistingModrinthData>, String> {
    if !crate::valid_app_identifier(target_app_identifier) {
        return Err("invalid target application identifier".into());
    }
    if std::env::var_os("THESEUS_CONFIG_DIR").is_some() {
        return Ok(None);
    }
    let Some(data_dir) = dirs::data_dir() else {
        return Err("could not locate the application data directory".into());
    };
    let source_settings_dir = data_dir.join(UPSTREAM_APP_IDENTIFIER);
    let source_database = source_settings_dir.join("app.db");
    if !fs::try_exists(&source_database)
        .await
        .map_err(|error| error.to_string())?
    {
        return Ok(None);
    }

    let source = read_database_summary(&source_database).await?;
    if source.instance_count == 0 {
        return Ok(None);
    }
    let target_settings_dir = data_dir.join(target_app_identifier);
    let fresh_start_marker = target_settings_dir.join(FRESH_START_MARKER);
    if fs::read_to_string(&fresh_start_marker)
        .await
        .is_ok_and(|contents| contents == "fresh-v1\n")
    {
        return Ok(None);
    }
    let target_database = target_settings_dir.join("app.db");
    if fs::try_exists(&target_database)
        .await
        .map_err(|error| error.to_string())?
        && read_database_summary(&target_database)
            .await
            .map(|summary| summary.instance_count > 0)?
    {
        return Ok(None);
    }
    if !fs::try_exists(&source.config_dir)
        .await
        .map_err(|error| error.to_string())?
    {
        return Err(format!(
            "the existing Modrinth profile directory is missing: {}",
            source.config_dir.display()
        ));
    }

    let settings_dir = source_settings_dir.clone();
    let config_dir = source.config_dir.clone();
    let (tree_bytes, database_bytes) = tokio::task::spawn_blocking(move || {
        let mut bytes = tree_size(&settings_dir)?;
        if !same_path(&settings_dir, &config_dir)?
            && !config_dir.starts_with(&settings_dir)
            && !settings_dir.starts_with(&config_dir)
        {
            bytes = bytes.saturating_add(tree_size(&config_dir)?);
        }
        let db_bytes = std::fs::metadata(&source_database)
            .map_err(|error| error.to_string())?
            .len()
            .saturating_add(
                std::fs::metadata(source_database.with_file_name("app.db-wal"))
                    .map_or(0, |metadata| metadata.len()),
            );
        Ok::<_, String>((bytes, db_bytes))
    })
    .await
    .map_err(|error| error.to_string())??;

    let target_parent = target_settings_dir
        .parent()
        .ok_or_else(|| "target data directory has no parent".to_owned())?
        .to_owned();
    let available_bytes = tokio::task::spawn_blocking(move || {
        fs4::available_space(target_parent).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;

    Ok(Some(ExistingModrinthData {
        instance_count: source.instance_count,
        copy_bytes: tree_bytes.saturating_add(database_bytes),
        available_bytes,
        source_settings_dir,
        source_config_dir: source.config_dir,
        target_settings_dir,
    }))
}

/// Copy the existing Modrinth library into a separate app directory.
///
/// The source remains unchanged. The destination is prepared in a sibling
/// staging directory, with the database snapshotted using `VACUUM INTO`; an
/// existing empty target is retained as a backup before the atomic swap.
pub async fn copy_existing_modrinth_data(
    plan: ExistingModrinthData,
) -> Result<ExistingModrinthMigration, String> {
    let source_database = plan.source_settings_dir.join("app.db");
    let target_root = plan.target_settings_dir.clone();
    let source_root = std::fs::canonicalize(&plan.source_settings_dir)
        .map_err(|error| error.to_string())?;
    let target_parent = target_root
        .parent()
        .ok_or_else(|| "target data directory has no parent".to_owned())?;
    fs::create_dir_all(target_parent)
        .await
        .map_err(|error| error.to_string())?;
    let target_parent = std::fs::canonicalize(target_parent)
        .map_err(|error| error.to_string())?;
    let target_name = target_root
        .file_name()
        .ok_or_else(|| "target data directory has no name".to_owned())?;
    let target_canonical = target_parent.join(target_name);
    if target_canonical == source_root
        || target_canonical.starts_with(&source_root)
        || source_root.starts_with(&target_canonical)
    {
        return Err("source and target application directories overlap".into());
    }
    let source_lock = acquire_store_lock(&plan.source_settings_dir).await?;
    let config_lock =
        if same_path(&plan.source_settings_dir, &plan.source_config_dir)? {
            None
        } else {
            Some(acquire_store_lock(&plan.source_config_dir).await?)
        };
    fs::create_dir_all(&target_root)
        .await
        .map_err(|error| error.to_string())?;
    let target_lock = acquire_or_create_store_lock(&target_root).await?;

    let current_source = read_database_summary(&source_database).await?;
    if current_source.instance_count != plan.instance_count
        || !same_path(&current_source.config_dir, &plan.source_config_dir)?
    {
        return Err(
            "the existing Modrinth library changed after its preview; scan it again".into(),
        );
    }
    if fs::try_exists(target_root.join("app.db"))
        .await
        .map_err(|error| error.to_string())?
        && read_database_summary(&target_root.join("app.db"))
            .await
            .map(|summary| summary.instance_count > 0)?
    {
        return Err("the Northstar library already contains instances".into());
    }

    let target_parent = target_root
        .parent()
        .ok_or_else(|| "target data directory has no parent".to_owned())?;
    let required_bytes =
        plan.copy_bytes.saturating_add(MIGRATION_RESERVE_BYTES);
    let available_bytes = fs4::available_space(target_parent)
        .map_err(|error| error.to_string())?;
    if available_bytes < required_bytes {
        return Err(format!(
            "copying the existing library needs about {} bytes plus a safety margin, but only {} bytes are available",
            required_bytes, available_bytes
        ));
    }

    let stage = tempfile::Builder::new()
        .prefix(".northstar-migration-")
        .tempdir_in(target_parent)
        .map_err(|error| error.to_string())?;
    let stage_root = stage.path().to_owned();
    let copy_settings = plan.source_settings_dir.clone();
    let copy_config = plan.source_config_dir.clone();
    let copy_stage = stage_root.clone();
    tokio::task::spawn_blocking(move || {
        copy_tree(&copy_settings, &copy_stage)?;
        if !same_path(&copy_settings, &copy_config)?
            && !copy_config.starts_with(&copy_settings)
            && !copy_settings.starts_with(&copy_config)
        {
            copy_tree(&copy_config, &copy_stage)?;
        }
        Ok::<_, String>(())
    })
    .await
    .map_err(|error| error.to_string())??;

    let staged_database = stage_root.join("app.db");
    snapshot_database(&source_database, &staged_database).await?;
    rewrite_staged_data_directory(&staged_database, &target_root).await?;

    drop(target_lock);
    let stage_root = stage.keep();
    let backup_dir = if fs::try_exists(&target_root)
        .await
        .map_err(|error| error.to_string())?
    {
        let backup = target_root
            .with_extension(format!("pre-migration-{}", uuid::Uuid::new_v4()));
        fs::rename(&target_root, &backup)
            .await
            .map_err(|error| error.to_string())?;
        Some(backup)
    } else {
        None
    };
    if let Err(error) = fs::rename(&stage_root, &target_root).await {
        if let Some(backup) = &backup_dir {
            let _ = fs::rename(backup, &target_root).await;
        }
        let _ = fs::remove_dir_all(&stage_root).await;
        return Err(format!("could not activate the copied library: {error}"));
    }

    drop(config_lock);
    drop(source_lock);
    Ok(ExistingModrinthMigration {
        instance_count: plan.instance_count,
        backup_dir,
    })
}

struct DatabaseSummary {
    instance_count: u64,
    config_dir: PathBuf,
}

async fn read_database_summary(
    database: &Path,
) -> Result<DatabaseSummary, String> {
    let options = SqliteConnectOptions::new()
        .filename(database)
        .read_only(true)
        .create_if_missing(false);
    let mut connection = sqlx::SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| {
        format!("could not read {}: {error}", database.display())
    })?;
    let row = sqlx::query(
        "SELECT custom_dir, (SELECT COUNT(*) FROM instances) AS instance_count FROM settings WHERE id = 0",
    )
    .fetch_optional(&mut connection)
    .await
    .map_err(|error| format!("could not inspect {}: {error}", database.display()))?
    .ok_or_else(|| format!("{} has no launcher settings row", database.display()))?;
    let config_dir: Option<String> = row
        .try_get("custom_dir")
        .map_err(|error| error.to_string())?;
    let instance_count: i64 = row
        .try_get("instance_count")
        .map_err(|error| error.to_string())?;
    connection
        .close()
        .await
        .map_err(|error| error.to_string())?;
    let fallback = database
        .parent()
        .ok_or_else(|| "database path has no parent".to_owned())?;
    let config_dir = config_dir
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| fallback.to_owned());
    Ok(DatabaseSummary {
        instance_count: instance_count.max(0) as u64,
        config_dir,
    })
}

async fn snapshot_database(
    source: &Path,
    destination: &Path,
) -> Result<(), String> {
    let options = SqliteConnectOptions::new()
        .filename(source)
        .read_only(true)
        .create_if_missing(false);
    let mut connection = sqlx::SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("VACUUM INTO ?")
        .bind(destination.to_string_lossy().into_owned())
        .execute(&mut connection)
        .await
        .map_err(|error| {
            format!("could not create a consistent database snapshot: {error}")
        })?;
    connection
        .close()
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn rewrite_staged_data_directory(
    database: &Path,
    config_dir: &Path,
) -> Result<(), String> {
    let options = SqliteConnectOptions::new()
        .filename(database)
        .create_if_missing(false);
    let mut connection = sqlx::SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| error.to_string())?;
    let path = config_dir.to_string_lossy().into_owned();
    let result = sqlx::query(
        "UPDATE settings SET custom_dir = ?, prev_custom_dir = ? WHERE id = 0",
    )
    .bind(&path)
    .bind(&path)
    .execute(&mut connection)
    .await
    .map_err(|error| error.to_string())?;
    if result.rows_affected() != 1 {
        return Err("the staged launcher database has no settings row".into());
    }
    connection
        .close()
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn acquire_store_lock(root: &Path) -> Result<File, String> {
    let lock_path = root.join("store.lock");
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .await
        .map_err(|error| {
            format!(
                "could not verify the source is idle at {}: {error}",
                lock_path.display()
            )
        })?;
    file.try_lock().map_err(|error| {
        format!(
            "the launcher is currently using {} or its data lock is unavailable: {error}",
            root.display()
        )
    })?;
    Ok(file)
}

async fn acquire_or_create_store_lock(root: &Path) -> Result<File, String> {
    fs::create_dir_all(root)
        .await
        .map_err(|error| error.to_string())?;
    let lock_path = root.join("store.lock");
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .await
        .map_err(|error| error.to_string())?;
    file.try_lock().map_err(|error| {
        format!("the Northstar data directory is already in use: {error}")
    })?;
    Ok(file)
}

fn tree_size(root: &Path) -> Result<u64, String> {
    let mut total = 0u64;
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).map_err(|error| {
            format!("could not inspect {}: {error}", directory.display())
        })? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if is_ignored_root_entry(root, &path) {
                continue;
            }
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|error| error.to_string())?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "migration does not follow symbolic links: {}",
                    path.display()
                ));
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    Ok(total)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let mut pending = vec![(source.to_owned(), destination.to_owned())];
    while let Some((source_dir, destination_dir)) = pending.pop() {
        std::fs::create_dir_all(&destination_dir)
            .map_err(|error| error.to_string())?;
        for entry in std::fs::read_dir(&source_dir).map_err(|error| {
            format!("could not copy {}: {error}", source_dir.display())
        })? {
            let entry = entry.map_err(|error| error.to_string())?;
            let source_path = entry.path();
            if is_ignored_root_entry(source, &source_path) {
                continue;
            }
            let metadata = std::fs::symlink_metadata(&source_path)
                .map_err(|error| error.to_string())?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "migration does not follow symbolic links: {}",
                    source_path.display()
                ));
            }
            let destination_path = destination_dir.join(entry.file_name());
            if metadata.is_dir() {
                pending.push((source_path, destination_path));
            } else if metadata.is_file() {
                if destination_path.exists() {
                    return Err(format!(
                        "source data directories contain a colliding path: {}",
                        destination_path.display()
                    ));
                }
                std::fs::copy(&source_path, &destination_path).map_err(
                    |error| {
                        format!(
                            "could not copy {}: {error}",
                            source_path.display()
                        )
                    },
                )?;
            }
        }
    }
    Ok(())
}

fn is_ignored_root_entry(root: &Path, path: &Path) -> bool {
    if path.parent() != Some(root) {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name == "store.lock"
        || name == "app-window-state.json"
        || name == "app.db"
        || name.starts_with("app.db-")
}

fn same_path(left: &Path, right: &Path) -> Result<bool, String> {
    let left =
        std::fs::canonicalize(left).map_err(|error| error.to_string())?;
    let right =
        std::fs::canonicalize(right).map_err(|error| error.to_string())?;
    Ok(left == right)
}
