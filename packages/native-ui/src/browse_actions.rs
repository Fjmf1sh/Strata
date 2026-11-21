use super::*;

impl Launcher {
    pub(super) fn create_browse_collection(&mut self) {
        if self.browse_collection_creating {
            return;
        }
        let name = self.browse_collection_create_name.trim().to_owned();
        if !(3..=64).contains(&name.chars().count()) {
            self.message = "Collection names must be 3–64 characters.".into();
            self.draw();
            return;
        }
        let Some(project_id) = self
            .browse_selected
            .as_ref()
            .map(|project| project.id.clone())
        else {
            return;
        };
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            self.draw();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.browse_collection_creating = true;
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .create_content_service_collection(&name, &project_id)
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::BrowseCollectionCreated(result));
        });
    }

    pub(super) fn begin_browse_collection_creation(&mut self) {
        self.browse_collection_create_name.clear();
        self.browse_collection_create_open = true;
        self.input_focus = 37;
        self.draw();
    }

    pub(super) fn open_browse_save_menu(&mut self) {
        self.browse_save_menu_open = !self.browse_save_menu_open;
        if self.browse_save_menu_open
            && !self.browse_save_collections_loaded
            && !self.browse_save_collections_loading
        {
            let Some(backend) = self.backend.clone() else {
                return;
            };
            let Some(proxy) = self.proxy.clone() else {
                return;
            };
            self.browse_save_collections_loading = true;
            self.runtime.spawn(async move {
                let result = backend
                    .content_service_collections()
                    .await
                    .map_err(|error| error.to_string());
                let _ = proxy
                    .send_event(UiMessage::BrowseSaveCollectionsLoaded(result));
            });
        }
        self.draw();
    }

    pub(super) fn toggle_browse_collection(&mut self, collection_id: &str) {
        if self.browse_save_collection_pending.contains(collection_id) {
            return;
        }
        let Some(project_id) = self
            .browse_selected
            .as_ref()
            .map(|project| project.id.clone())
        else {
            return;
        };
        let Some(collection) = self
            .browse_save_collections
            .iter()
            .find(|collection| collection.id == collection_id)
        else {
            return;
        };
        let mut projects = collection.projects.clone();
        if let Some(index) = projects.iter().position(|id| id == &project_id) {
            projects.remove(index);
        } else {
            projects.push(project_id.clone());
        }
        let collection_id = collection_id.to_owned();
        let Some(backend) = self.backend.clone() else {
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.browse_save_collection_pending
            .insert(collection_id.clone());
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .set_collection_projects(&collection_id, &projects)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::BrowseCollectionSaved(
                collection_id,
                project_id,
                projects,
                result,
            ));
        });
    }

    pub(super) fn load_browse_project_follow_status(&mut self) {
        if self.browse_project_follow_status_loaded
            || self.browse_project_follow_status_loading
        {
            return;
        }
        let Some(project) = self.browse_selected.as_ref() else {
            return;
        };
        let project_id = project.id.clone();
        let Some(backend) = self.backend.clone() else {
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.browse_project_follow_status_loading = true;
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .project_is_followed(&project_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::BrowseFollowStatus(project_id, result));
        });
    }

    pub(super) fn toggle_browse_project_follow(&mut self) {
        if self.browse_project_follow_pending
            || self.browse_project_follow_status_loading
        {
            return;
        }
        let Some(project) = self.browse_selected.as_ref() else {
            return;
        };
        let project_id = project.id.clone();
        let followed = !self.browse_project_followed;
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            self.draw();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.browse_project_follow_pending = true;
        self.message = if followed {
            "Following project…".into()
        } else {
            "Unfollowing project…".into()
        };
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .set_project_followed(&project_id, followed)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::BrowseFollowChanged(
                project_id, followed, result,
            ));
        });
    }

    pub(super) fn open_browse_project(&mut self, project: BrowseProject) {
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let project_id = project.id.clone();
        self.browse_filter_menu_open = false;
        self.browse_project_followed = false;
        self.browse_project_follow_pending = false;
        self.browse_project_follow_status_loaded = false;
        self.browse_project_follow_status_loading = false;
        self.browse_save_menu_open = false;
        self.browse_save_collections_loaded = false;
        self.browse_save_collections_loading = false;
        self.browse_save_collections.clear();
        self.browse_save_collection_pending.clear();
        self.browse_save_scroll_pixels = 0.0;
        self.browse_collection_create_open = false;
        self.browse_collection_create_name.clear();
        self.browse_collection_creating = false;
        self.browse_selected = Some(project);
        self.browse_details = None;
        self.browse_detail_scroll_pixels = 0.0;
        self.browse_detail_scroll_target_pixels = 0.0;
        self.browse_gallery_index = 0;
        self.browse_gallery_viewer_open = false;
        self.browse_gallery_zoom = 1.0;
        self.browse_detail_tab = 0;
        self.browse_description_lines = None;
        self.message = "Loading project details and versions…".into();
        self.runtime.spawn(async move {
            let (project_result, versions_result) = tokio::join!(
                backend.project(&project_id, None),
                backend.project_versions(&project_id, None),
            );
            let result = async {
                let project = project_result
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "project was not found".to_owned())?;
                let versions = versions_result
                    .map_err(|error| error.to_string())?
                    .unwrap_or_default()
                    .into_iter()
                    .map(BrowseVersion::from)
                    .collect();
                let string_array = |key: &str| {
                    project
                        .extra
                        .get(key)
                        .and_then(serde_json::Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_owned)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                };
                let string_value = |key: &str| {
                    project
                        .extra
                        .get(key)
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                };
                let license = project.extra.get("license");
                let license_name = license
                    .and_then(|value| value.get("name"))
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| {
                        license
                            .and_then(|value| value.get("id"))
                            .and_then(serde_json::Value::as_str)
                    })
                    .map(str::to_owned)
                    .or_else(|| string_value("license_id"))
                    .unwrap_or_else(|| "Unknown".into());
                let license_url = license
                    .and_then(|value| value.get("url"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
                let links = [
                    ("Source code", "source_url"),
                    ("Wiki", "wiki_url"),
                    ("Issue tracker", "issues_url"),
                    ("Discord", "discord_url"),
                ]
                .into_iter()
                .filter_map(|(label, key)| {
                    string_value(key).map(|url| (label.to_owned(), url))
                })
                .collect();
                Ok(BrowseProjectDetails {
                    slug: project.slug.unwrap_or_else(|| project.id.clone()),
                    body: project
                        .extra
                        .get("body")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    categories: string_array("categories"),
                    additional_categories: string_array(
                        "additional_categories",
                    ),
                    game_versions: string_array("game_versions"),
                    loaders: string_array("loaders"),
                    client_side: string_value("client_side")
                        .unwrap_or_else(|| "unknown".into()),
                    server_side: string_value("server_side")
                        .unwrap_or_else(|| "unknown".into()),
                    license_name,
                    license_url,
                    published: string_value("published")
                        .unwrap_or_else(|| "Unknown".into()),
                    updated: string_value("updated")
                        .unwrap_or_else(|| "Unknown".into()),
                    links,
                    gallery: project
                        .extra
                        .get("gallery")
                        .and_then(serde_json::Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|item| {
                                    let title = item
                                        .get("title")
                                        .and_then(serde_json::Value::as_str)
                                        .map(str::to_owned);
                                    if title.as_deref()
                                        == Some("__mc_server_banner__")
                                    {
                                        return None;
                                    }
                                    Some(BrowseGalleryImage {
                                        url: item
                                            .get("url")?
                                            .as_str()?
                                            .to_owned(),
                                        raw_url: item
                                            .get("raw_url")
                                            .and_then(serde_json::Value::as_str)
                                            .map(str::to_owned),
                                        title,
                                        description: item
                                            .get("description")
                                            .and_then(serde_json::Value::as_str)
                                            .map(str::to_owned),
                                        created: item
                                            .get("created")
                                            .and_then(serde_json::Value::as_str)
                                            .map(str::to_owned),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    versions,
                })
            }
            .await;
            let _ =
                proxy.send_event(UiMessage::BrowseDetails(project_id, result));
        });
    }

    pub(super) fn install_browse_project(
        &mut self,
        project: BrowseProject,
        version_id: Option<String>,
    ) {
        if version_id.as_deref().is_some_and(|version| {
            self.browse_version_is_installed(&project.id, version)
        }) {
            return;
        }
        if self.browse_project_is_installing(&project)
            || (version_id.is_none()
                && self.browse_project_is_installed(&project))
        {
            return;
        }
        if project.project_type == "modpack" {
            self.install_browse_modpack(project, version_id);
            return;
        }
        if self.global_browse {
            if self.instances.is_empty() {
                self.message =
                    "Create an instance before installing content.".into();
            } else {
                self.open_browse_install_choice(project, version_id);
                self.message = "Choose an instance for this project.".into();
            }
            return;
        }
        let Some((instance_id, _)) = self
            .selected_instance
            .clone()
            .filter(|(id, _)| id != GLOBAL_DISCOVER_ID)
        else {
            return;
        };
        self.install_browse_project_for(project, version_id, instance_id);
    }

    pub(super) fn open_browse_install_choice(
        &mut self,
        project: BrowseProject,
        version_id: Option<String>,
    ) {
        self.browse_install_choice_generation =
            self.browse_install_choice_generation.wrapping_add(1);
        let generation = self.browse_install_choice_generation;
        let project_id = project.id.clone();
        let versions = self
            .browse_selected
            .as_ref()
            .filter(|selected| selected.id == project_id)
            .and(self.browse_details.as_ref())
            .map(|details| Ok(details.versions.clone()));
        let fetch_versions = versions.is_none();
        self.browse_install_choice = Some(BrowseInstallChoice {
            project,
            version_id,
            versions,
            generation,
            installed: Default::default(),
        });
        self.browse_install_scroll = 0.0;
        self.browse_install_scroll_target = 0.0;
        let instance_ids = self
            .instances
            .iter()
            .map(|(id, _, _)| id.clone())
            .collect::<Vec<_>>();
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            if let Some(choice) = &mut self.browse_install_choice {
                choice.versions =
                    Some(Err("Launcher backend is not ready".into()));
                for id in &instance_ids {
                    choice.installed.insert(
                        id.clone(),
                        Err("Launcher backend is not ready".into()),
                    );
                }
            }
            return;
        };
        self.runtime.spawn(async move {
            for id in instance_ids {
                let presence_backend = backend.clone();
                let presence_proxy = proxy.clone();
                let presence_project = project_id.clone();
                tokio::spawn(async move {
                    let installed = presence_backend
                        .installed_project_ids(&id)
                        .await
                        .map(|items| {
                            items
                                .iter()
                                .any(|project| project == &presence_project)
                        })
                        .map_err(|error| error.to_string());
                    let _ = presence_proxy.send_event(
                        UiMessage::BrowseInstallPresenceLoaded(
                            generation, id, installed,
                        ),
                    );
                });
            }
            if !fetch_versions {
                return;
            }
            let versions = backend
                .project_versions(&project_id, None)
                .await
                .map_err(|error| error.to_string())
                .map(|versions| {
                    versions
                        .unwrap_or_default()
                        .into_iter()
                        .map(BrowseVersion::from)
                        .collect()
                });
            let _ = proxy.send_event(UiMessage::BrowseInstallVersionsLoaded(
                generation, versions,
            ));
        });
    }

    pub(super) fn install_browse_modpack(
        &mut self,
        project: BrowseProject,
        version_id: Option<String>,
    ) {
        let Some(version_id) = version_id else {
            self.message = "Choose a modpack version before installing.".into();
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let title = project.title;
        let project_id = project.id;
        if !self.browse_installing_modpacks.insert(project_id.clone()) {
            return;
        }
        self.message = format!("Installing modpack {title}…");
        self.runtime.spawn(async move {
            let result = async {
                let job = backend
                    .install_modrinth_pack_version(
                        project_id.clone(),
                        version_id,
                        title.clone(),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                let job_id = uuid::Uuid::parse_str(&job.job_id)
                    .map_err(|error| error.to_string())?;
                loop {
                    let snapshot = backend
                        .get_install_job(job_id)
                        .await
                        .map_err(|error| error.to_string())?;
                    if snapshot.status.is_finished() {
                        if matches!(
                            snapshot.status,
                            theseus::install::InstallJobStatus::Succeeded
                        ) {
                            return Ok(title);
                        }
                        return Err(snapshot
                            .error
                            .map(|error| error.message)
                            .unwrap_or_else(|| {
                                format!("Install {}.", snapshot.status.as_str())
                            }));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(300))
                        .await;
                }
            }
            .await;
            let _ = proxy.send_event(UiMessage::BrowseModpackInstallDone(
                project_id, result,
            ));
        });
    }

    pub(super) fn install_browse_project_for(
        &mut self,
        project: BrowseProject,
        version_id: Option<String>,
        instance_id: String,
    ) {
        if let Some(id) = version_id.as_deref() {
            let metadata = self
                .browse_details
                .as_ref()
                .filter(|_| {
                    self.browse_selected
                        .as_ref()
                        .is_some_and(|selected| selected.id == project.id)
                })
                .and_then(|details| {
                    details.versions.iter().find(|version| version.id == id)
                });
            let Some(version) = metadata else {
                self.show_browse_install_notice("Could not verify the selected version. Reopen the project and try again.".into(), true);
                return;
            };
            let card = self.instance_cards.get(&instance_id);
            if !card.is_some_and(|card| {
                browse_version_matches_target(
                    &project.project_type,
                    version,
                    &card.game_version,
                    &card.loader,
                )
            }) {
                let target_label = card.map_or_else(
                    || "the target instance".into(),
                    |card| format!("{} {}", card.loader, card.game_version),
                );
                self.browse_install_override = Some(BrowseInstallOverride {
                    project,
                    version_id: id.to_owned(),
                    version_label: version.version_number.clone(),
                    instance_id,
                    target_label,
                });
                return;
            }
        }
        self.start_browse_project_install(
            project,
            version_id,
            instance_id,
            false,
        );
    }

    pub(super) fn confirm_browse_install_override(&mut self) {
        if let Some(choice) = self.browse_install_override.take() {
            self.start_browse_project_install(
                choice.project,
                Some(choice.version_id),
                choice.instance_id,
                true,
            );
        }
    }

    pub(super) fn start_browse_project_install(
        &mut self,
        project: BrowseProject,
        version_id: Option<String>,
        instance_id: String,
        standalone: bool,
    ) {
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let project_id = project.id.clone();
        if !self
            .browse_installing_projects
            .entry(instance_id.clone())
            .or_default()
            .insert(project_id.clone())
        {
            return;
        }
        self.message = format!("Resolving dependencies for {}…", project.title);
        self.runtime.spawn(async move {
            let result = async {
                if standalone {
                    let version = version_id.as_deref().ok_or("No version selected")?;
                    theseus::instance::add_project_from_version(
                        &instance_id, version, theseus::DownloadReason::Standalone, None,
                    ).await.map_err(|error| error.to_string())?;
                    return Ok(0);
                }
                let content_type = project
                    .project_type
                    .parse::<theseus::data::ContentType>()
                    .map_err(|error| error.to_string())?;
                let installation = backend.install_project_with_dependencies(
                    &instance_id,
                    theseus::instance::InstallProjectWithDependenciesRequest {
                        project_id: project.id,
                        version_id,
                        content_type,
                        selected: theseus::data::ResolutionPreferences::default(),
                    },
                ).await.map_err(|error| error.to_string())?;
                let dependencies = installation.plan.dependencies.len();
                installation
                    .task
                    .await
                    .map_err(|error| error.to_string())?
                    .map_err(|error| format!("{error:#}"))?;
                Ok(dependencies)
            }
            .await;
            let _ = proxy.send_event(UiMessage::BrowseInstallDone(
                instance_id,
                project_id,
                project.title,
                result,
            ));
        });
    }
}
