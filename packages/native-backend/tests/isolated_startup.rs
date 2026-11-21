//! Exercises the real backend only when explicitly given an empty test directory.
#![recursion_limit = "256"]

use native_backend::{
    CreateInstance, ExportPack, NativeBackend, StartupStatus,
};
use std::io::Read;
use std::path::PathBuf;

#[tokio::test]
#[ignore = "requires an explicitly configured empty THESEUS_CONFIG_DIR"]
async fn initializes_and_lists_an_empty_library() {
    let config_dir =
        PathBuf::from(std::env::var_os("THESEUS_CONFIG_DIR").expect(
            "THESEUS_CONFIG_DIR must point to a dedicated test directory",
        ));
    assert!(config_dir.is_absolute() && config_dir.is_dir());
    assert!(config_dir.read_dir().unwrap().next().is_none());
    let legacy_settings = dirs::config_dir()
        .expect("OS config directory must be available")
        .join("com.modrinth.theseus/settings.json");
    assert!(
        !legacy_settings.exists(),
        "refusing to import a real legacy launcher configuration"
    );

    assert!(NativeBackend::start("../unsafe".to_string()).await.is_err());
    let (backend, _events) =
        NativeBackend::start("local.native-launcher-test".to_string())
            .await
            .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        backend.wait_ready(),
    )
    .await
    .expect("backend initialization timed out")
    .expect("backend initialization failed");
    assert_eq!(*backend.startup_status().borrow(), StartupStatus::Ready);
    if let Some(manifest_path) =
        std::env::var_os("NATIVE_TEST_MINECRAFT_MANIFEST")
    {
        let manifest = std::fs::read_to_string(manifest_path).unwrap();
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(config_dir.join("app.db"));
        let pool = sqlx::SqlitePool::connect_with(options).await.unwrap();
        sqlx::query("INSERT OR REPLACE INTO cache (id, data_type, data, expires) VALUES ('0', 'minecraft_manifest', json(?), 4102444800)")
            .bind(manifest)
            .execute(&pool)
            .await
            .unwrap();
        let catalog = backend.minecraft_versions().await.unwrap();
        assert!(!catalog.versions.is_empty());
        pool.close().await;
    }
    if let Some(version_path) =
        std::env::var_os("NATIVE_TEST_MINECRAFT_VERSION")
    {
        let version_dir = config_dir.join("meta/versions/1.20.1");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::copy(version_path, version_dir.join("1.20.1.json")).unwrap();
    }
    if let Some(library_root) = std::env::var_os("NATIVE_TEST_LIBRARIES") {
        let source = PathBuf::from(library_root);
        let version =
            std::fs::read(config_dir.join("meta/versions/1.20.1/1.20.1.json"))
                .unwrap();
        let version: serde_json::Value =
            serde_json::from_slice(&version).unwrap();
        let mut copied = std::collections::HashSet::new();
        for library in version["libraries"].as_array().unwrap() {
            let downloads = &library["downloads"];
            let artifacts = downloads["artifact"]
                .as_object()
                .into_iter()
                .map(|_| &downloads["artifact"])
                .chain(
                    downloads["classifiers"]
                        .as_object()
                        .into_iter()
                        .flat_map(|map| map.values()),
                );
            for artifact in artifacts {
                let (Some(path), Some(hash)) =
                    (artifact["path"].as_str(), artifact["sha1"].as_str())
                else {
                    continue;
                };
                assert!(
                    !path.contains(['\\', ':'])
                        && std::path::Path::new(path).components().all(
                            |part| matches!(
                                part,
                                std::path::Component::Normal(_)
                            )
                        )
                );
                assert!(
                    hash.len() == 40
                        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                );
                if !copied.insert(path) {
                    continue;
                }
                let path = std::path::Path::new(path);
                let input = source.join(path);
                if !input.is_file() {
                    continue;
                }
                let bytes = std::fs::read(input).unwrap();
                assert_eq!(sha1_smol::Sha1::from(&bytes).hexdigest(), hash);
                let output = config_dir.join("meta/libraries").join(path);
                std::fs::create_dir_all(output.parent().unwrap()).unwrap();
                std::fs::write(output, bytes).unwrap();
            }
        }
    }
    if let Some(index_path) = std::env::var_os("NATIVE_TEST_ASSET_INDEX") {
        let index_dir = config_dir.join("meta/assets/indexes");
        std::fs::create_dir_all(&index_dir).unwrap();
        std::fs::copy(index_path, index_dir.join("5.json")).unwrap();
    }
    if let Some(objects_path) = std::env::var_os("NATIVE_TEST_ASSET_OBJECTS") {
        let index =
            std::fs::read(config_dir.join("meta/assets/indexes/5.json"))
                .unwrap();
        let index: serde_json::Value = serde_json::from_slice(&index).unwrap();
        let source = PathBuf::from(objects_path);
        let mut copied = std::collections::HashSet::new();
        for object in index["objects"]
            .as_object()
            .expect("asset index must contain objects")
            .values()
        {
            let hash = object["hash"].as_str().expect("asset hash is missing");
            assert!(
                hash.len() == 40
                    && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            );
            if !copied.insert(hash) {
                continue;
            }
            let bytes =
                std::fs::read(source.join(&hash[..2]).join(hash)).unwrap();
            assert_eq!(sha1_smol::Sha1::from(&bytes).hexdigest(), hash);
            let destination =
                config_dir.join("meta/assets/objects").join(&hash[..2]);
            std::fs::create_dir_all(&destination).unwrap();
            std::fs::write(destination.join(hash), bytes).unwrap();
        }
    }
    if let Some(client_path) = std::env::var_os("NATIVE_TEST_CLIENT_JAR") {
        let bytes = std::fs::read(client_path).unwrap();
        assert_eq!(
            sha1_smol::Sha1::from(&bytes).hexdigest(),
            "0c3ec587af28e5a785c0b4a7b8a30f9a8f78f838"
        );
        let version_dir = config_dir.join("meta/versions/1.20.1");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(version_dir.join("1.20.1.jar"), bytes).unwrap();
    }
    if let Some(log_path) = std::env::var_os("NATIVE_TEST_LOG_CONFIG") {
        let bytes = std::fs::read(log_path).unwrap();
        assert_eq!(
            sha1_smol::Sha1::from(&bytes).hexdigest(),
            "bd65e7d2e3c237be76cfbef4c2405033d7f91521"
        );
        let log_dir = config_dir.join("meta/log_configs");
        std::fs::create_dir_all(&log_dir).unwrap();
        std::fs::write(log_dir.join("client-1.12.xml"), bytes).unwrap();
    }
    assert!(backend.minecraft_accounts().await.unwrap().is_empty());
    assert!(backend.configured_java_versions().await.unwrap().is_empty());
    if let Some(java_path) = std::env::var_os("NATIVE_TEST_JAVA_17") {
        let java_path = PathBuf::from(java_path);
        assert!(java_path.is_file());
        let version =
            theseus::jre::check_jre(java_path.parent().unwrap().to_path_buf())
                .await
                .expect("test Java could not be validated");
        assert_eq!(version.parsed_version, 17);
        theseus::jre::set_java_version(version).await.unwrap();
        assert!(
            backend
                .configured_java_versions()
                .await
                .unwrap()
                .iter()
                .any(|version| version.parsed_version == 17)
        );
    }
    assert!(
        backend
            .loader_versions(theseus::data::ModLoader::Vanilla)
            .await
            .is_err()
    );
    assert!(backend.content_service_accounts().await.unwrap().is_empty());
    assert!(
        backend
            .set_active_content_service_account("missing-user")
            .await
            .is_err()
    );
    assert!(backend.default_minecraft_account().await.unwrap().is_none());
    assert!(
        backend
            .set_default_minecraft_account(uuid::Uuid::nil())
            .await
            .is_err()
    );
    let mut settings = backend.settings().await.unwrap();
    let changed_hide_setting = !settings.hide_on_process_start;
    settings.hide_on_process_start = changed_hide_setting;
    backend.set_settings(settings).await.unwrap();
    assert_eq!(
        backend.settings().await.unwrap().hide_on_process_start,
        changed_hide_setting
    );
    let current = backend.settings().await.unwrap();
    let next_hide = !current.hide_on_process_start;
    let next_refocus = !current.refocus_on_game_close;
    let (hide_result, refocus_result) = tokio::join!(
        backend.update_settings(
            |settings| settings.hide_on_process_start = next_hide
        ),
        backend.update_settings(
            |settings| settings.refocus_on_game_close = next_refocus
        ),
    );
    hide_result.unwrap();
    refocus_result.unwrap();
    let current = backend.settings().await.unwrap();
    assert_eq!(current.hide_on_process_start, next_hide);
    assert_eq!(current.refocus_on_game_close, next_refocus);
    assert!(backend.list_instances().await.unwrap().is_empty());
    let source = config_dir.join("import-source");
    std::fs::create_dir_all(source.join("instances")).unwrap();
    assert!(
        backend
            .importable_instances(
                theseus::pack::import::ImportLauncherType::MultiMC,
                source.clone()
            )
            .await
            .unwrap()
            .is_empty()
    );
    let candidate = source.join("instances").join("Example pack");
    std::fs::create_dir(&candidate).unwrap();
    let cfg = b"[General]\nname=Example pack\n";
    let pack = br#"{"formatVersion":1,"components":[{"uid":"net.minecraft","version":"1.20.1"}]}"#;
    std::fs::write(candidate.join("instance.cfg"), cfg).unwrap();
    std::fs::write(candidate.join("mmc-pack.json"), pack).unwrap();
    std::fs::create_dir(source.join("instances").join("Not a pack")).unwrap();
    let discovered = backend
        .importable_instances(
            theseus::pack::import::ImportLauncherType::MultiMC,
            source.clone(),
        )
        .await
        .unwrap();
    assert_eq!(discovered, ["Example pack"]);
    assert_eq!(std::fs::read(candidate.join("instance.cfg")).unwrap(), cfg);
    assert_eq!(
        std::fs::read(candidate.join("mmc-pack.json")).unwrap(),
        pack
    );
    let minecraft_dir = candidate.join(".minecraft");
    std::fs::create_dir(&minecraft_dir).unwrap();
    std::fs::write(minecraft_dir.join("source-marker.txt"), b"original data")
        .unwrap();
    assert!(
        backend
            .import_instance(
                theseus::pack::import::ImportLauncherType::MultiMC,
                source.clone(),
                "../outside".to_string(),
            )
            .await
            .is_err()
    );
    assert!(backend.list_install_jobs(true).await.unwrap().is_empty());
    let imported = backend
        .import_instance(
            theseus::pack::import::ImportLauncherType::MultiMC,
            source,
            "Example pack".to_string(),
        )
        .await
        .unwrap();
    let import_id = uuid::Uuid::parse_str(&imported.job_id).unwrap();
    let import_result =
        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            loop {
                let snapshot =
                    backend.get_install_job(import_id).await.unwrap();
                if snapshot.status.is_finished() {
                    break snapshot;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("isolated import did not finish");
    assert!(matches!(
        import_result.status,
        theseus::install::InstallJobStatus::Succeeded
            | theseus::install::InstallJobStatus::Failed
    ));
    if std::env::var_os("NATIVE_TEST_REQUIRE_IMPORT_SUCCESS").is_some() {
        assert!(
            matches!(
                import_result.status,
                theseus::install::InstallJobStatus::Succeeded
            ),
            "isolated import did not succeed"
        );
    }
    assert_eq!(
        std::fs::read(minecraft_dir.join("source-marker.txt")).unwrap(),
        b"original data"
    );
    assert_eq!(std::fs::read(candidate.join("instance.cfg")).unwrap(), cfg);
    assert_eq!(
        std::fs::read(candidate.join("mmc-pack.json")).unwrap(),
        pack
    );
    if let Some(instance_id) = import_result.instance_id {
        if matches!(
            import_result.status,
            theseus::install::InstallJobStatus::Succeeded
        ) {
            let edited = backend
                .edit_instance(
                    &instance_id,
                    theseus::data::EditInstance {
                        name: Some("Renamed isolated instance".to_string()),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
            assert_eq!(edited.instance.name, "Renamed isolated instance");
            let launch_settings = backend
                .set_instance_launch_settings(
                    &instance_id,
                    None,
                    Some(vec!["-Dnative-test=enabled".to_string()]),
                    None,
                    Some(true),
                    Some((1280, 720)),
                    Some(vec![("NATIVE_TEST".into(), "enabled".into())]),
                    theseus::data::Hooks {
                        pre_launch: Some("echo pre".into()),
                        wrapper: None,
                        post_exit: Some("echo post".into()),
                    },
                )
                .await
                .unwrap();
            assert_eq!(
                launch_settings
                    .launch_overrides
                    .extra_launch_args
                    .as_deref(),
                Some(["-Dnative-test=enabled".to_string()].as_slice())
            );
            assert_eq!(
                launch_settings.launch_overrides.force_fullscreen,
                Some(true)
            );
            assert_eq!(
                launch_settings
                    .launch_overrides
                    .game_resolution
                    .map(|size| (size.0, size.1)),
                Some((1280, 720))
            );
            assert_eq!(
                launch_settings.launch_overrides.custom_env_vars,
                Some(vec![("NATIVE_TEST".into(), "enabled".into())])
            );
            assert_eq!(
                launch_settings.launch_overrides.hooks.pre_launch.as_deref(),
                Some("echo pre")
            );
            assert_eq!(
                launch_settings.launch_overrides.hooks.post_exit.as_deref(),
                Some("echo post")
            );
            let defaults_restored = backend
                .set_instance_launch_settings(
                    &instance_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    theseus::data::Hooks::default(),
                )
                .await
                .unwrap();
            assert!(
                defaults_restored
                    .launch_overrides
                    .extra_launch_args
                    .is_none()
            );
            assert!(
                defaults_restored
                    .launch_overrides
                    .force_fullscreen
                    .is_none()
            );
            assert!(
                defaults_restored.launch_overrides.game_resolution.is_none()
            );
            assert!(
                defaults_restored.launch_overrides.custom_env_vars.is_none()
            );
            assert!(
                defaults_restored
                    .launch_overrides
                    .hooks
                    .pre_launch
                    .is_none()
            );
            assert!(
                defaults_restored.launch_overrides.hooks.post_exit.is_none()
            );
            let content_backend = backend.clone();
            let content_instance = instance_id.clone();
            let content_dir = config_dir.clone();
            tokio::spawn(async move {
                exercise_content(
                    &content_backend,
                    &content_instance,
                    &content_dir,
                )
                .await;
            })
            .await
            .unwrap();
            assert_eq!(
                backend
                    .get_instance(&instance_id)
                    .await
                    .unwrap()
                    .unwrap()
                    .instance
                    .name,
                "Renamed isolated instance"
            );
            assert!(
                backend
                    .list_instance_files(&instance_id, "")
                    .await
                    .unwrap()
                    .iter()
                    .any(|file| file.name == "source-marker.txt")
            );
            assert_eq!(
                backend
                    .read_instance_file(&instance_id, "source-marker.txt")
                    .await
                    .unwrap(),
                b"original data"
            );
            assert!(
                backend
                    .export_candidates(&instance_id)
                    .await
                    .unwrap()
                    .iter()
                    .any(|candidate| candidate.path.as_str()
                        == "source-marker.txt")
            );
            let export_path = config_dir.join("fixture.mrpack");
            let request = || ExportPack {
                instance_id: instance_id.clone(),
                destination: export_path.clone(),
                included_paths: vec!["source-marker.txt".to_string()],
                excluded_paths: Vec::new(),
                version_id: Some("test-version".to_string()),
                description: None,
                name: None,
            };
            backend.export_pack(request()).await.unwrap();
            let original = std::fs::read(&export_path).unwrap();
            let mut archive =
                zip::ZipArchive::new(std::io::Cursor::new(&original)).unwrap();
            let mut contents = String::new();
            archive
                .by_name("overrides/source-marker.txt")
                .unwrap()
                .read_to_string(&mut contents)
                .unwrap();
            assert_eq!(contents, "original data");
            assert!(archive.by_name("modrinth.index.json").is_ok());
            assert!(backend.export_pack(request()).await.is_err());
            assert_eq!(std::fs::read(&export_path).unwrap(), original);
            let roundtrip = backend
                .import_pack_file(export_path.clone(), None)
                .await
                .unwrap();
            let roundtrip_id =
                uuid::Uuid::parse_str(&roundtrip.job_id).unwrap();
            let roundtrip = tokio::time::timeout(
                std::time::Duration::from_secs(60),
                async {
                    loop {
                        let snapshot = backend
                            .get_install_job(roundtrip_id)
                            .await
                            .unwrap();
                        if snapshot.status.is_finished() {
                            break snapshot;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(
                            100,
                        ))
                        .await;
                    }
                },
            )
            .await
            .expect("round-trip import timed out");
            assert!(
                matches!(
                    roundtrip.status,
                    theseus::install::InstallJobStatus::Succeeded
                ),
                "round-trip import failed: {:?}",
                roundtrip.error
            );
            let roundtrip_instance =
                roundtrip.instance_id.expect("round-trip instance missing");
            assert_eq!(
                backend
                    .read_instance_file(
                        &roundtrip_instance,
                        "source-marker.txt"
                    )
                    .await
                    .unwrap(),
                b"original data"
            );
            assert_eq!(std::fs::read(&export_path).unwrap(), original);
            backend.remove_instance(&roundtrip_instance).await.unwrap();
            backend.dismiss_install_job(roundtrip_id).await.unwrap();
            assert!(
                backend
                    .read_instance_file(&instance_id, "../outside")
                    .await
                    .is_err()
            );
            backend
                .create_instance_directory(&instance_id, "native-test")
                .await
                .unwrap();
            backend
                .write_instance_file(
                    &instance_id,
                    "native-test/note.txt",
                    b"test contents",
                    true,
                )
                .await
                .unwrap();
            assert!(
                backend
                    .write_instance_file(
                        &instance_id,
                        "native-test/note.txt",
                        b"overwrite",
                        true
                    )
                    .await
                    .is_err()
            );
            assert!(
                backend
                    .write_instance_file(
                        &instance_id,
                        "../outside",
                        b"blocked",
                        true
                    )
                    .await
                    .is_err()
            );
            assert!(
                backend
                    .write_instance_file(
                        &instance_id,
                        "mods/blocked.jar",
                        b"blocked",
                        true
                    )
                    .await
                    .is_err()
            );
            backend
                .rename_instance_file(
                    &instance_id,
                    "native-test/note.txt",
                    "native-test/renamed.txt",
                )
                .await
                .unwrap();
            assert_eq!(
                backend
                    .read_instance_file(&instance_id, "native-test/renamed.txt")
                    .await
                    .unwrap(),
                b"test contents"
            );
            backend
                .delete_instance_file(
                    &instance_id,
                    "native-test/renamed.txt",
                    false,
                )
                .await
                .unwrap();
            backend
                .delete_instance_file(&instance_id, "native-test", false)
                .await
                .unwrap();
            let group = backend
                .create_group("Imported test".to_string())
                .await
                .unwrap();
            backend
                .set_group_memberships(vec![
                    theseus::instance::InstanceGroupMembershipUpdate {
                        instance_id: instance_id.clone(),
                        group_ids: vec![group.id.clone()],
                    },
                ])
                .await
                .unwrap();
            assert!(
                backend
                    .get_instance(&instance_id)
                    .await
                    .unwrap()
                    .unwrap()
                    .group_ids
                    .contains(&group.id)
            );
            backend.delete_group(group.id).await.unwrap();
        }
        backend.remove_instance(&instance_id).await.unwrap();
    }
    backend.dismiss_install_job(import_id).await.unwrap();
    assert!(backend.list_instances().await.unwrap().is_empty());
    assert!(backend.running_processes().await.unwrap().is_empty());
    assert!(backend.instance_logs("missing-instance").await.is_err());
    assert!(
        backend
            .list_instance_files("missing-instance", "")
            .await
            .is_err()
    );
    assert!(
        backend
            .read_instance_file("missing-instance", "../outside")
            .await
            .is_err()
    );
    assert!(
        backend
            .latest_log_cursor("missing-instance", 0)
            .await
            .is_err()
    );
    assert!(
        backend
            .get_instance("missing-instance")
            .await
            .unwrap()
            .is_none()
    );
    assert!(backend.list_install_jobs(true).await.unwrap().is_empty());
    let missing_job = uuid::Uuid::nil();
    assert!(backend.get_install_job(missing_job).await.is_err());
    assert!(backend.pause_install_job(missing_job).await.is_err());
    assert!(backend.resume_install_job(missing_job).await.is_err());
    assert!(backend.cancel_install_job(missing_job).await.is_err());
    assert!(backend.retry_install_job(missing_job).await.is_err());
    assert!(
        backend
            .install_job_support_details(missing_job)
            .await
            .is_err()
    );
    assert!(backend.create_group("  ".to_string()).await.is_err());
    let group = backend
        .create_group("Test group".to_string())
        .await
        .unwrap();
    let second = backend.create_group("Second".to_string()).await.unwrap();
    backend
        .set_group_order(vec![second.id.clone(), group.id.clone()])
        .await
        .unwrap();
    let ordered = backend.list_groups().await.unwrap();
    let first_position = ordered
        .iter()
        .position(|entry| entry.id == second.id)
        .unwrap();
    let second_position = ordered
        .iter()
        .position(|entry| entry.id == group.id)
        .unwrap();
    assert!(first_position < second_position);
    assert!(
        backend
            .set_group_order(vec![group.id.clone(), group.id.clone()])
            .await
            .is_err()
    );
    backend.delete_group(second.id).await.unwrap();
    assert!(
        backend
            .list_groups()
            .await
            .unwrap()
            .iter()
            .any(|entry| entry.id == group.id && entry.name == "Test group")
    );
    let renamed = backend
        .rename_group(group.id.clone(), "Renamed".to_string())
        .await
        .unwrap();
    assert_eq!(renamed.name, "Renamed");
    backend.delete_group(group.id.clone()).await.unwrap();
    assert!(
        !backend
            .list_groups()
            .await
            .unwrap()
            .iter()
            .any(|entry| entry.id == group.id)
    );
    assert!(
        NativeBackend::start("local.another-launcher".to_string())
            .await
            .is_err()
    );
    assert!(backend.list_instances().await.unwrap().is_empty());
    let job = backend
        .create_instance(CreateInstance {
            name: "Isolated invalid version".to_string(),
            game_version: "invalid-integration-test-version".to_string(),
            loader: theseus::data::ModLoader::Vanilla,
            loader_version: None,
            icon_path: None,
            icon_config: None,
            link: theseus::data::InstanceLink::Unmanaged,
        })
        .await
        .expect("failed to enqueue the isolated installation job");
    let job_id = uuid::Uuid::parse_str(&job.job_id).unwrap();
    let finished =
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                let snapshot = backend.get_install_job(job_id).await.unwrap();
                if snapshot.status.is_finished() {
                    break snapshot;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("isolated installation did not finish");
    assert!(matches!(
        finished.status,
        theseus::install::InstallJobStatus::Failed
    ));
    assert!(finished.error.is_some());
    assert!(
        !backend
            .install_job_support_details(job_id)
            .await
            .unwrap()
            .is_empty()
    );
    if let Some(instance_id) = finished.instance_id {
        backend.remove_instance(&instance_id).await.unwrap();
        assert!(backend.get_instance(&instance_id).await.unwrap().is_none());
    }
    assert!(backend.list_instances().await.unwrap().is_empty());
}

async fn exercise_content(
    backend: &NativeBackend,
    instance_id: &str,
    config_dir: &std::path::Path,
) {
    let pack_path = config_dir.join("test-resource-pack.zip");
    {
        use std::io::Write;
        let mut archive =
            zip::ZipWriter::new(std::fs::File::create(&pack_path).unwrap());
        archive
            .start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive
            .write_all(
                br#"{"pack":{"pack_format":15,"description":"test pack"}}"#,
            )
            .unwrap();
        archive.finish().unwrap();
    }
    let original_pack = std::fs::read(&pack_path).unwrap();
    let installed = backend
        .add_content_file(
            &instance_id,
            &pack_path,
            Some(theseus::data::ProjectType::ResourcePack),
        )
        .await
        .unwrap();
    assert!(
        backend
            .instance_content(&instance_id, None)
            .await
            .unwrap()
            .iter()
            .any(|item| item.file_path == installed && item.enabled)
    );
    let disabled = backend
        .set_content_enabled(&instance_id, &installed, false)
        .await
        .unwrap();
    assert!(
        backend
            .instance_content(&instance_id, None)
            .await
            .unwrap()
            .iter()
            .any(|item| item.file_path == disabled && !item.enabled)
    );
    let enabled = backend
        .set_content_enabled(&instance_id, &disabled, true)
        .await
        .unwrap();
    backend
        .remove_content(&instance_id, &enabled)
        .await
        .unwrap();
    assert!(
        !backend
            .instance_content(&instance_id, None)
            .await
            .unwrap()
            .iter()
            .any(|item| item.file_path == enabled)
    );
    assert_eq!(std::fs::read(&pack_path).unwrap(), original_pack);
}
