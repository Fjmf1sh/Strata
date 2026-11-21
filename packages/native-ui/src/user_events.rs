use super::*;

impl Launcher {
    pub(super) fn handle_user_event(&mut self, event: UiMessage) {
        match event {
            UiMessage::ClearMessage(generation) => {
                if self.message_generation == generation {
                    self.message.clear();
                    self.last_timed_message.clear();
                    self.draw();
                }
            }
            UiMessage::ClearBrowseInstallNotice(expires) => {
                if self
                    .browse_install_notice
                    .as_ref()
                    .is_some_and(|notice| notice.expires == expires)
                {
                    self.browse_install_notice = None;
                    self.draw();
                }
            }
            UiMessage::ExistingDataChecked(Ok(Some(plan))) => {
                self.migration_busy = false;
                self.migration_plan = Some(plan);
                self.migration_prompt_open = true;
                self.message = "An existing Modrinth library was found.".into();
                self.draw();
            }
            UiMessage::ExistingDataChecked(Ok(None)) => {
                self.migration_busy = false;
                if self.ready {
                    self.migration_settings_entry = false;
                    self.message =
                        "No existing Modrinth library was found.".into();
                    self.draw();
                } else {
                    self.start_backend();
                }
            }
            UiMessage::ExistingDataChecked(Err(error)) => {
                self.migration_busy = false;
                self.message =
                    format!("Could not inspect the existing library: {error}");
                if self.ready {
                    self.migration_settings_entry = false;
                    self.draw();
                } else {
                    self.start_backend();
                }
            }
            UiMessage::ExistingDataCopied(Ok(summary)) => {
                self.migration_busy = false;
                self.migration_prompt_open = false;
                self.migration_settings_entry = false;
                self.migration_plan = None;
                self.message = format!(
                    "Copied {} instances from the existing library.",
                    summary.instance_count
                );
                if self.ready {
                    self.refresh_instances();
                    self.refresh_groups();
                    self.draw();
                } else {
                    self.start_backend();
                }
            }
            UiMessage::ExistingDataCopied(Err(error)) => {
                self.migration_busy = false;
                self.message = format!("Library copy failed: {error}");
                self.draw();
            }
            UiMessage::FreshStartChoiceSaved(result) => {
                self.migration_busy = false;
                self.migration_saving_fresh_choice = false;
                self.migration_prompt_open = false;
                self.migration_settings_entry = false;
                self.migration_plan = None;
                self.start_backend();
                if let Err(error) = result {
                    self.message = format!(
                        "Started a separate library, but could not save the choice: {error}"
                    );
                    self.draw();
                }
            }
            UiMessage::BackendStarted(Ok((backend, events))) => {
                self.attach_backend(backend, events);
            }
            UiMessage::BackendStarted(Err(error)) => {
                self.message = format!("Backend startup failed: {error}");
                self.sidebar_friends_loaded = true;
                self.sidebar_friends_error =
                    Some("Launcher services could not start.".into());
                self.draw();
            }
            UiMessage::BackendReady => {
                self.ready = true;
                self.message = "Launcher services are ready.".into();
                self.load_settings();
                self.load_quick_instance_count();
                self.load_native_color_theme();
                self.load_preferred_dark_theme();
                self.load_theme_sync_setting();
                self.refresh_instances();
                self.refresh_groups();
                self.refresh_running_processes();
                self.load_accounts();
                self.load_sidebar_friends();
                self.load_onboarding_checklist();
                self.load_recent_worlds();
                if self.is_browse_results_page()
                    && self.browse_error.as_deref()
                        == Some("Launcher backend is not ready.")
                {
                    self.search_projects();
                }
                if let Some(path) = self.pack_on_start.take() {
                    self.import_pack(path);
                }
            }
            UiMessage::BackendFailed(error) => {
                self.message = format!("Backend startup failed: {error}");
                self.sidebar_friends_loaded = true;
                self.sidebar_friends_error =
                    Some("Launcher services could not start.".into());
                self.draw();
            }
            UiMessage::OnboardingChecklistLoaded(Ok(checklist)) => {
                self.onboarding_checklist = Some(checklist);
                self.draw();
            }
            UiMessage::OnboardingChecklistLoaded(Err(error)) => {
                self.message =
                    format!("Could not load getting started steps: {error}");
                self.draw();
            }
            UiMessage::OnboardingSkipped(Ok(())) => {
                self.message = "You can create an instance any time.".into();
                self.load_onboarding_checklist();
            }
            UiMessage::OnboardingSkipped(Err(error)) => {
                self.message = format!("Could not save your choice: {error}");
                self.draw();
            }
            UiMessage::RecentWorldsLoaded(Ok(worlds)) => {
                self.recent_worlds = worlds;
                self.draw();
            }
            UiMessage::RecentWorldsLoaded(Err(error)) => {
                self.message = format!("Could not load recent worlds: {error}");
                self.draw();
            }
            UiMessage::HostingServersLoaded(Ok(servers)) => {
                self.hosting_servers = servers;
                self.hosting_servers_loading = false;
                self.hosting_servers_refreshing = false;
                self.hosting_servers_error = None;
                self.draw();
            }
            UiMessage::HostingServersLoaded(Err(error)) => {
                self.hosting_servers_loading = false;
                self.hosting_servers_refreshing = false;
                if self.hosting_servers.is_empty() {
                    self.hosting_servers_error = Some(error);
                } else {
                    self.message =
                        format!("Could not refresh servers: {error}");
                }
                self.draw();
            }
            UiMessage::HostingServerLoaded(server_id, Ok(details))
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_server_details_loading = false;
                self.hosting_server_details_error = None;
                self.hosting_server_details = Some(details);
                self.draw();
            }
            UiMessage::HostingServerLoaded(server_id, Err(error))
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_server_details_loading = false;
                self.hosting_server_details_error = Some(error);
                self.draw();
            }
            UiMessage::HostingServerLoaded(_, _) => {}
            UiMessage::HostingBackupsLoaded(server_id, Ok(backups))
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_backups_loading = false;
                self.hosting_backups_error = None;
                if backups.deleted_backups.is_empty() {
                    self.hosting_backup_show_deleted = false;
                }
                self.hosting_backups = Some(backups);
                self.draw();
            }
            UiMessage::HostingBackupsLoaded(server_id, Err(error))
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_backups_loading = false;
                self.hosting_backups_error = Some(error);
                self.draw();
            }
            UiMessage::HostingBackupsLoaded(_, _) => {}
            UiMessage::HostingBackupCreated(server_id, result)
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_backup_creating = false;
                match result {
                    Ok(()) => {
                        self.hosting_backup_create_open = false;
                        self.hosting_backup_create_name.clear();
                        self.input_focus = 0;
                        self.hosting_backups = None;
                        self.hosting_backups_loading = false;
                        self.message = "Backup queued.".into();
                        self.load_hosting_backups();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not create backup: {error}");
                        self.draw();
                    }
                }
            }
            UiMessage::HostingBackupCreated(_, _) => {}
            UiMessage::HostingBackupOptionsFinished(
                server_id,
                acknowledged_rotation,
                Ok(update),
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_backup_options_saving = false;
                if update.applied {
                    self.hosting_backup_options_open = false;
                    self.hosting_backup_rotation_confirmation = false;
                    self.hosting_backups = None;
                    self.hosting_backups_loading = false;
                    self.message = "Backup settings saved.".into();
                    self.load_hosting_backups();
                } else if acknowledged_rotation {
                    self.hosting_backup_rotation_confirmation = false;
                    self.message = "The server did not apply the confirmed backup rotation.".into();
                    self.draw();
                } else {
                    self.hosting_backup_rotation_count =
                        update.backup_ids.len();
                    self.hosting_backup_rotation_confirmation = true;
                    self.draw();
                }
            }
            UiMessage::HostingBackupOptionsFinished(
                server_id,
                _,
                Err(error),
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_backup_options_saving = false;
                self.hosting_backup_rotation_confirmation = false;
                self.message =
                    format!("Could not save backup settings: {error}");
                self.draw();
            }
            UiMessage::HostingBackupOptionsFinished(_, _, _) => {}
            UiMessage::HostingBackupMutationFinished(
                server_id,
                success_message,
                result,
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_backup_mutating = false;
                match result {
                    Ok(()) => {
                        self.hosting_backup_prompt = None;
                        self.hosting_backup_create_name.clear();
                        self.hosting_backup_selected.clear();
                        self.input_focus = 0;
                        self.hosting_backups = None;
                        self.hosting_backups_loading = false;
                        self.message = success_message;
                        self.load_hosting_backups();
                    }
                    Err(error) => {
                        self.message =
                            format!("Hosting backup action failed: {error}");
                        self.draw();
                    }
                }
            }
            UiMessage::HostingBackupMutationFinished(_, _, _) => {}
            UiMessage::HostingBackupBulkUndeleteFinished(
                server_id,
                recovered,
                failures,
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_backup_mutating = false;
                self.hosting_backup_selected
                    .retain(|id| !recovered.contains(id));
                if !recovered.is_empty() {
                    self.hosting_backups = None;
                    self.hosting_backups_loading = false;
                    self.message = format!(
                        "Recovered {} backup(s).{}",
                        recovered.len(),
                        if failures.is_empty() {
                            String::new()
                        } else {
                            format!(
                                " {} could not be recovered.",
                                failures.len()
                            )
                        }
                    );
                    self.load_hosting_backups();
                } else if let Some((_, error)) = failures.first() {
                    self.message =
                        format!("Could not recover selected backups: {error}");
                    self.draw();
                }
            }
            UiMessage::HostingBackupBulkUndeleteFinished(_, _, _) => {}
            UiMessage::HostingBackupDownloadFinished(server_id, result)
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_backup_downloading = false;
                match result {
                    Ok(Some(path)) => {
                        self.message = format!(
                            "Hosting backup saved to {}.",
                            path.display()
                        );
                        self.draw();
                    }
                    Ok(None) => self.draw(),
                    Err(error) => {
                        self.message = format!(
                            "Could not download Hosting backup: {error}"
                        );
                        self.draw();
                    }
                }
            }
            UiMessage::HostingBackupDownloadFinished(_, _) => {}
            UiMessage::HostingAccessLoaded(server_id, Ok(users))
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_access_users = users;
                self.hosting_access_loading = false;
                self.hosting_access_loaded = true;
                self.hosting_access_error = None;
                self.draw();
            }
            UiMessage::HostingAccessLoaded(server_id, Err(error))
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_access_loading = false;
                self.hosting_access_loaded = false;
                self.hosting_access_error = Some(error);
                self.draw();
            }
            UiMessage::HostingAccessLoaded(_, _) => {}
            UiMessage::HostingAccessSearchLoaded(server_id, query, result)
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str())
                    && self.hosting_access_search_query == query =>
            {
                self.hosting_access_search_loading = false;
                match result {
                    Ok(users) => {
                        self.hosting_access_search_results = users;
                        self.hosting_access_search_complete = true;
                        self.hosting_access_search_scroll_pixels = 0.0;
                        self.hosting_access_search_scroll_target_pixels = 0.0;
                        self.hosting_access_invite_error = None;
                    }
                    Err(error) => {
                        self.hosting_access_search_complete = false;
                        self.hosting_access_invite_error = Some(error);
                    }
                }
                self.draw();
            }
            UiMessage::HostingAccessSearchLoaded(_, _, _) => {}
            UiMessage::HostingAccessInviteFinished(
                server_id,
                username,
                result,
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_access_loading = false;
                match result {
                    Ok(()) => {
                        self.hosting_access_invite_open = false;
                        self.hosting_access_search_query.clear();
                        self.hosting_access_search_results.clear();
                        self.hosting_access_search_complete = false;
                        self.hosting_access_loaded = false;
                        self.hosting_access_users.clear();
                        self.message =
                            format!("Invited {username} to the server.");
                        self.load_hosting_server_access();
                    }
                    Err(error) => {
                        self.hosting_access_invite_error = Some(error);
                    }
                }
                self.draw();
            }
            UiMessage::HostingAccessInviteFinished(_, _, _) => {}
            UiMessage::HostingAccessReinviteFinished(
                server_id,
                username,
                result,
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_access_loading = false;
                match result {
                    Ok(response) if response.sent => {
                        self.message =
                            format!("Resent invitation to {username}.");
                        self.hosting_access_loaded = false;
                        self.load_hosting_server_access();
                    }
                    Ok(response) => {
                        let cooldown = response.cooldown_seconds.unwrap_or(0);
                        self.message = format!(
                            "Invitation for {username} cannot be resent yet ({} seconds remaining).",
                            cooldown
                        );
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not resend invitation: {error}");
                    }
                }
                self.draw();
            }
            UiMessage::HostingAccessReinviteFinished(_, _, _) => {}
            UiMessage::HostingAccessRoleFinished(
                server_id,
                _user_id,
                username,
                role,
                result,
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_access_loading = false;
                match result {
                    Ok(()) => {
                        self.message =
                            format!("Updated {username} to {:?} access.", role);
                        self.hosting_access_loaded = false;
                        self.load_hosting_server_access();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not change access role: {error}");
                        self.hosting_access_loaded = false;
                        self.load_hosting_server_access();
                    }
                }
                self.draw();
            }
            UiMessage::HostingAccessRoleFinished(_, _, _, _, _) => {}
            UiMessage::HostingAccessRemoveFinished(
                server_id,
                user_id,
                username,
                result,
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_access_loading = false;
                self.hosting_access_remove_confirmation = None;
                match result {
                    Ok(()) => {
                        self.message =
                            format!("Removed {username} from server access.");
                        self.hosting_access_users.retain(|entry| {
                            !entry.user.id.eq_ignore_ascii_case(&user_id)
                        });
                        self.hosting_access_loaded = false;
                        self.load_hosting_server_access();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not remove server access: {error}");
                        self.hosting_access_loaded = false;
                        self.load_hosting_server_access();
                    }
                }
                self.draw();
            }
            UiMessage::HostingAccessRemoveFinished(_, _, _, _) => {}
            UiMessage::HostingServerConnected(
                server_id,
                Ok((connection, mut events)),
            ) if self.hosting_server_id.as_deref()
                == Some(server_id.as_str()) =>
            {
                self.hosting_connection = Some(connection);
                self.hosting_socket_connecting = false;
                self.hosting_socket_connected = true;
                self.hosting_socket_error = None;
                if let Some(proxy) = self.proxy.clone() {
                    self.runtime.spawn(async move {
                        while let Some(event) = events.recv().await {
                            if proxy
                                .send_event(UiMessage::HostingServerEvent(
                                    server_id.clone(),
                                    event,
                                ))
                                .is_err()
                            {
                                break;
                            }
                        }
                    });
                }
                self.draw();
            }
            UiMessage::HostingServerConnected(server_id, Err(error))
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_socket_connecting = false;
                self.hosting_socket_connected = false;
                self.hosting_socket_error = Some(error);
                self.draw();
            }
            UiMessage::HostingServerConnected(_, _) => {}
            UiMessage::HostingServerEvent(server_id, event)
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                match event {
                    native_backend::HostingServerEvent::Log(line) => {
                        self.hosting_console_lines.extend(line.lines().map(str::to_owned));
                        while self.hosting_console_lines.len() > 500 {
                            self.hosting_console_lines.pop_front();
                        }
                    }
                    native_backend::HostingServerEvent::PowerState { state, .. } => self.hosting_power_state = Some(state),
                    native_backend::HostingServerEvent::State { power_variant, .. } => self.hosting_power_state = Some(power_variant),
                    native_backend::HostingServerEvent::Stats { cpu_percent, ram_usage_bytes, ram_total_bytes, storage_usage_bytes, storage_total_bytes } => self.hosting_live_stats = Some((cpu_percent, ram_usage_bytes, ram_total_bytes, storage_usage_bytes, storage_total_bytes)),
                    native_backend::HostingServerEvent::Uptime(_) => {}
                    native_backend::HostingServerEvent::AuthenticationFailed => {
                        self.hosting_socket_connected = false;
                        self.hosting_connection = None;
                        self.hosting_socket_error = Some("Hosting console authentication expired.".into());
                    }
                    native_backend::HostingServerEvent::Disconnected => {
                        self.hosting_socket_connected = false;
                        self.hosting_connection = None;
                    }
                }
                self.draw();
            }
            UiMessage::HostingServerEvent(_, _) => {}
            UiMessage::HostingPowerActionFinished(server_id, result)
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_power_pending = false;
                self.hosting_power_error = result.err();
                self.draw();
            }
            UiMessage::HostingPowerActionFinished(_, _) => {}
            UiMessage::HostingAddressCopyFeedbackClear(server_id)
                if self.hosting_server_id.as_deref()
                    == Some(server_id.as_str()) =>
            {
                self.hosting_address_copied = false;
                self.draw();
            }
            UiMessage::HostingAddressCopyFeedbackClear(_) => {}
            UiMessage::RefreshRecentWorlds => self.load_recent_worlds(),
            UiMessage::Instances(Ok(instances)) => {
                if self.is_home_library() {
                    self.message = if instances.is_empty() {
                        "No instances yet. Create or import one to get started."
                            .into()
                    } else if instances.len() == 1 {
                        "1 instance in your library".into()
                    } else {
                        format!("{} instances in your library", instances.len())
                    };
                }
                self.instance_cards = instances
                    .iter()
                    .map(
                        |(
                            id,
                            _,
                            _,
                            icon_path,
                            game_version,
                            loader,
                            _,
                            instance_type,
                            install_state,
                            content_locked,
                            launch_blocked,
                        )| {
                            (
                                id.clone(),
                                NativeLibraryCard {
                                    icon_path: icon_path.clone(),
                                    game_version: game_version.clone(),
                                    loader: loader.clone(),
                                    instance_type: instance_type.clone(),
                                    install_state: install_state.clone(),
                                    content_locked: *content_locked,
                                    launch_blocked: *launch_blocked,
                                },
                            )
                        },
                    )
                    .collect();
                self.library_game_version_filter_offset =
                    self.library_game_version_filter_offset.min(
                        self.library_game_version_options()
                            .len()
                            .saturating_sub(3),
                    );
                self.instance_metrics = instances
                    .iter()
                    .map(|(id, _, _, _, _, _, metrics, _, _, _, _)| {
                        (id.clone(), *metrics)
                    })
                    .collect();
                self.instances = instances
                    .into_iter()
                    .map(|(id, name, groups, _, _, _, _, _, _, _, _)| {
                        (id, name, groups)
                    })
                    .collect();
                let current_instance_ids: HashSet<String> = self
                    .instances
                    .iter()
                    .map(|(instance_id, _, _)| instance_id.clone())
                    .collect();
                self.selected_library_instances.retain(|instance_id| {
                    current_instance_ids.contains(instance_id)
                });
                let width = self
                    .window
                    .as_ref()
                    .map_or(1100, |window| window.inner_size().width as usize);
                let columns = self.library_grid_layout(width).columns.max(1);
                let maximum =
                    self.library_grid_rows(columns).len().saturating_sub(1);
                self.list_offset = self.list_offset.min(maximum);
            }
            UiMessage::Instances(Err(error)) => {
                self.message = format!("Could not load instances: {error}")
            }
            UiMessage::InstalledModpackIds(ids) => {
                let changed = self.installed_modpack_ids != ids;
                self.installed_modpack_ids = ids;
                if changed
                    && self.settings.as_ref().is_some_and(|settings| {
                        settings.hide_installed_modpacks
                    })
                    && self.browse_filter == 4
                {
                    self.browse_hidden_ids_dirty = true;
                    self.browse_results.retain(|project| {
                        !self.installed_modpack_ids.contains(&project.id)
                    });
                    if self.is_browse_results_page() {
                        self.search_projects();
                    }
                }
            }
            UiMessage::InstanceIconLoaded(instance_id, result) => {
                self.instance_icons_pending.remove(&instance_id);
                match result {
                    Ok(icon) => {
                        self.instance_icon_failures.remove(&instance_id);
                        self.instance_icons.insert(instance_id.clone(), icon);
                        cap_image_cache(
                            &mut self.instance_icons,
                            &instance_id,
                            INSTANCE_ICON_CACHE_BUDGET,
                        );
                    }
                    Err(_) => {
                        self.instance_icon_failures.insert(instance_id);
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::BrowseIconLoaded(project_id, result) => {
                self.browse_icons_pending.remove(&project_id);
                let is_visible_gallery_image =
                    self.browse_selected.as_ref().is_some_and(|project| {
                        project_id
                            .starts_with(&format!("gallery:{}:", project.id))
                            || project_id.starts_with(&format!(
                                "gallery-full:{}:",
                                project.id
                            ))
                    });
                let is_visible_description_image =
                    self.browse_selected.as_ref().is_some_and(|project| {
                        project_id.starts_with(&format!(
                            "description-image:{}:",
                            project.id
                        ))
                    });
                let is_visible_news_image =
                    self.news_articles.iter().any(|article| {
                        project_id == format!("news:{}", article.link)
                    });
                let is_visible_friend_avatar =
                    self.sidebar_friends.iter().take(2).any(|friend| {
                        project_id == format!("friend:{}", friend.user_id)
                    });
                let is_visible_world_icon =
                    self.jump_in_items().iter().any(|item| match item {
                        NativeJumpInItem::World(recent_world) => {
                            project_id
                                == format!(
                                    "world:{}:{}",
                                    recent_world.instance_id,
                                    recent_world.world.world_id()
                                )
                        }
                        NativeJumpInItem::Instance { .. } => false,
                    });
                if is_visible_gallery_image
                    || is_visible_description_image
                    || is_visible_news_image
                    || is_visible_friend_avatar
                    || is_visible_world_icon
                    || self
                        .browse_results
                        .iter()
                        .any(|project| project.id == project_id)
                {
                    match result {
                        Ok(icon) => {
                            self.browse_icon_failures.remove(&project_id);
                            self.browse_icons.insert(project_id.clone(), icon);
                            cap_image_cache(
                                &mut self.browse_icons,
                                &project_id,
                                BROWSE_IMAGE_CACHE_BUDGET,
                            );
                        }
                        Err(_) => {
                            self.browse_icon_failures.insert(project_id);
                        }
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::NewsArticlesLoaded(Ok(articles)) => {
                self.news_error = None;
                self.news_loaded = true;
                self.news_articles = articles;
                self.sidebar_scroll_pixels = 0.0;
                self.sidebar_scroll_target_pixels = 0.0;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::NewsArticlesLoaded(Err(error)) => {
                self.news_error = Some(error);
                self.news_loaded = true;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::SidebarFriendsLoaded(generation, result) => {
                if generation == self.sidebar_friends_request_generation {
                    match result {
                        Ok(friends) => {
                            self.sidebar_friends_error = None;
                            self.sidebar_friends_loaded = true;
                            self.sidebar_friends = friends;
                            self.sidebar_friends.sort_by(|left, right| {
                                right.last_updated.cmp(&left.last_updated)
                            });
                            if self.friend_incoming_open {
                                let max_scroll =
                                    friend_incoming_modal_max_scroll(
                                        sidebar_incoming_friend_indices(
                                            &self.sidebar_friends,
                                        )
                                        .len(),
                                        self.window.as_ref().map_or(
                                            720,
                                            |window| {
                                                window.inner_size().height
                                                    as usize
                                            },
                                        ),
                                    );
                                self.friend_incoming_scroll_target_pixels =
                                    self.friend_incoming_scroll_target_pixels
                                        .min(max_scroll);
                                self.friend_incoming_scroll_pixels = self
                                    .friend_incoming_scroll_pixels
                                    .min(max_scroll);
                            }
                        }
                        Err(error) => {
                            self.sidebar_friends_error = Some(error);
                            self.sidebar_friends_loaded = true;
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            UiMessage::RefreshSidebarFriends => self.load_sidebar_friends(),
            UiMessage::FriendSearchLoaded(Ok(results)) => {
                self.friend_search_loading = false;
                self.friend_search_error = None;
                self.friend_search_results = results;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::FriendSearchLoaded(Err(error)) => {
                self.friend_search_loading = false;
                self.friend_search_error = Some(error);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::UserProfileLoaded(
                user_id,
                Ok((profile, projects, organizations, collections)),
            ) => {
                if self.user_profile_id.as_deref() == Some(user_id.as_str()) {
                    self.user_profile = Some(profile);
                    self.user_profile_projects =
                        projects.as_array().cloned().unwrap_or_default();
                    self.user_profile_filtered_project_indices =
                        filtered_profile_project_indices(
                            &self.user_profile_projects,
                            self.user_profile_type_filter.as_deref(),
                        );
                    self.user_profile_organizations =
                        organizations.as_array().cloned().unwrap_or_default();
                    self.user_profile_collections =
                        collections.as_array().cloned().unwrap_or_default();
                    self.user_profile_scroll_pixels = 0.0;
                    self.user_profile_scroll_target_pixels = 0.0;
                    self.user_profile_loading = false;
                    self.user_profile_error = None;
                    self.draw();
                }
            }
            UiMessage::UserProfileLoaded(user_id, Err(error)) => {
                if self.user_profile_id.as_deref() == Some(user_id.as_str()) {
                    self.user_profile_loading = false;
                    self.user_profile_error = Some(error);
                    self.draw();
                }
            }
            UiMessage::FriendActionFinished(user_id, is_accepting, result) => {
                self.friend_request_pending.remove(&user_id);
                match result {
                    Ok(()) => {
                        self.message = if is_accepting {
                            "Friend request accepted.".into()
                        } else {
                            "Friend request sent.".into()
                        };
                        self.friend_search_results
                            .retain(|user| user.id != user_id);
                        self.load_sidebar_friends();
                    }
                    Err(error) => {
                        self.friend_search_error = Some(error);
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::FriendRemoveFinished(user_id, kind, result) => {
                self.friend_remove_pending.remove(&user_id);
                match result {
                    Ok(()) => {
                        self.friend_remove_confirmation = None;
                        self.friend_remove_error = None;
                        self.message = match kind {
                            FriendRemovalKind::Friend => {
                                "Friend removed.".into()
                            }
                            FriendRemovalKind::IncomingRequest => {
                                "Friend request ignored.".into()
                            }
                            FriendRemovalKind::SentRequest => {
                                "Friend request canceled.".into()
                            }
                        };
                        self.load_sidebar_friends();
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    Err(error) => {
                        self.friend_remove_error = Some(error);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
            }
            UiMessage::BrowseFilterOptionsLoaded(result) => {
                self.browse_filter_options_loading = false;
                match result {
                    Ok(options) => {
                        self.browse_filter_options = Some(options);
                    }
                    Err(error) => {
                        if !self.browse_filter_sidebar_visible() {
                            self.message = format!(
                                "Could not load browse filters: {error}"
                            );
                        }
                        self.browse_filter_options_error = Some(error);
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::Groups(Ok(groups)) => self.groups = groups,
            UiMessage::Groups(Err(error)) => {
                self.message = format!("Could not load groups: {error}")
            }
            UiMessage::HiddenInstancesLoaded(
                generation,
                load_generation,
                result,
            ) if generation == self.hidden_instance_generation
                && load_generation == self.hidden_instance_load_generation =>
            {
                match result {
                    Ok(ids) => {
                        self.hidden_instance_ids = ids.into_iter().collect();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not load hidden instances: {error}");
                    }
                }
                self.draw();
            }
            UiMessage::HiddenInstancesLoaded(_, _, _) => {}
            UiMessage::InstanceVisibilityChanged(
                instance_id,
                hidden,
                generation,
                result,
            ) => {
                let pending_matches =
                    self.hidden_instance_save_pending.get(&instance_id)
                        == Some(&generation);
                if pending_matches {
                    self.hidden_instance_save_pending.remove(&instance_id);
                }
                if generation == self.hidden_instance_generation
                    && pending_matches
                {
                    match result {
                        Ok(()) => {
                            if hidden {
                                self.hidden_instance_ids.insert(instance_id);
                                self.message =
                                    "Instance hidden from your library.".into();
                            } else {
                                self.hidden_instance_ids.remove(&instance_id);
                                self.message =
                                    "Instance shown in your library.".into();
                            }
                        }
                        Err(error) => {
                            self.message = format!(
                                "Could not change instance visibility: {error}"
                            );
                        }
                    }
                    self.draw();
                }
            }
            UiMessage::GroupCreated(Ok(group)) => {
                self.selected_group = Some(group.id.clone());
                self.group_input.clear();
                let new_group_id = group.id.clone();
                self.groups.push(group);
                self.list_offset = 0;
                self.message = "Group created.".into();
                if self.create_group_from_selection {
                    self.create_group_from_selection = false;
                    self.update_selected_group_membership(new_group_id, false);
                }
            }
            UiMessage::GroupCreated(Err(error)) => {
                self.create_group_from_selection = false;
                self.message = format!("Could not create group: {error}")
            }
            UiMessage::GroupRenamed(Ok(group)) => {
                if let Some(current) = self
                    .groups
                    .iter_mut()
                    .find(|current| current.id == group.id)
                {
                    current.name = group.name;
                }
                self.group_input.clear();
                self.message = "Group renamed.".into();
            }
            UiMessage::GroupRenamed(Err(error)) => {
                self.message = format!("Could not rename group: {error}")
            }
            UiMessage::GroupDeleted(group_id, result) => match result {
                Ok(()) => {
                    self.groups.retain(|group| group.id != group_id);
                    if self.selected_group.as_deref() == Some(group_id.as_str())
                    {
                        self.selected_group = None;
                    }
                    self.membership_mode = false;
                    self.refresh_groups();
                    self.refresh_instances();
                    self.message = "Group deleted.".into();
                }
                Err(error) => {
                    self.message = format!("Could not delete group: {error}")
                }
            },
            UiMessage::GroupOrderChanged(Ok(groups)) => {
                self.groups = groups;
                self.message = "Group order updated.".into();
            }
            UiMessage::GroupOrderChanged(Err(error)) => {
                self.message = format!("Could not reorder groups: {error}")
            }
            UiMessage::MembershipAction(instance_id, group_ids, result) => {
                match result {
                    Ok(()) => {
                        if let Some((_, _, current_groups)) = self
                            .instances
                            .iter_mut()
                            .find(|(id, _, _)| *id == instance_id)
                        {
                            *current_groups = group_ids;
                        }
                        self.message = "Group membership updated.".into();
                    }
                    Err(error) => {
                        self.message = format!(
                            "Could not update group membership: {error}"
                        )
                    }
                }
            }
            UiMessage::BatchMembershipAction(updates, result) => match result {
                Ok(()) => {
                    for update in updates {
                        if let Some((_, _, group_ids)) = self
                            .instances
                            .iter_mut()
                            .find(|(id, _, _)| id == &update.instance_id)
                        {
                            *group_ids = update.group_ids;
                        }
                    }
                    self.selected_library_instances.clear();
                    self.library_selection_anchor = None;
                    self.message =
                        "Updated selected instances in this group.".into();
                }
                Err(error) => {
                    self.message =
                        format!("Could not update selected instances: {error}");
                }
            },
            UiMessage::InstanceContent(instance_id, Ok(contents)) => {
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    let icon_requests: Vec<(String, String)> = contents
                        .iter()
                        .filter_map(|item| item.project.as_ref())
                        .filter_map(|project| {
                            project
                                .icon_url
                                .as_ref()
                                .map(|url| (project.id.clone(), url.clone()))
                        })
                        .collect();
                    self.contents = contents;
                    self.rebuild_filtered_content_cache();
                    for (project_id, icon_url) in icon_requests {
                        self.request_browse_icon(&project_id, &icon_url);
                    }
                    self.content_scroll_pixels = 0.0;
                    self.content_scroll_target_pixels = 0.0;
                    self.message =
                        format!("{} content items", self.contents.len());
                }
            }
            UiMessage::InstanceContent(instance_id, Err(error)) => {
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    self.message =
                        format!("Could not load instance content: {error}");
                }
            }
            UiMessage::WorldsLoaded(instance_id, result) => {
                if !self.global_browse
                    && self.instance_tab == 7
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                {
                    match result {
                        Ok(worlds) => {
                            self.world_offset = self
                                .world_offset
                                .min(worlds.len().saturating_sub(1));
                            self.message =
                                format!("{} worlds and servers", worlds.len());
                            self.worlds = worlds;
                            self.server_statuses.clear();
                            self.server_motds.clear();
                            for world in &self.worlds {
                                if let worlds::WorldDetails::Server {
                                    address,
                                    ..
                                } = &world.details
                                {
                                    self.load_server_status(address.clone());
                                }
                            }
                        }
                        Err(error) => {
                            self.worlds.clear();
                            self.message =
                                format!("Could not load worlds: {error}");
                        }
                    }
                }
            }
            UiMessage::ServerPingLoaded(address, result) => {
                self.jump_in_server_pings_pending.remove(&address);
                let (summary, motd) = match result {
                    Ok(status) => {
                        let motd = server_motd_text(&status);
                        let players = status.players.map(|p| {
                            format!("{} / {} players", p.online, p.max)
                        });
                        let version = status.version.map(|v| v.name);
                        (
                            [players, version]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>()
                                .join(" · "),
                            motd,
                        )
                    }
                    Err(_) => ("Offline or unavailable".into(), None),
                };
                if let Some(motd) = motd {
                    self.server_motds.insert(address.clone(), motd);
                } else {
                    self.server_motds.remove(&address);
                }
                self.server_statuses.insert(address, summary);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UiMessage::WorldStatusUpdated(instance_id, result) => {
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    match result {
                        Ok(()) => {
                            self.message = "World visibility updated.".into();
                            self.load_instance_worlds(instance_id);
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not update world: {error}");
                        }
                    }
                }
            }
            UiMessage::ServerAdded(instance_id, server_name, result) => {
                self.add_server_busy = false;
                match result {
                    Ok(_) => {
                        self.add_server_open = false;
                        self.server_edit_index = None;
                        self.server_edit_source = None;
                        self.server_name_input.clear();
                        self.server_address_input.clear();
                        self.input_focus = 0;
                        self.message = format!("Added server {server_name}.");
                        self.load_instance_worlds(instance_id);
                    }
                    Err(error) => {
                        self.message = format!("Could not add server: {error}");
                    }
                }
            }
            UiMessage::ServerSaved(instance_id, server_name, result) => {
                self.add_server_busy = false;
                match result {
                    Ok(()) => {
                        self.add_server_open = false;
                        self.server_edit_index = None;
                        self.server_edit_source = None;
                        self.input_focus = 0;
                        self.message = format!("Saved server {server_name}.");
                        self.load_instance_worlds(instance_id);
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not save server: {error}");
                    }
                }
            }
            UiMessage::ServerRemoved(instance_id, server_name, result) => {
                self.add_server_busy = false;
                self.server_remove_confirmation = None;
                match result {
                    Ok(()) => {
                        self.message = format!("Removed server {server_name}.");
                        self.load_instance_worlds(instance_id);
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not remove server: {error}");
                    }
                }
            }
            UiMessage::WorldRenamed(instance_id, world_name, result) => {
                self.add_server_busy = false;
                match result {
                    Ok(()) => {
                        self.world_editor_open = false;
                        self.world_edit_path.clear();
                        self.world_edit_name.clear();
                        self.world_edit_reset_icon = false;
                        self.input_focus = 0;
                        self.message = format!("Saved world {world_name}.");
                        self.load_instance_worlds(instance_id);
                    }
                    Err(error) => {
                        self.message = format!("Could not save world: {error}");
                    }
                }
            }
            UiMessage::WorldDeleted(instance_id, world_name, result) => {
                self.add_server_busy = false;
                self.world_delete_confirmation = None;
                match result {
                    Ok(()) => {
                        self.message = format!("Deleted world {world_name}.");
                        self.load_instance_worlds(instance_id);
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not delete world: {error}");
                    }
                }
            }
            UiMessage::SharedInstanceLoaded(instance_id, result) => {
                if self.instance_tab == 9
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                {
                    match result {
                        Ok((metadata, can_use, users, invites)) => {
                            self.shared_instance_can_use = Some(can_use);
                            self.shared_instance_users = users
                                .map(|users| users.users)
                                .unwrap_or_default();
                            self.shared_instance_invites = invites;
                            self.shared_instance_metadata = metadata;
                            self.shared_instance_invite_link = None;
                            self.message = if !can_use {
                                "Sign in to a Modrinth account to manage sharing.".into()
                            } else if self
                                .shared_instance_metadata
                                .as_ref()
                                .and_then(|metadata| {
                                    metadata.shared_instance.as_ref()
                                })
                                .is_some()
                            {
                                format!(
                                    "{} members · {} active invites",
                                    self.shared_instance_users.len(),
                                    self.shared_instance_invites.len()
                                )
                            } else {
                                "This instance has not been shared yet.".into()
                            };
                        }
                        Err(error) => {
                            self.shared_instance_metadata = None;
                            self.shared_instance_users.clear();
                            self.shared_instance_invites.clear();
                            self.message = format!(
                                "Could not load sharing details: {error}"
                            );
                        }
                    }
                }
            }
            UiMessage::SharedInstanceInviteCreated(result) => {
                self.shared_instance_busy = false;
                match result {
                    Ok(invite) => {
                        let url = format!(
                            "https://modrinth.com/share/{}",
                            urlencoding::encode(&invite.invite_id)
                        );
                        self.copy_shared_instance_invite(url);
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not create invite link: {error}");
                    }
                }
            }
            UiMessage::SharedInstanceInviteCopied(url, result) => {
                self.shared_instance_invite_link = Some(url);
                self.message = match result {
                    Ok(()) => {
                        "Invite link created and copied to clipboard.".into()
                    }
                    Err(error) => format!(
                        "Invite link created; clipboard copy failed: {error}"
                    ),
                };
                if let Some((instance_id, _)) = self.selected_instance.as_ref()
                {
                    self.load_shared_instance(instance_id.clone());
                }
            }
            UiMessage::SharedInstanceInviteRevoked(invite_id, result) => {
                self.shared_instance_busy = false;
                match result {
                    Ok(()) => {
                        self.shared_instance_invites
                            .retain(|invite| invite.id != invite_id);
                        self.message = "Invite link revoked.".into();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not revoke invite link: {error}");
                    }
                }
            }
            UiMessage::SharedInstancePublishPreview(result) => {
                self.shared_instance_busy = false;
                match result {
                    Ok(Some(preview)) => {
                        self.shared_instance_preview = Some(preview);
                        self.shared_instance_selected_files.clear();
                        self.shared_instance_preview_scroll_pixels = 0.0;
                        self.shared_instance_preview_scroll_target_pixels = 0.0;
                        self.message =
                            "Review changes and choose files to publish."
                                .into();
                    }
                    Ok(None) => {
                        self.shared_instance_preview = None;
                        self.message =
                            "This instance is not eligible for sharing.".into();
                    }
                    Err(error) => {
                        self.message = format!(
                            "Could not prepare shared-instance preview: {error}"
                        );
                    }
                }
            }
            UiMessage::SharedInstancePublishProgress(uploaded, total) => {
                self.shared_instance_progress = Some((uploaded, total));
                self.message = if total > 0 {
                    format!(
                        "Publishing shared files: {}%",
                        uploaded.saturating_mul(100) / total
                    )
                } else {
                    "Publishing shared files…".into()
                };
            }
            UiMessage::SharedInstancePublished(result) => {
                self.shared_instance_busy = false;
                self.shared_instance_progress = None;
                match result {
                    Ok(_) => {
                        self.shared_instance_preview = None;
                        self.message =
                            "Instance published successfully.".into();
                        if let Some((instance_id, _)) =
                            self.selected_instance.as_ref()
                        {
                            self.load_shared_instance(instance_id.clone());
                        }
                    }
                    Err(error) => {
                        self.message = format!(
                            "Could not publish shared instance: {error}"
                        );
                    }
                }
            }
            UiMessage::SharedUsersSearched(result) => {
                self.shared_instance_busy = false;
                match result {
                    Ok(users) => {
                        self.shared_instance_user_results = users;
                        self.message = format!(
                            "{} matching users",
                            self.shared_instance_user_results.len()
                        );
                    }
                    Err(error) => {
                        self.shared_instance_user_results.clear();
                        self.message =
                            format!("Could not search users: {error}");
                    }
                }
            }
            UiMessage::SharedUsersInvited(result) => {
                self.shared_instance_busy = false;
                match result {
                    Ok(users) => {
                        self.shared_instance_users = users.users;
                        self.shared_instance_selected_user_ids.clear();
                        self.shared_instance_user_results.clear();
                        self.shared_instance_invite_open = false;
                        self.shared_instance_user_query.clear();
                        self.message =
                            "Players invited to the shared instance.".into();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not invite players: {error}");
                    }
                }
            }
            UiMessage::SharedUserRemoved(user_id, result) => {
                self.shared_instance_busy = false;
                match result {
                    Ok(users) => {
                        self.shared_instance_users = users.users;
                        self.shared_instance_pending_removal = None;
                        self.message = "Player access revoked.".into();
                        if let Some((instance_id, _)) =
                            self.selected_instance.as_ref()
                        {
                            self.load_shared_instance(instance_id.clone());
                        }
                    }
                    Err(error) => {
                        self.message = format!(
                            "Could not revoke access for {user_id}: {error}"
                        );
                    }
                }
            }
            UiMessage::SharedModpackUnlinked(result) => {
                self.shared_instance_busy = false;
                match result {
                    Ok(metadata) => {
                        self.shared_instance_metadata = Some(metadata);
                        self.shared_instance_unlink_confirmation = false;
                        self.message = "Imported modpack link removed. Review sharing again to continue.".into();
                    }
                    Err(error) => {
                        self.message = format!(
                            "Could not unlink imported modpack: {error}"
                        );
                    }
                }
            }
            UiMessage::ScreenshotsLoaded(instance_id, result) => {
                let expected_id =
                    self.selected_instance.as_ref().map(|(id, _)| id.as_str());
                if self.instance_tab == 8
                    && expected_id == Some(instance_id.as_str())
                {
                    match result {
                        Ok(screenshots) => {
                            self.screenshot_offset = self
                                .screenshot_offset
                                .min(screenshots.len().saturating_sub(1));
                            self.selected_screenshot = None;
                            self.selected_screenshot_ids.clear();
                            self.message = if self.global_screenshots {
                                format!(
                                    "{} SHOTS | CTRL+CLICK MULTI-SELECT",
                                    screenshots.len()
                                )
                            } else {
                                format!(
                                    "{} SHOTS | CTRL+CLICK MULTI-SELECT",
                                    screenshots.len()
                                )
                            };
                            self.screenshots = screenshots;
                        }
                        Err(error) => {
                            self.screenshots.clear();
                            self.message =
                                format!("Could not load screenshots: {error}");
                        }
                    }
                }
            }
            UiMessage::ScreenshotGroupsLoaded(result) => match result {
                Ok(groups) => self.screenshot_groups = groups,
                Err(error) => {
                    self.message =
                        format!("Could not load screenshot groups: {error}");
                }
            },
            UiMessage::ScreenshotGroupCreated(result) => match result {
                Ok(group) => {
                    self.screenshot_group_action_pending = false;
                    self.message =
                        format!("Created screenshot group {}.", group.name);
                    self.screenshot_groups.insert(0, group);
                    self.screenshot_group_mode = 2;
                    self.screenshot_group_picker_open =
                        self.selected_screenshot.is_some();
                }
                Err(error) => {
                    self.screenshot_group_action_pending = false;
                    self.message =
                        format!("Could not create screenshot group: {error}");
                }
            },
            UiMessage::ScreenshotGroupRenamed(result) => match result {
                Ok(group) => {
                    self.screenshot_group_action_pending = false;
                    if let Some(current) = self
                        .screenshot_groups
                        .iter_mut()
                        .find(|current| current.id == group.id)
                    {
                        current.name.clone_from(&group.name);
                    }
                    self.screenshot_group_rename_id = None;
                    self.screenshot_group_name_input.clear();
                    self.screenshot_group_picker_open = false;
                    self.screenshot_offset = 0;
                    self.input_focus = 0;
                    self.message =
                        format!("Renamed screenshot group to {}.", group.name);
                }
                Err(error) => {
                    self.screenshot_group_action_pending = false;
                    self.message =
                        format!("Could not rename screenshot group: {error}");
                }
            },
            UiMessage::ScreenshotGroupDeleted(group_id, result) => match result
            {
                Ok(()) => {
                    self.screenshot_group_action_pending = false;
                    self.screenshot_groups.retain(|group| group.id != group_id);
                    for screenshot in &mut self.screenshots {
                        if screenshot.group_id.as_deref()
                            == Some(group_id.as_str())
                        {
                            screenshot.group_id = None;
                        }
                    }
                    self.screenshot_group_delete_confirmation = None;
                    self.screenshot_group_picker_open = false;
                    self.screenshot_offset = 0;
                    self.screenshot_group_offset = self
                        .screenshot_group_offset
                        .min(self.screenshot_groups.len().saturating_sub(8));
                    self.message = "Screenshot group deleted.".into();
                }
                Err(error) => {
                    self.screenshot_group_action_pending = false;
                    self.screenshot_group_delete_confirmation = None;
                    self.message =
                        format!("Could not delete screenshot group: {error}");
                }
            },
            UiMessage::ScreenshotMembershipsUpdated(updates, result) => {
                self.screenshot_group_action_pending = false;
                match result {
                    Ok(()) => {
                        for (screenshot_id, group_id) in updates {
                            if let Some(screenshot) =
                                self.screenshots.iter_mut().find(|screenshot| {
                                    screenshot.id == screenshot_id
                                })
                            {
                                screenshot.group_id = group_id;
                            }
                        }
                        self.screenshot_offset = 0;
                        self.message = "Screenshot groups updated.".into();
                    }
                    Err(error) => {
                        self.message = format!(
                            "Could not update screenshot groups: {error}"
                        );
                    }
                }
            }
            UiMessage::ScreenshotPreviewLoaded(screenshot_id, result) => {
                if self.screenshot_viewer
                    && self.selected_screenshot.is_some_and(|index| {
                        self.screenshots.get(index).is_some_and(|screenshot| {
                            screenshot.id == screenshot_id
                        })
                    })
                {
                    match result {
                        Ok(preview) => {
                            self.screenshot_preview = Some(preview);
                            self.message = "Screenshot preview ready.".into();
                        }
                        Err(error) => {
                            self.screenshot_preview = None;
                            self.message =
                                format!("Could not open screenshot: {error}");
                        }
                    }
                }
            }
            UiMessage::ScreenshotThumbnailLoaded(screenshot_id, result) => {
                self.screenshot_thumbnails_pending.remove(&screenshot_id);
                match result {
                    Ok(thumbnail) => {
                        self.screenshot_thumbnails
                            .insert(screenshot_id.clone(), thumbnail);
                        cap_image_cache(
                            &mut self.screenshot_thumbnails,
                            &screenshot_id,
                            SCREENSHOT_THUMBNAIL_CACHE_BUDGET,
                        );
                    }
                    Err(_) => {
                        self.screenshot_thumbnail_failures
                            .insert(screenshot_id);
                    }
                }
            }
            UiMessage::ScreenshotBatchDeleted(screenshot_ids, result) => {
                match result {
                    Ok(()) => {
                        let deleted = screenshot_ids
                            .iter()
                            .cloned()
                            .collect::<HashSet<_>>();
                        self.screenshots.retain(|screenshot| {
                            !deleted.contains(&screenshot.id)
                        });
                        for id in &screenshot_ids {
                            self.screenshot_thumbnails.remove(id);
                            self.screenshot_thumbnails_pending.remove(id);
                            self.screenshot_thumbnail_failures.remove(id);
                        }
                        self.selected_screenshot_ids.clear();
                        self.selected_screenshot = None;
                        self.screenshot_viewer = false;
                        self.screenshot_preview = None;
                        self.screenshot_delete_confirmation = false;
                        self.message = format!(
                            "Deleted {} screenshot{}.",
                            screenshot_ids.len(),
                            if screenshot_ids.len() == 1 { "" } else { "s" }
                        );
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not delete screenshots: {error}");
                    }
                }
            }
            UiMessage::ScreenshotCopied(result) => {
                self.message = match result {
                    Ok(()) => "Screenshot copied to clipboard.".into(),
                    Err(error) => format!("Could not copy screenshot: {error}"),
                };
                self.draw();
            }
            UiMessage::ClipboardTextCopied(result) => {
                self.message = match result {
                    Ok(()) => "Log copied.".into(),
                    Err(error) => format!("Could not copy log: {error}"),
                };
                self.draw();
            }
            UiMessage::ScreenshotExported(result) => match result {
                Ok(Some(path)) => {
                    self.message =
                        format!("Screenshot exported to {}", path.display());
                }
                Ok(None) => {
                    self.message = "Screenshot export cancelled.".into()
                }
                Err(error) => {
                    self.message =
                        format!("Could not export screenshot: {error}")
                }
            },
            UiMessage::ScreenshotBatchExported(result) => match result {
                Ok(Some(path)) => {
                    self.message = format!(
                        "Selected screenshots exported to {}",
                        path.display()
                    );
                }
                Ok(None) => {
                    self.message = "Screenshot export cancelled.".into()
                }
                Err(error) => {
                    self.message = format!(
                        "Could not export selected screenshots: {error}"
                    );
                }
            },
            UiMessage::InstanceLaunchSettingsLoaded(instance_id, result) => {
                self.instance_settings_loading = false;
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|item| item.0 == instance_id)
                {
                    match result {
                        Ok((metadata, maximum_memory)) => {
                            let overrides = metadata.launch_overrides;
                            self.instance_java_path =
                                overrides.java_path.unwrap_or_default();
                            self.instance_launch_args = overrides
                                .extra_launch_args
                                .unwrap_or_default()
                                .join(" ");
                            self.instance_memory_mb = overrides
                                .memory
                                .map(|memory| memory.maximum.to_string())
                                .unwrap_or_default();
                            self.instance_fullscreen_mode = overrides
                                .force_fullscreen
                                .map(|enabled| if enabled { 1 } else { 2 })
                                .unwrap_or(0);
                            self.instance_resolution_width = overrides
                                .game_resolution
                                .map(|size| size.0.to_string())
                                .unwrap_or_default();
                            self.instance_resolution_height = overrides
                                .game_resolution
                                .map(|size| size.1.to_string())
                                .unwrap_or_default();
                            self.instance_environment_json = overrides
                                .custom_env_vars
                                .map(|vars| {
                                    serde_json::to_string(&vars)
                                        .unwrap_or_else(|_| "[]".into())
                                })
                                .unwrap_or_default();
                            self.instance_pre_launch_hook =
                                overrides.hooks.pre_launch.unwrap_or_default();
                            self.instance_wrapper_hook =
                                overrides.hooks.wrapper.unwrap_or_default();
                            self.instance_post_exit_hook =
                                overrides.hooks.post_exit.unwrap_or_default();
                            self.instance_max_memory_mb = maximum_memory;
                            self.message =
                                "Instance launch settings loaded.".into();
                        }
                        Err(error) => {
                            self.message = format!(
                                "Could not load launch settings: {error}"
                            )
                        }
                    }
                }
            }
            UiMessage::InstanceLaunchSettingsSaved(instance_id, result) => {
                self.instance_settings_saving = false;
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|item| item.0 == instance_id)
                {
                    match result {
                        Ok(metadata) => {
                            let overrides = metadata.launch_overrides;
                            self.instance_java_path =
                                overrides.java_path.unwrap_or_default();
                            self.instance_launch_args = overrides
                                .extra_launch_args
                                .unwrap_or_default()
                                .join(" ");
                            self.instance_memory_mb = overrides
                                .memory
                                .map(|memory| memory.maximum.to_string())
                                .unwrap_or_default();
                            self.instance_fullscreen_mode = overrides
                                .force_fullscreen
                                .map(|enabled| if enabled { 1 } else { 2 })
                                .unwrap_or(0);
                            self.instance_resolution_width = overrides
                                .game_resolution
                                .map(|size| size.0.to_string())
                                .unwrap_or_default();
                            self.instance_resolution_height = overrides
                                .game_resolution
                                .map(|size| size.1.to_string())
                                .unwrap_or_default();
                            self.instance_environment_json = overrides
                                .custom_env_vars
                                .map(|vars| {
                                    serde_json::to_string(&vars)
                                        .unwrap_or_else(|_| "[]".into())
                                })
                                .unwrap_or_default();
                            self.instance_pre_launch_hook =
                                overrides.hooks.pre_launch.unwrap_or_default();
                            self.instance_wrapper_hook =
                                overrides.hooks.wrapper.unwrap_or_default();
                            self.instance_post_exit_hook =
                                overrides.hooks.post_exit.unwrap_or_default();
                            self.message =
                                "Instance launch settings saved.".into();
                        }
                        Err(error) => {
                            self.message = format!(
                                "Could not save launch settings: {error}"
                            )
                        }
                    }
                }
            }
            UiMessage::InstanceJavaPathPicked(instance_id, path) => {
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|item| item.0 == instance_id)
                {
                    if let Some(path) = path {
                        self.instance_java_path = path;
                        self.input_focus = 0;
                    }
                }
            }
            UiMessage::ContentAction(instance_id, result) => {
                if let Some((selected_id, _)) = self.selected_instance.as_ref()
                {
                    if *selected_id == instance_id {
                        self.message = match result {
                            Ok(()) => "Content updated.".into(),
                            Err(error) => {
                                format!("Content action failed: {error}")
                            }
                        };
                        self.load_instance_content(instance_id);
                    }
                }
            }
            UiMessage::ContentFileAdded(instance_id, result) => {
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    match result {
                        Ok(Some(path)) => {
                            self.message = format!("Added {path}.")
                        }
                        Ok(None) => {
                            self.message = "File selection canceled.".into()
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not add file: {error}")
                        }
                    }
                    self.load_instance_content(instance_id);
                }
            }
            UiMessage::LiveLogChanged(instance_id) => {
                if self.instance_tab == 1
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                {
                    self.refresh_live_log(instance_id);
                }
            }
            UiMessage::LiveLogLoaded(instance_id, result) => {
                if self.instance_tab == 1
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                {
                    match result {
                        Ok(output) => {
                            let lines: Vec<&str> = output.lines().collect();
                            self.console_text = lines
                                .iter()
                                .rev()
                                .take(2000)
                                .rev()
                                .copied()
                                .collect::<Vec<_>>()
                                .join("\n");
                            self.console_scroll = self.console_scroll.min(
                                self.console_text
                                    .lines()
                                    .count()
                                    .saturating_sub(1),
                            );
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not load live console: {error}")
                        }
                    }
                }
            }
            UiMessage::InstanceLogsLoaded(instance_id, result) => {
                if self.instance_tab == 5
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                {
                    match result {
                        Ok(logs) => {
                            self.message = format!(
                                "{} logs and crash reports",
                                logs.len()
                            );
                            self.instance_logs = logs;
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not list logs: {error}");
                            self.instance_logs.clear();
                        }
                    }
                }
            }
            UiMessage::InstanceLogContentLoaded(
                instance_id,
                filename,
                is_crash,
                result,
            ) => {
                if self.instance_tab == 5
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                    && self.selected_log.as_ref() == Some(&(filename, is_crash))
                {
                    match result {
                        Ok(content) => {
                            self.log_scroll = 0;
                            self.log_scroll_synced = 0;
                            self.log_scroll_pixels = 0.0;
                            self.log_scroll_target_pixels = 0.0;
                            self.log_text = content;
                            self.log_analysis = None;
                            self.log_analysis_pending = false;
                            self.log_share_pending = false;
                            self.log_delete_confirmation = false;
                            self.log_delete_pending = false;
                            self.message = "Share uploads this log text; Analyze sends it to mclo.gs for diagnostics.".into();
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not read log: {error}");
                            self.log_text.clear();
                            self.log_analysis = None;
                            self.log_analysis_pending = false;
                            self.log_share_pending = false;
                            self.log_delete_confirmation = false;
                            self.log_delete_pending = false;
                        }
                    }
                }
            }
            UiMessage::LogAnalysisFinished(instance_id, filename, result) => {
                if self.instance_tab == 5
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                    && self
                        .selected_log
                        .as_ref()
                        .is_some_and(|selected| selected.0 == filename)
                {
                    self.log_analysis_pending = false;
                    match result {
                        Ok(analysis) => {
                            let problem_count = analysis
                                .get("analysis")
                                .and_then(|value| value.get("problems"))
                                .and_then(serde_json::Value::as_array)
                                .map_or(0, Vec::len);
                            self.message = if problem_count == 0 {
                                "Analysis complete · no known crash problems found.".into()
                            } else {
                                format!(
                                    "Analysis found {problem_count} potential issue{}.",
                                    if problem_count == 1 { "" } else { "s" }
                                )
                            };
                            self.log_analysis = Some(analysis);
                        }
                        Err(error) => {
                            self.log_analysis = None;
                            self.message =
                                format!("Crash analysis failed: {error}");
                        }
                    }
                }
            }
            UiMessage::LogShareFinished(instance_id, filename, result) => {
                let is_current_share =
                    self.selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                        && ((self.instance_tab == 5
                            && self.selected_log.as_ref().is_some_and(
                                |selected| selected.0 == filename,
                            ))
                            || (self.instance_tab == 1
                                && filename == "__live_console__"));
                if is_current_share {
                    self.log_share_pending = false;
                    match result {
                        Ok(url) => match set_clipboard_text(&url) {
                            Ok(()) => {
                                self.message = format!(
                                    "Share link copied to clipboard: {url}"
                                )
                            }
                            Err(error) => {
                                self.message = format!(
                                    "Log shared, but the link could not be copied ({error}): {url}"
                                )
                            }
                        },
                        Err(error) => {
                            self.message =
                                format!("Could not share log: {error}")
                        }
                    }
                }
            }
            UiMessage::LogDeleted(instance_id, filename, result) => {
                if self.instance_tab == 5
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                    && self
                        .selected_log
                        .as_ref()
                        .is_some_and(|selected| selected.0 == filename)
                {
                    self.log_delete_pending = false;
                    match result {
                        Ok(()) => {
                            self.selected_log = None;
                            self.log_text.clear();
                            self.log_analysis = None;
                            self.log_analysis_pending = false;
                            self.log_share_pending = false;
                            self.message =
                                format!("Deleted {filename}. Refreshing logs…");
                            self.refresh_instance_logs(instance_id);
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not delete log: {error}");
                        }
                    }
                }
            }
            UiMessage::RefreshProcess(instance_id) => {
                if self.instance_tab == 1
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                {
                    self.refresh_process_state(instance_id);
                }
            }
            UiMessage::ProcessStateLoaded(instance_id, result) => {
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    match result {
                        Ok(process) => self.process_state = process,
                        Err(error) => {
                            self.message = format!(
                                "Could not check process status: {error}"
                            )
                        }
                    }
                }
            }
            UiMessage::RefreshRunningProcesses => {
                self.refresh_running_processes();
            }
            UiMessage::RunningProcessesLoaded(result) => {
                self.running_processes_pending = false;
                match result {
                    Ok(processes) => {
                        self.running_processes = processes
                            .into_iter()
                            .map(|process| {
                                (process.instance_id.clone(), process)
                            })
                            .collect();
                    }
                    Err(error) => {
                        self.message = format!(
                            "Could not refresh running instances: {error}"
                        );
                    }
                }
                self.draw();
                if self.running_processes_refresh_dirty {
                    self.running_processes_refresh_dirty = false;
                    self.refresh_running_processes();
                }
            }
            UiMessage::CardStopDone(instance_id, result) => {
                match result {
                    Ok(()) => {
                        self.running_processes.remove(&instance_id);
                        self.message = "Stopping Minecraft…".into();
                        self.refresh_running_processes();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not stop Minecraft: {error}");
                    }
                }
                self.draw();
            }
            UiMessage::StopProcessDone(instance_id, result) => {
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    self.message = match result {
                        Ok(()) => "Stopping Minecraft…".into(),
                        Err(error) => {
                            format!("Could not stop Minecraft: {error}")
                        }
                    };
                    self.refresh_process_state(instance_id);
                }
            }
            UiMessage::FilesLoaded(instance_id, path, result) => {
                if self.instance_tab == 2
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                    && self.file_path == path
                {
                    match result {
                        Ok(files) => {
                            self.message =
                                format!("{} files and folders", files.len());
                            self.file_items = files;
                            self.file_offset = 0;
                            self.file_list_scroll_pixels = 0.0;
                            self.file_list_scroll_target_pixels = 0.0;
                            self.file_list_scroll_synced = 0;
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not list files: {error}")
                        }
                    }
                }
            }
            UiMessage::FileRead(instance_id, path, result) => {
                if self.instance_tab == 2
                    && self
                        .selected_instance
                        .as_ref()
                        .is_some_and(|selected| selected.0 == instance_id)
                {
                    match result {
                        Ok(contents) => {
                            self.file_preview = Some((path, contents));
                            self.file_offset = 0;
                            self.file_preview_scroll_pixels = 0.0;
                            self.file_preview_scroll_target_pixels = 0.0;
                            self.file_list_scroll_pixels = 0.0;
                            self.file_list_scroll_target_pixels = 0.0;
                            self.file_list_scroll_synced = 0;
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not preview file: {error}")
                        }
                    }
                }
            }
            UiMessage::BrowseSearchDebounced(generation, query) => {
                if generation == self.browse_search_generation
                    && query == self.browse_query
                    && self.is_browse_results_page()
                {
                    self.search_projects();
                    self.draw();
                }
            }
            UiMessage::BrowseResults(generation, query, result) => {
                if generation != self.browse_request_generation
                    || query != self.browse_query.trim()
                    || self.browse_selected.is_some()
                {
                    return;
                }
                self.browse_search_pending = false;
                match result {
                    Ok(results) => {
                        self.browse_error = None;
                        self.reset_browse_scroll();
                        self.browse_total_hits = results.total_hits;
                        let incoming = results.projects;
                        for project in &self.browse_results {
                            if !incoming.iter().any(|next| {
                                next.id == project.id
                                    && next.icon_url == project.icon_url
                            }) {
                                self.browse_icons.remove(&project.id);
                                self.browse_icon_failures.remove(&project.id);
                            }
                        }
                        self.browse_results = incoming;
                        let first = if self.browse_total_hits == 0 {
                            0
                        } else {
                            self.browse_search_offset + 1
                        };
                        let last = self
                            .browse_search_offset
                            .saturating_add(self.browse_page_size)
                            .min(self.browse_total_hits);
                        self.message = format!(
                            "Showing {first}–{last} of {} projects",
                            self.browse_total_hits
                        );
                    }
                    Err(error) => {
                        self.browse_results.clear();
                        self.browse_total_hits = 0;
                        self.browse_error = Some(error.clone());
                        self.message =
                            format!("Project search failed: {error}");
                    }
                }
            }
            UiMessage::BrowseDetails(project_id, result) => {
                if self
                    .browse_selected
                    .as_ref()
                    .is_some_and(|project| project.id == project_id)
                {
                    match result {
                        Ok(details) => {
                            self.browse_detail_scroll_pixels = 0.0;
                            self.browse_detail_scroll_target_pixels = 0.0;
                            self.browse_description_lines = None;
                            self.message = format!(
                                "{} versions available",
                                details.versions.len()
                            );
                            self.browse_details = Some(details);
                        }
                        Err(error) => {
                            self.browse_details = None;
                            self.message = format!(
                                "Could not load project details: {error}"
                            );
                        }
                    }
                }
            }
            UiMessage::BrowseFollowStatus(project_id, result) => {
                if self
                    .browse_selected
                    .as_ref()
                    .is_some_and(|project| project.id == project_id)
                {
                    self.browse_project_follow_status_loading = false;
                    match result {
                        Ok(followed) => {
                            self.browse_project_followed = followed;
                            self.browse_project_follow_status_loaded = true;
                        }
                        Err(error) => {
                            self.browse_project_followed = false;
                            if !error.contains("Sign in to Modrinth") {
                                self.message = error;
                            }
                        }
                    }
                    self.draw();
                }
            }
            UiMessage::BrowseFollowChanged(project_id, followed, result) => {
                if self
                    .browse_selected
                    .as_ref()
                    .is_some_and(|project| project.id == project_id)
                {
                    self.browse_project_follow_pending = false;
                    match result {
                        Ok(()) => {
                            self.browse_project_followed = followed;
                            if let Some(project) = &mut self.browse_selected {
                                project.followers = if followed {
                                    project.followers.saturating_add(1)
                                } else {
                                    project.followers.saturating_sub(1)
                                };
                            }
                            self.message = if followed {
                                "Project followed.".into()
                            } else {
                                "Project unfollowed.".into()
                            };
                        }
                        Err(error) => {
                            self.message = format!(
                                "Could not update project follow: {error}"
                            );
                        }
                    }
                    self.draw();
                }
            }
            UiMessage::BrowseSaveCollectionsLoaded(result) => {
                self.browse_save_collections_loading = false;
                match result {
                    Ok(collections) => {
                        self.browse_save_collections = collections;
                        self.browse_save_collections_loaded = true;
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not load your collections: {error}");
                    }
                }
                self.draw();
            }
            UiMessage::BrowseCollectionSaved(
                collection_id,
                project_id,
                projects,
                result,
            ) => {
                self.browse_save_collection_pending.remove(&collection_id);
                if let Some(collection) = self
                    .browse_save_collections
                    .iter_mut()
                    .find(|collection| collection.id == collection_id)
                {
                    match result {
                        Ok(()) => {
                            let was_saved = collection
                                .projects
                                .iter()
                                .any(|id| id == &project_id);
                            let is_saved =
                                projects.iter().any(|id| id == &project_id);
                            collection.projects = projects;
                            self.message = if was_saved && !is_saved {
                                "Project removed from collection.".into()
                            } else {
                                "Project saved to collection.".into()
                            };
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not update collection: {error}");
                        }
                    }
                }
                self.draw();
            }
            UiMessage::BrowseCollectionCreated(result) => {
                self.browse_collection_creating = false;
                match result {
                    Ok(collection) => {
                        if !self
                            .browse_save_collections
                            .iter()
                            .any(|item| item.id == collection.id)
                        {
                            self.browse_save_collections.push(collection);
                        }
                        self.browse_save_collections_loaded = true;
                        self.browse_collection_create_open = false;
                        self.browse_collection_create_name.clear();
                        self.input_focus = 0;
                        self.message =
                            "Collection created and project saved.".into();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not create collection: {error}");
                    }
                }
                self.draw();
            }
            UiMessage::BrowseInstallVersionsLoaded(generation, versions) => {
                if let Some(choice) = &mut self.browse_install_choice
                    && choice.generation == generation
                {
                    choice.versions = Some(versions);
                    self.draw();
                }
            }
            UiMessage::BrowseInstallPresenceLoaded(
                generation,
                id,
                installed,
            ) => {
                if let Some(choice) = &mut self.browse_install_choice
                    && choice.generation == generation
                {
                    choice.installed.insert(id, installed);
                    self.draw();
                }
            }
            UiMessage::BrowseInstallDone(
                instance_id,
                project_id,
                title,
                result,
            ) => {
                if let std::collections::hash_map::Entry::Occupied(mut entry) =
                    self.browse_installing_projects.entry(instance_id.clone())
                {
                    entry.get_mut().remove(&project_id);
                    if entry.get().is_empty() {
                        entry.remove();
                    }
                }
                let failed = result.is_err();
                let notice = match result {
                    Ok(0) => format!("Installed {title}."),
                    Ok(1) => format!("Installed {title} and one dependency."),
                    Ok(dependencies) => format!(
                        "Installed {title} and {dependencies} dependencies."
                    ),
                    Err(error) => format!("Could not install {title}: {error}"),
                };
                self.show_browse_install_notice(notice, failed);
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    self.load_instance_content(instance_id);
                }
            }
            UiMessage::BrowseModpackInstallDone(project_id, result) => {
                self.browse_installing_modpacks.remove(&project_id);
                let failed = result.is_err();
                let notice = match result {
                    Ok(title) => {
                        format!("Installed modpack {title} as a new instance.")
                    }
                    Err(error) => format!("Could not install modpack: {error}"),
                };
                self.show_browse_install_notice(notice, failed);
                self.refresh_instances();
            }
            UiMessage::ExportFinished(instance_id, result) => {
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    self.message = match result {
                        Ok(Some(path)) => {
                            format!("Modpack exported to {}.", path.display())
                        }
                        Ok(None) => "Modpack export canceled.".into(),
                        Err(error) => {
                            format!("Could not export modpack: {error}")
                        }
                    };
                }
            }
            UiMessage::DuplicateFinished(instance_id, result) => {
                let succeeded = result.is_ok();
                if succeeded {
                    self.refresh_instances();
                }
                if self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|selected| selected.0 == instance_id)
                {
                    self.message = match result {
                        Ok(()) => "Instance duplicated successfully.".into(),
                        Err(error) => {
                            format!("Could not duplicate instance: {error}")
                        }
                    };
                }
            }
            UiMessage::InstanceRenamed(instance_id, result) => match result {
                Ok(name) => {
                    if let Some((selected_id, selected_name)) =
                        self.selected_instance.as_mut()
                        && *selected_id == instance_id
                    {
                        *selected_name = name.clone();
                    }
                    self.rename_instance_open = false;
                    self.rename_instance_input.clear();
                    self.input_focus = 0;
                    self.message = format!("Instance renamed to {name}.");
                    self.refresh_instances();
                }
                Err(error) => {
                    self.message =
                        format!("Could not rename instance: {error}");
                }
            },
            UiMessage::InstanceRemoved(instance_id, result) => {
                self.instance_deletion_pending = false;
                self.delete_confirmation_open = false;
                self.delete_confirmation_input.clear();
                self.input_focus = 0;
                match result {
                    Ok(()) => {
                        if self
                            .selected_instance
                            .as_ref()
                            .is_some_and(|selected| selected.0 == instance_id)
                        {
                            self.selected_instance = None;
                            self.contents.clear();
                            self.filtered_content_cache.clear();
                            self.file_items.clear();
                            self.file_preview = None;
                            self.file_preview_scroll_pixels = 0.0;
                            self.file_preview_scroll_target_pixels = 0.0;
                        }
                        self.message =
                            "Instance and its local files were removed.".into();
                        self.refresh_instances();
                        self.refresh_groups();
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not remove instance: {error}");
                    }
                }
            }
            UiMessage::BulkInstancesRemoved(removed_ids, errors) => {
                self.instance_deletion_pending = false;
                self.bulk_delete_confirmation_open = false;
                self.delete_confirmation_input.clear();
                self.input_focus = 0;
                let removed: HashSet<String> =
                    removed_ids.iter().cloned().collect();
                self.instances.retain(|(id, _, _)| !removed.contains(id));
                self.instance_cards.retain(|id, _| !removed.contains(id));
                self.instance_metrics.retain(|id, _| !removed.contains(id));
                self.selected_library_instances
                    .retain(|id| !removed.contains(id));
                self.refresh_instances();
                self.refresh_groups();
                self.message = if errors.is_empty() {
                    format!(
                        "Removed {} selected instances and their local files.",
                        removed_ids.len()
                    )
                } else {
                    format!(
                        "Removed {} instances; {} could not be removed: {}",
                        removed_ids.len(),
                        errors.len(),
                        errors.join("; ")
                    )
                };
            }
            UiMessage::SettingsLoaded(result) => match result {
                Ok(settings) => {
                    if !cfg!(target_os = "macos")
                        && let Some(window) = &self.window
                    {
                        window.set_decorations(settings.native_decorations);
                    }
                    if is_dark_theme(settings.theme) {
                        self.preferred_dark_theme = settings.theme;
                        self.persist_preferred_dark_theme(settings.theme);
                    }
                    self.default_window_width_input =
                        settings.game_resolution.0.to_string();
                    self.default_window_height_input =
                        settings.game_resolution.1.to_string();
                    self.default_java_arguments_input =
                        settings.extra_launch_args.join(" ");
                    self.default_environment_variables_input = settings
                        .custom_env_vars
                        .iter()
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    self.default_pre_launch_hook_input =
                        settings.hooks.pre_launch.clone().unwrap_or_default();
                    self.default_wrapper_hook_input =
                        settings.hooks.wrapper.clone().unwrap_or_default();
                    self.default_post_exit_hook_input =
                        settings.hooks.post_exit.clone().unwrap_or_default();
                    self.settings = Some(settings);
                    self.message = "Settings loaded.".into();
                }
                Err(error) => {
                    self.message = format!("Could not load settings: {error}")
                }
            },
            UiMessage::WindowDefaultsSaved(Ok(settings)) => {
                self.window_defaults_saving = false;
                self.default_window_width_input =
                    settings.game_resolution.0.to_string();
                self.default_window_height_input =
                    settings.game_resolution.1.to_string();
                self.settings = Some(settings);
                self.input_focus = 0;
                self.message = "Default game window size saved.".into();
                self.draw();
            }
            UiMessage::WindowDefaultsSaved(Err(error)) => {
                self.window_defaults_saving = false;
                self.message =
                    format!("Could not save default window size: {error}");
                self.draw();
            }
            UiMessage::DefaultLaunchSettingsSaved(Ok(settings)) => {
                self.default_launch_settings_saving = false;
                self.default_java_arguments_input =
                    settings.extra_launch_args.join(" ");
                self.default_environment_variables_input = settings
                    .custom_env_vars
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.default_pre_launch_hook_input =
                    settings.hooks.pre_launch.clone().unwrap_or_default();
                self.default_wrapper_hook_input =
                    settings.hooks.wrapper.clone().unwrap_or_default();
                self.default_post_exit_hook_input =
                    settings.hooks.post_exit.clone().unwrap_or_default();
                self.settings = Some(settings);
                self.input_focus = 0;
                self.message = "Default Java launch settings saved.".into();
                self.draw();
            }
            UiMessage::DefaultLaunchSettingsSaved(Err(error)) => {
                self.default_launch_settings_saving = false;
                self.message =
                    format!("Could not save default launch settings: {error}");
                self.draw();
            }
            UiMessage::ScreenshotSyncLoaded(Ok(enabled)) => {
                self.global_screenshot_sync = Some(enabled);
                self.draw();
            }
            UiMessage::ScreenshotSyncLoaded(Err(error)) => {
                self.global_screenshot_sync = None;
                self.message =
                    format!("Could not load screenshot sync setting: {error}");
                self.draw();
            }
            UiMessage::QuickInstanceCountLoaded(Ok(count)) => {
                self.jump_in_visible_items = count.clamp(1, 20);
                self.draw();
            }
            UiMessage::QuickInstanceCountLoaded(Err(error)) => {
                self.message =
                    format!("Could not load Jump In item count: {error}");
                self.draw();
            }
            UiMessage::QuickInstanceCountSaved(Ok(count)) => {
                self.quick_instance_count_saving = false;
                self.jump_in_visible_items = count.clamp(1, 20);
                self.message = "Jump In item count saved.".into();
                self.draw();
            }
            UiMessage::QuickInstanceCountSaved(Err(error)) => {
                self.quick_instance_count_saving = false;
                self.message =
                    format!("Could not save Jump In item count: {error}");
                self.draw();
            }
            UiMessage::ColorThemeLoaded(Ok(theme)) => {
                self.native_color_theme = theme.min(8);
                self.draw();
            }
            UiMessage::ColorThemeLoaded(Err(error)) => {
                self.message =
                    format!("Could not load Northstar colorway: {error}");
                self.draw();
            }
            UiMessage::ColorThemeSaved(Ok(theme)) => {
                self.native_color_theme_saving = false;
                self.native_color_theme = theme.min(8);
                self.message = "Northstar colorway saved.".into();
                self.draw();
            }
            UiMessage::ColorThemeSaved(Err(error)) => {
                self.native_color_theme_saving = false;
                self.message =
                    format!("Could not save Northstar colorway: {error}");
                self.draw();
            }
            UiMessage::PreferredDarkThemeLoaded(Ok(theme)) => {
                if !self
                    .settings
                    .as_ref()
                    .is_some_and(|settings| is_dark_theme(settings.theme))
                {
                    self.preferred_dark_theme = theme;
                    self.draw();
                }
            }
            UiMessage::PreferredDarkThemeLoaded(Err(error)) => {
                self.message =
                    format!("Could not load preferred dark theme: {error}");
                self.draw();
            }
            UiMessage::PreferredDarkThemeSaved(Ok(())) => {}
            UiMessage::PreferredDarkThemeSaved(Err(error)) => {
                self.message =
                    format!("Could not save preferred dark theme: {error}");
                self.draw();
            }
            UiMessage::ThemeSyncLoaded(generation, Ok(enabled))
                if generation == self.theme_sync_generation =>
            {
                self.theme_sync_across_devices = enabled;
                if enabled
                    && let Some(account) = self
                        .content_accounts
                        .iter()
                        .find(|account| account.active)
                {
                    self.load_account_theme_preferences(
                        account.user_id.clone(),
                    );
                }
                self.draw();
            }
            UiMessage::ThemeSyncLoaded(generation, Err(error))
                if generation == self.theme_sync_generation =>
            {
                self.message =
                    format!("Could not load theme sync setting: {error}");
                self.draw();
            }
            UiMessage::ThemeSyncSaved(generation, Ok(enabled))
                if generation == self.theme_sync_generation =>
            {
                self.theme_sync_saving = false;
                self.theme_sync_across_devices = enabled;
                self.message = if enabled {
                    "Theme sync enabled.".into()
                } else {
                    "Theme sync disabled.".into()
                };
                if enabled {
                    self.sync_current_theme_to_account();
                }
                self.draw();
            }
            UiMessage::ThemeSyncSaved(generation, Err(error))
                if generation == self.theme_sync_generation =>
            {
                self.theme_sync_saving = false;
                self.message =
                    format!("Could not save theme sync setting: {error}");
                self.draw();
            }
            UiMessage::ThemeSyncLoaded(_, _)
            | UiMessage::ThemeSyncSaved(_, _) => {}
            UiMessage::ThemeSyncThemeSaved(Ok(())) => {
                self.message = "Theme synced to your Modrinth account.".into();
                self.draw();
            }
            UiMessage::ThemeSyncThemeSaved(Err(error)) => {
                self.message =
                    format!("Could not sync theme to Modrinth: {error}");
                self.draw();
            }
            UiMessage::ThemePreferencesLoaded(user_id, Ok(preferences)) => {
                self.apply_account_theme_preferences(user_id, preferences);
            }
            UiMessage::ThemePreferencesLoaded(user_id, Err(error)) => {
                if self.theme_sync_across_devices
                    && self.content_accounts.iter().any(|account| {
                        account.active && account.user_id == user_id
                    })
                {
                    self.message =
                        format!("Could not load synced theme: {error}");
                    self.draw();
                }
            }
            UiMessage::ScreenshotSyncSaved(Ok(enabled)) => {
                self.global_screenshot_sync = Some(enabled);
                self.message = "Screenshot syncing preference saved.".into();
                self.draw();
            }
            UiMessage::ScreenshotSyncSaved(Err(error)) => {
                self.message =
                    format!("Could not save screenshot sync setting: {error}");
                self.draw();
            }
            UiMessage::MaximumMemoryLoaded(Ok(memory_mb)) => {
                self.maximum_memory_mb = Some(memory_mb);
                if self.message == "Checking available memory…" {
                    self.message = "Memory limit loaded.".into();
                }
                self.draw();
            }
            UiMessage::MaximumMemoryLoaded(Err(error)) => {
                self.maximum_memory_mb = None;
                self.message =
                    format!("Could not detect available memory: {error}");
                self.draw();
            }
            UiMessage::SettingsSaved(result) => match result {
                Ok(settings) => {
                    let preferred_dark_theme_changed =
                        self.settings.as_ref().is_none_or(|current| {
                            theme_label(current.theme)
                                != theme_label(settings.theme)
                        }) && is_dark_theme(settings.theme);
                    if preferred_dark_theme_changed {
                        self.preferred_dark_theme = settings.theme;
                        self.persist_preferred_dark_theme(settings.theme);
                    }
                    let native_decorations_changed =
                        self.settings.as_ref().is_some_and(|current| {
                            current.native_decorations
                                != settings.native_decorations
                        });
                    if native_decorations_changed
                        && !cfg!(target_os = "macos")
                        && let Some(window) = &self.window
                    {
                        window.set_decorations(settings.native_decorations);
                    }
                    let hide_installed_changed =
                        self.settings.as_ref().is_some_and(|old| {
                            old.hide_installed_modpacks
                                != settings.hide_installed_modpacks
                        });
                    if self.instance_tab == 2
                        && !settings.show_files_tab_in_instances
                    {
                        self.instance_tab = 0;
                    }
                    if self.instance_tab == 7
                        && !settings.show_worlds_tab_in_instances
                    {
                        self.instance_tab = 6;
                        self.worlds.clear();
                    }
                    if self.instance_tab == 8
                        && !self.global_screenshots
                        && !settings.show_screenshots_tab_in_instances
                    {
                        self.instance_tab = 6;
                        self.screenshot_viewer = false;
                    }
                    if self.global_skins
                        && !settings.show_skin_selector_in_sidebar
                    {
                        self.global_skins = false;
                    }
                    self.settings = Some(settings);
                    self.message = "Setting saved.".into();
                    if hide_installed_changed
                        && self.browse_filter == 4
                        && self.is_browse_results_page()
                    {
                        self.search_projects();
                    }
                    self.draw();
                }
                Err(error) => {
                    self.message = format!("Could not save setting: {error}");
                    self.draw();
                }
            },
            UiMessage::JavaVersionsLoaded(result) => match result {
                Ok(versions) => self.java_versions = versions,
                Err(error) => {
                    self.message =
                        format!("Could not load Java runtimes: {error}")
                }
            },
            UiMessage::JavaUpdated(result) => {
                self.java_busy = false;
                match result {
                    Ok(version) => {
                        self.message = format!(
                            "Java {} registered at {}.",
                            version.parsed_version, version.path
                        );
                        self.load_java_versions();
                    }
                    Err(error) => {
                        self.message =
                            format!("Java runtime operation failed: {error}")
                    }
                }
            }
            UiMessage::JavaPickerCancelled => {
                self.java_busy = false;
                self.message = "Java selection canceled.".into();
            }
            UiMessage::AccountsLoaded(result) => match result {
                Ok((accounts, default_account, content_accounts)) => {
                    self.account_offset = self
                        .account_offset
                        .min(accounts.len().saturating_sub(1));
                    self.minecraft_accounts = accounts;
                    self.default_account = default_account;
                    self.content_accounts = content_accounts;
                    if self.theme_sync_across_devices
                        && let Some(account) = self
                            .content_accounts
                            .iter()
                            .find(|account| account.active)
                    {
                        self.load_account_theme_preferences(
                            account.user_id.clone(),
                        );
                    }
                    self.message = format!(
                        "{} Minecraft accounts",
                        self.minecraft_accounts.len()
                    );
                }
                Err(error) => {
                    self.message =
                        format!("Could not load Minecraft accounts: {error}")
                }
            },
            UiMessage::AccountDefaultChanged(id, result) => match result {
                Ok(()) => {
                    self.default_account = Some(id);
                    self.message = "Default Minecraft account changed.".into();
                    self.load_accounts();
                }
                Err(error) => {
                    self.message =
                        format!("Could not change default account: {error}")
                }
            },
            UiMessage::MinecraftAccountRemoved(id, result) => match result {
                Ok(()) => {
                    self.message = format!("Removed Minecraft account {id}.");
                    self.load_accounts();
                }
                Err(error) => {
                    self.message =
                        format!("Could not remove Minecraft account: {error}")
                }
            },
            UiMessage::LoginFlowReady(result) => match result {
                Ok(session) => {
                    let trusted = url::Url::parse(&session.auth_request_uri)
                        .is_ok_and(|url| is_trusted_microsoft_url(&url));
                    if !self.accounts_open {
                        self.message = "Microsoft sign-in canceled.".into();
                    } else if !trusted {
                        self.message =
                            "Microsoft returned an unexpected sign-in URL."
                                .into();
                    } else if let Err(error) =
                        open::that_detached(&session.auth_request_uri)
                    {
                        self.message = format!(
                            "Could not open the system browser: {error}"
                        );
                    } else {
                        self.login_flow = Some(session);
                        self.login_code_input.clear();
                        self.input_focus = 6;
                        self.message = "Sign in in your browser, then paste the returned code here.".into();
                    }
                }
                Err(error) => {
                    self.message =
                        format!("Could not begin Microsoft sign-in: {error}")
                }
            },
            UiMessage::LoginFinished(result) => {
                self.login_flow = None;
                self.login_code_input.clear();
                self.input_focus = 0;
                match result {
                    Ok(account) => {
                        self.message =
                            format!("Signed in as {}.", account.username);
                        self.load_accounts();
                    }
                    Err(error) => {
                        self.message =
                            format!("Microsoft sign-in failed: {error}")
                    }
                }
            }
            UiMessage::ModrinthLoginReady(result) => match result {
                Ok(session) => {
                    if !self.accounts_open || self.account_tab != 1 {
                        self.message = "Modrinth sign-in canceled.".into();
                    } else if let Err(error) =
                        open::that_detached(&session.auth_request_uri)
                    {
                        self.message = format!(
                            "Could not open the system browser: {error}"
                        );
                    } else if let (Some(backend), Some(proxy)) =
                        (self.backend.clone(), self.proxy.clone())
                    {
                        self.message =
                            "Modrinth sign-in is waiting for browser approval…"
                                .into();
                        self.modrinth_login_task =
                            Some(self.runtime.spawn(async move {
                                let result = backend
                                    .finish_content_service_login(session)
                                    .await
                                    .map_err(|error| error.to_string());
                                let _ = proxy.send_event(
                                    UiMessage::ModrinthLoginFinished(result),
                                );
                            }));
                    }
                }
                Err(error) => {
                    self.message =
                        format!("Could not begin Modrinth sign-in: {error}")
                }
            },
            UiMessage::ModrinthLoginFinished(result) => {
                self.modrinth_login_task = None;
                match result {
                    Ok(account) => {
                        self.message = format!(
                            "Signed in to Modrinth as {}.",
                            account.user_id
                        );
                        self.load_accounts();
                        self.load_sidebar_friends();
                        self.load_hidden_instance_ids();
                    }
                    Err(error) => {
                        self.message =
                            format!("Modrinth sign-in failed: {error}")
                    }
                }
            }
            UiMessage::ContentAccountChanged(user_id, result) => match result {
                Ok(()) => {
                    self.message =
                        format!("Active Modrinth account set to {user_id}.");
                    self.load_accounts();
                    self.load_sidebar_friends();
                    self.load_hidden_instance_ids();
                }
                Err(error) => {
                    self.message =
                        format!("Could not change Modrinth account: {error}")
                }
            },
            UiMessage::ContentAccountRemoved(user_id, result) => match result {
                Ok(()) => {
                    self.message =
                        format!("Removed Modrinth account {user_id}.");
                    self.load_accounts();
                    self.load_sidebar_friends();
                    self.load_hidden_instance_ids();
                }
                Err(error) => {
                    self.message =
                        format!("Could not remove Modrinth account: {error}")
                }
            },
            UiMessage::ClipboardLoaded(result) => {
                if self.input_focus == 5 && self.instance_tab == 3 {
                    match result {
                        Ok(value) if value.chars().count() <= 128 => {
                            let inserted = value
                                .chars()
                                .filter(|character| !character.is_control())
                                .collect::<Vec<_>>();
                            let mut chars =
                                self.browse_query.chars().collect::<Vec<_>>();
                            if self.browse_query_select_all {
                                chars.clear();
                                self.browse_query_cursor = 0;
                                self.browse_query_select_all = false;
                            }
                            let cursor =
                                self.browse_query_cursor.min(chars.len());
                            chars.splice(
                                cursor..cursor,
                                inserted.iter().copied(),
                            );
                            if chars.len() <= 128 {
                                self.browse_query = chars.into_iter().collect();
                                self.browse_query_cursor =
                                    cursor + inserted.len();
                                self.schedule_browse_search();
                            }
                        }
                        Ok(_) => {
                            self.message = "Clipboard text is too long for project search.".into();
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not read clipboard: {error}");
                        }
                    }
                } else if self.accounts_open && self.login_flow.is_some() {
                    match result {
                        Ok(value) if value.trim().len() <= 4096 => {
                            self.login_code_input = value.trim().to_owned();
                            self.input_focus = 6;
                            self.message = "Authorization code pasted. Press Finish to continue.".into();
                        }
                        Ok(_) => self.message = "The clipboard text is too long for an authorization code.".into(),
                        Err(error) => self.message = format!("Could not read clipboard: {error}"),
                    }
                } else if self.instance_tab == 4
                    && (7..=15).contains(&self.input_focus)
                {
                    match result {
                        Ok(value) if value.len() <= 4096 => {
                            match self.paste_into_instance_setup(&value) {
                                Ok(()) => self.message = "Text pasted.".into(),
                                Err(error) => self.message = error,
                            }
                        }
                        Ok(_) => {
                            self.message = "Clipboard text is too long.".into()
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not read clipboard: {error}")
                        }
                    }
                } else if self.rename_instance_open && self.input_focus == 16 {
                    match result {
                        Ok(value) if value.trim().chars().count() <= 64 => {
                            let value = value.trim_end_matches(['\r', '\n']);
                            if self
                                .rename_instance_input
                                .chars()
                                .count()
                                .saturating_add(value.chars().count())
                                <= 64
                            {
                                self.rename_instance_input.push_str(value);
                                self.message = "Text pasted.".into();
                            } else {
                                self.message =
                                    "The pasted text is too long.".into();
                            }
                        }
                        Ok(_) => {
                            self.message = "The pasted text is too long.".into()
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not read clipboard: {error}")
                        }
                    }
                } else if self.instance_tab == 8 && self.input_focus == 18 {
                    match result {
                        Ok(value) if value.trim().chars().count() <= 64 => {
                            self.screenshot_group_name_input
                                .push_str(value.trim_end_matches(['\r', '\n']));
                            self.message = "Text pasted.".into();
                        }
                        Ok(_) => {
                            self.message =
                                "Group names must be 64 characters or fewer."
                                    .into()
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not read clipboard: {error}")
                        }
                    }
                } else if self.instance_tab == 8 && self.input_focus == 19 {
                    match result {
                        Ok(value) if value.trim().chars().count() <= 128 => {
                            self.screenshot_search_query
                                .push_str(value.trim_end_matches(['\r', '\n']));
                            self.screenshot_offset = 0;
                            self.selected_screenshot = None;
                            self.selected_screenshot_ids.clear();
                        }
                        Ok(_) => {
                            self.message = "Search text is too long.".into()
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not read clipboard: {error}")
                        }
                    }
                } else if (self.delete_confirmation_open
                    || self.bulk_delete_confirmation_open)
                    && self.input_focus == 17
                {
                    match result {
                        Ok(value) if value.trim().chars().count() <= 512 => {
                            let value = value.trim_end_matches(['\r', '\n']);
                            if self
                                .delete_confirmation_input
                                .chars()
                                .count()
                                .saturating_add(value.chars().count())
                                <= 512
                            {
                                self.delete_confirmation_input.push_str(value);
                                self.message =
                                    "Confirmation text pasted.".into();
                            } else {
                                self.message =
                                    "The pasted text is too long.".into();
                            }
                        }
                        Ok(_) => {
                            self.message = "The pasted text is too long.".into()
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not read clipboard: {error}")
                        }
                    }
                } else if self.shared_instance_invite_open
                    && self.input_focus == 20
                {
                    match result {
                        Ok(value) if value.trim().chars().count() <= 64 => {
                            self.shared_instance_user_query
                                .push_str(value.trim());
                        }
                        Ok(_) => {
                            self.message =
                                "The pasted search is too long.".into()
                        }
                        Err(error) => {
                            self.message =
                                format!("Could not read clipboard: {error}")
                        }
                    }
                }
            }
            UiMessage::EventsLagged(count) => {
                self.message = format!(
                    "Recovered library state after missing {count} events."
                );
                self.refresh_running_processes();
            }
            UiMessage::RefreshInstances => {
                self.refresh_instances();
                self.refresh_groups();
            }
            UiMessage::Notice(message) => self.message = message,
            UiMessage::Launch(id, server, world) => {
                self.launch_instance(id, "Instance".to_string(), server, world)
            }
            UiMessage::Import(path) => self.import_pack(path),
            UiMessage::LauncherInstances(result) => match result {
                Ok(source) => {
                    self.launcher_import_busy = false;
                    self.launcher_import_offset = 0;
                    self.launcher_import_focus = 0;
                    self.message = format!(
                        "{} importable instance(s) found.",
                        source.folders.len()
                    );
                    self.launcher_import_source = Some(source);
                    self.launcher_import_open = true;
                }
                Err(error) => {
                    self.launcher_import_busy = false;
                    self.message =
                        format!("Could not read launcher instances: {error}")
                }
            },
            UiMessage::LauncherImportFinished(name, result) => {
                self.launcher_import_busy = false;
                self.message = match result {
                    Ok(()) => format!("Imported {name} successfully."),
                    Err(error) => format!("Could not import {name}: {error}"),
                };
                self.launcher_import_open = false;
                self.launcher_import_source = None;
                self.refresh_instances();
            }
            UiMessage::MinecraftSkinsLoaded(result) => match result {
                Ok(skins) => {
                    self.minecraft_skin_scroll_pixels = 0.0;
                    self.minecraft_skin_scroll_target_pixels = 0.0;
                    self.minecraft_skin_preview_failures.clear();
                    self.message =
                        format!("{} Minecraft skins available.", skins.len());
                    self.minecraft_skins = skins;
                    let previews = self.minecraft_skins.clone();
                    for skin in previews {
                        self.request_minecraft_skin_preview(skin);
                    }
                }
                Err(error) => {
                    self.message =
                        format!("Could not load Minecraft skins: {error}")
                }
            },
            UiMessage::MinecraftSkinPreviewLoaded(key, result) => {
                self.minecraft_skin_previews_pending.remove(&key);
                match result {
                    Ok(preview) => {
                        self.minecraft_skin_previews
                            .insert(key.clone(), preview);
                        cap_image_cache(
                            &mut self.minecraft_skin_previews,
                            &key,
                            SKIN_PREVIEW_CACHE_BUDGET,
                        );
                    }
                    Err(_) => {
                        self.minecraft_skin_preview_failures.insert(key);
                    }
                }
            }
            UiMessage::MinecraftSkinActionFinished(label, result) => {
                match result {
                    Ok(()) => {
                        self.message = format!(
                            "Applied {label} to your Minecraft profile."
                        );
                        self.load_minecraft_skins();
                    }
                    Err(error) if error == "Skin selection canceled." => {
                        self.message = error;
                    }
                    Err(error) => {
                        self.message =
                            format!("Could not apply {label}: {error}");
                    }
                }
            }
            UiMessage::MinecraftSkinRemoved(name, result) => match result {
                Ok(()) => {
                    self.message = format!("Removed saved skin {name}.");
                    self.load_minecraft_skins();
                }
                Err(error) => {
                    self.message =
                        format!("Could not remove saved skin: {error}");
                }
            },
            UiMessage::Minimize => {
                if let Some(window) = &self.window {
                    window.set_minimized(true);
                }
            }
            UiMessage::CloseWindow => {}
            UiMessage::Restore => {
                if let Some(window) = &self.window {
                    window.set_minimized(false);
                    let _ = window.focus_window();
                }
            }
            UiMessage::InstallJob(snapshot) => {
                self.active_job = Some((*snapshot).clone());
                self.job_control_pending = false;
                self.install_progress = snapshot
                    .progress
                    .as_ref()
                    .filter(|progress| progress.total > 0)
                    .map(|progress| {
                        progress.current as f32 / progress.total as f32
                    });
                self.message = match snapshot.progress.as_ref() {
                    Some(progress) if progress.total > 0 => format!(
                        "{:?}: {}%",
                        snapshot.phase,
                        progress.current.saturating_mul(100) / progress.total
                    ),
                    _ => format!("{:?}…", snapshot.phase),
                };
                if !snapshot.status.is_finished() {
                    self.message.push_str(if snapshot.paused {
                        "  [P] RESUME  [X] CANCEL"
                    } else {
                        "  [P] PAUSE  [X] CANCEL"
                    });
                } else {
                    self.active_job = None;
                    self.install_progress = None;
                }
            }
            UiMessage::JobActionResult(message) => {
                self.job_control_pending = false;
                self.message = message;
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
