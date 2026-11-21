use super::*;

impl Launcher {
    pub(super) fn handle_mouse_input(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) {
        match (state, button) {
            (ElementState::Pressed, MouseButton::Left) => {
                if self.friend_incoming_open {
                    return;
                }
                if self.input_focus == 36 {
                    self.input_focus = 0;
                }
                let (screen_x, y) = self.cursor;
                let x = screen_x;
                let height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height)
                    as f64;
                let width = self
                    .window
                    .as_ref()
                    .map_or(1024, |window| window.inner_size().width)
                    as usize;
                if self.input_focus == 40 {
                    let search_bounds = if self.instance_tab == 1 {
                        Some(instance_console_search_bounds(width))
                    } else if self.instance_tab == 5
                        && self.selected_log.is_some()
                    {
                        Some(instance_log_search_bounds(
                            width,
                            self.log_analysis.is_some(),
                        ))
                    } else {
                        None
                    };
                    if search_bounds.is_none_or(|bounds| !bounds.contains(x, y))
                    {
                        self.input_focus = 0;
                        self.log_search_select_all = false;
                    }
                }
                let library_width =
                    width.saturating_sub(if self.should_show_right_sidebar() {
                        300
                    } else {
                        0
                    });
                if self.hosting_backup_prompt.is_some() {
                    let (panel, field, cancel, confirm) =
                        hosting_backup_prompt_layout(width, height as usize);
                    let renaming = self
                        .hosting_backup_prompt
                        .as_ref()
                        .is_some_and(|prompt| {
                            prompt.action == HostingBackupPromptAction::Rename
                        });
                    if renaming && field.contains(x, y) {
                        self.input_focus = 39;
                    } else if cancel.contains(x, y) {
                        if !self.hosting_backup_mutating {
                            self.hosting_backup_prompt = None;
                            self.hosting_backup_create_name.clear();
                            self.input_focus = 0;
                        }
                    } else if confirm.contains(x, y) {
                        self.submit_hosting_backup_prompt();
                    } else if !panel.contains(x, y)
                        && !self.hosting_backup_mutating
                    {
                        self.hosting_backup_prompt = None;
                        self.hosting_backup_create_name.clear();
                        self.input_focus = 0;
                    }
                    self.draw();
                    return;
                }
                if self.hosting_backup_rotation_confirmation {
                    let (panel, cancel, confirm) =
                        hosting_backup_rotation_layout(width, height as usize);
                    if confirm.contains(x, y) {
                        self.save_hosting_backup_options(true);
                    } else if cancel.contains(x, y) || !panel.contains(x, y) {
                        self.hosting_backup_rotation_confirmation = false;
                    }
                    self.draw();
                    return;
                }
                if self.browse_collection_create_open {
                    let panel_width = 460usize.min(width.saturating_sub(24));
                    let panel_height =
                        210usize.min((height as usize).saturating_sub(24));
                    let left = (width.saturating_sub(panel_width) / 2) as f64;
                    let top = ((height as usize).saturating_sub(panel_height)
                        / 2) as f64;
                    if (left + 20.0..left + panel_width as f64 - 20.0)
                        .contains(&x)
                        && (top + 76.0..top + 116.0).contains(&y)
                    {
                        self.input_focus = 37;
                    } else if (top + 158.0..top + 198.0).contains(&y)
                        && (left + panel_width as f64 - 220.0
                            ..left + panel_width as f64 - 116.0)
                            .contains(&x)
                    {
                        self.browse_collection_create_open = false;
                        self.input_focus = 0;
                        self.browse_collection_create_name.clear();
                    } else if (top + 158.0..top + 198.0).contains(&y)
                        && (left + panel_width as f64 - 108.0
                            ..left + panel_width as f64 - 20.0)
                            .contains(&x)
                    {
                        self.create_browse_collection();
                    } else if !(left..left + panel_width as f64).contains(&x)
                        || !(top..top + panel_height as f64).contains(&y)
                    {
                        self.browse_collection_create_open = false;
                        self.input_focus = 0;
                    }
                    self.draw();
                    return;
                }
                if self.browse_install_override.is_some() {
                    let (panel, [cancel, install]) =
                        browse_install_override_layout(width, height as usize);
                    if install.contains(x, y) {
                        self.confirm_browse_install_override();
                    } else if cancel.contains(x, y) || !panel.contains(x, y) {
                        self.browse_install_override = None;
                    }
                    self.draw();
                    return;
                }
                if self.browse_install_choice.is_some() {
                    let layout = InstallInstancePickerLayout::new(
                        width,
                        height as usize,
                        self.instances.len(),
                    );
                    let scroll = self
                        .browse_install_scroll
                        .clamp(0.0, layout.max_scroll(self.instances.len()));
                    if layout.close.contains(x, y)
                        || !layout.panel.contains(x, y)
                    {
                        self.browse_install_choice = None;
                    } else if let Some(index) = layout.row_at(x, y, scroll)
                        && let Some((id, name, _)) =
                            self.instances.get(index).cloned()
                        && can_pick_install_instance(
                            self.instance_cards.get(&id),
                        )
                        && self.browse_install_choice.as_ref().is_some_and(
                            |choice| {
                                choice.versions.is_some()
                                    && choice.presence(&id).is_some()
                                    && (choice.version_id.is_some()
                                        || !matches!(
                                            choice.presence(&id),
                                            Some(Ok(true))
                                        ))
                                    && !self
                                        .browse_installing_projects
                                        .get(&id)
                                        .is_some_and(|projects| {
                                            projects
                                                .contains(&choice.project.id)
                                        })
                            },
                        )
                        && let Some(choice) = self.browse_install_choice.take()
                    {
                        let review_versions = choice.version_id.is_none()
                            && self.instance_cards.get(&id).is_some_and(
                                |card| {
                                    choice.compatibility(card) == Some(false)
                                },
                            );
                        self.global_browse = false;
                        self.open_library_instance(id.clone(), name);
                        self.instance_tab = 3;
                        if review_versions {
                            self.open_browse_project(choice.project);
                            self.browse_detail_tab = 1;
                        } else {
                            self.install_browse_project_for(
                                choice.project,
                                choice.version_id,
                                id,
                            );
                        }
                    }
                    self.draw();
                    return;
                }
                if self.browse_gallery_viewer_open {
                    let toolbar_x = width.saturating_sub(444) as f64 / 2.0;
                    let toolbar_y = height - 72.0;
                    if y >= toolbar_y + 8.0
                        && y < toolbar_y + 40.0
                        && x >= toolbar_x
                        && x < toolbar_x + 444.0
                    {
                        let toolbar_position = x - toolbar_x;
                        let count = self
                            .browse_details
                            .as_ref()
                            .map_or(0, |details| details.gallery.len());
                        if (12.0..48.0).contains(&toolbar_position) {
                            if count > 0 {
                                self.browse_gallery_index = self
                                    .browse_gallery_index
                                    .wrapping_add(count - 1)
                                    % count;
                                self.browse_gallery_zoom = 1.0;
                            }
                        } else if (120.0..156.0).contains(&toolbar_position) {
                            if count > 0 {
                                self.browse_gallery_index =
                                    (self.browse_gallery_index + 1) % count;
                                self.browse_gallery_zoom = 1.0;
                            }
                        } else if (168.0..204.0).contains(&toolbar_position) {
                            self.browse_gallery_zoom =
                                (self.browse_gallery_zoom / 1.25).max(1.0);
                        } else if (212.0..276.0).contains(&toolbar_position) {
                            self.browse_gallery_zoom = 1.0;
                        } else if (284.0..320.0).contains(&toolbar_position) {
                            self.browse_gallery_zoom =
                                (self.browse_gallery_zoom * 1.25).min(5.0);
                        } else if (340.0..376.0).contains(&toolbar_position) {
                            if let Some(url) =
                                self.browse_details
                                    .as_ref()
                                    .and_then(|details| {
                                        details
                                            .gallery
                                            .get(self.browse_gallery_index)
                                    })
                                    .map(|image| {
                                        image.raw_url.clone().unwrap_or_else(
                                            || image.url.clone(),
                                        )
                                    })
                            {
                                let _ = open::that(url);
                            }
                        } else if (384.0..420.0).contains(&toolbar_position) {
                            self.browse_gallery_viewer_open = false;
                        }
                    } else if (32.0..width as f64 - 32.0).contains(&x)
                        && (140.0..height - 92.0).contains(&y)
                    {
                        self.browse_gallery_zoom =
                            if self.browse_gallery_zoom > 1.001 {
                                1.0
                            } else {
                                2.0
                            };
                    } else if !(32.0..width as f64 - 32.0).contains(&x)
                        || !(140.0..height - 92.0).contains(&y)
                    {
                        self.browse_gallery_viewer_open = false;
                    }
                    return;
                }
                if (10.0..38.0).contains(&y) {
                    if (168.0..196.0).contains(&x) {
                        self.navigate_history(true);
                        return;
                    }
                    if (204.0..232.0).contains(&x) {
                        self.navigate_history(false);
                        return;
                    }
                }
                if uses_custom_window_controls(
                    cfg!(target_os = "macos"),
                    self.settings
                        .as_ref()
                        .is_some_and(|settings| settings.native_decorations),
                ) {
                    let window_width = width as f64;
                    let west = x < 8.0;
                    let east = x >= window_width - 8.0;
                    let north = y < 8.0;
                    let south = y >= height - 8.0;
                    let resize_direction = match (west, east, north, south) {
                        (true, _, true, _) => Some(ResizeDirection::NorthWest),
                        (_, true, true, _) => Some(ResizeDirection::NorthEast),
                        (true, _, _, true) => Some(ResizeDirection::SouthWest),
                        (_, true, _, true) => Some(ResizeDirection::SouthEast),
                        (true, _, _, _) => Some(ResizeDirection::West),
                        (_, true, _, _) => Some(ResizeDirection::East),
                        (_, _, true, _) => Some(ResizeDirection::North),
                        (_, _, _, true) => Some(ResizeDirection::South),
                        _ => None,
                    };
                    if let Some(direction) = resize_direction {
                        if let Some(window) = &self.window {
                            let _ = window.drag_resize_window(direction);
                        }
                        return;
                    }
                    if y < 48.0 && x >= window_width - 144.0 {
                        if x >= window_width - 48.0 {
                            if let Some(proxy) = &self.proxy {
                                let _ =
                                    proxy.send_event(UiMessage::CloseWindow);
                            }
                        } else if x >= window_width - 96.0 {
                            if let Some(window) = &self.window {
                                self.window_maximized = !window.is_maximized();
                                window.set_maximized(self.window_maximized);
                                self.draw();
                            }
                        } else if let Some(window) = &self.window {
                            window.set_minimized(true);
                        }
                        return;
                    }
                    if y < 48.0 {
                        if let Some(window) = &self.window {
                            let _ = window.drag_window();
                        }
                        return;
                    }
                }
                let content_y = if self.home_page_scroll_input_active() {
                    render::page_y_to_logical(
                        y,
                        self.library_scroll_pixels.round() as usize,
                    )
                } else {
                    y
                };
                if let Some((handle_x, handle_y)) =
                    self.jump_in_resize_handle(library_width)
                    && (handle_x as f64..(handle_x + 48) as f64).contains(&x)
                    && (handle_y as f64..(handle_y + 16) as f64)
                        .contains(&content_y)
                {
                    let visible_count = self.jump_in_visible_count(
                        library_width,
                        self.jump_in_items().len(),
                    );
                    self.jump_in_resize_drag =
                        Some((y, visible_count, self.jump_in_visible_items));
                    self.input_focus = 0;
                    return;
                }
                let skin_logical_x = if self.global_skins {
                    render::page_x_to_logical(x, library_width)
                } else {
                    x
                };
                self.minecraft_skin_dragging = self.global_skins
                    && skin_logical_x >= 306.0
                    && y >= 178.0
                    && y < height - 42.0;
                self.minecraft_skin_last_cursor_x = skin_logical_x;
            }
            (ElementState::Released, MouseButton::Left) => {
                if let Some((_, _, original_count)) =
                    self.jump_in_resize_drag.take()
                {
                    if self.jump_in_visible_items != original_count {
                        self.persist_quick_instance_count(
                            self.jump_in_visible_items,
                        );
                    }
                    self.draw();
                    return;
                }
                self.minecraft_skin_dragging = false;
                let width = self
                    .window
                    .as_ref()
                    .map_or(0.0, |window| window.inner_size().width as f64);
                let (screen_x, mut y) = self.cursor;
                let mut x = screen_x;
                let sidebar_height = self
                    .window
                    .as_ref()
                    .map_or(720, |window| window.inner_size().height)
                    as f64;
                let group_y = sidebar_height - 88.0;
                let right_sidebar_left = width - 300.0;
                if self.friend_incoming_open {
                    let panel_width = 560.0_f64.min(width - 32.0);
                    let panel_height = 500.0_f64.min(sidebar_height - 32.0);
                    let panel_x = (width - panel_width) / 2.0;
                    let panel_y = (sidebar_height - panel_height) / 2.0;
                    if x < panel_x
                        || x >= panel_x + panel_width
                        || y < panel_y
                        || y >= panel_y + panel_height
                        || (y >= panel_y + 12.0
                            && y < panel_y + 48.0
                            && x >= panel_x + panel_width - 72.0)
                    {
                        self.friend_incoming_open = false;
                        self.friend_incoming_scroll_pixels = 0.0;
                        self.friend_incoming_scroll_target_pixels = 0.0;
                    } else {
                        let incoming = sidebar_incoming_friend_indices(
                            &self.sidebar_friends,
                        );
                        let list_top = panel_y + 82.0;
                        let index = ((y - list_top
                            + self.friend_incoming_scroll_pixels)
                            / 64.0)
                            .floor()
                            .max(0.0)
                            as usize;
                        if let Some(friend_index) = incoming.get(index)
                            && let Some(friend) =
                                self.sidebar_friends.get(*friend_index)
                        {
                            let accept_x = panel_x + panel_width - 220.0;
                            let ignore_x = panel_x + panel_width - 116.0;
                            let row_y = list_top + index as f64 * 64.0
                                - self.friend_incoming_scroll_pixels;
                            if (row_y..row_y + 56.0).contains(&y)
                                && (accept_x..accept_x + 92.0).contains(&x)
                            {
                                self.send_friend_request(
                                    friend.user_id.clone(),
                                );
                            } else if (row_y..row_y + 56.0).contains(&y)
                                && (ignore_x..ignore_x + 92.0).contains(&x)
                            {
                                self.remove_sidebar_friend(
                                    friend.user_id.clone(),
                                );
                            }
                        }
                    }
                    self.draw();
                    return;
                }
                if let Some(friend) = self.friend_remove_confirmation.clone() {
                    let panel_x = (width - 440.0) / 2.0;
                    let panel_y = (sidebar_height - 210.0) / 2.0;
                    if x < panel_x
                        || x >= panel_x + 440.0
                        || y < panel_y
                        || y >= panel_y + 210.0
                    {
                        self.friend_remove_confirmation = None;
                        self.friend_remove_error = None;
                    } else if (panel_y + 150.0..panel_y + 190.0).contains(&y)
                        && (panel_x + 220.0..panel_x + 316.0).contains(&x)
                    {
                        self.remove_sidebar_friend(friend.user_id);
                    } else if (panel_y + 150.0..panel_y + 190.0).contains(&y)
                        && x >= panel_x + 324.0
                    {
                        self.friend_remove_confirmation = None;
                        self.friend_remove_error = None;
                    }
                    self.draw();
                    return;
                }
                if self.friend_request_open {
                    let panel_x = (width - 560.0) / 2.0;
                    let panel_y = (sidebar_height - 470.0) / 2.0;
                    if x < panel_x
                        || x >= panel_x + 560.0
                        || y < panel_y
                        || y >= panel_y + 470.0
                        || (y >= panel_y + 12.0
                            && y < panel_y + 48.0
                            && x >= panel_x + 512.0)
                    {
                        self.friend_request_open = false;
                        self.input_focus = 0;
                    } else if (panel_y + 74.0..panel_y + 116.0).contains(&y)
                        && (panel_x + 24.0..panel_x + 536.0).contains(&x)
                    {
                        self.input_focus = 25;
                    } else if (panel_y + 124.0..panel_y + 160.0).contains(&y)
                        && x >= panel_x + 424.0
                    {
                        self.input_focus = 0;
                        self.search_friend_users();
                    } else if y >= panel_y + 172.0 {
                        let result_index =
                            ((y - panel_y - 170.0) / 54.0) as usize;
                        if x >= panel_x + 424.0
                            && let Some(user) =
                                self.friend_search_results.get(result_index)
                        {
                            self.send_friend_request(user.id.clone());
                        }
                    }
                    self.draw();
                    return;
                }
                if self.should_show_right_sidebar()
                    && x >= right_sidebar_left
                    && y >= 56.0
                {
                    if self.browse_filter_sidebar_visible() {
                        let options = self.browse_filter_menu_options();
                        let layout = browse_filter_sidebar_layout(
                            width as usize,
                            sidebar_height as usize,
                            self.browse_filter_group,
                            options.len(),
                            self.browse_filter_group_count(),
                        );
                        if layout.reset.contains(x, y) {
                            if self.browse_filter_options_error.is_some()
                                && self.browse_filter_group
                                    < BROWSE_LICENSE_GROUP
                            {
                                self.load_browse_filter_options();
                            } else if !self
                                .browse_selected_game_versions
                                .is_empty()
                                || !self.browse_selected_loaders.is_empty()
                                || !self.browse_selected_categories.is_empty()
                                || !self.browse_selected_environments.is_empty()
                                || !self.browse_selected_licenses.is_empty()
                            {
                                self.browse_selected_game_versions.clear();
                                self.browse_selected_loaders.clear();
                                self.browse_selected_categories.clear();
                                self.browse_selected_environments.clear();
                                self.browse_selected_licenses.clear();
                                self.search_projects();
                            }
                        } else if let Some(index) = layout
                            .headings
                            .iter()
                            .position(|bounds| bounds.contains(x, y))
                        {
                            if self.browse_filter_group != index as u8 {
                                self.browse_filter_group = index as u8;
                                self.reset_browse_filter_scroll();
                            }
                        } else if layout
                            .version_toggle
                            .is_some_and(|bounds| bounds.contains(x, y))
                        {
                            self.toggle_browse_all_versions();
                        } else if let Some(index) = layout.option_at(
                            x,
                            y,
                            self.browse_filter_scroll_pixels,
                        ) && let Some(option) =
                            options.get(index).cloned()
                        {
                            self.toggle_browse_filter_option(option);
                        }
                        self.draw();
                        return;
                    }
                    if let (Some(project), Some(details)) = (
                        self.browse_selected.as_ref(),
                        self.browse_details.as_ref(),
                    ) {
                        let max_scroll = draw::project_sidebar_content_height(
                            project,
                            details,
                            width as usize,
                        )
                        .saturating_sub(
                            (sidebar_height as usize).saturating_sub(48),
                        ) as f64;
                        y += self.sidebar_scroll_pixels.min(max_scroll);
                        if let Some((group, label)) =
                            draw::project_sidebar_chip_action_at(
                                project,
                                details,
                                width as usize,
                                x,
                                y,
                            )
                        {
                            let project_type = project.project_type.clone();
                            self.browse_filter = match project_type.as_str() {
                                "resourcepack" => 1,
                                "datapack" => 2,
                                "shader" => 3,
                                "modpack" => 4,
                                _ => 0,
                            };
                            self.browse_query.clear();
                            self.browse_query_cursor = 0;
                            self.browse_query_select_all = false;
                            self.browse_search_offset = 0;
                            self.browse_scroll_pixels = 0.0;
                            self.browse_scroll_target_pixels = 0.0;
                            self.browse_selected_game_versions.clear();
                            self.browse_selected_loaders.clear();
                            self.browse_selected_categories.clear();
                            self.browse_selected_environments.clear();
                            self.browse_selected_licenses.clear();
                            self.browse_filter_group = 0;
                            self.reset_browse_filter_scroll();
                            if group == 1 {
                                self.browse_selected_loaders.insert(label);
                            } else {
                                self.browse_selected_categories.insert(label);
                            }
                            self.browse_selected = None;
                            self.browse_details = None;
                            self.browse_detail_scroll_pixels = 0.0;
                            self.browse_detail_scroll_target_pixels = 0.0;
                            self.browse_detail_filter_menu_group = None;
                            self.browse_detail_filter_option_offset = 0;
                            self.sidebar_scroll_pixels = 0.0;
                            self.sidebar_scroll_target_pixels = 0.0;
                            self.search_projects();
                            self.draw();
                            return;
                        }
                        if let Some(url) = draw::project_sidebar_link_at(
                            project,
                            details,
                            width as usize,
                            y,
                        ) {
                            std::thread::spawn(move || {
                                let _ = open::that(url);
                            });
                        }
                        self.draw();
                        return;
                    }
                    y += self.sidebar_scroll_pixels.min(sidebar_scroll_max(
                        self.news_articles.len(),
                        &self.sidebar_friends,
                        sidebar_height as usize,
                        self.onboarding_checklist.as_ref(),
                        &self.sidebar_friend_query,
                        sidebar_collapsed_groups(self.settings.as_ref()),
                    )
                        as f64) as usize as f64;
                    let news_top = sidebar_news_top(
                        &self.sidebar_friends,
                        self.onboarding_checklist.as_ref(),
                        &self.sidebar_friend_query,
                        sidebar_collapsed_groups(self.settings.as_ref()),
                    ) as f64;
                    if self.global_browse {
                        self.browse_filter_menu_open = false;
                    }
                    let onboarding = self
                        .onboarding_checklist
                        .as_ref()
                        .filter(|checklist| checklist.show_checklist);
                    let onboarding_step = if (104.0..208.0).contains(&y) {
                        Some(((y - 104.0) / 36.0).floor() as usize)
                    } else {
                        None
                    };
                    if let (Some(checklist), Some(step)) =
                        (onboarding, onboarding_step)
                    {
                        match step {
                            0 if !checklist.has_created_instance => {
                                self.return_to_library(None);
                                self.create_instance_open = true;
                                self.name_input.clear();
                                self.version_input.clear();
                                self.input_focus = 1;
                            }
                            1 if !checklist.has_logged_into_minecraft => {
                                self.return_to_library(None);
                                self.accounts_open = true;
                                self.account_tab = 0;
                                self.account_offset = 0;
                                self.account_scroll_pixels = 0.0;
                                self.account_scroll_target_pixels = 0.0;
                                self.input_focus = 0;
                                self.load_accounts();
                            }
                            2 if !checklist.has_logged_into_modrinth => {
                                self.return_to_library(None);
                                self.accounts_open = true;
                                self.account_tab = 1;
                                self.account_offset = 0;
                                self.account_scroll_pixels = 0.0;
                                self.account_scroll_target_pixels = 0.0;
                                self.input_focus = 0;
                                self.load_accounts();
                            }
                            _ => {}
                        }
                    } else if self.onboarding_checklist.as_ref().is_some_and(
                        |checklist| {
                            checklist.show_checklist
                                && checklist.has_logged_into_minecraft
                        },
                    ) && (270.0..338.0).contains(&y)
                    {
                        self.accounts_open = true;
                        self.account_tab = 0;
                        self.account_offset = 0;
                        self.account_scroll_pixels = 0.0;
                        self.account_scroll_target_pixels = 0.0;
                        self.load_accounts();
                    } else if self
                        .onboarding_checklist
                        .as_ref()
                        .is_none_or(|checklist| !checklist.show_checklist)
                        && (104.0..212.0).contains(&y)
                    {
                        self.accounts_open = true;
                        self.account_tab = 0;
                        self.load_accounts();
                    } else if (sidebar_friend_header_y(
                        self.onboarding_checklist.as_ref(),
                    ) as f64
                        - 6.0
                        ..sidebar_friend_header_y(
                            self.onboarding_checklist.as_ref(),
                        ) as f64
                            + 24.0)
                        .contains(&y)
                        && ((!self.sidebar_friends.is_empty()
                            && (right_sidebar_left + 16.0
                                ..right_sidebar_left + 48.0)
                                .contains(&x))
                            || (self.sidebar_friends.is_empty()
                                && x >= width - 58.0))
                    {
                        self.friend_request_open = true;
                        self.friend_search_query.clear();
                        self.friend_search_results.clear();
                        self.friend_search_error = None;
                        self.input_focus = 25;
                    } else if sidebar_incoming_friend_indices(
                        &self.sidebar_friends,
                    )
                    .len()
                        > 0
                        && (sidebar_friend_header_y(
                            self.onboarding_checklist.as_ref(),
                        ) as f64
                            - 6.0
                            ..sidebar_friend_header_y(
                                self.onboarding_checklist.as_ref(),
                            ) as f64
                                + 24.0)
                            .contains(&y)
                        && (width - 44.0..width - 16.0).contains(&x)
                    {
                        self.friend_incoming_open = true;
                        self.friend_incoming_scroll_pixels = 0.0;
                        self.friend_incoming_scroll_target_pixels = 0.0;
                        self.friend_remove_error = None;
                        self.friend_search_error = None;
                    } else if !self.sidebar_friends.is_empty()
                        && (sidebar_friend_header_y(
                            self.onboarding_checklist.as_ref(),
                        ) as f64
                            - 6.0
                            ..sidebar_friend_header_y(
                                self.onboarding_checklist.as_ref(),
                            ) as f64
                                + 24.0)
                            .contains(&y)
                        && (right_sidebar_left + 54.0
                            ..if sidebar_incoming_friend_indices(
                                &self.sidebar_friends,
                            )
                            .is_empty()
                            {
                                width - 12.0
                            } else {
                                width - 50.0
                            })
                            .contains(&x)
                    {
                        self.input_focus = 36;
                    } else if !self.news_articles.is_empty()
                        && y >= news_top
                            + self.news_articles.len().min(4) as f64
                                * SIDEBAR_NEWS_CARD_STRIDE as f64
                            + 4.0
                        && y < news_top
                            + 36.0
                            + self.news_articles.len().min(4) as f64
                                * SIDEBAR_NEWS_CARD_STRIDE as f64
                    {
                        std::thread::spawn(|| {
                            let _ = open::that("https://modrinth.com/news");
                        });
                    } else if self.sidebar_friend_query.trim().is_empty()
                        && let Some(section) = sidebar_friend_sections(
                            &self.sidebar_friends,
                            self.onboarding_checklist.as_ref(),
                            &self.sidebar_friend_query,
                            sidebar_collapsed_groups(self.settings.as_ref()),
                        )
                        .iter()
                        .find(|section| {
                            (section.header_y as f64
                                ..(section.header_y + 22) as f64)
                                .contains(&y)
                        })
                    {
                        self.change_setting(match section.group {
                            SidebarFriendGroup::Active => {
                                SettingsChange::FriendsActiveCollapsed
                            }
                            SidebarFriendGroup::Online => {
                                SettingsChange::FriendsOnlineCollapsed
                            }
                            SidebarFriendGroup::Offline => {
                                SettingsChange::FriendsOfflineCollapsed
                            }
                            SidebarFriendGroup::Pending => {
                                SettingsChange::FriendsPendingCollapsed
                            }
                        });
                    } else if let Some(friend) = sidebar_friend_sections(
                        &self.sidebar_friends,
                        self.onboarding_checklist.as_ref(),
                        &self.sidebar_friend_query,
                        sidebar_collapsed_groups(self.settings.as_ref()),
                    )
                    .iter()
                    .find_map(|section| {
                        if section.collapsed {
                            return None;
                        }
                        let rows_top = section.header_y + 24;
                        if y < rows_top as f64 {
                            return None;
                        }
                        let row =
                            ((y - rows_top as f64) / 38.0).floor() as usize;
                        if y < (rows_top + section.friend_indices.len() * 38)
                            as f64
                            && (y - (rows_top + row * 38) as f64) < 34.0
                        {
                            section.friend_indices.get(row).and_then(|index| {
                                self.sidebar_friends.get(*index)
                            })
                        } else {
                            None
                        }
                    }) {
                        if x >= width - 56.0 {
                            self.friend_remove_confirmation =
                                Some(friend.clone());
                            self.friend_remove_error = None;
                        } else if friend.incoming_request {
                            self.send_friend_request(friend.user_id.clone());
                        } else {
                            self.open_user_profile(friend.user_id.clone());
                        }
                    } else if y >= news_top {
                        if self.news_articles.is_empty()
                            && self.news_error.is_some()
                            && (news_top + 38.0..news_top + 68.0).contains(&y)
                            && x < right_sidebar_left + 112.0
                        {
                            self.news_error = None;
                            self.news_loaded = false;
                            self.load_news_articles();
                            self.draw();
                            return;
                        }
                        let local_y = (y - news_top).max(0.0);
                        let card_index =
                            (local_y / SIDEBAR_NEWS_CARD_STRIDE as f64).floor()
                                as usize;
                        if local_y.rem_euclid(SIDEBAR_NEWS_CARD_STRIDE as f64)
                            < SIDEBAR_NEWS_CARD_HEIGHT as f64
                            && card_index < self.news_articles.len().min(4)
                        {
                            if let Some(article) =
                                self.news_articles.get(card_index)
                            {
                                let link = article.link.clone();
                                std::thread::spawn(move || {
                                    let _ = open::that(link);
                                });
                            }
                        }
                    }
                    self.draw();
                    return;
                }
                let library_width = width
                    - if self.should_show_right_sidebar() {
                        300.0
                    } else {
                        0.0
                    };
                x = render::pointer_x_to_logical(
                    x,
                    library_width.max(0.0) as usize,
                    self.settings_open,
                );
                let wide_instance_header =
                    self.uses_wide_instance_header(width as usize);
                if self.home_page_scroll_input_active() {
                    y = render::page_y_to_logical(
                        y,
                        self.library_scroll_pixels.round() as usize,
                    );
                }
                let (browse_filter_x, browse_filter_width) =
                    browse_filter_button_layout(library_width as usize);
                if self.is_browse_results_page()
                    && !self.browse_filter_sidebar_visible()
                    && (146.0..178.0).contains(&y)
                    && (browse_filter_x as f64
                        ..(browse_filter_x + browse_filter_width) as f64)
                        .contains(&x)
                {
                    self.browse_view_menu_open = false;
                    self.browse_sort_menu_open = false;
                    self.browse_filter_menu_open =
                        !self.browse_filter_menu_open;
                    self.reset_browse_filter_scroll();
                    if self.browse_filter_menu_open {
                        self.load_browse_filter_options();
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.is_browse_results_page()
                    && !self.browse_filter_sidebar_visible()
                    && self.browse_filter_menu_open
                {
                    let panel = browse_filter_popup_bounds(
                        library_width as usize,
                        sidebar_height as usize,
                    );
                    if panel.contains(x, y) {
                        let (version_toggle, viewport) =
                            browse_popup_filter_rows(
                                panel,
                                self.browse_filter_group,
                            );
                        let groups = self.browse_filter_group_count();
                        if let Some(index) = (0..groups).find(|&index| {
                            browse_filter_popup_tab(
                                panel.x,
                                panel.y,
                                panel.width,
                                groups,
                                index,
                            )
                            .contains(x, y)
                        }) {
                            self.browse_filter_group = index as u8;
                            self.reset_browse_filter_scroll();
                        } else if version_toggle
                            .is_some_and(|bounds| bounds.contains(x, y))
                        {
                            self.toggle_browse_all_versions();
                        } else if viewport.contains(x, y)
                            && (self.browse_filter_group
                                >= BROWSE_LICENSE_GROUP
                                || !self.browse_filter_options_loading)
                        {
                            let index = ((y - viewport.y as f64
                                + self.browse_filter_scroll_pixels.floor())
                                / 24.0)
                                as usize;
                            let options = self.browse_filter_menu_options();
                            if let Some(option) = options.get(index).cloned() {
                                self.toggle_browse_filter_option(option);
                            }
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }
                    self.browse_filter_menu_open = false;
                }
                if let Some(skin) = self.skin_delete_confirmation.clone() {
                    let dialog_width = 460.0_f64.min(width - 24.0);
                    let dialog_height = 190.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if (dialog_y + dialog_height - 48.0
                        ..dialog_y + dialog_height - 16.0)
                        .contains(&y)
                    {
                        if (dialog_x + dialog_width - 204.0
                            ..dialog_x + dialog_width - 120.0)
                            .contains(&x)
                        {
                            self.remove_minecraft_skin(skin);
                        } else if x >= dialog_x + dialog_width - 108.0 {
                            self.skin_delete_confirmation = None;
                        }
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.skin_delete_confirmation = None;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.log_delete_confirmation {
                    let dialog_width = 460.0_f64.min(width - 24.0);
                    let dialog_height = 190.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if (dialog_y + dialog_height - 48.0
                        ..dialog_y + dialog_height - 16.0)
                        .contains(&y)
                    {
                        if (dialog_x + dialog_width - 204.0
                            ..dialog_x + dialog_width - 120.0)
                            .contains(&x)
                        {
                            self.delete_selected_log();
                        } else if (dialog_x + dialog_width - 108.0
                            ..dialog_x + dialog_width - 20.0)
                            .contains(&x)
                        {
                            self.log_delete_confirmation = false;
                        }
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.log_delete_confirmation = false;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.world_delete_confirmation.is_some() {
                    let dialog_width = 460.0_f64.min(width - 24.0);
                    let dialog_height = 190.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if (dialog_y + dialog_height - 48.0
                        ..dialog_y + dialog_height - 16.0)
                        .contains(&y)
                    {
                        if (dialog_x + dialog_width - 204.0
                            ..dialog_x + dialog_width - 112.0)
                            .contains(&x)
                        {
                            self.delete_singleplayer_world();
                        } else if x >= dialog_x + dialog_width - 108.0 {
                            self.world_delete_confirmation = None;
                            self.input_focus = 0;
                        }
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.world_delete_confirmation = None;
                        self.input_focus = 0;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.world_editor_open {
                    let dialog_width = 500.0_f64.min(width - 24.0);
                    let dialog_height = 270.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if (dialog_y + 76.0..dialog_y + 114.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + dialog_width - 24.0)
                            .contains(&x)
                    {
                        self.input_focus = 24;
                    } else if (dialog_y + 132.0..dialog_y + 168.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + dialog_width - 24.0)
                            .contains(&x)
                    {
                        self.world_edit_reset_icon =
                            !self.world_edit_reset_icon;
                    } else if (dialog_y + 212.0..dialog_y + 246.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + 108.0).contains(&x)
                        && !self.add_server_busy
                    {
                        self.world_editor_open = false;
                        self.world_delete_confirmation = Some((
                            self.world_edit_path.clone(),
                            self.world_edit_name.clone(),
                        ));
                        self.input_focus = 0;
                    } else if (dialog_y + 212.0..dialog_y + 246.0).contains(&y)
                        && x >= dialog_x + dialog_width - 204.0
                        && x < dialog_x + dialog_width - 112.0
                    {
                        self.world_editor_open = false;
                        self.world_edit_path.clear();
                        self.world_edit_name.clear();
                        self.input_focus = 0;
                    } else if (dialog_y + 212.0..dialog_y + 246.0).contains(&y)
                        && x >= dialog_x + dialog_width - 112.0
                        && x < dialog_x + dialog_width - 24.0
                    {
                        self.save_singleplayer_world();
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.world_editor_open = false;
                        self.world_edit_path.clear();
                        self.world_edit_name.clear();
                        self.input_focus = 0;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.server_remove_confirmation.is_some() {
                    let dialog_width = 460.0_f64.min(width - 24.0);
                    let dialog_height = 190.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if (dialog_y + dialog_height - 48.0
                        ..dialog_y + dialog_height - 16.0)
                        .contains(&y)
                    {
                        if (dialog_x + dialog_width - 204.0
                            ..dialog_x + dialog_width - 112.0)
                            .contains(&x)
                        {
                            self.remove_server_from_instance();
                        } else if x >= dialog_x + dialog_width - 108.0 {
                            self.server_remove_confirmation = None;
                            self.input_focus = 0;
                        }
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.server_remove_confirmation = None;
                        self.input_focus = 0;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.add_server_open {
                    let dialog_width = 500.0_f64.min(width - 24.0);
                    let dialog_height = 330.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if (dialog_y + 76.0..dialog_y + 114.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + dialog_width - 24.0)
                            .contains(&x)
                    {
                        self.input_focus = 22;
                    } else if (dialog_y + 142.0..dialog_y + 180.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + dialog_width - 24.0)
                            .contains(&x)
                    {
                        self.input_focus = 23;
                    } else if (dialog_y + 196.0..dialog_y + 232.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + 234.0).contains(&x)
                    {
                        self.server_pack_status = match self.server_pack_status
                        {
                            worlds::ServerPackStatus::Enabled => {
                                worlds::ServerPackStatus::Prompt
                            }
                            worlds::ServerPackStatus::Prompt => {
                                worlds::ServerPackStatus::Disabled
                            }
                            worlds::ServerPackStatus::Disabled => {
                                worlds::ServerPackStatus::Enabled
                            }
                        };
                    } else if (dialog_y + 270.0..dialog_y + 304.0).contains(&y)
                        && x >= dialog_x + dialog_width - 204.0
                        && x < dialog_x + dialog_width - 112.0
                    {
                        self.add_server_open = false;
                        self.server_name_input.clear();
                        self.server_address_input.clear();
                        self.input_focus = 0;
                    } else if (dialog_y + 270.0..dialog_y + 304.0).contains(&y)
                        && x >= dialog_x + dialog_width - 112.0
                        && x < dialog_x + dialog_width - 24.0
                    {
                        self.add_server_to_instance();
                    } else if (dialog_y + 270.0..dialog_y + 304.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + 108.0).contains(&x)
                        && self.server_edit_index.is_some()
                        && self.server_edit_source
                            == Some(
                                theseus::instance::ServerSource::LocalDesynced,
                            )
                    {
                        let index = self.server_edit_index.take().unwrap();
                        let name = self.server_name_input.clone();
                        self.add_server_open = false;
                        self.server_edit_source = None;
                        self.input_focus = 0;
                        self.server_remove_confirmation =
                            Some((index, name, false));
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.add_server_open = false;
                        self.input_focus = 0;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.migration_prompt_open {
                    let dialog_width = 520.0_f64.min(width - 24.0);
                    let dialog_height = 258.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if !self.migration_busy
                        && (dialog_y + 190.0..dialog_y + 226.0).contains(&y)
                    {
                        if x >= dialog_x + dialog_width - 230.0
                            && x < dialog_x + dialog_width - 120.0
                        {
                            if self.migration_settings_entry {
                                self.migration_prompt_open = false;
                                self.migration_settings_entry = false;
                                self.migration_plan = None;
                            } else {
                                self.migration_busy = true;
                                self.migration_saving_fresh_choice = true;
                                self.message =
                                    "Saving fresh-start choice…".into();
                                let proxy =
                                    self.proxy.as_ref().unwrap().clone();
                                self.runtime.spawn(async move {
                                    let result = native_backend::remember_fresh_start_choice(
                                        "org.northstar.launcher",
                                    )
                                    .await;
                                    let _ = proxy.send_event(
                                        UiMessage::FreshStartChoiceSaved(result),
                                    );
                                });
                            }
                        } else if x >= dialog_x + dialog_width - 118.0 {
                            if let Some(plan) =
                                self.migration_plan.clone().filter(|plan| {
                                    plan.available_bytes
                                        >= plan.copy_bytes.saturating_add(
                                            MIGRATION_SPACE_RESERVE,
                                        )
                                })
                            {
                                self.migration_busy = true;
                                self.message =
                                    "Copying the existing library…".into();
                                let proxy =
                                    self.proxy.as_ref().unwrap().clone();
                                self.runtime.spawn(async move {
                                    let result = native_backend::copy_existing_modrinth_data(plan).await;
                                    let _ = proxy.send_event(UiMessage::ExistingDataCopied(result));
                                });
                            }
                        }
                    } else if !self.migration_busy
                        && self.migration_settings_entry
                    {
                        self.migration_prompt_open = false;
                        self.migration_settings_entry = false;
                        self.migration_plan = None;
                    }
                    self.draw();
                } else if self.library_sort_menu_open
                    || self.library_group_menu_open
                {
                    if !self.is_home_library() {
                        self.library_sort_menu_open = false;
                        self.library_group_menu_open = false;
                        self.draw();
                        return;
                    }
                    let toolbar = self
                        .current_library_toolbar_layout(library_width as usize);
                    let sort_menu = self.library_sort_menu_open;
                    let anchor = if sort_menu {
                        toolbar.sort
                    } else {
                        toolbar.group
                    };
                    let options: &[&str] = if sort_menu {
                        &LIBRARY_SORT_LABELS
                    } else {
                        &LIBRARY_GROUP_LABELS
                    };
                    let bounds = library_dropdown_layout(
                        anchor.on_screen(
                            library_width as usize,
                            self.library_scroll_pixels.round() as usize,
                        ),
                        library_width as usize,
                        sidebar_height as usize,
                        options.len(),
                        208,
                    );
                    if bounds.contains(self.cursor.0, self.cursor.1) {
                        if let Some(index) = (0..options.len()).find(|&index| {
                            library_dropdown_row(bounds, index)
                                .contains(self.cursor.0, self.cursor.1)
                        }) {
                            if sort_menu {
                                self.library_sort_mode = index as u8 * 2
                                    + self.library_sort_mode % 2;
                            } else {
                                self.library_group_mode = index as u8;
                            }
                            self.list_offset = 0;
                        }
                        self.library_sort_menu_open = false;
                        self.library_group_menu_open = false;
                    } else if toolbar.sort.contains(x, y) {
                        if sort_menu
                            && x >= (toolbar.sort.x + toolbar.sort.width - 32)
                                as f64
                        {
                            self.library_sort_mode ^= 1;
                            self.library_sort_menu_open = false;
                        } else {
                            self.library_sort_menu_open = !sort_menu;
                        }
                        self.library_group_menu_open = false;
                    } else if toolbar.group.contains(x, y) {
                        self.library_group_menu_open = sort_menu;
                        self.library_sort_menu_open = false;
                    } else {
                        self.library_sort_menu_open = false;
                        self.library_group_menu_open = false;
                    }
                    self.draw();
                    return;
                } else if self.library_filter_open {
                    if !self.is_home_library() {
                        self.library_filter_open = false;
                        self.draw();
                        return;
                    }
                    let mut loaders: Vec<String> = self
                        .instance_cards
                        .values()
                        .map(|card| card.loader.clone())
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect();
                    loaders.sort_by_key(|loader| loader.to_lowercase());
                    let toolbar = self
                        .current_library_toolbar_layout(library_width as usize);
                    let layout = library_filter_popup_layout(
                        toolbar.filter.on_screen(
                            library_width as usize,
                            self.library_scroll_pixels.round() as usize,
                        ),
                        library_width as usize,
                        sidebar_height as usize,
                        loaders.len(),
                    );
                    let filter_button = toolbar.filter;
                    let anchor_hit = filter_button.contains(x, y);
                    let (x, y) = self.cursor;
                    if anchor_hit {
                        self.library_filter_open = false;
                    } else if (layout.x as f64
                        ..(layout.x + layout.width) as f64)
                        .contains(&x)
                        && (layout.y as f64..(layout.y + layout.height) as f64)
                            .contains(&y)
                    {
                        if y >= layout.clear_y as f64 {
                            self.library_loader_filters.clear();
                            self.library_instance_type_filters.clear();
                            self.library_install_state_filters.clear();
                            self.library_game_version_filters.clear();
                            self.library_game_version_filter_offset = 0;
                            self.show_hidden_instances = false;
                            self.list_offset = 0;
                        } else if y >= layout.hidden_toggle_y as f64
                            && y < layout.loader_rows_y as f64
                        {
                            self.show_hidden_instances =
                                !self.show_hidden_instances;
                            self.list_offset = 0;
                        } else if y >= layout.loader_rows_y as f64
                            && y < layout.type_header_y as f64
                        {
                            let index = ((y - layout.loader_rows_y as f64)
                                / 24.0)
                                as usize;
                            if let Some(loader) = loaders.get(index) {
                                let key = loader.to_lowercase();
                                if !self.library_loader_filters.remove(&key) {
                                    self.library_loader_filters.insert(key);
                                }
                                self.list_offset = 0;
                            }
                        } else if y >= layout.type_rows_y as f64
                            && y < layout.game_version_header_y as f64
                        {
                            let options = ["custom", "modpack", "server"];
                            let index = ((y - layout.type_rows_y as f64) / 24.0)
                                as usize;
                            if let Some(key) = options.get(index) {
                                if !self
                                    .library_instance_type_filters
                                    .remove(*key)
                                {
                                    self.library_instance_type_filters
                                        .insert((*key).to_owned());
                                }
                                self.list_offset = 0;
                            }
                        } else if y >= layout.game_version_rows_y as f64
                            && y < layout.state_header_y as f64
                        {
                            let row = ((y - layout.game_version_rows_y as f64
                                + self
                                    .library_game_version_filter_scroll_pixels
                                    .rem_euclid(24.0))
                                / 24.0)
                                as usize;
                            if row < 3 {
                                let options =
                                    self.library_game_version_options();
                                if let Some(version) = options.get(
                                    self.library_game_version_filter_offset
                                        + row,
                                ) {
                                    if !self
                                        .library_game_version_filters
                                        .remove(version)
                                    {
                                        self.library_game_version_filters
                                            .insert(version.clone());
                                    }
                                    self.list_offset = 0;
                                }
                            }
                        } else if y >= layout.state_rows_y as f64
                            && y < layout.clear_y as f64
                        {
                            let options =
                                ["installed", "installing", "not_installed"];
                            let index = ((y - layout.state_rows_y as f64)
                                / 24.0)
                                as usize;
                            if let Some(key) = options.get(index) {
                                if !self
                                    .library_install_state_filters
                                    .remove(*key)
                                {
                                    self.library_install_state_filters
                                        .insert((*key).to_owned());
                                }
                                self.list_offset = 0;
                            }
                        }
                    } else {
                        self.library_filter_open = false;
                    }
                    self.draw();
                    return;
                } else if self.create_group_open {
                    let dialog_width = 460.0_f64.min(width - 24.0);
                    let dialog_height = 224.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if (dialog_y + 96.0..dialog_y + 136.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + dialog_width - 24.0)
                            .contains(&x)
                    {
                        self.input_focus = 3;
                    } else if (dialog_y + 164.0..dialog_y + 198.0).contains(&y)
                        && (dialog_x + dialog_width - 204.0
                            ..dialog_x + dialog_width - 120.0)
                            .contains(&x)
                    {
                        self.create_group_open = false;
                        self.create_group_from_selection = false;
                        self.group_input.clear();
                        self.input_focus = 0;
                    } else if (dialog_y + 164.0..dialog_y + 198.0).contains(&y)
                        && x >= dialog_x + dialog_width - 112.0
                    {
                        if self.group_input.trim().is_empty() {
                            self.message = "Enter a name for the group.".into();
                        } else {
                            self.create_group_open = false;
                            self.input_focus = 0;
                            self.create_group();
                        }
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.create_group_open = false;
                        self.create_group_from_selection = false;
                        self.group_input.clear();
                        self.input_focus = 0;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.create_instance_open {
                    let dialog_width = 520.0_f64.min(width - 24.0);
                    let dialog_height = 320.0_f64.min(sidebar_height - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    if (dialog_y + 92.0..dialog_y + 130.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + dialog_width - 24.0)
                            .contains(&x)
                    {
                        self.input_focus = 1;
                    } else if (dialog_y + 140.0..dialog_y + 178.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + dialog_width - 24.0)
                            .contains(&x)
                    {
                        self.input_focus = 2;
                    } else if (dialog_y + 188.0..dialog_y + 226.0).contains(&y)
                        && (dialog_x + 24.0..dialog_x + 244.0).contains(&x)
                    {
                        self.loader_choice = (self.loader_choice + 1) % 5;
                        self.input_focus = 0;
                    } else if (dialog_y + 260.0..dialog_y + 294.0).contains(&y)
                        && (dialog_x + dialog_width - 204.0
                            ..dialog_x + dialog_width - 120.0)
                            .contains(&x)
                    {
                        self.create_instance_open = false;
                        self.name_input.clear();
                        self.version_input.clear();
                        self.input_focus = 0;
                    } else if (dialog_y + 260.0..dialog_y + 294.0).contains(&y)
                        && x >= dialog_x + dialog_width - 112.0
                    {
                        let can_start =
                            self.ready && !self.name_input.trim().is_empty();
                        self.create_instance();
                        if can_start {
                            self.create_instance_open = false;
                            self.input_focus = 0;
                        }
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.create_instance_open = false;
                        self.input_focus = 0;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.delete_confirmation_open
                    || self.bulk_delete_confirmation_open
                {
                    if self.instance_deletion_pending {
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }
                    let dialog_x = (width - 460.0).max(0.0) / 2.0;
                    let dialog_y = (sidebar_height - 220.0).max(0.0) / 2.0;
                    if (dialog_y + 102.0..dialog_y + 136.0).contains(&y)
                        && (dialog_x + 20.0..dialog_x + 440.0).contains(&x)
                    {
                        self.input_focus = 17;
                    } else if !self.instance_deletion_pending
                        && (dialog_y + 164.0..dialog_y + 196.0).contains(&y)
                        && (dialog_x + 260.0..dialog_x + 340.0).contains(&x)
                    {
                        self.delete_confirmation_open = false;
                        self.bulk_delete_confirmation_open = false;
                        self.delete_confirmation_input.clear();
                        self.input_focus = 0;
                    } else if (dialog_y + 164.0..dialog_y + 196.0).contains(&y)
                        && (dialog_x + 350.0..dialog_x + 440.0).contains(&x)
                    {
                        if self.bulk_delete_confirmation_open {
                            self.remove_selected_library_instances();
                        } else {
                            self.remove_selected_instance();
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.launcher_import_open {
                    if self.launcher_import_busy {
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }
                    let dialog_width = 520.0_f64.min(width - 24.0);
                    let dialog_x = (width - dialog_width) / 2.0;
                    let source = self.launcher_import_source.clone();
                    let rows = source
                        .as_ref()
                        .map_or(import_launcher_options().len() + 1, |s| {
                            s.folders.len()
                        });
                    let dialog_height = (112 + rows.min(7) * 44) as f64;
                    let dialog_y = (sidebar_height - dialog_height) / 2.0;
                    let row_top = if source.is_some() {
                        dialog_y + 68.0
                    } else {
                        dialog_y + 56.0
                    };
                    let clicked = (0..rows.min(7)).find(|index| {
                        let row_y = row_top + *index as f64 * 44.0
                            - self
                                .launcher_import_scroll_pixels
                                .rem_euclid(44.0);
                        (row_y..row_y + 38.0).contains(&y)
                            && (dialog_x + 16.0..dialog_x + dialog_width - 16.0)
                                .contains(&x)
                    });
                    if let Some(index) = clicked {
                        if let Some(source) = source {
                            self.launcher_import_focus =
                                self.launcher_import_offset + index;
                            if let Some(folder) = source
                                .folders
                                .get(self.launcher_import_offset + index)
                                .cloned()
                            {
                                self.import_launcher_folder(source, folder);
                            }
                        } else if index == 0 {
                            self.launcher_import_focus = 0;
                            self.launcher_import_open = false;
                            self.pick_modpack_file();
                        } else if let Some((_, launcher)) =
                            import_launcher_options().get(index - 1).copied()
                        {
                            self.launcher_import_focus = index;
                            self.load_launcher_instances(launcher);
                        }
                    } else if x < dialog_x
                        || x >= dialog_x + dialog_width
                        || y < dialog_y
                        || y >= dialog_y + dialog_height
                    {
                        self.launcher_import_open = false;
                        self.launcher_import_source = None;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if x < 64.0 {
                    if navigation_rail_button(56).contains(x, y) {
                        self.return_to_library(None);
                        self.accounts_open = false;
                        self.settings_open = false;
                        self.java_open = false;
                        self.create_instance_open = false;
                    } else if navigation_rail_button(108).contains(x, y) {
                        self.accounts_open = false;
                        self.settings_open = false;
                        self.java_open = false;
                        self.open_global_browse();
                    } else if navigation_rail_button(160).contains(x, y)
                        && self.settings.as_ref().is_none_or(|settings| {
                            settings.show_skin_selector_in_sidebar
                        })
                    {
                        self.accounts_open = false;
                        self.settings_open = false;
                        self.java_open = false;
                        self.open_minecraft_skins();
                    } else if navigation_rail_button(212).contains(x, y) {
                        self.accounts_open = false;
                        self.settings_open = false;
                        self.java_open = false;
                        self.open_global_screenshots();
                    } else if navigation_rail_button(264).contains(x, y) {
                        self.accounts_open = false;
                        self.settings_open = false;
                        self.java_open = false;
                        self.open_global_hosting();
                    } else if let Some((instance_id, instance_name)) = self
                        .sidebar_instances(sidebar_height as usize)
                        .iter()
                        .enumerate()
                        .find_map(|(index, (instance_id, instance_name))| {
                            sidebar_instance_button(index).contains(x, y).then(
                                || (instance_id.clone(), instance_name.clone()),
                            )
                        })
                    {
                        self.return_to_library(None);
                        self.accounts_open = false;
                        self.settings_open = false;
                        self.java_open = false;
                        self.create_instance_open = false;
                        self.open_library_instance(instance_id, instance_name);
                    } else if sidebar_footer_button(sidebar_height as usize, 0)
                        .contains(x, y)
                    {
                        self.return_to_library(None);
                        self.create_instance_open = true;
                        self.name_input.clear();
                        self.version_input.clear();
                        self.input_focus = 1;
                    } else if sidebar_footer_button(sidebar_height as usize, 1)
                        .contains(x, y)
                    {
                        self.global_browse = false;
                        self.global_screenshots = false;
                        self.global_hosting = false;
                        self.global_skins = false;
                        self.global_user_profile = false;
                        self.accounts_open = false;
                        self.settings_open = true;
                        self.java_open = false;
                        self.create_instance_open = false;
                        self.settings = None;
                        self.settings_section = 0;
                        self.settings_jump_in_focused = false;
                        self.settings_scroll_pixels = 0.0;
                        self.settings_scroll_target_pixels = 0.0;
                        self.load_global_screenshot_sync();
                        self.selected_instance = None;
                        self.input_focus = 0;
                        self.load_settings();
                        self.load_maximum_memory();
                    } else if sidebar_footer_button(sidebar_height as usize, 2)
                        .contains(x, y)
                    {
                        self.global_browse = false;
                        self.global_screenshots = false;
                        self.global_hosting = false;
                        self.global_skins = false;
                        self.global_user_profile = false;
                        self.settings_open = false;
                        self.java_open = false;
                        self.create_instance_open = false;
                        self.accounts_open = true;
                        self.selected_instance = None;
                        self.account_tab = 0;
                        self.minecraft_accounts.clear();
                        self.content_accounts.clear();
                        self.default_account = None;
                        self.account_offset = 0;
                        self.account_scroll_pixels = 0.0;
                        self.account_scroll_target_pixels = 0.0;
                        self.message = "Loading Minecraft accounts…".into();
                        self.input_focus = 0;
                        self.load_accounts();
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.global_hosting {
                    if self.hosting_server_id.is_some() {
                        if (130.0..162.0).contains(&y)
                            && (96.0..272.0).contains(&x)
                        {
                            self.hosting_connection = None;
                            self.hosting_server_id = None;
                            self.hosting_server_details = None;
                            self.hosting_server_details_error = None;
                            self.hosting_server_details_loading = false;
                            self.hosting_socket_connecting = false;
                            self.hosting_socket_connected = false;
                            self.hosting_socket_error = None;
                            self.hosting_power_state = None;
                            self.hosting_live_stats = None;
                            self.hosting_console_lines.clear();
                            self.hosting_address_copied = false;
                            self.hosting_access_users.clear();
                            self.hosting_access_loaded = false;
                            self.hosting_access_loading = false;
                            self.hosting_access_error = None;
                            self.hosting_access_invite_open = false;
                            self.hosting_access_search_query.clear();
                            self.hosting_access_search_results.clear();
                            self.hosting_access_search_complete = false;
                            self.hosting_access_search_scroll_pixels = 0.0;
                            self.hosting_access_search_scroll_target_pixels =
                                0.0;
                        } else if (130.0..162.0).contains(&y)
                            && (440.0..536.0).contains(&x)
                        {
                            self.hosting_tab = 0;
                            self.hosting_access_scroll_pixels = 0.0;
                            self.hosting_access_scroll_target_pixels = 0.0;
                            self.hosting_access_search_scroll_pixels = 0.0;
                            self.hosting_access_search_scroll_target_pixels =
                                0.0;
                        } else if (130.0..162.0).contains(&y)
                            && (544.0..632.0).contains(&x)
                        {
                            self.hosting_tab = 1;
                            self.input_focus = 0;
                            self.hosting_backup_create_open = false;
                            self.hosting_backup_scroll_pixels = 0.0;
                            self.hosting_backup_scroll_target_pixels = 0.0;
                            self.load_hosting_backups();
                        } else if (130.0..162.0).contains(&y)
                            && (648.0..736.0).contains(&x)
                        {
                            self.hosting_tab = 2;
                            self.input_focus = 0;
                            self.hosting_access_search_scroll_pixels = 0.0;
                            self.hosting_access_search_scroll_target_pixels =
                                0.0;
                            self.load_hosting_server_access();
                        } else if self.hosting_tab == 1
                            && (194.0..230.0).contains(&y)
                        {
                            let panel_width =
                                (library_width - 144.0).max(0.0).min(940.0);
                            let create_x = 96.0 + panel_width - 156.0;
                            let delete_x = create_x - 160.0;
                            let has_deleted_backups = self
                                .hosting_backups
                                .as_ref()
                                .is_some_and(|backups| {
                                    !backups.deleted_backups.is_empty()
                                });
                            if has_deleted_backups
                                && (120.0..180.0).contains(&x)
                            {
                                self.hosting_backup_show_deleted = false;
                                self.hosting_backup_selected.clear();
                                self.hosting_backup_scroll_pixels = 0.0;
                                self.hosting_backup_scroll_target_pixels = 0.0;
                                self.input_focus = 0;
                            } else if has_deleted_backups
                                && (184.0..244.0).contains(&x)
                            {
                                self.hosting_backup_show_deleted = true;
                                self.hosting_backup_selected.clear();
                                self.hosting_backup_scroll_pixels = 0.0;
                                self.hosting_backup_scroll_target_pixels = 0.0;
                                self.input_focus = 0;
                            } else if self.hosting_backup_show_deleted
                                && !self.hosting_backup_selected.is_empty()
                                && self.hosting_backups.as_ref().is_some_and(
                                    |backups| {
                                        backups.deleted_backups.iter().any(
                                            |backup| {
                                                backup.can_undelete
                                                    && self
                                                        .hosting_backup_selected
                                                        .contains(&backup.id)
                                            },
                                        )
                                    },
                                )
                                && (delete_x..delete_x + 148.0).contains(&x)
                            {
                                self.undelete_selected_hosting_backups();
                            } else if self.hosting_backup_show_deleted
                                && (create_x..create_x + 132.0).contains(&x)
                            {
                                self.hosting_backup_deleted_sort =
                                    (self.hosting_backup_deleted_sort + 1) % 6;
                                self.hosting_backup_scroll_pixels = 0.0;
                                self.hosting_backup_scroll_target_pixels = 0.0;
                            } else if !self.hosting_backup_show_deleted
                                && !self.hosting_backup_selected.is_empty()
                                && (delete_x..delete_x + 148.0).contains(&x)
                            {
                                self.open_hosting_backup_bulk_delete_prompt();
                            } else if !self.hosting_backup_show_deleted
                                && !self.hosting_backup_options_open
                                && self.hosting_backup_selected.is_empty()
                                && panel_width >= 520.0
                                && (create_x - 112.0..create_x - 8.0)
                                    .contains(&x)
                            {
                                self.open_hosting_backup_options();
                            } else if !self.hosting_backup_show_deleted
                                && !self.hosting_backup_options_open
                                && (create_x..create_x + 132.0).contains(&x)
                            {
                                let slots_full =
                                    !self.hosting_backups.as_ref().is_some_and(
                                        |backups| {
                                            backups.manual_count
                                                < backups.options.split_manual
                                        },
                                    ) || !self.hosting_can_manage_backups();
                                if slots_full {
                                    self.message =
                                        "All manual backup slots are in use."
                                            .into();
                                } else {
                                    self.hosting_backup_create_open = true;
                                    self.hosting_backup_options_open = false;
                                    self.hosting_backup_create_name.clear();
                                    self.input_focus = 38;
                                }
                            }
                        } else if self.hosting_tab == 1
                            && self.hosting_backup_create_open
                            && (248.0..280.0).contains(&y)
                        {
                            let panel_width =
                                (library_width - 144.0).max(0.0).min(940.0);
                            let field_width = (panel_width - 252.0).max(120.0);
                            let submit_x = 128.0 + field_width;
                            if (120.0..120.0 + field_width).contains(&x) {
                                self.input_focus = 38;
                            } else if (submit_x..submit_x + 84.0).contains(&x) {
                                self.create_hosting_backup();
                            } else if (submit_x + 92.0..submit_x + 176.0)
                                .contains(&x)
                            {
                                self.hosting_backup_create_open = false;
                                self.hosting_backup_create_name.clear();
                                self.input_focus = 0;
                            }
                        } else if self.hosting_tab == 1
                            && self.hosting_backup_options_open
                            && y >= 248.0
                            && y < 422.0
                        {
                            if (248.0..280.0).contains(&y)
                                && (120.0..340.0).contains(&x)
                            {
                                let slot_limit = self
                                    .hosting_backups
                                    .as_ref()
                                    .map_or(0, |backups| {
                                        backups.options.limit.min(15)
                                    });
                                if !self.hosting_backup_auto_enabled
                                    && slot_limit == 0
                                {
                                    self.message = "This server has no backup slots available.".into();
                                } else {
                                    self.hosting_backup_auto_enabled =
                                        !self.hosting_backup_auto_enabled;
                                    self.hosting_backup_manual_slots =
                                        hosting_backup_manual_slots_after_auto_toggle(
                                            self.hosting_backup_auto_enabled,
                                            self.hosting_backup_manual_slots,
                                            slot_limit,
                                        );
                                }
                            } else if (306.0..338.0).contains(&y) {
                                let choices = hosting_backup_interval_choices(
                                    self.hosting_backup_interval_hours,
                                );
                                for (index, interval) in
                                    choices.into_iter().enumerate()
                                {
                                    let left = 120.0 + index as f64 * 64.0;
                                    if (left..left + 56.0).contains(&x)
                                        && self.hosting_backup_auto_enabled
                                    {
                                        self.hosting_backup_interval_hours =
                                            interval;
                                    }
                                }
                            } else if (342.0..374.0).contains(&y)
                                && self.hosting_backup_auto_enabled
                            {
                                let slot_limit = self
                                    .hosting_backups
                                    .as_ref()
                                    .map_or(0, |backups| {
                                        backups.options.limit.min(15)
                                    });
                                if (344.0..380.0).contains(&x) {
                                    self.hosting_backup_manual_slots = self
                                        .hosting_backup_manual_slots
                                        .saturating_sub(1);
                                } else if (390.0..426.0).contains(&x) {
                                    self.hosting_backup_manual_slots = self
                                        .hosting_backup_manual_slots
                                        .saturating_add(1)
                                        .min(slot_limit.saturating_sub(1));
                                }
                            } else if (390.0..422.0).contains(&y) {
                                if (120.0..224.0).contains(&x) {
                                    self.save_hosting_backup_options(false);
                                } else if (232.0..336.0).contains(&x)
                                    && !self.hosting_backup_options_saving
                                {
                                    self.cancel_hosting_backup_options();
                                }
                            }
                        } else if self.hosting_tab == 1
                            && self.hosting_backup_show_deleted
                            && !self.hosting_backup_options_open
                            && (286.0..318.0).contains(&y)
                        {
                            if (112.0..192.0).contains(&x) {
                                self.hosting_backup_deleted_reason_filter =
                                    (self.hosting_backup_deleted_reason_filter
                                        + 1)
                                        % 3;
                            } else if (198.0..262.0).contains(&x) {
                                self.hosting_backup_deleted_type_filter = (self
                                    .hosting_backup_deleted_type_filter
                                    + 1)
                                    % 3;
                            } else if (268.0..380.0).contains(&x) {
                                self.hosting_backup_deleted_recoverability_filter =
                                    (self.hosting_backup_deleted_recoverability_filter + 1) % 3;
                            }
                            self.hosting_backup_scroll_pixels = 0.0;
                            self.hosting_backup_scroll_target_pixels = 0.0;
                        } else if self.hosting_tab == 1
                            && !self.hosting_backup_create_open
                            && (248.0..280.0).contains(&y)
                        {
                            let panel_width =
                                (library_width - 144.0).max(0.0).min(940.0);
                            let field_width = (panel_width - 48.0).max(0.0);
                            if (120.0..120.0 + field_width).contains(&x) {
                                self.input_focus = 39;
                            }
                        } else if self.hosting_tab == 1
                            && !self.hosting_backup_create_open
                            && y >= if self.hosting_backup_options_open {
                                436.0
                            } else if self.hosting_backup_show_deleted {
                                328.0
                            } else {
                                296.0
                            }
                            && y < sidebar_height - 62.0
                        {
                            let panel_width =
                                (library_width - 144.0).max(0.0).min(940.0);
                            let offset = self.hosting_backup_scroll_pixels;
                            let list_top = if self.hosting_backup_options_open {
                                436.0
                            } else if self.hosting_backup_show_deleted {
                                328.0
                            } else {
                                296.0
                            };
                            let query = self
                                .hosting_backup_search
                                .trim()
                                .to_lowercase();
                            let matching_indices = self
                                .hosting_backups
                                .as_ref()
                                .map_or_else(Vec::new, |backups| {
                                    if self.hosting_backup_show_deleted {
                                        let mut indices = backups
                                            .deleted_backups
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(index, backup)| {
                                            hosting_deleted_backup_matches(
                                                backup,
                                                    &query,
                                                self.hosting_backup_deleted_reason_filter,
                                                self.hosting_backup_deleted_type_filter,
                                                self.hosting_backup_deleted_recoverability_filter,
                                                )
                                                .then_some(index)
                                            })
                                            .collect::<Vec<_>>();
                                        indices.sort_by(|left, right| {
                                            hosting_deleted_backup_sort_order(
                                                &backups.deleted_backups[*left],
                                                &backups.deleted_backups[*right],
                                                self.hosting_backup_deleted_sort,
                                            )
                                        });
                                        indices
                                    } else {
                                        backups
                                            .backups
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(index, backup)| {
                                                (backup.status == "done"
                                                    && hosting_backup_search_matches(
                                                        &backup.name,
                                                        &query,
                                                    ))
                                                .then_some(index)
                                            })
                                            .collect::<Vec<_>>()
                                    }
                                });
                            let index = ((y - list_top + offset) / 62.0)
                                .floor()
                                .max(0.0)
                                as usize;
                            let row_y = list_top + index as f64 * 62.0 - offset;
                            if (row_y + 11.0..row_y + 43.0).contains(&y)
                                && self.hosting_backup_show_deleted
                                && let Some(backup_index) =
                                    matching_indices.get(index).copied()
                                && let Some(backup) = self
                                    .hosting_backups
                                    .as_ref()
                                    .and_then(|backups| {
                                        backups
                                            .deleted_backups
                                            .get(backup_index)
                                    })
                                    .cloned()
                            {
                                let row_y_for_bounds =
                                    row_y.max(0.0).round() as usize;
                                let row_right = 112 + panel_width as usize - 32;
                                let [
                                    selection_bounds,
                                    copy_bounds,
                                    recover_bounds,
                                ] = hosting_deleted_backup_row_controls(
                                    row_right,
                                    row_y_for_bounds,
                                );
                                if recover_bounds.contains(x, y)
                                    && !self.hosting_backup_mutating
                                {
                                    self.undelete_hosting_backup(backup);
                                } else if selection_bounds.contains(x, y)
                                    && !self.hosting_backup_mutating
                                {
                                    if !self
                                        .hosting_backup_selected
                                        .remove(&backup.id)
                                    {
                                        self.hosting_backup_selected
                                            .insert(backup.id);
                                    }
                                } else if copy_bounds.contains(x, y) {
                                    match set_clipboard_text(&backup.id) {
                                        Ok(()) => {
                                            self.message =
                                                "Backup ID copied.".into();
                                        }
                                        Err(error) => {
                                            self.message = format!(
                                                "Could not copy backup ID: {error}"
                                            );
                                        }
                                    }
                                }
                            } else if (row_y + 11.0..row_y + 43.0).contains(&y)
                                && !self.hosting_backup_show_deleted
                                && let Some(backup_index) =
                                    matching_indices.get(index).copied()
                                && let Some(backup) = self
                                    .hosting_backups
                                    .as_ref()
                                    .and_then(|backups| {
                                        backups.backups.get(backup_index)
                                    })
                                    .cloned()
                            {
                                let can_manage =
                                    self.hosting_can_manage_backups();
                                let row_right = 112.0 + panel_width - 32.0;
                                let [
                                    download_bounds,
                                    restore_bounds,
                                    rename_bounds,
                                    delete_bounds,
                                ] = hosting_backup_action_bounds(row_right);
                                if (120.0..140.0).contains(&x) {
                                    if !self
                                        .hosting_backup_selected
                                        .remove(&backup.id)
                                    {
                                        self.hosting_backup_selected
                                            .insert(backup.id.clone());
                                    }
                                } else if (download_bounds.0..download_bounds.1)
                                    .contains(&x)
                                {
                                    if backup.status == "done"
                                        && !self.hosting_backup_downloading
                                    {
                                        self.download_hosting_backup(backup);
                                    }
                                } else if (restore_bounds.0..restore_bounds.1)
                                    .contains(&x)
                                {
                                    if can_manage
                                        && backup.status == "done"
                                        && !self.hosting_server_is_running()
                                    {
                                        self.open_hosting_backup_prompt(
                                            HostingBackupPromptAction::Restore,
                                            backup,
                                        );
                                    } else if self.hosting_server_is_running() {
                                        self.message = "Stop the server before restoring a backup.".into();
                                    }
                                } else if (rename_bounds.0..rename_bounds.1)
                                    .contains(&x)
                                {
                                    if can_manage && backup.status == "done" {
                                        self.open_hosting_backup_prompt(
                                            HostingBackupPromptAction::Rename,
                                            backup,
                                        );
                                    }
                                } else if (delete_bounds.0..delete_bounds.1)
                                    .contains(&x)
                                    && can_manage
                                    && backup.status == "done"
                                {
                                    self.open_hosting_backup_prompt(
                                        HostingBackupPromptAction::Delete,
                                        backup,
                                    );
                                }
                            }
                        } else if self.hosting_tab == 2
                            && (194.0..230.0).contains(&y)
                        {
                            let panel_width =
                                (library_width - 144.0).max(0.0).min(940.0);
                            let invite_x = 96.0 + panel_width - 156.0;
                            if (invite_x..invite_x + 132.0).contains(&x) {
                                self.hosting_access_invite_open =
                                    !self.hosting_access_invite_open;
                                self.input_focus =
                                    if self.hosting_access_invite_open {
                                        34
                                    } else {
                                        0
                                    };
                                self.hosting_access_invite_error = None;
                                self.hosting_access_search_complete = false;
                                self.hosting_access_search_scroll_pixels = 0.0;
                                self.hosting_access_search_scroll_target_pixels = 0.0;
                            }
                        } else if self.hosting_tab == 2
                            && self.hosting_access_invite_open
                        {
                            let panel_width =
                                (library_width - 144.0).max(0.0).min(940.0);
                            let search_width =
                                (panel_width - 200.0).max(120.0).min(360.0);
                            if (248.0..280.0).contains(&y)
                                && (120.0..120.0 + search_width).contains(&x)
                            {
                                self.input_focus = 34;
                            } else if (248.0..280.0).contains(&y)
                                && (132.0 + search_width..220.0 + search_width)
                                    .contains(&x)
                            {
                                self.search_hosting_access_users();
                            } else if (292.0..324.0).contains(&y)
                                && (176.0..260.0).contains(&x)
                            {
                                self.hosting_access_role = 0;
                            } else if (292.0..324.0).contains(&y)
                                && (268.0..352.0).contains(&x)
                            {
                                self.hosting_access_role = 1;
                            } else if y >= 360.0 {
                                let list_bottom = sidebar_height - 76.0;
                                let add_x = 96.0 + panel_width - 112.0;
                                if y < list_bottom
                                    && (add_x..add_x + 88.0).contains(&x)
                                {
                                    let existing_ids = self
                                        .hosting_access_users
                                        .iter()
                                        .map(|member| {
                                            member.user.id.to_lowercase()
                                        })
                                        .collect::<HashSet<_>>();
                                    let candidates = self
                                        .hosting_access_search_results
                                        .iter()
                                        .filter(|user| {
                                            !existing_ids.contains(
                                                &user.id.to_lowercase(),
                                            )
                                        })
                                        .collect::<Vec<_>>();
                                    let index = ((y - 360.0
                                        + self.hosting_access_search_scroll_pixels)
                                        / 54.0)
                                        .floor()
                                        .max(0.0) as usize;
                                    let row_y = 360.0 + index as f64 * 54.0
                                        - self.hosting_access_search_scroll_pixels;
                                    if (row_y + 8.0..row_y + 40.0).contains(&y)
                                        && let Some(user) =
                                            candidates.get(index).copied()
                                    {
                                        self.invite_hosting_access_user(
                                            user.clone(),
                                        );
                                    }
                                }
                            }
                        } else if self.hosting_tab == 2
                            && self.hosting_access_loaded
                            && !self.hosting_access_invite_open
                            && y >= 264.0
                            && y < sidebar_height - 62.0
                        {
                            let panel_width =
                                (library_width - 144.0).max(0.0).min(940.0);
                            let index = ((y - 264.0
                                + self.hosting_access_scroll_pixels)
                                / 62.0)
                                .floor()
                                .max(0.0)
                                as usize;
                            let row_y = 264.0 + index as f64 * 62.0
                                - self.hosting_access_scroll_pixels;
                            let row_layout = hosting_access_row_layout(
                                panel_width as usize,
                                row_y.max(0.0).round() as usize,
                            );
                            if let Some(member) =
                                self.hosting_access_users.get(index).cloned()
                            {
                                let owner_id = self
                                    .hosting_server_details
                                    .as_ref()
                                    .map(|details| {
                                        details.server.owner_id.clone()
                                    })
                                    .or_else(|| {
                                        self.hosting_servers
                                            .iter()
                                            .find(|server| {
                                                self.hosting_server_id
                                                    .as_deref()
                                                    == Some(
                                                        server
                                                            .server_id
                                                            .as_str(),
                                                    )
                                            })
                                            .map(|server| {
                                                server.owner_id.clone()
                                            })
                                    });
                                let is_owner = owner_id.as_deref().is_some_and(
                                    |id| {
                                        id.eq_ignore_ascii_case(
                                            &member.user.id,
                                        )
                                    },
                                ) || matches!(
                                    member.role(),
                                    native_backend::HostingServerAccessRole::Owner
                                );
                                if member.added_on.is_none()
                                    && !is_owner
                                    && row_layout.cancel_invite.contains(x, y)
                                {
                                    let confirming = self
                                        .hosting_access_remove_confirmation
                                        .as_deref()
                                        == Some(member.user.id.as_str());
                                    if confirming {
                                        self.remove_hosting_access_user(
                                            &member,
                                        );
                                    } else {
                                        self.hosting_access_remove_confirmation =
                                            Some(member.user.id.clone());
                                    }
                                } else if row_layout.remove.contains(x, y) {
                                    let confirming = self
                                        .hosting_access_remove_confirmation
                                        .as_deref()
                                        == Some(member.user.id.as_str());
                                    if member.added_on.is_none() {
                                        self.reinvite_hosting_access_user(
                                            &member,
                                        );
                                    } else if confirming {
                                        self.remove_hosting_access_user(
                                            &member,
                                        );
                                    } else {
                                        self.hosting_access_remove_confirmation =
                                            Some(member.user.id.clone());
                                    }
                                } else if row_layout.role.contains(x, y) {
                                    if !is_owner {
                                        let role = match member.role() {
                                            native_backend::HostingServerAccessRole::Viewer => {
                                                Some(
                                                    native_backend::HostingServerAccessRole::Editor,
                                                )
                                            }
                                            native_backend::HostingServerAccessRole::Editor => {
                                                Some(
                                                    native_backend::HostingServerAccessRole::Viewer,
                                                )
                                            }
                                            native_backend::HostingServerAccessRole::Owner => None,
                                        };
                                        if let Some(role) = role {
                                            self.update_hosting_access_role(
                                                &member, role,
                                            );
                                        }
                                    }
                                }
                            }
                        } else if self.hosting_tab == 2
                            && (280.0..312.0).contains(&y)
                            && (120.0..208.0).contains(&x)
                        {
                            self.hosting_access_loading = false;
                            self.hosting_access_loaded = false;
                            self.hosting_access_error = None;
                            self.load_hosting_server_access();
                        } else if (130.0..162.0).contains(&y)
                            && (288.0..424.0).contains(&x)
                        {
                            if let Some(details) =
                                self.hosting_server_details.as_ref()
                            {
                                let state = self
                                    .hosting_power_state
                                    .as_deref()
                                    .unwrap_or(&details.server.status);
                                if state == "running" {
                                    self.request_hosting_power(
                                        HostingPowerAction::Stop,
                                    );
                                } else if matches!(
                                    state,
                                    "stopped"
                                        | "idle"
                                        | "crashed"
                                        | "available"
                                ) {
                                    self.request_hosting_power(
                                        HostingPowerAction::Start,
                                    );
                                }
                            }
                        } else if (296.0..326.0).contains(&y) {
                            let panel_width =
                                (library_width - 144.0).max(0.0).min(940.0);
                            let copy_x = 96.0 + panel_width - 88.0;
                            if (copy_x..copy_x + 72.0).contains(&x) {
                                if let Some(details) =
                                    self.hosting_server_details.as_ref()
                                {
                                    let address = format!(
                                        "{}:{}",
                                        details.server.net.domain,
                                        details.server.net.port
                                    );
                                    match set_clipboard_text(&address) {
                                        Ok(()) => {
                                            self.hosting_address_copied = true;
                                            let server_id =
                                                self.hosting_server_id.clone();
                                            if let (
                                                Some(server_id),
                                                Some(proxy),
                                            ) =
                                                (server_id, self.proxy.clone())
                                            {
                                                self.runtime.spawn(async move {
                                                    tokio::time::sleep(std::time::Duration::from_millis(1600)).await;
                                                    let _ = proxy.send_event(UiMessage::HostingAddressCopyFeedbackClear(server_id));
                                                });
                                            }
                                        }
                                        Err(error) => {
                                            self.hosting_power_error = Some(
                                                format!(
                                                    "Could not copy address: {error}"
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                        } else if y >= sidebar_height - 102.0
                            && y < sidebar_height - 66.0
                            && (112.0
                                ..(96.0
                                    + (library_width - 144.0)
                                        .max(0.0)
                                        .min(940.0)))
                                .contains(&x)
                        {
                            self.input_focus = 33;
                        }
                    } else if hosting_servers_refresh_button(
                        library_width as usize,
                    )
                    .contains(x, y)
                    {
                        if !self.hosting_servers_loading
                            && !self.hosting_servers_refreshing
                        {
                            self.hosting_servers_refreshing = true;
                            self.hosting_servers_error = None;
                            self.load_hosting_servers();
                        }
                    } else if y >= 148.0
                        && y < sidebar_height - 42.0
                        && x >= 96.0
                        && x < 96.0
                            + (library_width - 144.0).max(0.0).min(900.0)
                    {
                        let row =
                            ((y - 148.0 + self.hosting_scroll_pixels) / 82.0)
                                .floor()
                                .max(0.0) as usize;
                        if let Some(server) = self.hosting_servers.get(row) {
                            self.open_hosting_server(server.server_id.clone());
                        }
                    }
                    self.draw();
                    return;
                }
                if self.accounts_open {
                    if (82.0..114.0).contains(&y) && x >= width - 142.0 {
                        self.accounts_open = false;
                        self.input_focus = 0;
                        if let Some(task) = self.modrinth_login_task.take() {
                            task.abort();
                            self.message = "Modrinth sign-in canceled.".into();
                        }
                    } else if (119.0..144.0).contains(&y)
                        && (292.0..418.0).contains(&x)
                    {
                        self.account_tab = 0;
                        self.account_offset = 0;
                        self.account_scroll_pixels = 0.0;
                        self.account_scroll_target_pixels = 0.0;
                        self.message = format!(
                            "{} Minecraft accounts",
                            self.minecraft_accounts.len()
                        );
                    } else if (119.0..144.0).contains(&y)
                        && (424.0..550.0).contains(&x)
                    {
                        self.account_tab = 1;
                        self.account_offset = 0;
                        self.account_scroll_pixels = 0.0;
                        self.account_scroll_target_pixels = 0.0;
                        self.message = format!(
                            "{} Modrinth accounts",
                            self.content_accounts.len()
                        );
                    } else if self.account_tab == 1 {
                        if (154.0..188.0).contains(&y)
                            && (292.0..458.0).contains(&x)
                        {
                            if let Some(task) = self.modrinth_login_task.take()
                            {
                                task.abort();
                                self.message =
                                    "Modrinth sign-in canceled.".into();
                            } else {
                                self.begin_modrinth_login();
                            }
                        } else if y >= 204.0 && y < sidebar_height - 48.0 {
                            let row = ((y + self.account_scroll_pixels - 204.0)
                                / 58.0)
                                .floor()
                                .max(0.0)
                                as usize;
                            if let Some(account) =
                                self.content_accounts.get(row)
                            {
                                if x >= width - 212.0 && x < width - 48.0 {
                                    self.remove_content_service_account(
                                        account.user_id.clone(),
                                    );
                                    self.message = format!(
                                        "Removing Modrinth account {}…",
                                        account.user_id
                                    );
                                } else if x >= width - 350.0
                                    && x < width - 218.0
                                    && !account.active
                                {
                                    self.select_content_service_account(
                                        account.user_id.clone(),
                                    );
                                    self.message = format!(
                                        "Setting {} as the active Modrinth account…",
                                        account.user_id
                                    );
                                }
                            }
                        }
                    } else if self.login_flow.is_some() {
                        if (190.0..224.0).contains(&y)
                            && (292.0..622.0).contains(&x)
                        {
                            self.input_focus = 6;
                        } else if (190.0..224.0).contains(&y)
                            && (632.0..694.0).contains(&x)
                        {
                            self.paste_login_code();
                        } else if (190.0..224.0).contains(&y)
                            && (700.0..762.0).contains(&x)
                        {
                            self.finish_minecraft_login();
                        } else if (236.0..266.0).contains(&y)
                            && (292.0..388.0).contains(&x)
                        {
                            self.login_flow = None;
                            self.login_code_input.clear();
                            self.input_focus = 0;
                            self.message = "Microsoft sign-in canceled.".into();
                        }
                    } else if (154.0..188.0).contains(&y)
                        && (292.0..458.0).contains(&x)
                    {
                        self.begin_minecraft_login();
                    } else if y >= 204.0 && y < sidebar_height - 48.0 {
                        let row =
                            ((y + self.account_scroll_pixels - 204.0) / 58.0)
                                .floor()
                                .max(0.0) as usize;
                        if let Some(account) = self.minecraft_accounts.get(row)
                        {
                            if x >= width - 212.0 && x < width - 48.0 {
                                self.remove_minecraft_account(account.id);
                                self.message = format!(
                                    "Removing Minecraft account {}…",
                                    account.username
                                );
                            } else if x >= width - 350.0
                                && x < width - 218.0
                                && self.default_account != Some(account.id)
                            {
                                self.select_minecraft_account(account.id);
                                self.message = format!(
                                    "Setting {} as the default account…",
                                    account.username
                                );
                            }
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.settings_open {
                    if self.java_open {
                        if settings_close_button(width.max(0.0) as usize)
                            .contains(x, y)
                        {
                            self.java_open = false;
                        } else if (146.0..182.0).contains(&y)
                            && (292.0..502.0).contains(&x)
                        {
                            self.pick_java_runtime();
                        } else if y >= 214.0
                            && x >= width - 212.0
                            && x < width - 48.0
                        {
                            let row = (y as usize).saturating_sub(214) / 58;
                            if let Some(major) =
                                [25u32, 21, 17, 8].get(row).copied()
                            {
                                if !self.java_versions.iter().any(|version| {
                                    version.parsed_version == major
                                }) {
                                    self.install_java_runtime(major);
                                }
                            }
                        }
                    } else if settings_close_button(
                        library_width.max(0.0) as usize
                    )
                    .contains(x, y)
                    {
                        self.settings_open = false;
                    } else if (sidebar_height as f64 - 56.0
                        ..sidebar_height as f64 - 20.0)
                        .contains(&y)
                        && (292.0..502.0).contains(&x)
                    {
                        self.java_open = true;
                        self.java_versions.clear();
                        self.java_busy = false;
                        self.message = "Loading Java runtimes…".into();
                        self.load_java_versions();
                    } else if self.settings_open
                        && !self.java_open
                        && (292.0..476.0).contains(&x)
                        && settings_section_at_y(y).is_some()
                    {
                        if let Some(section) = settings_section_at_y(y)
                            && section < SETTINGS_SECTION_ROW_INDICES.len()
                        {
                            self.settings_section = section;
                            self.settings_jump_in_focused = false;
                            self.input_focus = 0;
                            self.settings_scroll_pixels = 0.0;
                            self.settings_scroll_target_pixels = 0.0;
                        }
                    } else if self.settings.is_some() {
                        let content_bottom = (sidebar_height - 56.0).max(0.0);
                        let settings_layout = settings_row_layout(
                            library_width.max(0.0) as usize,
                        );
                        let section_rows = SETTINGS_SECTION_ROW_INDICES[self
                            .settings_section
                            .min(SETTINGS_SECTION_ROW_INDICES.len() - 1)];
                        let row = if y >= 146.0
                            && x >= 496.0
                            && x < settings_layout.row_right as f64
                            && y < content_bottom
                        {
                            settings_row_position_at(
                                self.settings_section,
                                section_rows.len(),
                                settings_layout.row_width,
                                y,
                                self.settings_scroll_pixels,
                            )
                            .and_then(|position| section_rows.get(position))
                            .copied()
                        } else {
                            None
                        };
                        let editing_launch_defaults =
                            matches!(row, Some(27 | 28 | 30 | 31 | 32))
                                && x >= settings_layout.launch_field_x as f64;
                        if !matches!(row, Some(25 | 26))
                            && !editing_launch_defaults
                            && matches!(self.input_focus, 26 | 27 | 30..=34)
                        {
                            self.input_focus = 0;
                        }
                        if matches!(row, Some(25 | 26))
                            && (settings_layout.dimension_x as f64
                                ..settings_layout.row_right as f64)
                                .contains(&x)
                            && self.settings.as_ref().is_some_and(|settings| {
                                !settings.force_fullscreen
                            })
                            && !self.window_defaults_saving
                        {
                            if let Some(settings) = self.settings.as_ref() {
                                self.default_window_width_input =
                                    settings.game_resolution.0.to_string();
                                self.default_window_height_input =
                                    settings.game_resolution.1.to_string();
                            }
                            self.input_focus =
                                if row == Some(25) { 26 } else { 27 };
                            if self.input_focus == 26 {
                                self.default_window_width_input.clear();
                            } else {
                                self.default_window_height_input.clear();
                            }
                            self.message =
                                "Enter the dimension, then press Enter to save both.".into();
                        } else if matches!(row, Some(27 | 28 | 30 | 31 | 32))
                            && x >= settings_layout.launch_field_x as f64
                            && !self.default_launch_settings_saving
                        {
                            if let Some(settings) = self.settings.as_ref() {
                                self.default_java_arguments_input =
                                    settings.extra_launch_args.join(" ");
                                self.default_environment_variables_input =
                                    settings
                                        .custom_env_vars
                                        .iter()
                                        .map(|(key, value)| {
                                            format!("{key}={value}")
                                        })
                                        .collect::<Vec<_>>()
                                        .join(" ");
                                self.default_pre_launch_hook_input = settings
                                    .hooks
                                    .pre_launch
                                    .clone()
                                    .unwrap_or_default();
                                self.default_wrapper_hook_input = settings
                                    .hooks
                                    .wrapper
                                    .clone()
                                    .unwrap_or_default();
                                self.default_post_exit_hook_input = settings
                                    .hooks
                                    .post_exit
                                    .clone()
                                    .unwrap_or_default();
                            }
                            self.input_focus = match row {
                                Some(27) => 30,
                                Some(28) => 31,
                                Some(30) => 32,
                                Some(31) => 33,
                                _ => 34,
                            };
                            self.message = match row {
                                Some(30) => "Pre-launch hook runs before the game. Use $INST_NAME, $INST_ID, $INST_DIR, $INST_JAVA, or $INST_JAVA_ARGS. Enter saves; Escape cancels.".into(),
                                Some(31) => "Wrapper hook wraps the Minecraft process. Use $INST_NAME, $INST_ID, $INST_DIR, $INST_JAVA, or $INST_JAVA_ARGS. Enter saves; Escape cancels.".into(),
                                Some(32) => "Post-exit hook runs after the game closes. Use $INST_NAME, $INST_ID, $INST_DIR, $INST_JAVA, or $INST_JAVA_ARGS. Enter saves; Escape cancels.".into(),
                                _ => "Press Enter to save, Escape to cancel. Use Ctrl+A to replace the field.".into(),
                            };
                        } else if row == Some(23)
                            && (settings_layout.toggle_x as f64
                                ..settings_layout.row_right as f64)
                                .contains(&x)
                        {
                            self.inspect_existing_modrinth_instances();
                        } else {
                            let change = row.and_then(|index| {
                                match index {
                                0 if (settings_layout.decrement_x as f64
                                    ..(settings_layout.decrement_x + 40) as f64)
                                    .contains(&x) => Some(
                                    SettingsChange::DownloadConcurrency(-1),
                                ),
                                0 if (settings_layout.increment_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::DownloadConcurrency(1))
                                }
                                1 if (settings_layout.decrement_x as f64
                                    ..(settings_layout.decrement_x + 40) as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::WriteConcurrency(-1))
                                }
                                1 if (settings_layout.increment_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::WriteConcurrency(1))
                                }
                                2 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::Telemetry)
                                }
                                3 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::DiscordRpc)
                                }
                                4 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::Fullscreen)
                                }
                                5 if self.settings_section == 0 => {
                                    let row_y = settings_row_y(
                                        self.settings_section,
                                        0,
                                        settings_layout.row_width,
                                        self.settings_scroll_pixels,
                                    );
                                    settings_theme_option_at(
                                        settings_layout.row_width,
                                        row_y,
                                        x,
                                        y,
                                    )
                                    .map(SettingsChange::Theme)
                                }
                                6 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::FilesTab)
                                }
                                7 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::WorldsTab)
                                }
                                8 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::ScreenshotsTab)
                                }
                                9 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::SkinSelector)
                                }
                                10 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::PlayTime)
                                }
                                11 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::CompactCards)
                                }
                                12 => Some(SettingsChange::JumpIn),
                                13 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::HideRightSidebar)
                                }
                                14 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::HideOnProcessStart)
                                }
                                15 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::RefocusOnGameClose)
                                }
                                16 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::WarnOnUnknownModpacks)
                                }
                                17 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => Some(
                                    SettingsChange::SkipNonEssentialWarnings,
                                ),
                                18 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::PersonalizedAds)
                                }
                                19 if self.maximum_memory_mb.is_some()
                                    && (settings_layout.decrement_x as f64
                                        ..(settings_layout.decrement_x + 40)
                                            as f64)
                                        .contains(&x) =>
                                {
                                    Some(SettingsChange::DefaultMemory(-1))
                                }
                                19 if self.maximum_memory_mb.is_some()
                                    && (settings_layout.increment_x as f64
                                        ..settings_layout.row_right as f64)
                                        .contains(&x) =>
                                {
                                    Some(SettingsChange::DefaultMemory(1))
                                }
                                20 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::HideInstalledModpacks)
                                }
                                21 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    self.global_screenshot_sync.map(|enabled| {
                                        SettingsChange::ScreenshotSync(!enabled)
                                    })
                                }
                                22 if (settings_layout.toggle_x as f64
                                    ..settings_layout.row_right as f64)
                                    .contains(&x) => {
                                    Some(SettingsChange::HideNametag)
                                }
                                29 if !self.native_color_theme_saving =>
                                {
                                    Some(SettingsChange::ColorTheme)
                                }
                                33 if !self.theme_sync_saving
                                    && (settings_layout.toggle_x as f64
                                        ..settings_layout.row_right as f64)
                                        .contains(&x) =>
                                {
                                    Some(SettingsChange::ThemeSync(
                                        !self.theme_sync_across_devices,
                                    ))
                                }
                                34 if !cfg!(target_os = "macos")
                                    && (settings_layout.toggle_x as f64
                                        ..settings_layout.row_right as f64)
                                        .contains(&x) =>
                                {
                                    Some(SettingsChange::NativeDecorations)
                                }
                                24 if !self.quick_instance_count_saving
                                    && (settings_layout.decrement_x as f64
                                        ..(settings_layout.decrement_x + 40)
                                            as f64)
                                        .contains(&x) =>
                                {
                                    Some(SettingsChange::QuickInstanceCount(-1))
                                }
                                24 if !self.quick_instance_count_saving
                                    && (settings_layout.increment_x as f64
                                        ..settings_layout.row_right as f64)
                                        .contains(&x) =>
                                {
                                    Some(SettingsChange::QuickInstanceCount(1))
                                }
                                _ => None,
                            }
                            });
                            if let Some(change) = change {
                                if row == Some(24) {
                                    self.settings_jump_in_focused = true;
                                }
                                self.change_setting(change);
                            }
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.global_user_profile && x >= 248.0 {
                    if (80.0..112.0).contains(&y) && (292.0..430.0).contains(&x)
                    {
                        self.return_to_library(None);
                    } else if library_width >= 980.0
                        && x >= library_width as f64 - 252.0
                        && (176.0
                            ..(176
                                + self.user_profile_organizations.len().min(5)
                                    * 46) as f64)
                            .contains(&y)
                    {
                        let index = ((y - 176.0) / 46.0) as usize;
                        if let Some(organization) =
                            self.user_profile_organizations.get(index)
                            && let Some(slug) = organization
                                .get("slug")
                                .or_else(|| organization.get("id"))
                                .and_then(serde_json::Value::as_str)
                        {
                            let url = format!(
                                "https://modrinth.com/organization/{}",
                                url::form_urlencoded::byte_serialize(
                                    slug.as_bytes()
                                )
                                .collect::<String>()
                            );
                            std::thread::spawn(move || {
                                let _ = open::that(url);
                            });
                        }
                    } else if (306.0..334.0).contains(&y) && x >= 292.0 {
                        let tab_index = ((x - 292.0) / 96.0) as usize;
                        let within_tab = (x - 292.0).rem_euclid(96.0) < 88.0;
                        let mut types = vec![None];
                        types.extend(
                            profile_project_types(&self.user_profile_projects)
                                .into_iter()
                                .map(Some),
                        );
                        if !self.user_profile_collections.is_empty() {
                            types.push(Some("collection".into()));
                        }
                        if within_tab && let Some(tab) = types.get(tab_index) {
                            self.user_profile_show_collections =
                                tab.as_deref() == Some("collection");
                            self.user_profile_type_filter =
                                if self.user_profile_show_collections {
                                    None
                                } else {
                                    tab.clone()
                                };
                            self.user_profile_filtered_project_indices =
                                filtered_profile_project_indices(
                                    &self.user_profile_projects,
                                    self.user_profile_type_filter.as_deref(),
                                );
                            self.user_profile_scroll_pixels = 0.0;
                            self.user_profile_scroll_target_pixels = 0.0;
                        }
                    } else if (352.0..sidebar_height - 40.0).contains(&y) {
                        let scrolled_y =
                            y - 352.0 + self.user_profile_scroll_pixels;
                        let index = (scrolled_y / 62.0) as usize;
                        if scrolled_y.rem_euclid(62.0) >= 54.0 {
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                            return;
                        }
                        if self.user_profile_show_collections {
                            if let Some(collection) =
                                self.user_profile_collections.get(index)
                                && let Some(id) = collection
                                    .get("id")
                                    .and_then(serde_json::Value::as_str)
                            {
                                let url = format!(
                                    "https://modrinth.com/collection/{}",
                                    url::form_urlencoded::byte_serialize(
                                        id.as_bytes(),
                                    )
                                    .collect::<String>()
                                );
                                std::thread::spawn(move || {
                                    let _ = open::that(url);
                                });
                            }
                        } else {
                            let value = self
                                .user_profile_filtered_project_indices
                                .get(index)
                                .and_then(|project_index| {
                                    self.user_profile_projects
                                        .get(*project_index)
                                });
                            if let Some(value) = value {
                                let project = BrowseProject {
                                    id: value
                                        .get("project_id")
                                        .or_else(|| value.get("id"))
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or_default()
                                        .to_owned(),
                                    title: value
                                        .get("title")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or("Untitled project")
                                        .to_owned(),
                                    description: value
                                        .get("description")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or_default()
                                        .to_owned(),
                                    icon_url: value
                                        .get("icon_url")
                                        .and_then(serde_json::Value::as_str)
                                        .map(str::to_owned),
                                    author: value
                                        .get("author")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or("Unknown author")
                                        .to_owned(),
                                    project_type: value
                                        .get("project_type")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or("mod")
                                        .to_owned(),
                                    downloads: value
                                        .get("downloads")
                                        .and_then(serde_json::Value::as_u64)
                                        .unwrap_or_default(),
                                    followers: value
                                        .get("followers")
                                        .or_else(|| value.get("follows"))
                                        .and_then(serde_json::Value::as_u64)
                                        .unwrap_or_default(),
                                    categories: value
                                        .get("categories")
                                        .and_then(serde_json::Value::as_array)
                                        .map(|categories| {
                                            categories
                                                .iter()
                                                .filter_map(
                                                    serde_json::Value::as_str,
                                                )
                                                .map(str::to_owned)
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                };
                                if !project.id.is_empty() {
                                    self.global_user_profile = false;
                                    self.global_browse = true;
                                    self.open_browse_project(project);
                                }
                            }
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.global_skins && x >= 248.0 {
                    if (80.0..106.0).contains(&y) && (292.0..414.0).contains(&x)
                    {
                        self.return_to_library(None);
                    } else if (82.0..116.0).contains(&y)
                        && x >= library_width - 152.0
                    {
                        self.pick_minecraft_skin();
                    } else if (82.0..116.0).contains(&y)
                        && (library_width - 388.0..library_width - 352.0)
                            .contains(&x)
                    {
                        self.minecraft_skin_rotation = (self
                            .minecraft_skin_rotation
                            - std::f32::consts::PI / 6.0)
                            .rem_euclid(std::f32::consts::TAU);
                    } else if (82.0..116.0).contains(&y)
                        && (library_width - 344.0..library_width - 308.0)
                            .contains(&x)
                    {
                        self.minecraft_skin_rotation = (self
                            .minecraft_skin_rotation
                            + std::f32::consts::PI / 6.0)
                            .rem_euclid(std::f32::consts::TAU);
                    } else if (82.0..116.0).contains(&y)
                        && (library_width - 304.0..library_width - 164.0)
                            .contains(&x)
                    {
                        self.minecraft_skin_variant = match self.minecraft_skin_variant {
                            theseus::minecraft_skins::MinecraftSkinVariant::Classic => theseus::minecraft_skins::MinecraftSkinVariant::Slim,
                            theseus::minecraft_skins::MinecraftSkinVariant::Slim | theseus::minecraft_skins::MinecraftSkinVariant::Unknown => theseus::minecraft_skins::MinecraftSkinVariant::Classic,
                        };
                    } else {
                        let (columns, card_width, card_height, gap) =
                            minecraft_skin_grid_layout(
                                library_width.max(0.0) as usize
                            );
                        let start_x = 292.0;
                        let start_y = 166.0;
                        if y >= start_y {
                            let row_height = (card_height + gap) as f64;
                            let content_y =
                                y - start_y + self.minecraft_skin_scroll_pixels;
                            let row = (content_y / row_height).floor() as usize;
                            let card_y = start_y as f64
                                + row as f64 * row_height
                                - self.minecraft_skin_scroll_pixels;
                            let column_stride = card_width + gap;
                            let column = ((x - start_x).max(0.0) as usize)
                                / column_stride;
                            let card_x =
                                start_x + (column * column_stride) as f64;
                            if column < columns
                                && x >= card_x
                                && x < card_x + card_width as f64
                                && (card_y + 178.0..card_y + 210.0).contains(&y)
                            {
                                let index = row * columns + column;
                                if let Some(skin) =
                                    self.minecraft_skins.get(index).cloned()
                                {
                                    let is_saved = matches!(
                                        &skin.source,
                                        theseus::minecraft_skins::SkinSource::Custom
                                    );
                                    if is_saved
                                        && x >= card_x + card_width as f64
                                            - 58.0
                                    {
                                        self.skin_delete_confirmation =
                                            Some(skin);
                                    } else if skin.is_equipped {
                                        self.message =
                                            "That skin is already equipped."
                                                .into();
                                    } else {
                                        self.equip_minecraft_skin(skin);
                                    }
                                }
                            }
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if (56.0..248.0).contains(&x) && (104.0..128.0).contains(&y) {
                    self.return_to_library(None);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if (56.0..248.0).contains(&x) && (128.0..152.0).contains(&y) {
                    self.return_to_library(Some("group:favorites".into()));
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.instance_tab == 8
                    && self.selected_instance.as_ref().is_some_and(|(id, _)| {
                        id != GLOBAL_SCREENSHOTS_ID && id != GLOBAL_DISCOVER_ID
                    })
                    && (80.0..106.0).contains(&y)
                    && (292.0..414.0).contains(&x)
                {
                    self.return_to_library(None);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.is_home_library()
                    && self.selected_group.is_none()
                    && self.library_search_query.is_empty()
                    && self.filtered_instance_indices().is_empty()
                {
                    let toolbar = self.current_library_toolbar_layout(
                        library_width.max(0.0) as usize,
                    );
                    let welcome = self.library_search_query.is_empty()
                        && self.onboarding_checklist.as_ref().is_some_and(
                            |checklist| !checklist.has_created_instance,
                        );
                    if welcome {
                        let layout = welcome_screen_layout(
                            library_width.max(0.0) as usize,
                            sidebar_height as usize,
                        );
                        let button_x = layout.center_x as f64
                            - WELCOME_SPLIT_BUTTON_WIDTH as f64 / 2.0;
                        let menu_clicked = self.welcome_options_open
                            && (button_x
                                ..button_x + WELCOME_SPLIT_BUTTON_WIDTH as f64)
                                .contains(&x)
                            && (layout.options_y as f64
                                ..layout.options_y as f64 + 36.0)
                                .contains(&y);
                        if menu_clicked {
                            self.welcome_options_open = false;
                            self.skip_create_instance_onboarding();
                        } else if (button_x
                            ..button_x + WELCOME_SPLIT_BUTTON_WIDTH as f64)
                            .contains(&x)
                            && (layout.create_y as f64
                                ..layout.create_y as f64 + 40.0)
                                .contains(&y)
                        {
                            if x < button_x
                                + WELCOME_PRIMARY_BUTTON_WIDTH as f64
                            {
                                self.welcome_options_open = false;
                                self.create_instance_open = true;
                                self.name_input.clear();
                                self.version_input.clear();
                                self.input_focus = 1;
                            } else {
                                self.welcome_options_open =
                                    !self.welcome_options_open;
                            }
                        } else if (layout.center_x as f64
                            - WELCOME_IMPORT_BUTTON_WIDTH as f64 / 2.0
                            ..layout.center_x as f64
                                + WELCOME_IMPORT_BUTTON_WIDTH as f64 / 2.0)
                            .contains(&x)
                            && (layout.import_y as f64
                                ..layout.import_y as f64 + 40.0)
                                .contains(&y)
                        {
                            self.welcome_options_open = false;
                            self.open_launcher_import();
                            return;
                        } else {
                            self.welcome_options_open = false;
                        }
                        if x < library_width {
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                            return;
                        }
                    }
                    let card_x = 292.0;
                    let card_y = (toolbar.cards_top + 112) as f64;
                    let card_width = (library_width.max(0.0) as usize)
                        .saturating_sub(324)
                        .min(620) as f64;
                    if (card_x..card_x + card_width).contains(&x)
                        && (card_y + 48.0..card_y + 150.0).contains(&y)
                        && let Some(checklist) = self
                            .onboarding_checklist
                            .as_ref()
                            .filter(|checklist| {
                                checklist.show_checklist
                                    && checklist.has_created_instance
                            })
                    {
                        let step = ((y - card_y - 48.0) / 34.0) as usize;
                        match step {
                            0 if !checklist.has_created_instance => {
                                self.create_instance_open = true;
                                self.name_input.clear();
                                self.version_input.clear();
                                self.input_focus = 1;
                            }
                            1 if !checklist.has_logged_into_minecraft => {
                                self.accounts_open = true;
                                self.account_tab = 0;
                                self.account_offset = 0;
                                self.account_scroll_pixels = 0.0;
                                self.account_scroll_target_pixels = 0.0;
                                self.input_focus = 0;
                                self.load_accounts();
                            }
                            2 if !checklist.has_logged_into_modrinth => {
                                self.accounts_open = true;
                                self.account_tab = 1;
                                self.account_offset = 0;
                                self.account_scroll_pixels = 0.0;
                                self.account_scroll_target_pixels = 0.0;
                                self.input_focus = 0;
                                self.load_accounts();
                            }
                            _ => {}
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }
                }
                if self.selected_instance.is_some() {
                    let show_instance_tabs = !self.global_browse
                        && !self.global_screenshots
                        && self.instance_tab != 8;
                    let instance_tab_target =
                        if show_instance_tabs && (112.0..140.0).contains(&y) {
                            self.native_instance_tabs(wide_instance_header)
                                .into_iter()
                                .find(|(left, width, _, _)| {
                                    (*left as f64..(*left + *width) as f64)
                                        .contains(&x)
                                })
                                .map(|(_, _, _, tab)| tab)
                        } else {
                            None
                        };
                    let instance_more_target =
                        if show_instance_tabs && self.instance_tab == 6 {
                            self.instance_more_rows(
                                wide_instance_header,
                                library_width as usize,
                            )
                            .find(|(bounds, _, _, _)| bounds.contains(x, y))
                            .map(|(_, _, _, tab)| tab)
                        } else {
                            None
                        };
                    if self.instance_tab == 9
                        && self.shared_instance_unlink_confirmation
                    {
                        let panel_width = 480.0_f64.min(width - 32.0);
                        let panel_height = 220.0_f64.min(sidebar_height - 32.0);
                        let panel_x = (width - panel_width) / 2.0;
                        let panel_y = (sidebar_height - panel_height) / 2.0;
                        if y >= panel_y + panel_height - 56.0
                            && x >= panel_x + panel_width - 204.0
                            && x < panel_x + panel_width - 112.0
                        {
                            self.shared_instance_unlink_confirmation = false;
                        } else if y >= panel_y + panel_height - 56.0
                            && x >= panel_x + panel_width - 108.0
                            && !self.shared_instance_busy
                        {
                            self.unlink_imported_modpack_for_sharing();
                        }
                    } else if self.instance_tab == 9
                        && self.shared_instance_pending_removal.is_some()
                    {
                        let panel_width = 480.0_f64.min(width - 32.0);
                        let panel_height = 230.0_f64.min(sidebar_height - 32.0);
                        let panel_x = (width - panel_width) / 2.0;
                        let panel_y = (sidebar_height - panel_height) / 2.0;
                        if y >= panel_y + panel_height - 56.0
                            && x >= panel_x + panel_width - 204.0
                            && x < panel_x + panel_width - 112.0
                            && !self.shared_instance_busy
                        {
                            self.shared_instance_pending_removal = None;
                        } else if y >= panel_y + panel_height - 56.0
                            && x >= panel_x + panel_width - 108.0
                            && !self.shared_instance_busy
                        {
                            self.remove_shared_instance_user();
                        }
                    } else if self.instance_tab == 9
                        && self.shared_instance_invite_open
                    {
                        let panel_width = 500.0_f64.min(width - 32.0);
                        let panel_height = 420.0_f64.min(sidebar_height - 32.0);
                        let panel_x = (width - panel_width) / 2.0;
                        let panel_y = (sidebar_height - panel_height) / 2.0;
                        if (panel_y + 42.0..panel_y + 76.0).contains(&y)
                            && (panel_x + 16.0..panel_x + panel_width - 150.0)
                                .contains(&x)
                        {
                            self.input_focus = 20;
                        } else if (panel_y + 42.0..panel_y + 76.0).contains(&y)
                            && x >= panel_x + panel_width - 134.0
                            && !self.shared_instance_busy
                        {
                            self.search_shared_instance_users();
                        } else if y >= panel_y + 84.0
                            && y < panel_y + 336.0
                            && x >= panel_x + 16.0
                            && x < panel_x + panel_width - 16.0
                        {
                            let row = ((y - panel_y - 84.0) / 42.0) as usize;
                            if let Some(user) =
                                self.shared_instance_user_results.get(row)
                            {
                                if !self
                                    .shared_instance_selected_user_ids
                                    .remove(&user.id)
                                {
                                    self.shared_instance_selected_user_ids
                                        .insert(user.id.clone());
                                }
                            }
                        } else if y >= panel_y + panel_height - 52.0
                            && x >= panel_x + panel_width - 192.0
                            && x < panel_x + panel_width - 100.0
                        {
                            self.shared_instance_invite_open = false;
                            self.shared_instance_selected_user_ids.clear();
                            self.shared_instance_user_query.clear();
                            self.input_focus = 0;
                        } else if y >= panel_y + panel_height - 52.0
                            && x >= panel_x + panel_width - 96.0
                            && !self.shared_instance_busy
                        {
                            self.invite_selected_shared_instance_users();
                        }
                    } else if self.instance_tab == 8 {
                        if self.screenshot_group_picker_open {
                            if self.screenshot_group_rename_id.is_some() {
                                let panel_width = 400.min(width as usize - 32);
                                let panel_x =
                                    (width as usize - panel_width) / 2;
                                let panel_y = (sidebar_height as usize)
                                    .saturating_sub(154)
                                    / 2;
                                if y >= (panel_y + 100) as f64
                                    && y < (panel_y + 130) as f64
                                    && x >= (panel_x + 16) as f64
                                    && x < (panel_x + 96) as f64
                                {
                                    self.save_screenshot_group_rename();
                                } else if y >= (panel_y + 100) as f64
                                    && y < (panel_y + 130) as f64
                                    && x >= (panel_x + 104) as f64
                                    && x < (panel_x + 184) as f64
                                {
                                    self.screenshot_group_rename_id = None;
                                    self.screenshot_group_name_input.clear();
                                    self.screenshot_group_picker_open = false;
                                    self.input_focus = 0;
                                }
                            } else if self
                                .screenshot_group_delete_confirmation
                                .is_some()
                            {
                                let panel_width = 380.min(width as usize - 32);
                                let panel_x =
                                    (width as usize - panel_width) / 2;
                                let panel_y = (sidebar_height as usize)
                                    .saturating_sub(122)
                                    / 2;
                                if y >= (panel_y + 66) as f64
                                    && y < (panel_y + 96) as f64
                                    && x >= (panel_x + 16) as f64
                                    && x < (panel_x + 96) as f64
                                {
                                    self.delete_selected_screenshot_group();
                                } else if y >= (panel_y + 66) as f64
                                    && y < (panel_y + 96) as f64
                                    && x >= (panel_x + 104) as f64
                                    && x < (panel_x + 184) as f64
                                {
                                    self.screenshot_group_delete_confirmation =
                                        None;
                                }
                            } else {
                                let panel_width = 420.min(width as usize - 32);
                                let can_manage_group = self
                                    .selected_screenshot
                                    .and_then(|index| {
                                        self.screenshots.get(index)
                                    })
                                    .and_then(|screenshot| {
                                        screenshot.group_id.as_ref()
                                    })
                                    .is_some_and(|id| {
                                        self.screenshot_groups
                                            .iter()
                                            .any(|group| &group.id == id)
                                    });
                                let group_count = self
                                    .screenshot_groups
                                    .len()
                                    .saturating_sub(
                                        self.screenshot_group_offset,
                                    )
                                    .min(8);
                                let panel_height = 58
                                    + (group_count + 1) * 34
                                    + if can_manage_group { 78 } else { 42 };
                                let panel_x = (width as usize)
                                    .saturating_sub(panel_width)
                                    / 2;
                                let panel_y = (sidebar_height as usize)
                                    .saturating_sub(panel_height)
                                    / 2;
                                let scroll_remainder = self
                                    .screenshot_group_scroll_pixels
                                    .rem_euclid(34.0);
                                let clicked_row =
                                    (0..=group_count).find(|row| {
                                        let row_y = (panel_y + 42 + row * 34)
                                            as f64
                                            - scroll_remainder;
                                        (row_y..row_y + 28.0).contains(&y)
                                    });
                                if x >= (panel_x + 12) as f64
                                    && x < (panel_x + panel_width - 12) as f64
                                    && let Some(row) = clicked_row
                                {
                                    let group_id = if row < group_count {
                                        self.screenshot_groups
                                            .get(
                                                self.screenshot_group_offset
                                                    + row,
                                            )
                                            .map(|group| group.id.clone())
                                    } else {
                                        None
                                    };
                                    self.screenshot_group_picker_open = false;
                                    self.assign_selected_screenshot_group(
                                        group_id,
                                    );
                                } else {
                                    let cancel_y =
                                        panel_y + 46 + (group_count + 1) * 34;
                                    if can_manage_group
                                        && y >= (cancel_y + 34) as f64
                                        && y < (cancel_y + 62) as f64
                                        && x >= (panel_x + 12) as f64
                                        && x < (panel_x + 276) as f64
                                    {
                                        if x < (panel_x + 140) as f64 {
                                            self.begin_rename_selected_screenshot_group();
                                        } else {
                                            self.screenshot_group_delete_confirmation =
                                                self.selected_screenshot
                                                    .and_then(|index| self.screenshots.get(index))
                                                    .and_then(|screenshot| screenshot.group_id.clone());
                                        }
                                    } else if y >= cancel_y as f64
                                        && x >= (panel_x + panel_width - 92)
                                            as f64
                                    {
                                        self.screenshot_group_picker_open =
                                            false;
                                    }
                                }
                            }
                        } else if self.screenshot_viewer {
                            if self.screenshot_delete_confirmation {
                                if (library_width - 202.0
                                    ..library_width - 130.0)
                                    .contains(&x)
                                    && y >= sidebar_height - 110.0
                                {
                                    self.screenshot_delete_confirmation = false;
                                    self.delete_selected_screenshot();
                                } else if (library_width - 122.0
                                    ..library_width - 50.0)
                                    .contains(&x)
                                    && y >= sidebar_height - 110.0
                                {
                                    self.screenshot_delete_confirmation = false;
                                }
                            } else if (146.0..174.0).contains(&y)
                                && (292.0..404.0).contains(&x)
                            {
                                self.screenshot_viewer = false;
                                self.screenshot_preview = None;
                            } else if (146.0..174.0).contains(&y)
                                && x >= library_width - 356.0
                                && x < library_width - 252.0
                            {
                                if let Some(index) = self.selected_screenshot {
                                    if let Some(screenshot) =
                                        self.screenshots.get(index).cloned()
                                    {
                                        std::thread::spawn(move || {
                                            let _ = open::that(screenshot.path);
                                        });
                                    }
                                }
                            } else if (146.0..174.0).contains(&y)
                                && x >= library_width - 246.0
                                && x < library_width - 150.0
                            {
                                if let Some(index) = self.selected_screenshot {
                                    if let Some(screenshot) =
                                        self.screenshots.get(index).cloned()
                                    {
                                        self.export_screenshot(screenshot);
                                    }
                                }
                            } else if (146.0..174.0).contains(&y)
                                && x >= library_width - 144.0
                            {
                                self.screenshot_delete_confirmation = true;
                            }
                        } else if (146.0..174.0).contains(&y)
                            && x >= 292.0
                            && x < library_width - 270.0
                        {
                            self.input_focus = 19;
                        } else if (146.0..174.0).contains(&y)
                            && self.screenshot_group_mode % 4 == 2
                            && x >= library_width - 252.0
                            && x < library_width - 142.0
                        {
                            self.input_focus = 0;
                            self.create_screenshot_group();
                        } else if (146.0..174.0).contains(&y)
                            && x >= library_width - 142.0
                        {
                            self.input_focus = 0;
                            self.screenshots.clear();
                            self.screenshot_offset = 0;
                            self.message = "Loading screenshots…".into();
                            let target = self
                                .selected_instance
                                .as_ref()
                                .filter(|(id, _)| id != GLOBAL_SCREENSHOTS_ID)
                                .map(|(id, _)| id.clone());
                            self.load_screenshots(target);
                        } else if (180.0..208.0).contains(&y)
                            && (292.0..432.0).contains(&x)
                        {
                            self.input_focus = 0;
                            self.screenshot_sort_mode =
                                (self.screenshot_sort_mode + 1) % 4;
                            self.screenshot_offset = 0;
                        } else if (180.0..208.0).contains(&y)
                            && (440.0..580.0).contains(&x)
                        {
                            self.input_focus = 0;
                            self.screenshot_group_mode =
                                (self.screenshot_group_mode + 1) % 4;
                            self.screenshot_offset = 0;
                        } else if (180.0..208.0).contains(&y)
                            && self.screenshot_group_mode % 4 == 2
                            && (588.0..640.0).contains(&x)
                        {
                            self.input_focus = 0;
                            if self.selected_screenshot.is_some() {
                                self.screenshot_group_picker_open = true;
                            } else {
                                self.message =
                                    "Select a screenshot first.".into();
                            }
                        } else if (180.0..208.0).contains(&y)
                            && self.selected_screenshot_ids.len() > 1
                            && x >= library_width - 116.0
                            && x < library_width - 64.0
                        {
                            self.input_focus = 0;
                            self.export_selected_screenshots();
                        } else if (180.0..208.0).contains(&y)
                            && self.selected_screenshot_ids.len() > 1
                            && x >= library_width - 60.0
                        {
                            self.input_focus = 0;
                            self.screenshot_viewer = false;
                            self.screenshot_delete_confirmation = true;
                        } else if self.screenshot_delete_confirmation {
                            if (library_width - 202.0..library_width - 130.0)
                                .contains(&x)
                                && y >= sidebar_height - 110.0
                            {
                                self.screenshot_delete_confirmation = false;
                                self.delete_selected_screenshot();
                            } else if (library_width - 122.0
                                ..library_width - 50.0)
                                .contains(&x)
                                && y >= sidebar_height - 110.0
                            {
                                self.screenshot_delete_confirmation = false;
                            }
                        } else if y >= 230.0
                            && x >= 292.0
                            && x < library_width - 32.0
                        {
                            self.input_focus = 0;
                            let (columns, card_width, _, card_height) =
                                screenshot_grid_layout(
                                    library_width.max(0.0) as usize
                                );
                            let horizontal_step = card_width + 12;
                            let vertical_step = card_height + 12;
                            let local_x = (x as usize).saturating_sub(292);
                            let local_y = screenshot_local_y(
                                y as usize,
                                self.screenshot_scroll_pixels as usize,
                                vertical_step,
                            );
                            let column = local_x / horizontal_step;
                            let row = local_y / vertical_step;
                            let in_card = column < columns
                                && local_x % horizontal_step < card_width
                                && local_y % vertical_step < card_height;
                            let card_x = 292 + column * horizontal_step;
                            let screenshot_index = in_card
                                .then_some(
                                    (self.screenshot_scroll_pixels
                                        / vertical_step as f64)
                                        .floor()
                                        as usize
                                        * columns
                                        + row * columns
                                        + column,
                                )
                                .and_then(|position| {
                                    self.screenshot_order()
                                        .get(position)
                                        .copied()
                                });
                            if let Some((index, screenshot)) = screenshot_index
                                .and_then(|index| {
                                    self.screenshots
                                        .get(index)
                                        .cloned()
                                        .map(|screenshot| (index, screenshot))
                                })
                            {
                                let card_local_x =
                                    (x as usize).saturating_sub(card_x);
                                let card_local_y = local_y % vertical_step;
                                if screenshot_copy_bounds(card_width).contains(
                                    card_local_x as f64,
                                    card_local_y as f64,
                                ) {
                                    self.copy_screenshot(screenshot);
                                    self.draw();
                                    return;
                                }
                                let is_action =
                                    card_local_y >= card_height - 30;
                                if is_action {
                                    if !self
                                        .selected_screenshot_ids
                                        .contains(&screenshot.id)
                                    {
                                        self.select_screenshot(index, false);
                                    } else {
                                        self.selected_screenshot = Some(index);
                                    }
                                } else {
                                    self.select_screenshot(
                                        index,
                                        self.modifiers.control_key(),
                                    );
                                }
                                if card_local_y >= card_height - 30 {
                                    if (8..56).contains(&card_local_x) {
                                        self.show_screenshot_preview(
                                            screenshot,
                                        );
                                    } else if (60..112).contains(&card_local_x)
                                    {
                                        self.export_screenshot(screenshot);
                                    } else if (116..172).contains(&card_local_x)
                                    {
                                        self.screenshot_delete_confirmation =
                                            true;
                                    }
                                }
                            }
                        }
                    } else if (self.global_browse || self.global_screenshots)
                        && (112.0..140.0).contains(&y)
                    {
                        self.input_focus = 0;
                    } else if let Some(tab) = instance_tab_target {
                        self.activate_instance_tab(tab);
                    } else if let Some(tab) = instance_more_target {
                        self.activate_instance_tab(tab);
                    } else if self.instance_tab == 7
                        && (146.0..174.0).contains(&y)
                        && (width - 260.0..width - 148.0).contains(&x)
                    {
                        self.add_server_open = true;
                        self.add_server_busy = false;
                        self.server_edit_index = None;
                        self.server_edit_source = None;
                        self.server_name_input.clear();
                        self.server_address_input.clear();
                        self.server_pack_status =
                            worlds::ServerPackStatus::Enabled;
                        self.input_focus = 23;
                    } else if self.instance_tab == 7
                        && (146.0..174.0).contains(&y)
                        && x >= width - 142.0
                    {
                        self.worlds.clear();
                        self.world_offset = 0;
                        self.world_scroll_pixels = 0.0;
                        self.world_scroll_target_pixels = 0.0;
                        self.world_scroll_synced = 0;
                        self.message = "Loading worlds and servers…".into();
                        if let Some((id, _)) = self.selected_instance.as_ref() {
                            self.load_instance_worlds(id.clone());
                        }
                    } else if self.instance_tab == 9
                        && (146.0..174.0).contains(&y)
                    {
                        self.input_focus = 0;
                        if (292.0..388.0).contains(&x)
                            && self.shared_instance_can_use == Some(true)
                            && self
                                .shared_instance_metadata
                                .as_ref()
                                .and_then(|metadata| metadata.shared_instance.as_ref())
                                .is_some_and(|attachment| {
                                    attachment.role == theseus::data::SharedInstanceRole::Owner
                                })
                        {
                            self.shared_instance_invite_open = true;
                            self.shared_instance_user_query.clear();
                            self.shared_instance_user_results.clear();
                            self.shared_instance_selected_user_ids.clear();
                            self.input_focus = 20;
                        } else if x >= width - 142.0 {
                            if let Some((id, _)) = self.selected_instance.as_ref() {
                                self.load_shared_instance(id.clone());
                            }
                        } else if x >= width - 252.0
                            && x < width - 150.0
                            && self.shared_instance_can_use == Some(true)
                            && self
                                .shared_instance_metadata
                                .as_ref()
                                .and_then(|metadata| metadata.shared_instance.as_ref())
                                .is_some_and(|attachment| {
                                    attachment.role == theseus::data::SharedInstanceRole::Owner
                                })
                            && !self.shared_instance_busy
                        {
                            self.create_shared_instance_invite();
                        } else if x >= width - 362.0
                            && x < width - 252.0
                            && self.shared_instance_can_use == Some(true)
                            && self
                                .shared_instance_metadata
                                .as_ref()
                                .and_then(|metadata| metadata.shared_instance.as_ref())
                                .is_none_or(|attachment| {
                                    attachment.role == theseus::data::SharedInstanceRole::Owner
                                })
                            && !self.shared_instance_busy
                        {
                            let needs_unlink = self
                                .shared_instance_metadata
                                .as_ref()
                                .is_some_and(|metadata| {
                                    matches!(
                                        &metadata.link,
                                        theseus::data::InstanceLink::ImportedModpack { .. }
                                    )
                                });
                            if needs_unlink {
                                self.shared_instance_unlink_confirmation = true;
                            } else {
                                self.preview_shared_instance_publish();
                            }
                        }
                    } else if self.instance_tab == 9 && y >= 244.0 {
                        if let Some(preview) =
                            self.shared_instance_preview.as_ref()
                        {
                            let content_height =
                                sidebar_height.max(0.0) as usize;
                            let visible = (content_height.saturating_sub(452)
                                / 28)
                                .max(1);
                            let list_bottom = (332 + visible * 28)
                                .min(content_height.saturating_sub(100));
                            if (332.0..list_bottom as f64).contains(&y) {
                                let scroll = self
                                    .shared_instance_preview_scroll_pixels
                                    .round()
                                    .max(0.0)
                                    as usize;
                                let position = y - 332.0 + scroll as f64;
                                let within_row = position.rem_euclid(28.0);
                                let row = (position / 28.0).floor() as usize;
                                if within_row < 24.0
                                    && let Some(file) = preview.files.get(row)
                                    && !file.blocked
                                {
                                    if !self
                                        .shared_instance_selected_files
                                        .remove(&file.path)
                                    {
                                        self.shared_instance_selected_files
                                            .insert(file.path.clone());
                                    }
                                }
                            }
                            if y >= sidebar_height - 80.0
                                && x >= width - 204.0
                                && !self.shared_instance_busy
                            {
                                self.publish_shared_instance_selection();
                            }
                        } else {
                            let attachment = self
                                .shared_instance_metadata
                                .as_ref()
                                .and_then(|metadata| {
                                    metadata.shared_instance.as_ref()
                                });
                            let visible_members = self
                                .shared_instance_users
                                .iter()
                                .filter(|member| {
                                    !attachment.is_some_and(|attachment| {
                                        attachment.linked_user_id.as_deref()
                                            == Some(member.id.as_str())
                                            || attachment.manager_id.as_deref()
                                                == Some(member.id.as_str())
                                    })
                                })
                                .take(8)
                                .collect::<Vec<_>>();
                            let invite_top = 252 + visible_members.len() * 40;
                            if (244.0
                                ..(244 + visible_members.len() * 40) as f64)
                                .contains(&y)
                            {
                                let row = (y as usize - 244) / 40;
                                if x >= width - 100.0
                                    && !self.shared_instance_busy
                                {
                                    if let Some(member) =
                                        visible_members.get(row)
                                    {
                                        if member.join_type != theseus::instance::SharedInstanceJoinType::Owner {
                                            self.shared_instance_pending_removal =
                                                Some(member.id.clone());
                                        }
                                    }
                                }
                            } else if y >= (invite_top + 20) as f64
                                && !self.shared_instance_busy
                                && x >= width - 100.0
                            {
                                let relative_y = (y as usize)
                                    .saturating_sub(invite_top + 20);
                                let invite_index = relative_y / 40;
                                if let Some(invite) = self
                                    .shared_instance_invites
                                    .get(invite_index)
                                {
                                    self.revoke_shared_instance_invite(
                                        invite.id.clone(),
                                    );
                                }
                            }
                        }
                    } else if self.instance_tab == 7 && y >= 190.0 {
                        let row =
                            (y as usize).saturating_sub(190).saturating_add(
                                (self.world_scroll_pixels as usize) % 54,
                            ) / 54;
                        if let Some(world) =
                            self.worlds.get(self.world_offset + row).cloned()
                        {
                            let row_y = 190 + row * 54
                                - ((self.world_scroll_pixels as usize) % 54);
                            if (row_y + 9..row_y + 39).contains(&(y as usize))
                                && x >= width - 330.0
                                && x < width - 272.0
                            {
                                match &world.details {
                                    worlds::WorldDetails::Singleplayer {
                                        path,
                                        locked: false,
                                        ..
                                    } => {
                                        self.world_edit_path = path.clone();
                                        self.world_edit_name = world.name.clone();
                                        self.world_edit_reset_icon = false;
                                        self.world_editor_open = true;
                                        self.input_focus = 24;
                                    }
                                    worlds::WorldDetails::Server {
                                        index,
                                        source,
                                        address,
                                        pack_status,
                                        ..
                                    } if *source
                                        == theseus::instance::ServerSource::LocalDesynced =>
                                    {
                                        self.server_edit_index = Some(*index);
                                        self.server_edit_source = Some(*source);
                                        self.server_name_input = world.name.clone();
                                        self.server_address_input = address.clone();
                                        self.server_pack_status = *pack_status;
                                        self.add_server_open = true;
                                        self.input_focus = 22;
                                    }
                                    _ => {}
                                }
                            } else if (row_y + 9..row_y + 39)
                                .contains(&(y as usize))
                                && x >= width - 260.0
                                && x < width - 194.0
                            {
                                let status = if matches!(
                                    &world.display_status,
                                    worlds::DisplayStatus::Hidden
                                ) {
                                    worlds::DisplayStatus::Normal
                                } else {
                                    worlds::DisplayStatus::Hidden
                                };
                                self.update_world_status(&world, status);
                            } else if (row_y + 9..row_y + 39)
                                .contains(&(y as usize))
                                && x >= width - 188.0
                                && x < width - 106.0
                            {
                                let status = if matches!(
                                    &world.display_status,
                                    worlds::DisplayStatus::Favorite
                                ) {
                                    worlds::DisplayStatus::Normal
                                } else {
                                    worlds::DisplayStatus::Favorite
                                };
                                self.update_world_status(&world, status);
                            } else if (row_y + 9..row_y + 39)
                                .contains(&(y as usize))
                                && x >= width - 100.0
                                && x < width - 24.0
                            {
                                let Some((instance_id, instance_name)) =
                                    self.selected_instance.clone()
                                else {
                                    return;
                                };
                                match &world.details {
                                    worlds::WorldDetails::Singleplayer {
                                        locked,
                                        ..
                                    } if *locked => {
                                        self.message =
                                            "That world is already open."
                                                .into();
                                    }
                                    worlds::WorldDetails::Singleplayer {
                                        path,
                                        ..
                                    } => self.launch_instance(
                                        instance_id,
                                        instance_name,
                                        None,
                                        Some(path.clone()),
                                    ),
                                    worlds::WorldDetails::Server {
                                        address,
                                        ..
                                    } => self.launch_instance(
                                        instance_id,
                                        instance_name,
                                        Some(address.clone()),
                                        None,
                                    ),
                                }
                            }
                        }
                    } else if self.rename_instance_open
                        && !self.global_browse
                        && !self.global_screenshots
                        && ((wide_instance_header
                            && (78.0..106.0).contains(&y)
                            && (360.0..500.0).contains(&x))
                            || (!wide_instance_header
                                && (112.0..140.0).contains(&y)
                                && (292.0..414.0).contains(&x)))
                    {
                        self.input_focus = 16;
                    } else if self.rename_instance_open
                        && !self.global_browse
                        && !self.global_screenshots
                        && ((wide_instance_header
                            && (78.0..106.0).contains(&y)
                            && (506.0..550.0).contains(&x))
                            || (!wide_instance_header
                                && (112.0..140.0).contains(&y)
                                && (418.0..456.0).contains(&x)))
                    {
                        self.save_instance_name();
                    } else if self.rename_instance_open
                        && !self.global_browse
                        && !self.global_screenshots
                        && ((wide_instance_header
                            && (78.0..106.0).contains(&y)
                            && (554.0..606.0).contains(&x))
                            || (!wide_instance_header
                                && (112.0..140.0).contains(&y)
                                && (460.0..500.0).contains(&x)))
                    {
                        self.rename_instance_open = false;
                        self.rename_instance_input.clear();
                        self.input_focus = 0;
                    } else if !self.rename_instance_open
                        && !self.global_browse
                        && !self.global_screenshots
                        && ((wide_instance_header
                            && (78.0..106.0).contains(&y)
                            && (506.0..562.0).contains(&x))
                            || (!wide_instance_header
                                && (112.0..140.0).contains(&y)
                                && (414.0..498.0).contains(&x)))
                    {
                        if let Some((_, name)) = self.selected_instance.as_ref()
                        {
                            self.rename_instance_input = name.clone();
                            self.rename_instance_open = true;
                            self.input_focus = 16;
                        }
                    } else if self.instance_tab == 4
                        && (146.0..172.0).contains(&y)
                        && (560.0..668.0).contains(&x)
                    {
                        self.instance_setup_page = 0;
                        self.input_focus = 0;
                    } else if self.instance_tab == 5
                        && ((self.selected_log.is_some()
                            && (146.0..174.0).contains(&y)
                            && (292.0..404.0).contains(&x))
                            || (self.selected_log.is_some()
                                && instance_log_search_bounds(
                                    width as usize,
                                    self.log_analysis.is_some(),
                                )
                                .contains(x, y))
                            || (self.selected_log.is_some()
                                && instance_log_filter_bounds(
                                    width as usize,
                                    instance_log_search_bounds(
                                        width as usize,
                                        self.log_analysis.is_some(),
                                    )
                                    .y + 38,
                                    native_log_present_levels(&self.log_text),
                                )
                                .iter()
                                .any(|(_, bounds)| bounds.contains(x, y)))
                            || (self.selected_log.is_some()
                                && instance_log_action_bounds(
                                    width as usize,
                                    self.log_analysis.is_some(),
                                )
                                .is_some_and(
                                    |[copy, share, delete, analyse]| {
                                        copy.contains(x, y)
                                            || share.contains(x, y)
                                            || delete.contains(x, y)
                                            || analyse.contains(x, y)
                                    },
                                ))
                            || (self.selected_log.is_none()
                                && (146.0..174.0).contains(&y)
                                && x >= width - 142.0)
                            || (self.selected_log.is_none()
                                && y >= 190.0
                                && x >= 292.0))
                    {
                        if self.selected_log.is_some()
                            && (146.0..174.0).contains(&y)
                            && (292.0..404.0).contains(&x)
                        {
                            self.selected_log = None;
                            self.log_text.clear();
                            self.log_scroll = 0;
                            self.log_scroll_synced = 0;
                            self.log_scroll_pixels = 0.0;
                            self.log_scroll_target_pixels = 0.0;
                            self.log_analysis = None;
                            self.log_analysis_pending = false;
                            self.log_share_pending = false;
                            self.log_delete_confirmation = false;
                            self.log_delete_pending = false;
                            self.input_focus = 0;
                        } else if self.selected_log.is_some()
                            && instance_log_search_bounds(
                                width as usize,
                                self.log_analysis.is_some(),
                            )
                            .contains(x, y)
                        {
                            self.input_focus = 40;
                            self.log_search_select_all = true;
                        } else if self.selected_log.is_some()
                            && instance_log_filter_bounds(
                                width as usize,
                                instance_log_search_bounds(
                                    width as usize,
                                    self.log_analysis.is_some(),
                                )
                                .y + 38,
                                native_log_present_levels(&self.log_text),
                            )
                            .iter()
                            .any(|(_, bounds)| bounds.contains(x, y))
                        {
                            let filters = instance_log_filter_bounds(
                                width as usize,
                                instance_log_search_bounds(
                                    width as usize,
                                    self.log_analysis.is_some(),
                                )
                                .y + 38,
                                native_log_present_levels(&self.log_text),
                            );
                            if let Some((level, _)) = filters
                                .iter()
                                .find(|(_, bounds)| bounds.contains(x, y))
                            {
                                self.toggle_log_level_filter(*level);
                            }
                        } else if self.selected_log.is_some()
                            && instance_log_action_bounds(
                                width as usize,
                                self.log_analysis.is_some(),
                            )
                            .is_some_and(
                                |[_, _, _, bounds]| bounds.contains(x, y),
                            )
                        {
                            self.analyse_selected_log();
                        } else if self.selected_log.is_some()
                            && instance_log_action_bounds(
                                width as usize,
                                self.log_analysis.is_some(),
                            )
                            .is_some_and(
                                |[_, bounds, _, _]| bounds.contains(x, y),
                            )
                        {
                            self.share_selected_log();
                        } else if self.selected_log.is_some()
                            && instance_log_action_bounds(
                                width as usize,
                                self.log_analysis.is_some(),
                            )
                            .is_some_and(
                                |[_, _, bounds, _]| bounds.contains(x, y),
                            )
                        {
                            if self.log_delete_pending {
                                self.message =
                                    "A log deletion is already running.".into();
                            } else if self.selected_log_delete_disabled() {
                                self.message = "Cannot delete latest.log while Minecraft is running.".into();
                            } else {
                                self.log_delete_confirmation = true;
                            }
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        } else if self.selected_log.is_some()
                            && instance_log_action_bounds(
                                width as usize,
                                self.log_analysis.is_some(),
                            )
                            .is_some_and(
                                |[bounds, _, _, _]| bounds.contains(x, y),
                            )
                        {
                            self.message = "Copying log to clipboard…".into();
                            self.copy_text_to_clipboard(self.log_text.clone());
                        } else if self.selected_log.is_none()
                            && (146.0..174.0).contains(&y)
                            && x >= width - 142.0
                        {
                            if let Some((id, _)) =
                                self.selected_instance.as_ref()
                            {
                                self.message =
                                    "Refreshing logs and crash reports…".into();
                                self.refresh_instance_logs(id.clone());
                            }
                        } else if self.selected_log.is_none()
                            && y >= 190.0
                            && x >= 292.0
                        {
                            let visible = (sidebar_height as usize)
                                .saturating_sub(208)
                                / 42;
                            let visible = visible.min(14);
                            let scroll_pixels = self.log_scroll_pixels;
                            let row_offset =
                                (scroll_pixels / 42.0).floor() as usize;
                            let scroll_remainder =
                                scroll_pixels.rem_euclid(42.0);
                            let content_y = y - 190.0 + scroll_remainder;
                            let local_row = (content_y / 42.0).floor() as usize;
                            let row_top = 190.0 + local_row as f64 * 42.0
                                - scroll_remainder;
                            let row = (content_y >= 0.0
                                && local_row < visible
                                && y >= row_top
                                && y < row_top + 36.0)
                                .then_some(local_row);
                            if let Some(index) = row {
                                let (filename, is_crash, _) = self
                                    .instance_logs[row_offset + index]
                                    .clone();
                                self.selected_log =
                                    Some((filename.clone(), is_crash));
                                self.log_scroll = 0;
                                self.log_scroll_synced = 0;
                                self.log_scroll_pixels = 0.0;
                                self.log_scroll_target_pixels = 0.0;
                                self.log_text.clear();
                                self.log_analysis = None;
                                self.log_analysis_pending = false;
                                self.log_share_pending = false;
                                self.log_delete_confirmation = false;
                                self.log_delete_pending = false;
                                self.message = format!("Loading {}…", filename);
                                self.read_instance_log(filename, is_crash);
                            }
                        }
                    } else if self.instance_tab == 4
                        && (146.0..172.0).contains(&y)
                        && (674.0..782.0).contains(&x)
                    {
                        self.instance_setup_page = 1;
                        self.input_focus = 0;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 1
                        && (196.0..230.0).contains(&y)
                        && (292.0..width - 32.0).contains(&x)
                    {
                        self.input_focus = 12;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 1
                        && (249.0..283.0).contains(&y)
                        && (292.0..width - 32.0).contains(&x)
                    {
                        self.input_focus = 13;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 1
                        && (305.0..339.0).contains(&y)
                        && (292.0..width - 32.0).contains(&x)
                    {
                        self.input_focus = 14;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 1
                        && (361.0..395.0).contains(&y)
                        && (292.0..width - 32.0).contains(&x)
                    {
                        self.input_focus = 15;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 1
                        && (412.0..446.0).contains(&y)
                        && (292.0..458.0).contains(&x)
                    {
                        self.save_instance_launch_settings();
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 0
                        && (164.0..198.0).contains(&y)
                        && x >= width - 154.0
                    {
                        self.pick_instance_java_path();
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 0
                        && (164.0..198.0).contains(&y)
                        && (292.0..width - 154.0).contains(&x)
                    {
                        self.input_focus = 7;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 0
                        && (224.0..258.0).contains(&y)
                        && (292.0..732.0).contains(&x)
                    {
                        self.input_focus = 8;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 0
                        && (284.0..318.0).contains(&y)
                        && (292.0..472.0).contains(&x)
                    {
                        self.input_focus = 9;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 0
                        && (344.0..378.0).contains(&y)
                        && (292.0..492.0).contains(&x)
                    {
                        self.instance_fullscreen_mode =
                            (self.instance_fullscreen_mode + 1) % 3;
                        self.input_focus = 0;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 0
                        && (401.0..435.0).contains(&y)
                        && (292.0..412.0).contains(&x)
                    {
                        self.input_focus = 10;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 0
                        && (401.0..435.0).contains(&y)
                        && (446.0..566.0).contains(&x)
                    {
                        self.input_focus = 11;
                    } else if self.instance_tab == 4
                        && self.instance_setup_page == 0
                        && (455.0..489.0).contains(&y)
                        && (292.0..458.0).contains(&x)
                    {
                        self.save_instance_launch_settings();
                    } else if self.instance_tab == 1
                        && instance_log_filter_bounds(
                            width as usize,
                            instance_console_search_bounds(width as usize).y
                                + 38,
                            native_log_present_levels(&self.console_text),
                        )
                        .iter()
                        .any(|(_, bounds)| bounds.contains(x, y))
                    {
                        let filters = instance_log_filter_bounds(
                            width as usize,
                            instance_console_search_bounds(width as usize).y
                                + 38,
                            native_log_present_levels(&self.console_text),
                        );
                        if let Some((level, _)) = filters
                            .iter()
                            .find(|(_, bounds)| bounds.contains(x, y))
                        {
                            self.toggle_log_level_filter(*level);
                        }
                    } else if self.instance_tab == 1
                        && !self.console_text.is_empty()
                        && instance_console_share_bounds(width as usize)
                            .contains(x, y)
                    {
                        self.share_live_console();
                    } else if self.instance_tab == 1
                        && !self.console_text.is_empty()
                        && instance_console_clear_bounds(width as usize)
                            .contains(x, y)
                    {
                        self.clear_live_console();
                    } else if self.instance_tab == 1
                        && instance_console_search_bounds(width as usize)
                            .contains(x, y)
                    {
                        self.input_focus = 40;
                        self.log_search_select_all = true;
                    } else if self.instance_tab == 1
                        && (154.0..182.0).contains(&y)
                        && x >= width - 142.0
                    {
                        self.stop_selected_process();
                    } else if self.instance_tab != 8
                        && !self.global_browse
                        && self.selected_instance.as_ref().is_some_and(
                            |(instance_id, _)| {
                                instance_id != GLOBAL_DISCOVER_ID
                            },
                        )
                        && instance_primary_bounds(library_width as usize)
                            .contains(x, y)
                    {
                        if let Some((instance_id, name)) =
                            self.selected_instance.clone()
                        {
                            let card = self.instance_cards.get(&instance_id);
                            let action = native_library_card_action(
                                self.running_processes
                                    .contains_key(&instance_id),
                                card.is_some_and(|card| {
                                    card.install_state == "installing"
                                }),
                                card.is_none_or(|card| {
                                    card.install_state == "installed"
                                }),
                                card.is_some_and(|card| card.content_locked),
                                card.is_some_and(|card| card.launch_blocked),
                                false,
                                true,
                            );
                            match action {
                                NativeLibraryCardAction::Stop => {
                                    self.stop_card_process(instance_id);
                                }
                                NativeLibraryCardAction::Repair => {
                                    self.repair_instance(instance_id);
                                }
                                NativeLibraryCardAction::Play => {
                                    self.launch_instance(
                                        instance_id,
                                        name,
                                        None,
                                        None,
                                    );
                                }
                                NativeLibraryCardAction::Installing
                                | NativeLibraryCardAction::None => {}
                            }
                        }
                    } else if show_instance_tabs
                        && instance_back_bounds(wide_instance_header)
                            .contains(x, y)
                    {
                        if let Ok(mut active) = self.active_log_instance.lock()
                        {
                            *active = None;
                        }
                        self.selected_instance = None;
                        self.rename_instance_open = false;
                        self.rename_instance_input.clear();
                        self.input_focus = 0;
                        self.process_state = None;
                        self.contents.clear();
                        self.filtered_content_cache.clear();
                        self.file_items.clear();
                        self.file_preview = None;
                    } else if self.instance_tab == 0
                        && (146.0..174.0).contains(&y)
                        && x >= width - 392.0
                        && x < width - 280.0
                    {
                        self.delete_confirmation_open = true;
                        self.bulk_delete_confirmation_open = false;
                        self.delete_confirmation_input.clear();
                        self.input_focus = 17;
                    } else if self.instance_tab == 0
                        && (146.0..174.0).contains(&y)
                        && x >= width - 270.0
                        && x < width - 158.0
                    {
                        self.duplicate_selected_instance();
                    } else if self.instance_tab == 0
                        && (146.0..174.0).contains(&y)
                        && x >= width - 142.0
                    {
                        self.export_selected_instance();
                    } else if self.instance_tab == 0
                        && (82.0..114.0).contains(&y)
                        && x >= width - 270.0
                        && x < width - 158.0
                    {
                        self.instance_tab = 3;
                        self.search_projects();
                    } else if self.instance_tab == 2 {
                        if self.file_preview.is_some()
                            && (146.0..174.0).contains(&y)
                            && (292.0..404.0).contains(&x)
                        {
                            self.file_preview = None;
                            self.file_preview_scroll_pixels = 0.0;
                            self.file_preview_scroll_target_pixels = 0.0;
                            self.file_offset = 0;
                        } else if self.file_preview.is_none()
                            && !self.file_path.is_empty()
                            && (146.0..174.0).contains(&y)
                            && x >= width - 104.0
                        {
                            self.file_path = self
                                .file_path
                                .rsplit_once('/')
                                .map(|(parent, _)| parent.to_string())
                                .unwrap_or_default();
                            self.file_offset = 0;
                            if let Some((id, _)) =
                                self.selected_instance.as_ref()
                            {
                                self.load_instance_files(
                                    id.clone(),
                                    self.file_path.clone(),
                                );
                            }
                        } else if self.file_preview.is_none() {
                            let max_rows = (sidebar_height as usize)
                                .saturating_sub(208)
                                / 42;
                            let local_y = (y as usize)
                                .saturating_sub(202)
                                .saturating_add(
                                    (self.file_list_scroll_pixels as usize)
                                        % 42,
                                );
                            let row_index = local_y / 42;
                            let row = (local_y % 42 < 36
                                && row_index
                                    < self
                                        .file_items
                                        .len()
                                        .saturating_sub(self.file_offset)
                                        .min(max_rows + 1))
                            .then_some(row_index);
                            if let Some(index) = row {
                                let (_name, kind, path, size, _) = self
                                    .file_items[self.file_offset + index]
                                    .clone();
                                if kind == "directory" {
                                    self.file_path = path;
                                    self.file_offset = 0;
                                    if let Some((id, _)) =
                                        self.selected_instance.as_ref()
                                    {
                                        self.load_instance_files(
                                            id.clone(),
                                            self.file_path.clone(),
                                        );
                                    }
                                } else if kind == "symlink" {
                                    self.message = "Symbolic links cannot be opened in Files.".into();
                                } else {
                                    self.read_instance_file(path, size);
                                }
                            }
                        }
                    } else if self.instance_tab == 3 {
                        let results_layout = BrowseResultsLayout::new(
                            library_width as usize,
                            sidebar_height as usize,
                        );
                        let [previous_page, next_page] =
                            results_layout.pagination();
                        let search_layout =
                            browse_search_layout(library_width as usize);
                        let (view_x, view_width) =
                            browse_view_layout(library_width as usize);
                        if self.browse_sort_menu_open {
                            if (search_layout.sort_x as f64
                                ..(search_layout.sort_x
                                    + search_layout.sort_width)
                                    as f64)
                                .contains(&x)
                                && (254.0..384.0).contains(&y)
                            {
                                let index =
                                    ((y - 254.0) / 26.0).floor().max(0.0) as u8;
                                if index < 5 {
                                    self.browse_sort = index;
                                    self.browse_sort_menu_open = false;
                                    self.browse_selected = None;
                                    self.browse_details = None;
                                    self.search_projects();
                                    self.draw();
                                    return;
                                }
                            }
                            self.browse_sort_menu_open = false;
                        }
                        if self.browse_view_menu_open {
                            if (view_x as f64..(view_x + view_width) as f64)
                                .contains(&x)
                                && (254.0..410.0).contains(&y)
                            {
                                let index =
                                    ((y - 254.0) / 26.0).floor().max(0.0)
                                        as usize;
                                if let Some(size) = BROWSE_PAGE_SIZES.get(index)
                                {
                                    self.browse_page_size = *size;
                                    self.browse_view_menu_open = false;
                                    self.search_projects();
                                    self.draw();
                                    return;
                                }
                            }
                            self.browse_view_menu_open = false;
                        }
                        if browse_hide_installed_bounds(
                            library_width as usize,
                            self.browse_filter,
                        )
                        .is_some_and(|bounds| bounds.contains(x, y))
                        {
                            self.change_setting(
                                SettingsChange::HideInstalledModpacks,
                            );
                        } else if (148.0..176.0).contains(&y) {
                            if let Some((_, _, _, project_filter)) =
                                browse_project_type_tabs(library_width as usize)
                                    .iter()
                                    .find(|(_, tab_x, tab_width, _)| {
                                        (x >= *tab_x as f64)
                                            && (x < tab_x
                                                .saturating_add(*tab_width)
                                                as f64)
                                    })
                            {
                                self.browse_selected = None;
                                self.browse_details = None;
                                self.browse_filter = *project_filter;
                                self.reset_browse_filter_scroll();
                                self.browse_selected_loaders.clear();
                                self.browse_selected_categories.clear();
                                if self.browse_filter_group_count()
                                    < BROWSE_FILTER_GROUPS.len()
                                {
                                    self.browse_selected_environments.clear();
                                    if self.browse_filter_group
                                        == BROWSE_ENVIRONMENT_GROUP
                                    {
                                        self.browse_filter_group = 0;
                                    }
                                }
                                self.browse_filter_menu_open = false;
                                self.search_projects();
                            }
                        } else if (182.0..216.0).contains(&y)
                            && (search_layout.query_x as f64
                                ..(search_layout.query_x
                                    + search_layout.query_width)
                                    as f64)
                                .contains(&x)
                        {
                            self.input_focus = 5;
                            let length = self.browse_query.chars().count();
                            let (first_visible, _) = browse_query_viewport(
                                search_layout.query_width,
                                length,
                                self.browse_query_cursor,
                            );
                            let clicked = ((x
                                - (search_layout.query_x + 34) as f64)
                                / 8.0)
                                .round()
                                .max(0.0)
                                as usize;
                            self.browse_query_cursor =
                                (first_visible + clicked).min(length);
                            self.browse_query_select_all = false;
                            if !self.browse_query.is_empty()
                                && search_layout.query_width >= 44
                                && x >= (search_layout.query_x
                                    + search_layout.query_width
                                    - 32)
                                    as f64
                            {
                                self.browse_query.clear();
                                self.browse_query_cursor = 0;
                                self.browse_query_select_all = false;
                                self.search_projects();
                            }
                        } else if (218.0..246.0).contains(&y)
                            && (view_x as f64..(view_x + view_width) as f64)
                                .contains(&x)
                        {
                            self.browse_view_menu_open = true;
                            self.browse_sort_menu_open = false;
                            self.browse_filter_menu_open = false;
                        } else if (218.0..246.0).contains(&y)
                            && (search_layout.sort_x as f64
                                ..(search_layout.sort_x
                                    + search_layout.sort_width)
                                    as f64)
                                .contains(&x)
                        {
                            self.browse_sort_menu_open = true;
                            self.browse_view_menu_open = false;
                            self.browse_filter_menu_open = false;
                        } else if self.browse_selected.is_some() {
                            let project_menu_x =
                                (library_width - 220.0).max(0.0);
                            let in_project_menu = (project_menu_x
                                ..project_menu_x + 220.0)
                                .contains(&x)
                                && (334.0..494.0).contains(&y);
                            let save_menu_x =
                                (project_menu_x - 228.0).max(292.0);
                            let in_project_save_menu =
                                (save_menu_x..save_menu_x + 220.0).contains(&x)
                                    && (368.0..620.0).contains(&y);
                            let in_project_menu_button = x
                                >= library_width - 180.0
                                && x < library_width - 140.0
                                && (286.0..326.0).contains(&y);
                            if self.browse_project_menu_open
                                && !in_project_menu
                                && !(self.browse_save_menu_open
                                    && in_project_save_menu)
                                && !in_project_menu_button
                            {
                                self.browse_project_menu_open = false;
                                self.browse_save_menu_open = false;
                            }
                            if self.browse_project_menu_open
                                && self.browse_save_menu_open
                                && in_project_save_menu
                            {
                                if (562.0..598.0).contains(&y)
                                    && (save_menu_x + 8.0..save_menu_x + 212.0)
                                        .contains(&x)
                                {
                                    self.begin_browse_collection_creation();
                                }
                                let scroll =
                                    self.browse_save_scroll_pixels.round()
                                        as usize;
                                let row_position = (y as usize)
                                    .saturating_add(scroll)
                                    .saturating_sub(405);
                                if !self.browse_save_collections.is_empty()
                                    && (402.0..556.0).contains(&y)
                                    && row_position % 34 < 32
                                {
                                    let index = row_position / 34;
                                    if let Some(collection_id) = self
                                        .browse_save_collections
                                        .get(index)
                                        .map(|collection| collection.id.clone())
                                    {
                                        self.toggle_browse_collection(
                                            &collection_id,
                                        );
                                    }
                                }
                            } else if self.browse_project_menu_open
                                && in_project_menu
                            {
                                if (342.0..376.0).contains(&y) {
                                    self.browse_project_menu_open = false;
                                    self.browse_save_menu_open = false;
                                    if !self.browse_project_follow_pending {
                                        self.toggle_browse_project_follow();
                                    }
                                } else if (376.0..410.0).contains(&y)
                                    && x >= project_menu_x
                                {
                                    self.open_browse_save_menu();
                                } else if (410.0..444.0).contains(&y)
                                    && let Some(project) =
                                        self.browse_selected.as_ref()
                                {
                                    let web_type =
                                        if project.project_type == "server" {
                                            "project"
                                        } else {
                                            project.project_type.as_str()
                                        };
                                    let slug = self
                                        .browse_details
                                        .as_ref()
                                        .map(|details| details.slug.clone())
                                        .unwrap_or_else(|| project.id.clone());
                                    let url = format!(
                                        "https://modrinth.com/{web_type}/{slug}"
                                    );
                                    std::thread::spawn(move || {
                                        let _ = open::that(url);
                                    });
                                    self.browse_project_menu_open = false;
                                    self.browse_save_menu_open = false;
                                } else if (452.0..486.0).contains(&y)
                                    && let Some(project) =
                                        self.browse_selected.as_ref()
                                {
                                    let url = format!(
                                        "https://modrinth.com/report?item=project&itemID={}",
                                        project.id
                                    );
                                    std::thread::spawn(move || {
                                        let _ = open::that(url);
                                    });
                                    self.browse_project_menu_open = false;
                                    self.browse_save_menu_open = false;
                                }
                            } else if in_project_menu_button {
                                self.browse_project_menu_open =
                                    !self.browse_project_menu_open;
                                if self.browse_project_menu_open {
                                    self.load_browse_project_follow_status();
                                }
                            } else if (246.0..276.0).contains(&y)
                                && (292.0..388.0).contains(&x)
                            {
                                self.browse_project_menu_open = false;
                                self.browse_selected = None;
                                self.browse_details = None;
                                self.browse_project_menu_open = false;
                                self.browse_detail_scroll_pixels = 0.0;
                                self.browse_detail_scroll_target_pixels = 0.0;
                                self.browse_detail_tab = 0;
                                self.browse_gallery_viewer_open = false;
                                self.message = "Back to project search".into();
                                if self.browse_hidden_ids_dirty {
                                    self.search_projects();
                                }
                            } else if (286.0..326.0).contains(&y)
                                && x >= library_width - 132.0
                                && x < library_width - 20.0
                                && self.browse_selected.as_ref().is_some_and(
                                    |project| {
                                        !self.browse_project_is_installed(
                                            project,
                                        ) && !self.browse_project_is_installing(
                                            project,
                                        )
                                    },
                                )
                            {
                                if let Some(project) =
                                    self.browse_selected.clone()
                                {
                                    self.input_focus = 0;
                                    if project.project_type == "modpack" {
                                        self.browse_detail_tab = 1;
                                        self.browse_detail_scroll_pixels = 0.0;
                                        self.browse_detail_scroll_target_pixels = 0.0;
                                        self.message = "Choose a modpack version to install.".into();
                                    } else {
                                        self.install_browse_project(
                                            project, None,
                                        );
                                    }
                                }
                            } else if self.browse_details.is_some()
                                && (398.0..426.0).contains(&y)
                                && (292.0..598.0).contains(&x)
                            {
                                self.browse_project_menu_open = false;
                                self.browse_detail_tab = if x < 404.0 {
                                    0
                                } else if x < 502.0 {
                                    1
                                } else {
                                    2
                                };
                                self.browse_detail_scroll_pixels = 0.0;
                                self.browse_detail_scroll_target_pixels = 0.0;
                                self.browse_gallery_index = 0;
                                self.browse_gallery_viewer_open = false;
                            } else if self.browse_detail_tab == 0
                                && y >= 440.0
                                && let Some((_, _, lines, links, _, _)) =
                                    self.browse_description_lines.as_ref()
                            {
                                let scroll_y =
                                    self.browse_detail_scroll_pixels.floor()
                                        as isize;
                                let hit = links.iter().find(|link| {
                                    let Some(line) = lines.get(link.line_index)
                                    else {
                                        return false;
                                    };
                                    let line_y = 442
                                        + super::draw::browse_body_content_offset(
                                            lines,
                                            link.line_index,
                                        )
                                            as isize
                                        - scroll_y;
                                    let display = line
                                        .strip_prefix("│ ")
                                        .or_else(|| line.strip_prefix("▌ "))
                                        .or_else(|| {
                                            line.split_once(' ')
                                                .filter(|(hashes, _)| {
                                                    hashes.starts_with('#')
                                                        && hashes.chars().all(
                                                            |character| {
                                                                character
                                                                    == '#'
                                                            },
                                                        )
                                                })
                                                .map(|(_, body)| body)
                                        })
                                        .unwrap_or(line);
                                    let prefix: String = display
                                        .chars()
                                        .take(link.start_char)
                                        .collect();
                                    let link_text: String = display
                                        .chars()
                                        .skip(link.start_char)
                                        .take(link.end_char - link.start_char)
                                        .collect();
                                    let link_left = link.text_x
                                        + super::render::measure_text_width(
                                            &prefix,
                                            link.scale,
                                        );
                                    let link_right = link_left
                                        + super::render::measure_text_width(
                                            &link_text,
                                            link.scale,
                                        );
                                    let in_x = x >= link_left as f64
                                        && x < link_right as f64;
                                    let in_y = y >= line_y as f64
                                        && y < (line_y
                                            + super::draw::browse_body_line_height(line)
                                                as isize)
                                            as f64;
                                    in_x && in_y
                                });
                                if let Some(link) = hit {
                                    let _ = open::that(&link.url);
                                }
                            } else if self.browse_details.is_some()
                                && self.browse_detail_tab == 1
                                && let Some(group) =
                                    self.browse_detail_filter_menu_group
                                && let Some(details) =
                                    self.browse_details.as_ref()
                                && let Some((_, menu_x, _)) =
                                    browse_version_filter_buttons(
                                        library_width as usize,
                                    )
                                    .into_iter()
                                    .find(
                                        |(button_group, _, _)| {
                                            *button_group == group
                                        },
                                    )
                                && y >= 480.0
                                && y < 480.0
                                    + browse_version_filter_options(
                                        details, group,
                                    )
                                    .len()
                                    .min(7)
                                        as f64
                                        * 28.0
                                && x >= menu_x as f64
                                && x < (menu_x + 248) as f64
                            {
                                let option_index = self
                                    .browse_detail_filter_option_offset
                                    + ((y - 480.0) / 28.0).floor() as usize;
                                let option = self
                                    .browse_details
                                    .as_ref()
                                    .and_then(|details| {
                                        browse_version_filter_options(
                                            details, group,
                                        )
                                        .get(option_index)
                                        .cloned()
                                    });
                                if let Some(value) = option {
                                    let selected = match group {
                                        0 => &mut self
                                            .browse_detail_selected_channels,
                                        1 => &mut self
                                            .browse_detail_selected_game_versions,
                                        _ => &mut self
                                            .browse_detail_selected_loaders,
                                    };
                                    if !selected.insert(value.clone()) {
                                        selected.remove(&value);
                                    }
                                    self.browse_detail_scroll_pixels = 0.0;
                                    self.browse_detail_scroll_target_pixels =
                                        0.0;
                                }
                            } else if self.browse_details.is_some()
                                && self.browse_detail_tab == 1
                                && (440.0..470.0).contains(&y)
                            {
                                let group = browse_version_filter_buttons(
                                    library_width as usize,
                                )
                                .into_iter()
                                .find(|(_, button_x, button_width)| {
                                    x >= *button_x as f64
                                        && x < (button_x + button_width) as f64
                                })
                                .map(|(group, _, _)| group);
                                if let Some(group) = group {
                                    self.browse_detail_filter_menu_group =
                                        if self.browse_detail_filter_menu_group
                                            == Some(group)
                                        {
                                            None
                                        } else {
                                            Some(group)
                                        };
                                    self.browse_detail_filter_option_offset = 0;
                                }
                            } else if self.browse_details.is_some()
                                && self.browse_detail_tab == 1
                                && let Some(row) = browse_version_install_row(
                                    library_width as usize,
                                    sidebar_height as usize,
                                    x,
                                    y,
                                    self.browse_detail_scroll_pixels,
                                )
                            {
                                if let (Some(project), Some(details)) = (
                                    self.browse_selected.clone(),
                                    self.browse_details.as_ref(),
                                ) && let Some(version_index) =
                                    browse_filtered_version_indices(
                                        details,
                                        &self
                                            .browse_detail_selected_channels,
                                        &self
                                            .browse_detail_selected_game_versions,
                                        &self
                                            .browse_detail_selected_loaders,
                                    )
                                    .get(row)
                                    && let Some(version) = details
                                        .versions
                                        .get(*version_index)
                                {
                                    self.input_focus = 0;
                                    self.install_browse_project(
                                        project,
                                        Some(version.id.clone()),
                                    );
                                }
                            } else if self.browse_details.is_some()
                                && self.browse_detail_tab == 2
                            {
                                if self.browse_gallery_viewer_open {
                                    let toolbar_x =
                                        (library_width - 396.0).max(0.0) / 2.0;
                                    let toolbar_y = sidebar_height - 72.0;
                                    if y >= toolbar_y + 8.0
                                        && y < toolbar_y + 40.0
                                        && x >= toolbar_x
                                        && x < toolbar_x + 396.0
                                    {
                                        let toolbar_position = x - toolbar_x;
                                        let count = self
                                            .browse_details
                                            .as_ref()
                                            .map_or(0, |details| {
                                                details.gallery.len()
                                            });
                                        if toolbar_position < 68.0 {
                                            if count > 0 {
                                                self.browse_gallery_index = self
                                                    .browse_gallery_index
                                                    .wrapping_add(count - 1)
                                                    % count;
                                            }
                                        } else if (140.0..208.0)
                                            .contains(&toolbar_position)
                                        {
                                            if count > 0 {
                                                self.browse_gallery_index =
                                                    (self.browse_gallery_index
                                                        + 1)
                                                        % count;
                                            }
                                        } else if (216.0..288.0)
                                            .contains(&toolbar_position)
                                        {
                                            if let Some(url) = self
                                                .browse_details
                                                .as_ref()
                                                .and_then(|details| {
                                                    details.gallery.get(
                                                        self.browse_gallery_index,
                                                    )
                                                })
                                                .map(|image| {
                                                    image.raw_url.clone().unwrap_or_else(|| image.url.clone())
                                                })
                                            {
                                                let _ = open::that(url);
                                            }
                                        } else if (296.0..364.0)
                                            .contains(&toolbar_position)
                                        {
                                            self.browse_gallery_viewer_open =
                                                false;
                                        }
                                    } else if x < 32.0
                                        || x >= width as f64 - 32.0
                                        || y < 72.0
                                        || y >= sidebar_height - 24.0
                                    {
                                        self.browse_gallery_viewer_open = false;
                                    }
                                } else if let Some(index) = self
                                    .browse_details
                                    .as_ref()
                                    .and_then(|details| {
                                        super::draw::browse_gallery_item_at(
                                            library_width as usize,
                                            details.gallery.len(),
                                            self.browse_detail_scroll_pixels,
                                            x,
                                            y,
                                        )
                                    })
                                {
                                    self.browse_gallery_index = index;
                                    self.browse_gallery_viewer_open = true;
                                    self.browse_gallery_zoom = 1.0;
                                    self.input_focus = 0;
                                }
                            }
                        } else if previous_page.contains(x, y)
                            || next_page.contains(x, y)
                        {
                            if next_page.contains(x, y) {
                                self.search_next_projects_page();
                            } else if previous_page.contains(x, y) {
                                self.search_previous_projects_page();
                            }
                        } else if y >= 246.0 && !self.browse_search_pending {
                            if let Some((row, within_card)) = results_layout
                                .card_at(x, y, self.browse_scroll_pixels)
                                && let Some(project) =
                                    self.browse_results.get(row).cloned()
                            {
                                self.input_focus = 0;
                                let action = results_layout.action(0);
                                if action.contains(x, within_card) {
                                    if self
                                        .browse_project_is_installing(&project)
                                    {
                                        return;
                                    }
                                    if project.project_type == "modpack" {
                                        self.open_browse_project(project);
                                        self.browse_detail_tab = 1;
                                        self.message =
                                            "Choose a modpack version to install.".into();
                                    } else {
                                        self.install_browse_project(
                                            project, None,
                                        );
                                    }
                                } else {
                                    self.open_browse_project(project);
                                }
                            }
                        }
                    } else if self.instance_tab == 0
                        && (82.0..114.0).contains(&y)
                        && x >= library_width - 142.0
                    {
                        self.pick_content_file();
                    } else if self.instance_tab == 0
                        && (164.0..188.0).contains(&y)
                    {
                        self.content_filter = if (292.0..340.0).contains(&x) {
                            0
                        } else if (346.0..404.0).contains(&x) {
                            1
                        } else if (410.0..492.0).contains(&x) {
                            2
                        } else if (498.0..552.0).contains(&x) {
                            3
                        } else if (558.0..630.0).contains(&x) {
                            4
                        } else {
                            self.content_filter
                        };
                        self.rebuild_filtered_content_cache();
                        self.content_scroll_pixels = 0.0;
                        self.content_scroll_target_pixels = 0.0;
                    } else if self.instance_tab == 0 {
                        let visible_contents = &self.filtered_content_cache;
                        let row_height = CONTENT_ROW_HEIGHT as f64;
                        let content_y = y - CONTENT_LIST_TOP as f64
                            + self.content_scroll_pixels;
                        let row = if content_y >= 0.0 {
                            Some((content_y / row_height).floor() as usize)
                        } else {
                            None
                        };
                        if let Some(index) = row.filter(|index| {
                            let card_y = CONTENT_LIST_TOP as f64
                                + *index as f64 * row_height
                                - self.content_scroll_pixels;
                            y >= card_y
                                && y < card_y + CONTENT_ROW_HEIGHT as f64
                                && *index < visible_contents.len()
                        }) {
                            let item =
                                self.contents[visible_contents[index]].clone();
                            if !item.locked {
                                if item.has_update
                                    && x >= library_width - 336.0
                                    && x < library_width - 244.0
                                {
                                    self.update_content(item);
                                } else if x >= library_width - 236.0
                                    && x < library_width - 148.0
                                {
                                    self.remove_content(item);
                                } else if x >= library_width - 136.0 {
                                    self.toggle_content(item);
                                }
                            }
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if !self.is_home_library() {
                    return;
                }
                if self
                    .selected_group
                    .as_deref()
                    .is_some_and(|id| id != "group:favorites")
                    && (group_y - 34.0..group_y - 8.0).contains(&y)
                    && (292.0..472.0).contains(&x)
                {
                    self.input_focus = 0;
                    if x < 329.0 {
                        self.move_selected_group(-1);
                    } else if x < 372.0 {
                        self.move_selected_group(1);
                    } else if x < 427.0 {
                        self.rename_selected_group();
                    } else {
                        self.delete_selected_group();
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if !self.selected_library_instances.is_empty()
                    && self.selected_instance.is_none()
                    && !self.global_browse
                    && !self.global_skins
                    && !self.global_screenshots
                    && y >= sidebar_height - 62.0
                    && x >= 292.0
                    && x < library_width - 20.0
                {
                    let bar_x = 292.0;
                    let bar_width = library_width - 324.0;
                    let clear_x = bar_x + bar_width - 72.0;
                    if x >= clear_x && x < clear_x + 64.0 {
                        self.selected_library_instances.clear();
                        self.library_selection_anchor = None;
                    } else if self.selected_group.is_some()
                        && x >= clear_x - 108.0
                        && x < clear_x - 8.0
                    {
                        if let Some(group_id) = self.selected_group.clone() {
                            let all_selected_are_members = self
                                .selected_group_action_removes_membership(
                                    &group_id,
                                );
                            self.update_selected_group_membership(
                                group_id,
                                all_selected_are_members,
                            );
                        }
                    } else {
                        let delete_x = if self.selected_group.is_some() {
                            clear_x - 188.0
                        } else {
                            clear_x - 80.0
                        };
                        let new_group_x = if self.selected_group.is_some() {
                            clear_x - 272.0
                        } else {
                            clear_x - 164.0
                        };
                        if x >= new_group_x && x < new_group_x + 76.0 {
                            self.create_group_from_selection = true;
                            self.create_group_open = true;
                            self.group_input.clear();
                            self.input_focus = 3;
                        } else if x >= delete_x && x < delete_x + 72.0 {
                            self.bulk_delete_confirmation_open = true;
                            self.delete_confirmation_input.clear();
                            self.input_focus = 17;
                        }
                    }
                    self.draw();
                    return;
                }
                let toolbar = self.current_library_toolbar_layout(
                    library_width.max(0.0) as usize,
                );
                if toolbar.refresh(library_width as usize).contains(x, y) {
                    self.refresh_instances();
                    return;
                }
                if self
                    .selected_group
                    .as_deref()
                    .is_some_and(|id| id != "group:favorites")
                    && toolbar.membership(library_width as usize).contains(x, y)
                {
                    self.membership_mode = !self.membership_mode;
                    self.draw();
                    return;
                }
                if toolbar.search.contains(x, y) {
                    if !self.library_search_query.is_empty()
                        && x >= (toolbar.search.x
                            + toolbar.search.width.saturating_sub(34))
                            as f64
                    {
                        self.library_search_query.clear();
                        self.list_offset = 0;
                    }
                    self.input_focus = 21;
                } else if toolbar.new_group.contains(x, y) {
                    self.create_group_from_selection = false;
                    self.create_group_open = true;
                    self.group_input.clear();
                    self.input_focus = 3;
                } else if toolbar.new_instance.contains(x, y) {
                    self.create_instance_open = true;
                    self.name_input.clear();
                    self.version_input.clear();
                    self.input_focus = 1;
                } else if toolbar.import_pack.contains(x, y) {
                    self.input_focus = 0;
                    self.open_launcher_import();
                } else if toolbar.sort.contains(x, y) {
                    if x >= (toolbar.sort.x + toolbar.sort.width - 32) as f64 {
                        self.library_sort_mode ^= 1;
                        self.list_offset = 0;
                    } else {
                        self.library_sort_menu_open = true;
                        self.library_group_menu_open = false;
                        self.library_filter_open = false;
                    }
                } else if toolbar.group.contains(x, y) {
                    self.library_group_menu_open = true;
                    self.library_sort_menu_open = false;
                    self.library_filter_open = false;
                } else if toolbar.filter.contains(x, y) {
                    self.library_sort_menu_open = false;
                    self.library_group_menu_open = false;
                    self.library_filter_open = true;
                } else if self
                    .selected_group
                    .as_deref()
                    .is_some_and(|id| id != "group:favorites")
                    && y >= (self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as f64
                        - 88.0)
                    && y < (self
                        .window
                        .as_ref()
                        .map_or(720, |window| window.inner_size().height)
                        as f64
                        - 60.0)
                    && (292.0..464.0).contains(&x)
                {
                    self.input_focus = 3;
                } else {
                    self.input_focus = 0;
                }
                let [all, favorites] = toolbar.view_tabs();
                if all.contains(x, y) || favorites.contains(x, y) {
                    self.selected_group = favorites
                        .contains(x, y)
                        .then(|| "group:favorites".into());
                    self.library_group_mode = 0;
                    self.membership_mode = false;
                    self.list_offset = 0;
                    self.draw();
                    return;
                }
                let grid =
                    self.library_grid_layout(library_width.max(0.0) as usize);
                if grid.jump_in {
                    let items = self.jump_in_items();
                    let content_width =
                        (library_width.max(0.0) as usize).saturating_sub(324);
                    let visible_count = self.jump_in_visible_count(
                        library_width.max(0.0) as usize,
                        items.len(),
                    );
                    let columns =
                        Self::jump_in_columns(library_width.max(0.0) as usize)
                            .min(visible_count);
                    let tile_width = content_width
                        .saturating_sub(12 * columns.saturating_sub(1))
                        / columns.max(1);
                    let tile_y = HOME_JUMP_IN_TILES_Y;
                    if let Some(menu_index) = self.jump_in_overflow {
                        let menu_column = menu_index % columns.max(1);
                        let menu_row = menu_index / columns.max(1);
                        let menu_tile_x = 292 + menu_column * (tile_width + 12);
                        let menu_tile_y =
                            tile_y + menu_row * HOME_JUMP_IN_ROW_HEIGHT;
                        let menu_x =
                            menu_tile_x + tile_width.saturating_sub(176);
                        let menu_y = menu_tile_y + HOME_JUMP_IN_TILE_HEIGHT + 4;
                        if (menu_x as f64..(menu_x + 172) as f64).contains(&x)
                            && (menu_y as f64..(menu_y + 76) as f64)
                                .contains(&y)
                        {
                            let option =
                                ((y as usize).saturating_sub(menu_y + 6) / 34)
                                    .min(2);
                            if option < 2
                                && let Some(item) = items.get(menu_index)
                            {
                                let item = item.clone();
                                self.jump_in_overflow = None;
                                if option == 0 {
                                    self.view_jump_in_item(item);
                                    self.draw();
                                } else {
                                    self.open_jump_in_instance_folder(&item);
                                }
                                return;
                            }
                        }
                        self.jump_in_overflow = None;
                    }
                    if y >= tile_y as f64 && x >= 292.0 {
                        let relative_x = x as usize - 292;
                        let column = relative_x / (tile_width + 12).max(1);
                        let relative_y = (y as usize).saturating_sub(tile_y);
                        let row = relative_y / HOME_JUMP_IN_ROW_HEIGHT;
                        let item_index = row * columns.max(1) + column;
                        if relative_y % HOME_JUMP_IN_ROW_HEIGHT
                            < HOME_JUMP_IN_TILE_HEIGHT
                            && column < columns
                            && row < visible_count.div_ceil(columns.max(1))
                            && relative_x % (tile_width + 12).max(1)
                                < tile_width
                            && item_index < visible_count
                            && let Some(item) = items.get(item_index)
                        {
                            let tile_x = 292 + column * (tile_width + 12);
                            let local_x = x as usize - tile_x;
                            let local_y = relative_y % HOME_JUMP_IN_ROW_HEIGHT;
                            if local_y >= 24
                                && local_y < 56
                                && local_x >= tile_width.saturating_sub(36)
                            {
                                self.jump_in_overflow = (self.jump_in_overflow
                                    != Some(item_index))
                                .then_some(item_index);
                            } else if local_y >= 24
                                && local_y < 56
                                && local_x >= tile_width.saturating_sub(124)
                                && local_x < tile_width.saturating_sub(44)
                            {
                                self.jump_in_overflow = None;
                                self.activate_jump_in_item(item.clone());
                            } else {
                                self.jump_in_overflow = None;
                                self.view_jump_in_item(item.clone());
                            }
                            self.draw();
                            return;
                        }
                    }
                }
                let column_stride = grid.card_width + grid.gap;
                let grid_rows = self.library_grid_rows(grid.columns);
                let mut row_y = grid.cards_top as f64;
                let mut hit_card = None;
                let mut hit_header = None;
                for row in
                    grid_rows.iter().filter(|_| y >= grid.cards_top as f64)
                {
                    match row {
                        NativeLibraryGridRow::Header(key, _, _) => {
                            if y >= row_y && y < row_y + 40.0 && x >= 292.0 {
                                hit_header = Some(key.clone());
                                break;
                            }
                        }
                        NativeLibraryGridRow::Cards(indices) => {
                            if y >= row_y
                                && y < row_y + grid.card_height as f64
                                && x >= 292.0
                            {
                                let relative_x = x as usize - 292;
                                let column = relative_x / column_stride.max(1);
                                if column < indices.len()
                                    && relative_x % column_stride.max(1)
                                        < grid.card_width
                                {
                                    hit_card = Some((
                                        indices[column],
                                        column,
                                        row_y.max(0.0).round() as usize,
                                    ));
                                }
                            }
                        }
                    }
                    row_y += library_grid_row_height(row, grid) as f64;
                    if row_y > y && (hit_card.is_some() || hit_header.is_some())
                    {
                        break;
                    }
                }
                if let Some(key) = hit_header {
                    let is_instance_group = key == "group:favorites"
                        || self.groups.iter().any(|group| group.id == key);
                    if is_instance_group && x >= 306.0 {
                        self.selected_group = Some(key);
                        self.membership_mode = false;
                    } else if !self.library_collapsed_groups.remove(&key) {
                        self.library_collapsed_groups.insert(key);
                    }
                    self.list_offset = 0;
                } else if let Some((instance_index, column, card_y)) = hit_card
                {
                    self.library_focused_instance =
                        Some(self.instances[instance_index].0.clone());
                    let multi_select_modifier = self.modifiers.control_key()
                        || self.modifiers.super_key();
                    if self.modifiers.shift_key() && !self.membership_mode {
                        let instance_id =
                            self.instances[instance_index].0.clone();
                        self.select_library_instance_range(&instance_id);
                        self.draw();
                        return;
                    }
                    if multi_select_modifier && !self.membership_mode {
                        let instance_id =
                            self.instances[instance_index].0.clone();
                        if !self.selected_library_instances.remove(&instance_id)
                        {
                            self.selected_library_instances.insert(instance_id);
                        }
                        self.library_selection_anchor =
                            Some(self.instances[instance_index].0.clone());
                        self.draw();
                        return;
                    }
                    let card_x = 292 + column * column_stride;
                    let compact_cards =
                        self.settings.as_ref().is_some_and(|settings| {
                            settings.compact_instance_cards
                        });
                    let selection_button = library_selection_button_bounds(
                        card_x,
                        card_y,
                        grid.card_width,
                        compact_cards,
                    );
                    if !self.membership_mode && selection_button.contains(x, y)
                    {
                        let instance_id =
                            self.instances[instance_index].0.clone();
                        toggle_library_instance_selection(
                            &mut self.selected_library_instances,
                            &instance_id,
                        );
                        self.library_selection_anchor = Some(instance_id);
                        self.draw();
                        return;
                    }
                    if !self.membership_mode
                        && !multi_select_modifier
                        && !self.selected_library_instances.is_empty()
                    {
                        let instance_id =
                            self.instances[instance_index].0.clone();
                        let action =
                            self.library_card_action(&instance_id, true);
                        let compact_cards =
                            self.settings.as_ref().is_some_and(|settings| {
                                settings.compact_instance_cards
                            });
                        let action_left = if compact_cards {
                            card_x + 10
                        } else {
                            card_x + grid.card_width.saturating_sub(62)
                        };
                        let action_top = if compact_cards {
                            card_y + 12
                        } else {
                            card_y + grid.art_height.saturating_sub(38)
                        };
                        let action_hit =
                            if action == NativeLibraryCardAction::Stop {
                                (action_left..action_left + 40)
                                    .contains(&(x as usize))
                                    && (action_top..action_top + 40)
                                        .contains(&(y as usize))
                            } else {
                                false
                            };
                        if action_hit {
                            self.stop_card_process(instance_id);
                        } else {
                            toggle_library_instance_selection(
                                &mut self.selected_library_instances,
                                &instance_id,
                            );
                            self.library_selection_anchor = Some(instance_id);
                        }
                        self.draw();
                        return;
                    }
                    if !multi_select_modifier {
                        self.selected_library_instances.clear();
                        self.library_selection_anchor = None;
                    }
                    let compact_cards =
                        self.settings.as_ref().is_some_and(|settings| {
                            settings.compact_instance_cards
                        });
                    let action = self.library_card_action(
                        &self.instances[instance_index].0,
                        true,
                    );
                    let membership_x = if compact_cards {
                        card_x + grid.card_width.saturating_sub(30)
                    } else {
                        card_x + grid.card_width.saturating_sub(36)
                    };
                    let membership_y = if compact_cards {
                        card_y + 8
                    } else {
                        card_y + grid.art_height + 14
                    };
                    let membership_size = if compact_cards { 22 } else { 28 };
                    if library_membership_toggle_visible(
                        self.membership_mode,
                        action,
                    ) && (membership_x..membership_x + membership_size)
                        .contains(&(x as usize))
                        && (membership_y..membership_y + membership_size)
                            .contains(&(y as usize))
                    {
                        let (instance_id, _, mut group_ids) =
                            self.instances[instance_index].clone();
                        if let Some(group_id) = self.selected_group.clone() {
                            if group_ids.iter().any(|id| id == &group_id) {
                                group_ids.retain(|id| id != &group_id);
                            } else {
                                group_ids.push(group_id);
                            }
                            self.update_group_membership(
                                instance_id,
                                group_ids,
                            );
                        }
                    } else if let Some((id, name, _)) =
                        self.instances.get(instance_index).cloned()
                    {
                        let action_left = if compact_cards {
                            card_x + 10
                        } else {
                            card_x + grid.card_width.saturating_sub(62)
                        };
                        let action_top = if compact_cards {
                            card_y + 12
                        } else {
                            card_y + grid.art_height.saturating_sub(38)
                        };
                        let action_hit = action
                            != NativeLibraryCardAction::None
                            && (action_left..action_left + 40)
                                .contains(&(x as usize))
                            && (action_top..action_top + 40)
                                .contains(&(y as usize));
                        if action_hit {
                            match action {
                                NativeLibraryCardAction::Stop => {
                                    self.stop_card_process(id)
                                }
                                NativeLibraryCardAction::Repair => {
                                    self.repair_instance(id)
                                }
                                NativeLibraryCardAction::Play => {
                                    self.launch_instance(id, name, None, None)
                                }
                                NativeLibraryCardAction::Installing
                                | NativeLibraryCardAction::None => {}
                            }
                            self.draw();
                            return;
                        }
                        self.selected_instance = Some((id.clone(), name));
                        self.rename_instance_open = false;
                        self.rename_instance_input.clear();
                        self.instance_tab = 0;
                        self.process_state = None;
                        self.file_path.clear();
                        self.file_items.clear();
                        self.file_preview = None;
                        self.file_offset = 0;
                        self.console_text.clear();
                        self.console_scroll = 0;
                        self.instance_logs.clear();
                        self.selected_log = None;
                        self.log_text.clear();
                        self.log_analysis = None;
                        self.log_analysis_pending = false;
                        self.log_share_pending = false;
                        self.log_delete_confirmation = false;
                        self.log_delete_pending = false;
                        self.log_scroll = 0;
                        self.log_scroll_synced = 0;
                        self.log_scroll_pixels = 0.0;
                        self.log_scroll_target_pixels = 0.0;
                        self.contents.clear();
                        self.filtered_content_cache.clear();
                        self.content_scroll_pixels = 0.0;
                        self.content_scroll_target_pixels = 0.0;
                        self.load_instance_content(id);
                    }
                }
            }
            _ => {}
        }
    }
}
