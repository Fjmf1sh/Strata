use super::*;

impl Launcher {
    pub(super) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self.browse_install_override.is_some() {
            return;
        }
        let raw_delta_y = match &delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(*y),
            MouseScrollDelta::PixelDelta(position) => position.y,
        };
        if self.browse_install_choice.is_some() {
            if let Some(window) = &self.window {
                let size = window.inner_size();
                let layout = InstallInstancePickerLayout::new(
                    size.width as usize,
                    size.height as usize,
                    self.instances.len(),
                );
                if layout.list.contains(self.cursor.0, self.cursor.1) {
                    let pixels = match delta {
                        MouseScrollDelta::LineDelta(_, steps) => {
                            f64::from(steps)
                                * INSTALL_INSTANCE_ROW_HEIGHT as f64
                        }
                        MouseScrollDelta::PixelDelta(position) => position.y,
                    };
                    self.browse_install_scroll_target = (self
                        .browse_install_scroll_target
                        - pixels)
                        .clamp(0.0, layout.max_scroll(self.instances.len()));
                    window.request_redraw();
                }
            }
            return;
        }
        let gallery_scroll_delta = match &delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 58.0,
            MouseScrollDelta::PixelDelta(position) => position.y,
        };
        let amount = match &delta {
            MouseScrollDelta::LineDelta(_, y) => y.round() as isize,
            MouseScrollDelta::PixelDelta(position) => {
                (position.y / 58.0).round() as isize
            }
        };
        if amount != 0 || raw_delta_y.abs() > 0.01 {
            if self.browse_gallery_viewer_open {
                self.browse_gallery_zoom = gallery_zoom_after_scroll(
                    self.browse_gallery_zoom,
                    gallery_scroll_delta,
                );
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return;
            }
            let window_width = self
                .window
                .as_ref()
                .map_or(0.0, |window| window.inner_size().width as f64);
            if self.friend_incoming_open {
                let width = self
                    .window
                    .as_ref()
                    .map_or(1024, |window| window.inner_size().width as usize);
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height as usize);
                let panel_width = 560usize.min(width.saturating_sub(32));
                let panel_height = 500usize.min(height.saturating_sub(32));
                let panel_x = width.saturating_sub(panel_width) / 2;
                let panel_y = height.saturating_sub(panel_height) / 2;
                let list_top = panel_y + 82;
                let list_bottom = panel_y + panel_height.saturating_sub(52);
                let incoming_count =
                    sidebar_incoming_friend_indices(&self.sidebar_friends)
                        .len();
                if (panel_x as f64..(panel_x + panel_width) as f64)
                    .contains(&self.cursor.0)
                    && (list_top as f64..list_bottom as f64)
                        .contains(&self.cursor.1)
                {
                    let pixel_delta = match &delta {
                        MouseScrollDelta::PixelDelta(position) => position.y,
                        MouseScrollDelta::LineDelta(_, y) => {
                            f64::from(*y) * 64.0
                        }
                    };
                    let max_scroll = friend_incoming_modal_max_scroll(
                        incoming_count,
                        height,
                    );
                    self.friend_incoming_scroll_target_pixels = (self
                        .friend_incoming_scroll_target_pixels
                        - pixel_delta)
                        .clamp(0.0, max_scroll);
                }
            } else if self.should_show_right_sidebar()
                && self.cursor.0 >= window_width - 300.0
                && self.cursor.1 >= 48.0
            {
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height as usize);
                let width = self
                    .window
                    .as_ref()
                    .map_or(1024, |window| window.inner_size().width)
                    as usize;
                if self.browse_filter_sidebar_visible() {
                    let count = self.browse_filter_menu_options().len();
                    let layout = browse_filter_sidebar_layout(
                        width,
                        height,
                        self.browse_filter_group,
                        count,
                        self.browse_filter_group_count(),
                    );
                    if layout.options.contains(self.cursor.0, self.cursor.1) {
                        let pixel_delta = match &delta {
                            MouseScrollDelta::PixelDelta(position) => {
                                position.y
                            }
                            MouseScrollDelta::LineDelta(_, y) => {
                                f64::from(*y) * BROWSE_FILTER_ROW_HEIGHT as f64
                            }
                        };
                        self.browse_filter_scroll_target_pixels = (self
                            .browse_filter_scroll_target_pixels
                            - pixel_delta)
                            .clamp(0.0, layout.max_scroll(count));
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    return;
                }
                let max_scroll = if let (Some(project), Some(details)) = (
                    self.browse_selected.as_ref(),
                    self.browse_details.as_ref(),
                ) {
                    draw::project_sidebar_content_height(
                        project, details, width,
                    )
                    .saturating_sub(height.saturating_sub(48))
                        as f64
                } else {
                    sidebar_scroll_max(
                        self.news_articles.len(),
                        &self.sidebar_friends,
                        height,
                        self.onboarding_checklist.as_ref(),
                        &self.sidebar_friend_query,
                        sidebar_collapsed_groups(self.settings.as_ref()),
                    ) as f64
                };
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 58.0,
                };
                self.sidebar_scroll_target_pixels =
                    (self.sidebar_scroll_target_pixels - pixel_delta)
                        .clamp(0.0, max_scroll);
                self.sidebar_scroll_pixels =
                    self.sidebar_scroll_pixels.min(max_scroll);
            } else if self.settings_open && !self.java_open {
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height as usize);
                let section_rows = SETTINGS_SECTION_ROW_INDICES[self
                    .settings_section
                    .min(SETTINGS_SECTION_ROW_INDICES.len() - 1)];
                let width = self
                    .window
                    .as_ref()
                    .map_or(1024, |window| window.inner_size().width as usize);
                let row_width = settings_row_layout(width).row_width;
                let theme_selector_extra_height = if self.settings_section == 0
                {
                    settings_theme_selector_extra_height(row_width)
                } else {
                    0
                };
                let content_height = section_rows
                    .len()
                    .saturating_mul(50)
                    .saturating_add(theme_selector_extra_height);
                let viewport_height = height.saturating_sub(202);
                let max_scroll =
                    content_height.saturating_sub(viewport_height) as f64;
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 50.0,
                };
                self.settings_scroll_target_pixels =
                    (self.settings_scroll_target_pixels - pixel_delta)
                        .clamp(0.0, max_scroll);
                self.settings_scroll_pixels =
                    self.settings_scroll_pixels.min(max_scroll);
            } else if self.global_skins {
                let width = self
                    .window
                    .as_ref()
                    .map_or(1024, |window| window.inner_size().width as usize);
                let layout_width =
                    width.saturating_sub(if self.should_show_right_sidebar() {
                        300
                    } else {
                        0
                    });
                let (columns, _, card_height, gap) =
                    minecraft_skin_grid_layout(layout_width);
                let visible = self
                    .window
                    .as_ref()
                    .map_or(768, |window| window.inner_size().height as usize)
                    .saturating_sub(208)
                    / (card_height + gap).max(1);
                let total_rows =
                    self.minecraft_skins.len().div_ceil(columns.max(1));
                let max_offset = total_rows.saturating_sub(visible.max(1));
                let row_height = (card_height + gap) as f64;
                let max_scroll = max_offset as f64 * row_height;
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => {
                        f64::from(*y) * row_height
                    }
                };
                self.minecraft_skin_scroll_target_pixels =
                    (self.minecraft_skin_scroll_target_pixels - pixel_delta)
                        .clamp(0.0, max_scroll);
                self.minecraft_skin_scroll_pixels =
                    self.minecraft_skin_scroll_pixels.min(max_scroll);
            } else if self.global_user_profile {
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height as usize);
                let project_count = if self.user_profile_show_collections {
                    self.user_profile_collections.len()
                } else {
                    self.user_profile_projects
                        .iter()
                        .filter(|project| {
                            self.user_profile_type_filter.as_deref().is_none_or(
                                |project_type| {
                                    profile_project_matches(
                                        project,
                                        project_type,
                                    )
                                },
                            )
                        })
                        .count()
                };
                let max_scroll =
                    profile_projects_max_scroll(project_count, height);
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 62.0,
                };
                self.user_profile_scroll_target_pixels =
                    (self.user_profile_scroll_target_pixels - pixel_delta)
                        .clamp(0.0, max_scroll);
                self.user_profile_scroll_pixels =
                    self.user_profile_scroll_pixels.min(max_scroll);
            } else if self.global_hosting
                && self.hosting_server_id.is_some()
                && self.hosting_tab == 0
            {
                let (width, height) =
                    self.window.as_ref().map_or((1024, 720), |window| {
                        let size = window.inner_size();
                        (size.width as usize, size.height as usize)
                    });
                let bounds = hosting_console_log_bounds(
                    width,
                    height,
                    self.should_show_right_sidebar(),
                );
                if bounds.contains(self.cursor.0, self.cursor.1) {
                    let pixel_delta = match &delta {
                        MouseScrollDelta::PixelDelta(position) => position.y,
                        MouseScrollDelta::LineDelta(_, y) => {
                            f64::from(*y) * 48.0
                        }
                    };
                    let max_scroll = hosting_console_max_scroll(
                        self.hosting_console_lines.len(),
                        bounds.height,
                    );
                    self.hosting_console_scroll_target_pixels = (self
                        .hosting_console_scroll_target_pixels
                        - pixel_delta)
                        .clamp(0.0, max_scroll);
                    self.hosting_console_scroll_pixels =
                        self.hosting_console_scroll_pixels.min(max_scroll);
                    self.hosting_console_follow_tail = self
                        .hosting_console_scroll_target_pixels
                        >= max_scroll - 0.5;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            } else if self.global_hosting
                && self.hosting_server_id.is_some()
                && self.hosting_tab == 1
                && !self.hosting_backup_create_open
                && self.cursor.1
                    >= if self.hosting_backup_options_open {
                        436.0
                    } else if self.hosting_backup_show_deleted {
                        328.0
                    } else {
                        296.0
                    }
            {
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height as usize);
                let list_top = if self.hosting_backup_options_open {
                    436
                } else if self.hosting_backup_show_deleted {
                    328
                } else {
                    296
                };
                let viewport = height.saturating_sub(list_top + 54);
                let query = self.hosting_backup_search.trim().to_lowercase();
                let matching_count =
                    self.hosting_backups.as_ref().map_or(0, |backups| {
                        if self.hosting_backup_show_deleted {
                            backups
                                .deleted_backups
                                .iter()
                                .filter(|backup| {
                                    hosting_deleted_backup_matches(
                                        backup,
                                        &query,
                                        self.hosting_backup_deleted_reason_filter,
                                        self.hosting_backup_deleted_type_filter,
                                        self.hosting_backup_deleted_recoverability_filter,
                                    )
                                })
                                .count()
                        } else {
                            backups
                                .backups
                                .iter()
                                .filter(|backup| {
                                    backup.status == "done"
                                        && hosting_backup_search_matches(
                                            &backup.name,
                                            &query,
                                        )
                                })
                                .count()
                        }
                    });
                let max_scroll =
                    matching_count.saturating_mul(62).saturating_sub(viewport)
                        as f64;
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 62.0,
                };
                self.hosting_backup_scroll_target_pixels =
                    (self.hosting_backup_scroll_target_pixels - pixel_delta)
                        .clamp(0.0, max_scroll);
                self.hosting_backup_scroll_pixels =
                    self.hosting_backup_scroll_pixels.min(max_scroll);
            } else if self.global_hosting
                && self.hosting_server_id.is_some()
                && self.hosting_tab == 2
                && self.cursor.1
                    >= if self.hosting_access_invite_open {
                        360.0
                    } else {
                        182.0
                    }
            {
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height as usize);
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => {
                        f64::from(*y)
                            * if self.hosting_access_invite_open {
                                54.0
                            } else {
                                62.0
                            }
                    }
                };
                if self.hosting_access_invite_open {
                    let existing_ids = self
                        .hosting_access_users
                        .iter()
                        .map(|member| member.user.id.to_lowercase())
                        .collect::<HashSet<_>>();
                    let candidate_count = self
                        .hosting_access_search_results
                        .iter()
                        .filter(|user| {
                            !existing_ids.contains(&user.id.to_lowercase())
                        })
                        .count();
                    let viewport = height.saturating_sub(436);
                    let max_scroll = candidate_count
                        .saturating_mul(54)
                        .saturating_sub(viewport)
                        as f64;
                    self.hosting_access_search_scroll_target_pixels = (self
                        .hosting_access_search_scroll_target_pixels
                        - pixel_delta)
                        .clamp(0.0, max_scroll);
                    self.hosting_access_search_scroll_pixels = self
                        .hosting_access_search_scroll_pixels
                        .min(max_scroll);
                } else {
                    let viewport = height.saturating_sub(278);
                    let max_scroll = self
                        .hosting_access_users
                        .len()
                        .saturating_mul(62)
                        .saturating_sub(viewport)
                        as f64;
                    self.hosting_access_scroll_target_pixels = (self
                        .hosting_access_scroll_target_pixels
                        - pixel_delta)
                        .clamp(0.0, max_scroll);
                    self.hosting_access_scroll_pixels =
                        self.hosting_access_scroll_pixels.min(max_scroll);
                }
            } else if self.global_hosting
                && self.hosting_server_id.is_none()
                && self.cursor.1 >= 148.0
            {
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height as usize);
                let max_scroll =
                    self.hosting_servers
                        .len()
                        .saturating_sub(1)
                        .saturating_mul(82)
                        .saturating_add(256)
                        .saturating_sub(height) as f64;
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 82.0,
                };
                self.hosting_scroll_target_pixels =
                    (self.hosting_scroll_target_pixels - pixel_delta)
                        .clamp(0.0, max_scroll);
                self.hosting_scroll_pixels =
                    self.hosting_scroll_pixels.min(max_scroll);
            } else if self.launcher_import_open
                && self.launcher_import_source.is_some()
            {
                if let Some(source) = &self.launcher_import_source {
                    let pixel_delta = match &delta {
                        MouseScrollDelta::PixelDelta(position) => position.y,
                        MouseScrollDelta::LineDelta(_, y) => {
                            f64::from(*y) * 44.0
                        }
                    };
                    let max_scroll =
                        source.folders.len().saturating_sub(7) as f64 * 44.0;
                    self.launcher_import_scroll_target_pixels = (self
                        .launcher_import_scroll_target_pixels
                        - pixel_delta)
                        .clamp(0.0, max_scroll);
                }
            } else if self.library_filter_open {
                let max_scroll =
                    self.library_game_version_options().len().saturating_sub(3)
                        as f64
                        * 24.0;
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 24.0,
                };
                self.library_game_version_filter_scroll_target_pixels = (self
                    .library_game_version_filter_scroll_target_pixels
                    - pixel_delta)
                    .clamp(0.0, max_scroll);
            } else if self.screenshot_group_picker_open {
                let max_scroll = self.screenshot_groups.len().saturating_sub(8)
                    as f64
                    * 34.0;
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 34.0,
                };
                self.screenshot_group_scroll_target_pixels =
                    (self.screenshot_group_scroll_target_pixels - pixel_delta)
                        .clamp(0.0, max_scroll);
            } else if self.accounts_open
                && (self.account_tab == 1 || self.login_flow.is_none())
            {
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height)
                    as usize;
                let visible = (height.saturating_sub(258) / 58).min(12);
                let account_count = if self.account_tab == 1 {
                    self.content_accounts.len()
                } else {
                    self.minecraft_accounts.len()
                };
                let max_scroll = account_count
                    .saturating_sub(visible)
                    .saturating_mul(58) as f64;
                let pixel_delta = match &delta {
                    MouseScrollDelta::PixelDelta(position) => position.y,
                    MouseScrollDelta::LineDelta(_, y) => f64::from(*y) * 58.0,
                };
                self.account_scroll_target_pixels =
                    (self.account_scroll_target_pixels - pixel_delta)
                        .clamp(0.0, max_scroll);
                self.account_scroll_pixels =
                    self.account_scroll_pixels.min(max_scroll);
            } else if self.selected_instance.is_some() {
                if self.instance_tab == 9
                    && self.shared_instance_preview.is_some()
                {
                    let height = self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as usize;
                    let visible = (height.saturating_sub(452) / 28).max(1);
                    let count = self
                        .shared_instance_preview
                        .as_ref()
                        .map_or(0, |preview| preview.files.len());
                    let max_scroll =
                        count.saturating_sub(visible) as f64 * 28.0;
                    let pixel_delta = match &delta {
                        MouseScrollDelta::PixelDelta(position) => position.y,
                        MouseScrollDelta::LineDelta(_, y) => {
                            f64::from(*y) * 28.0
                        }
                    };
                    self.shared_instance_preview_scroll_target_pixels = (self
                        .shared_instance_preview_scroll_target_pixels
                        - pixel_delta)
                        .clamp(0.0, max_scroll);
                    self.shared_instance_preview_scroll_pixels = self
                        .shared_instance_preview_scroll_pixels
                        .min(max_scroll);
                } else if self.instance_tab == 2 {
                    if self.file_preview.is_some() {
                        let line_count = self
                            .file_preview
                            .as_ref()
                            .map_or(0, |(_, text)| text.lines().count());
                        let height =
                            self.window.as_ref().map_or(720, |window| {
                                window.inner_size().height as usize
                            });
                        let visible = (height.saturating_sub(220) / 12).max(1);
                        let max_scroll =
                            line_count.saturating_sub(visible) as f64 * 12.0;
                        let pixel_delta = match &delta {
                            MouseScrollDelta::PixelDelta(position) => {
                                position.y
                            }
                            MouseScrollDelta::LineDelta(_, y) => {
                                f64::from(*y) * 12.0
                            }
                        };
                        self.file_preview_scroll_target_pixels = (self
                            .file_preview_scroll_target_pixels
                            + pixel_delta)
                            .clamp(0.0, max_scroll);
                        self.file_preview_scroll_pixels =
                            self.file_preview_scroll_pixels.min(max_scroll);
                    } else {
                        let height =
                            self.window.as_ref().map_or(720, |window| {
                                window.inner_size().height as usize
                            });
                        let visible =
                            (height.saturating_sub(208) / 42).min(14).max(1);
                        if self.file_offset != self.file_list_scroll_synced {
                            self.file_list_scroll_target_pixels =
                                (self.file_offset * 42) as f64;
                            self.file_list_scroll_pixels =
                                self.file_list_scroll_target_pixels;
                        }
                        let max_scroll = self
                            .file_items
                            .len()
                            .saturating_sub(visible)
                            .saturating_mul(42)
                            as f64;
                        let pixel_delta = match &delta {
                            MouseScrollDelta::PixelDelta(position) => {
                                position.y
                            }
                            MouseScrollDelta::LineDelta(_, y) => {
                                f64::from(*y) * 42.0
                            }
                        };
                        self.file_list_scroll_target_pixels =
                            (self.file_list_scroll_target_pixels - pixel_delta)
                                .clamp(0.0, max_scroll);
                        self.file_list_scroll_pixels =
                            self.file_list_scroll_pixels.min(max_scroll);
                        self.file_offset = (self.file_list_scroll_target_pixels
                            / 42.0)
                            .floor()
                            as usize;
                        self.file_list_scroll_synced = self.file_offset;
                    }
                } else if self.instance_tab == 1 {
                    let height = self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as usize;
                    let filter_top =
                        instance_console_search_bounds(window_width as usize).y
                            + 38;
                    let body_top = instance_log_filter_bottom(
                        window_width as usize,
                        filter_top,
                        native_log_present_levels(&self.console_text),
                    ) + 8;
                    let visible =
                        (height.saturating_sub(body_top + 40) / 12).max(1);
                    let max_scroll = native_log_match_count(
                        &self.console_text,
                        &self.log_search_query,
                        self.log_level_filter,
                    )
                    .saturating_sub(visible)
                        as f64
                        * 12.0;
                    let pixel_delta = match &delta {
                        MouseScrollDelta::PixelDelta(position) => position.y,
                        MouseScrollDelta::LineDelta(_, y) => {
                            f64::from(*y) * 12.0
                        }
                    };
                    self.console_scroll_target_pixels =
                        (self.console_scroll_target_pixels + pixel_delta)
                            .clamp(0.0, max_scroll);
                    self.console_scroll_pixels =
                        self.console_scroll_pixels.min(max_scroll);
                    self.console_scroll =
                        (self.console_scroll_target_pixels / 12.0).floor()
                            as usize;
                    self.console_scroll_synced = self.console_scroll;
                } else if self.instance_tab == 3 {
                    let height = self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as usize;
                    let layout_width = (window_width as usize).saturating_sub(
                        if self.should_show_right_sidebar() {
                            300
                        } else {
                            0
                        },
                    );
                    let save_menu_x = layout_width
                        .saturating_sub(220)
                        .saturating_sub(228)
                        .max(292);
                    if self.browse_selected.is_some()
                        && self.browse_save_menu_open
                        && (save_menu_x as f64..(save_menu_x + 220) as f64)
                            .contains(&self.cursor.0)
                        && (402.0..556.0).contains(&self.cursor.1)
                    {
                        let max_scroll = self
                            .browse_save_collections
                            .len()
                            .saturating_mul(34)
                            .saturating_sub(154)
                            as f64;
                        let pixel_delta = match &delta {
                            MouseScrollDelta::PixelDelta(position) => {
                                position.y
                            }
                            MouseScrollDelta::LineDelta(_, y) => {
                                f64::from(*y) * 34.0
                            }
                        };
                        self.browse_save_scroll_pixels =
                            (self.browse_save_scroll_pixels - pixel_delta)
                                .clamp(0.0, max_scroll);
                    } else if self.browse_selected.is_none()
                        && self.browse_filter_menu_open
                        && !self.browse_filter_sidebar_visible()
                    {
                        let width = (window_width as usize).saturating_sub(
                            if self.should_show_right_sidebar() {
                                300
                            } else {
                                0
                            },
                        );
                        let panel = browse_filter_popup_bounds(width, height);
                        let (_, options) = browse_popup_filter_rows(
                            panel,
                            self.browse_filter_group,
                        );
                        let max_scroll = self
                            .browse_filter_menu_options()
                            .len()
                            .saturating_mul(24)
                            .saturating_sub(options.height);
                        let pixel_delta = match &delta {
                            MouseScrollDelta::PixelDelta(position) => {
                                position.y
                            }
                            MouseScrollDelta::LineDelta(_, y) => {
                                f64::from(*y) * 24.0
                            }
                        };
                        self.browse_filter_scroll_target_pixels = (self
                            .browse_filter_scroll_target_pixels
                            - pixel_delta)
                            .clamp(0.0, max_scroll as f64);
                    } else if self.browse_detail_tab == 1
                        && let (Some(details), Some(group)) = (
                            self.browse_details.as_ref(),
                            self.browse_detail_filter_menu_group,
                        )
                        && let Some((_, menu_x, _)) =
                            browse_version_filter_buttons(
                                self.window
                                    .as_ref()
                                    .map_or(1100, |window| {
                                        window.inner_size().width
                                    })
                                    .saturating_sub(
                                        if self.should_show_right_sidebar() {
                                            300
                                        } else {
                                            0
                                        },
                                    ) as usize,
                            )
                            .into_iter()
                            .find(|(button_group, _, _)| *button_group == group)
                        && (self.cursor.0 >= menu_x as f64
                            && self.cursor.0 < (menu_x + 248) as f64
                            && self.cursor.1 >= 474.0
                            && self.cursor.1 < 690.0)
                    {
                        let option_count =
                            browse_version_filter_options(details, group).len();
                        let max_offset = option_count.saturating_sub(7);
                        self.browse_detail_filter_option_offset = if amount > 0
                        {
                            self.browse_detail_filter_option_offset
                                .saturating_sub(amount as usize)
                        } else {
                            self.browse_detail_filter_option_offset
                                .saturating_add(amount.unsigned_abs())
                                .min(max_offset)
                        };
                    } else if let Some(details) = &self.browse_details {
                        if self.browse_detail_tab == 0 {
                            let width =
                                self.window.as_ref().map_or(1100, |window| {
                                    window.inner_size().width
                                }) as usize;
                            let layout_width = width.saturating_sub(
                                if self.should_show_right_sidebar() {
                                    300
                                } else {
                                    0
                                },
                            );
                            let max_chars = layout_width
                                .saturating_sub(324)
                                .div_euclid(7)
                                .max(24);
                            let selected_id = self
                                .browse_selected
                                .as_ref()
                                .map(|project| project.id.clone());
                            let cached_line_count = self
                                .browse_description_lines
                                .as_ref()
                                .filter(|(id, cached_width, _, _, _, _)| {
                                    Some(id) == selected_id.as_ref()
                                        && *cached_width == max_chars
                                })
                                .map(|(_, _, lines, _, _, _)| {
                                    super::draw::browse_body_height(lines)
                                });
                            let content_height = cached_line_count
                                .unwrap_or_else(|| {
                                    let lines = super::draw::wrap_browse_body(
                                        &details.body,
                                        max_chars,
                                    );
                                    super::draw::browse_body_height(&lines)
                                });
                            let viewport = height.saturating_sub(458);
                            let max_scroll =
                                content_height.saturating_sub(viewport) as f64;
                            let pixel_delta = match &delta {
                                MouseScrollDelta::PixelDelta(position) => {
                                    position.y
                                }
                                MouseScrollDelta::LineDelta(_, y) => {
                                    f64::from(*y) * 54.0
                                }
                            };
                            self.browse_detail_scroll_target_pixels = (self
                                .browse_detail_scroll_target_pixels
                                - pixel_delta)
                                .clamp(0.0, max_scroll);
                            self.browse_detail_scroll_pixels = self
                                .browse_detail_scroll_pixels
                                .min(max_scroll);
                        } else if self.browse_detail_tab == 1 {
                            let viewport = height.saturating_sub(494);
                            let filtered = browse_filtered_version_indices(
                                details,
                                &self.browse_detail_selected_channels,
                                &self.browse_detail_selected_game_versions,
                                &self.browse_detail_selected_loaders,
                            );
                            let max_scroll = filtered
                                .len()
                                .saturating_mul(56)
                                .saturating_sub(viewport)
                                as f64;
                            let pixel_delta = match &delta {
                                MouseScrollDelta::PixelDelta(position) => {
                                    position.y
                                }
                                MouseScrollDelta::LineDelta(_, y) => {
                                    f64::from(*y) * 56.0
                                }
                            };
                            self.browse_detail_scroll_target_pixels = (self
                                .browse_detail_scroll_target_pixels
                                - pixel_delta)
                                .clamp(0.0, max_scroll);
                            self.browse_detail_scroll_pixels = self
                                .browse_detail_scroll_pixels
                                .min(max_scroll);
                        } else if self.browse_gallery_viewer_open {
                            let factor =
                                1.1_f32.powi(amount.unsigned_abs() as i32);
                            self.browse_gallery_zoom = if amount > 0 {
                                (self.browse_gallery_zoom / factor).max(1.0)
                            } else {
                                (self.browse_gallery_zoom * factor).min(5.0)
                            };
                        } else {
                            let width =
                                self.window.as_ref().map_or(1100, |window| {
                                    window.inner_size().width
                                }) as usize;
                            let layout_width = width.saturating_sub(
                                if self.should_show_right_sidebar() {
                                    300
                                } else {
                                    0
                                },
                            );
                            let total_height =
                                super::draw::browse_gallery_grid_metrics(
                                    layout_width,
                                    details.gallery.len(),
                                )
                                .3;
                            let viewport = height.saturating_sub(458);
                            let max_scroll =
                                total_height.saturating_sub(viewport) as f64;
                            let pixel_delta = match &delta {
                                MouseScrollDelta::PixelDelta(position) => {
                                    position.y
                                }
                                MouseScrollDelta::LineDelta(_, y) => {
                                    f64::from(*y) * 58.0
                                }
                            };
                            self.browse_detail_scroll_target_pixels = (self
                                .browse_detail_scroll_target_pixels
                                - pixel_delta)
                                .clamp(0.0, max_scroll);
                            self.browse_detail_scroll_pixels = self
                                .browse_detail_scroll_pixels
                                .min(max_scroll);
                        }
                    } else {
                        let width = (window_width as usize).saturating_sub(
                            if self.should_show_right_sidebar() {
                                300
                            } else {
                                0
                            },
                        );
                        let layout = BrowseResultsLayout::new(width, height);
                        let max_scroll =
                            layout.max_scroll(self.browse_results.len());
                        let pixel_delta = match &delta {
                            MouseScrollDelta::PixelDelta(position) => {
                                position.y
                            }
                            MouseScrollDelta::LineDelta(_, y) => {
                                f64::from(*y) * 58.0
                            }
                        };
                        self.browse_scroll_target_pixels =
                            (self.browse_scroll_target_pixels - pixel_delta)
                                .clamp(0.0, max_scroll);
                        self.browse_scroll_pixels =
                            self.browse_scroll_pixels.min(max_scroll);
                    }
                } else if self.instance_tab == 5 {
                    let height = self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as usize;
                    if self.selected_log.is_some() {
                        let width =
                            self.window.as_ref().map_or(1024, |window| {
                                window.inner_size().width as usize
                            });
                        let search_bounds = instance_log_search_bounds(
                            width,
                            self.log_analysis.is_some(),
                        );
                        let filter_top =
                            search_bounds.y + search_bounds.height + 8;
                        let log_body_top = instance_log_filter_bottom(
                            width,
                            filter_top,
                            native_log_present_levels(&self.log_text),
                        ) + 8;
                        let visible =
                            height.saturating_sub(log_body_top + 38) / 12;
                        let max_scroll = native_log_match_count(
                            &self.log_text,
                            &self.log_search_query,
                            self.log_level_filter,
                        )
                        .saturating_sub(visible)
                            as f64
                            * 12.0;
                        let pixel_delta = match &delta {
                            MouseScrollDelta::PixelDelta(position) => {
                                position.y
                            }
                            MouseScrollDelta::LineDelta(_, y) => {
                                f64::from(*y) * 12.0
                            }
                        };
                        self.log_scroll_target_pixels =
                            (self.log_scroll_target_pixels + pixel_delta)
                                .clamp(0.0, max_scroll);
                        self.log_scroll_pixels =
                            self.log_scroll_pixels.min(max_scroll);
                        self.log_scroll = (self.log_scroll_target_pixels / 12.0)
                            .floor()
                            as usize;
                        self.log_scroll_synced = self.log_scroll;
                    } else {
                        let visible = (height.saturating_sub(208) / 42).min(14);
                        let max_scroll = self
                            .instance_logs
                            .len()
                            .saturating_sub(visible)
                            .saturating_mul(42)
                            as f64;
                        let pixel_delta = match &delta {
                            MouseScrollDelta::PixelDelta(position) => {
                                position.y
                            }
                            MouseScrollDelta::LineDelta(_, y) => {
                                f64::from(*y) * 42.0
                            }
                        };
                        self.log_scroll_target_pixels =
                            (self.log_scroll_target_pixels - pixel_delta)
                                .clamp(0.0, max_scroll);
                        self.log_scroll_pixels =
                            self.log_scroll_pixels.min(max_scroll);
                        self.log_scroll = (self.log_scroll_target_pixels / 42.0)
                            .floor()
                            as usize;
                        self.log_scroll_synced = self.log_scroll;
                    }
                } else if self.instance_tab == 8 {
                    let height = self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as usize;
                    let screenshot_count = self.screenshot_order().len();
                    let width = self
                        .window
                        .as_ref()
                        .map_or(760, |window| window.inner_size().width)
                        as usize;
                    let layout_width = width.saturating_sub(
                        if self.should_show_right_sidebar() {
                            300
                        } else {
                            0
                        },
                    );
                    let (columns, _, _, card_height) =
                        screenshot_grid_layout(layout_width);
                    let stride = card_height.saturating_add(12);
                    if self.screenshot_offset != self.screenshot_scroll_synced {
                        self.screenshot_scroll_target_pixels =
                            (self.screenshot_offset / columns * stride) as f64;
                        self.screenshot_scroll_pixels =
                            self.screenshot_scroll_target_pixels;
                    }
                    let max_scroll = screenshot_max_scroll(
                        height,
                        screenshot_count,
                        columns,
                        stride,
                    );
                    let pixel_delta = match &delta {
                        MouseScrollDelta::PixelDelta(position) => position.y,
                        MouseScrollDelta::LineDelta(_, y) => {
                            f64::from(*y) * stride as f64
                        }
                    };
                    self.screenshot_scroll_target_pixels =
                        (self.screenshot_scroll_target_pixels - pixel_delta)
                            .clamp(0.0, max_scroll);
                    self.screenshot_scroll_pixels =
                        self.screenshot_scroll_pixels.min(max_scroll);
                    self.screenshot_offset =
                        (self.screenshot_scroll_target_pixels / stride as f64)
                            .floor() as usize
                            * columns;
                    self.screenshot_scroll_synced = self.screenshot_offset;
                } else if self.instance_tab == 7 {
                    let height = self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as usize;
                    if self.world_offset != self.world_scroll_synced {
                        self.world_scroll_target_pixels =
                            (self.world_offset * 54) as f64;
                        self.world_scroll_pixels =
                            self.world_scroll_target_pixels;
                    }
                    let viewport = height.saturating_sub(190);
                    let max_scroll = self
                        .worlds
                        .len()
                        .saturating_mul(54)
                        .saturating_sub(viewport)
                        as f64;
                    let pixel_delta = match &delta {
                        MouseScrollDelta::PixelDelta(position) => position.y,
                        MouseScrollDelta::LineDelta(_, y) => {
                            f64::from(*y) * 54.0
                        }
                    };
                    self.world_scroll_target_pixels =
                        (self.world_scroll_target_pixels - pixel_delta)
                            .clamp(0.0, max_scroll);
                    self.world_scroll_pixels =
                        self.world_scroll_pixels.min(max_scroll);
                    self.world_offset = (self.world_scroll_target_pixels / 54.0)
                        .floor()
                        as usize;
                    self.world_scroll_synced = self.world_offset;
                } else {
                    let height = self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as usize;
                    let viewport_height =
                        height.saturating_sub(CONTENT_LIST_TOP);
                    let max_scroll = super::content_scroll_max(
                        self.filtered_content_cache.len(),
                        viewport_height,
                    );
                    let pixel_delta = match &delta {
                        MouseScrollDelta::PixelDelta(position) => position.y,
                        MouseScrollDelta::LineDelta(_, y) => {
                            f64::from(*y) * CONTENT_ROW_HEIGHT as f64
                        }
                    };
                    self.content_scroll_target_pixels =
                        (self.content_scroll_target_pixels - pixel_delta)
                            .clamp(0.0, max_scroll);
                    self.content_scroll_pixels =
                        self.content_scroll_pixels.min(max_scroll);
                }
            } else {
                let width = self
                    .window
                    .as_ref()
                    .map_or(0, |window| window.inner_size().width as usize);
                let library_width =
                    width.saturating_sub(if self.should_show_right_sidebar() {
                        300
                    } else {
                        0
                    });
                let grid = self.library_grid_layout(library_width);
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height as usize);
                let rows = self.library_grid_rows(grid.columns);
                let content_height: usize = rows
                    .iter()
                    .map(|row| library_grid_row_height(row, grid))
                    .sum();
                let home_scroll_active = self.home_page_scroll_active();
                let page_start = if home_scroll_active {
                    grid.cards_top.saturating_sub(64)
                } else {
                    0
                };
                let max_scroll = if home_scroll_active {
                    grid.cards_top
                        .saturating_add(content_height)
                        .saturating_sub(height.saturating_sub(24))
                } else {
                    content_height
                        .saturating_sub(height.saturating_sub(grid.cards_top))
                } as f64;
                let scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => {
                        -f64::from(lines) * (grid.card_height + grid.gap) as f64
                    }
                    MouseScrollDelta::PixelDelta(position) => -position.y,
                };
                self.library_scroll_target_pixels =
                    (self.library_scroll_target_pixels + scroll_delta)
                        .clamp(0.0, max_scroll);
                let mut offset = 0usize;
                let mut covered = 0usize;
                let row_scroll_target =
                    self.library_scroll_target_pixels.max(0.0)
                        - page_start as f64;
                while offset < rows.len()
                    && (covered + library_grid_row_height(&rows[offset], grid))
                        as f64
                        <= row_scroll_target
                {
                    covered += library_grid_row_height(&rows[offset], grid);
                    offset += 1;
                }
                self.list_offset = offset;
                self.library_scroll_offset_synced = offset;
            }
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}
