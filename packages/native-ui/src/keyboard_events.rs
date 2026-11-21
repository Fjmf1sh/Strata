use super::*;

impl Launcher {
    pub(super) fn handle_keyboard_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. }
                if self.hosting_backup_rotation_confirmation
                    && event.state == ElementState::Pressed =>
            {
                if matches!(&event.logical_key, Key::Named(NamedKey::Escape)) {
                    self.hosting_backup_rotation_confirmation = false;
                } else if matches!(
                    &event.logical_key,
                    Key::Named(NamedKey::Enter)
                ) {
                    self.save_hosting_backup_options(true);
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.hosting_backup_options_open
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape)
                        if !self.hosting_backup_options_saving =>
                    {
                        self.cancel_hosting_backup_options()
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.save_hosting_backup_options(false)
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.hosting_backup_prompt.is_some()
                    && event.state == ElementState::Pressed =>
            {
                let renaming =
                    self.hosting_backup_prompt.as_ref().is_some_and(|prompt| {
                        prompt.action == HostingBackupPromptAction::Rename
                    });
                match &event.logical_key {
                    Key::Named(NamedKey::Escape)
                        if !self.hosting_backup_mutating =>
                    {
                        self.hosting_backup_prompt = None;
                        self.hosting_backup_create_name.clear();
                        self.input_focus = 0;
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.submit_hosting_backup_prompt();
                    }
                    Key::Named(NamedKey::Backspace) if renaming => {
                        self.hosting_backup_create_name.pop();
                    }
                    Key::Character(value)
                        if renaming
                            && !event.repeat
                            && value.chars().all(|ch| !ch.is_control())
                            && self
                                .hosting_backup_create_name
                                .chars()
                                .count()
                                < 64 =>
                    {
                        self.hosting_backup_create_name.push_str(value);
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.browse_collection_create_open
                    && self.input_focus == 37
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.browse_collection_create_open = false;
                        self.browse_collection_create_name.clear();
                        self.input_focus = 0;
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.create_browse_collection()
                    }
                    Key::Named(NamedKey::Backspace) => {
                        self.browse_collection_create_name.pop();
                    }
                    Key::Character(value)
                        if !event.repeat
                            && !value.chars().any(char::is_control)
                            && self
                                .browse_collection_create_name
                                .chars()
                                .count()
                                < 64 =>
                    {
                        self.browse_collection_create_name.push_str(value);
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.hosting_access_remove_confirmation.is_some()
                    && event.state == ElementState::Pressed
                    && matches!(
                        &event.logical_key,
                        Key::Named(NamedKey::Escape)
                    ) =>
            {
                self.hosting_access_remove_confirmation = None;
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.browse_install_override.is_some()
                    && event.state == ElementState::Pressed =>
            {
                if event.logical_key == Key::Named(NamedKey::Escape) {
                    self.browse_install_override = None;
                    self.draw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.browse_install_choice.is_some()
                    && event.state == ElementState::Pressed =>
            {
                if event.logical_key == Key::Named(NamedKey::Escape) {
                    self.browse_install_choice = None;
                    self.draw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.browse_gallery_viewer_open
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.browse_gallery_viewer_open = false;
                        self.browse_gallery_zoom = 1.0;
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        let count = self
                            .browse_details
                            .as_ref()
                            .map_or(0, |details| details.gallery.len());
                        if count > 0 {
                            self.browse_gallery_index = self
                                .browse_gallery_index
                                .wrapping_add(count - 1)
                                % count;
                            self.browse_gallery_zoom = 1.0;
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        let count = self
                            .browse_details
                            .as_ref()
                            .map_or(0, |details| details.gallery.len());
                        if count > 0 {
                            self.browse_gallery_index =
                                (self.browse_gallery_index + 1) % count;
                            self.browse_gallery_zoom = 1.0;
                        }
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.browse_view_menu_open
                    || self.browse_sort_menu_open
                        && event.state == ElementState::Pressed
                        && matches!(
                            &event.logical_key,
                            Key::Named(NamedKey::Escape)
                        ) =>
            {
                self.browse_view_menu_open = false;
                self.browse_sort_menu_open = false;
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.friend_incoming_open
                    && event.state == ElementState::Pressed =>
            {
                if matches!(&event.logical_key, Key::Named(NamedKey::Escape)) {
                    self.friend_incoming_open = false;
                    self.friend_incoming_scroll_pixels = 0.0;
                    self.friend_incoming_scroll_target_pixels = 0.0;
                    self.input_focus = 0;
                    self.draw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.input_focus == 36
                    && !self.friend_request_open
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.sidebar_friend_query.clear();
                        self.input_focus = 0;
                    }
                    Key::Named(NamedKey::Backspace) => {
                        self.sidebar_friend_query.pop();
                    }
                    Key::Character(value) => {
                        if value.chars().all(|ch| !ch.is_control())
                            && self.sidebar_friend_query.chars().count() < 64
                        {
                            self.sidebar_friend_query.push_str(value);
                        }
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.global_hosting
                    && self.hosting_server_id.is_some()
                    && self.hosting_tab == 1
                    && !self.hosting_backup_create_open
                    && self.input_focus == 39
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => self.input_focus = 0,
                    Key::Named(NamedKey::Backspace) => {
                        self.hosting_backup_search.pop();
                        self.hosting_backup_scroll_pixels = 0.0;
                        self.hosting_backup_scroll_target_pixels = 0.0;
                    }
                    Key::Character(value)
                        if !event.repeat
                            && value.chars().all(|ch| !ch.is_control())
                            && self.hosting_backup_search.chars().count()
                                < 64 =>
                    {
                        self.hosting_backup_search.push_str(value);
                        self.hosting_backup_scroll_pixels = 0.0;
                        self.hosting_backup_scroll_target_pixels = 0.0;
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.global_hosting
                    && self.hosting_server_id.is_some()
                    && self.hosting_tab == 1
                    && self.hosting_backup_create_open
                    && self.input_focus == 38
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.hosting_backup_create_open = false;
                        self.hosting_backup_create_name.clear();
                        self.input_focus = 0;
                    }
                    Key::Named(NamedKey::Enter) => self.create_hosting_backup(),
                    Key::Named(NamedKey::Backspace) => {
                        self.hosting_backup_create_name.pop();
                    }
                    Key::Character(value)
                        if value.chars().all(|ch| !ch.is_control())
                            && self
                                .hosting_backup_create_name
                                .chars()
                                .count()
                                < 64 =>
                    {
                        self.hosting_backup_create_name.push_str(value);
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.global_hosting
                    && self.hosting_server_id.is_some()
                    && self.hosting_tab == 2
                    && self.hosting_access_invite_open
                    && self.input_focus == 34
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.hosting_access_invite_open = false;
                        self.input_focus = 0;
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.search_hosting_access_users();
                    }
                    Key::Named(NamedKey::Backspace) => {
                        self.hosting_access_search_query.pop();
                        self.hosting_access_search_results.clear();
                        self.hosting_access_search_loading = false;
                        self.hosting_access_search_complete = false;
                        self.hosting_access_search_scroll_pixels = 0.0;
                        self.hosting_access_search_scroll_target_pixels = 0.0;
                        self.hosting_access_invite_error = None;
                    }
                    Key::Character(value) => {
                        if value.chars().all(|ch| !ch.is_control())
                            && self.hosting_access_search_query.len()
                                + value.len()
                                <= 128
                        {
                            self.hosting_access_search_query.push_str(value);
                            self.hosting_access_search_results.clear();
                            self.hosting_access_search_loading = false;
                            self.hosting_access_search_complete = false;
                            self.hosting_access_search_scroll_pixels = 0.0;
                            self.hosting_access_search_scroll_target_pixels =
                                0.0;
                            self.hosting_access_invite_error = None;
                        }
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.global_hosting
                    && self.hosting_server_id.is_some()
                    && self.input_focus == 33
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => self.input_focus = 0,
                    Key::Named(NamedKey::Enter) => {
                        self.send_hosting_console_command()
                    }
                    Key::Named(NamedKey::Backspace) => {
                        self.hosting_console_input.pop();
                    }
                    Key::Character(value) => {
                        if value.chars().all(|ch| !ch.is_control())
                            && self.hosting_console_input.len() + value.len()
                                <= 4096
                        {
                            self.hosting_console_input.push_str(value);
                        }
                    }
                    _ => {}
                }
                self.draw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.friend_remove_confirmation.is_some()
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.friend_remove_confirmation = None;
                        self.friend_remove_error = None;
                    }
                    Key::Named(NamedKey::Enter) => {
                        if let Some(friend) =
                            self.friend_remove_confirmation.clone()
                        {
                            self.remove_sidebar_friend(friend.user_id);
                        }
                    }
                    _ => {}
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.friend_request_open
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.friend_request_open = false;
                        self.friend_search_query.clear();
                        self.friend_search_results.clear();
                        self.friend_search_error = None;
                        self.input_focus = 0;
                    }
                    Key::Named(NamedKey::Enter) if self.input_focus == 25 => {
                        self.search_friend_users();
                    }
                    Key::Named(NamedKey::Backspace)
                        if self.input_focus == 25 =>
                    {
                        self.friend_search_query.pop();
                    }
                    Key::Character(value) if self.input_focus == 25 => {
                        if value.chars().all(|ch| !ch.is_control())
                            && self.friend_search_query.chars().count() < 64
                        {
                            self.friend_search_query.push_str(value);
                        }
                    }
                    _ => {}
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.launcher_import_open
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::ArrowDown) => {
                        self.move_launcher_import_focus(1)
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        self.move_launcher_import_focus(-1)
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.activate_launcher_import_focus()
                    }
                    Key::Named(NamedKey::Escape) => {
                        self.launcher_import_open = false;
                        self.launcher_import_busy = false;
                        self.launcher_import_source = None;
                        self.input_focus = 0;
                    }
                    _ => {}
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.settings_open
                    && !self.java_open
                    && self.input_focus == 0
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.settings_open = false;
                        self.settings_jump_in_focused = false;
                        self.settings_scroll_pixels = 0.0;
                        self.settings_scroll_target_pixels = 0.0;
                    }
                    Key::Named(NamedKey::ArrowLeft)
                        if self.settings_section == 0 && !event.repeat =>
                    {
                        if let Some(settings) = &self.settings {
                            self.change_setting(SettingsChange::Theme(
                                next_settings_theme(settings.theme, -1),
                            ));
                        }
                    }
                    Key::Named(NamedKey::ArrowRight)
                        if self.settings_section == 0 && !event.repeat =>
                    {
                        if let Some(settings) = &self.settings {
                            self.change_setting(SettingsChange::Theme(
                                next_settings_theme(settings.theme, 1),
                            ));
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        self.settings_jump_in_focused = false;
                        self.settings_section = (self.settings_section + 1)
                            % SETTINGS_SECTION_ROW_INDICES.len();
                        self.settings_scroll_pixels = 0.0;
                        self.settings_scroll_target_pixels = 0.0;
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        self.settings_jump_in_focused = false;
                        self.settings_section = self
                            .settings_section
                            .checked_sub(1)
                            .unwrap_or(SETTINGS_SECTION_ROW_INDICES.len() - 1);
                        self.settings_scroll_pixels = 0.0;
                        self.settings_scroll_target_pixels = 0.0;
                    }
                    Key::Named(NamedKey::Tab)
                        if !self.modifiers.shift_key()
                            && self.settings_section == 1
                            && !self.settings_jump_in_focused =>
                    {
                        self.settings_jump_in_focused = true;
                        let height =
                            self.window.as_ref().map_or(720, |window| {
                                window.inner_size().height as usize
                            });
                        let visible_rows =
                            (height.saturating_sub(206) / 50).max(1);
                        let offset = SETTINGS_SECTION_ROW_INDICES[1]
                            .len()
                            .saturating_sub(visible_rows)
                            * 50;
                        self.settings_scroll_pixels = offset as f64;
                        self.settings_scroll_target_pixels = offset as f64;
                    }
                    Key::Named(NamedKey::Tab)
                        if !self.modifiers.shift_key() =>
                    {
                        self.settings_jump_in_focused = false;
                        self.settings_section = (self.settings_section + 1)
                            % SETTINGS_SECTION_ROW_INDICES.len();
                        self.settings_scroll_pixels = 0.0;
                        self.settings_scroll_target_pixels = 0.0;
                    }
                    Key::Named(NamedKey::Tab)
                        if self.modifiers.shift_key()
                            && self.settings_jump_in_focused =>
                    {
                        self.settings_jump_in_focused = false;
                    }
                    Key::Named(NamedKey::Tab) => {
                        self.settings_jump_in_focused = false;
                        self.settings_section = self
                            .settings_section
                            .checked_sub(1)
                            .unwrap_or(SETTINGS_SECTION_ROW_INDICES.len() - 1);
                        self.settings_scroll_pixels = 0.0;
                        self.settings_scroll_target_pixels = 0.0;
                    }
                    Key::Named(NamedKey::ArrowLeft)
                        if self.settings_jump_in_focused =>
                    {
                        self.change_setting(
                            SettingsChange::QuickInstanceCount(-1),
                        );
                    }
                    Key::Named(NamedKey::ArrowRight)
                        if self.settings_jump_in_focused =>
                    {
                        self.change_setting(
                            SettingsChange::QuickInstanceCount(1),
                        );
                    }
                    Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight) => {}
                    _ => {}
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.input_focus == 0
                    && !self.delete_confirmation_open
                    && event.state == ElementState::Pressed =>
            {
                match &event.logical_key {
                    Key::Character(key) => {
                        match key.to_ascii_lowercase().as_str() {
                            "n" if !event.repeat
                                && !self.modifiers.control_key()
                                && !self.modifiers.super_key()
                                && !self.modifiers.alt_key()
                                && self
                                    .onboarding_checklist
                                    .as_ref()
                                    .is_some_and(|checklist| {
                                        !checklist.has_created_instance
                                    }) =>
                            {
                                self.create_instance_open = true;
                                self.name_input.clear();
                                self.version_input.clear();
                                self.input_focus = 1;
                            }
                            "p" => self.control_active_job(false),
                            "x" => self.control_active_job(true),
                            _ => {}
                        }
                    }
                    Key::Named(NamedKey::ArrowRight)
                        if self.is_home_library() =>
                    {
                        self.move_library_focus(1)
                    }
                    Key::Named(NamedKey::ArrowLeft)
                        if self.is_home_library() =>
                    {
                        self.move_library_focus(-1)
                    }
                    Key::Named(NamedKey::ArrowDown)
                        if self.is_home_library() =>
                    {
                        self.move_library_focus(
                            self.library_focus_columns() as isize
                        )
                    }
                    Key::Named(NamedKey::ArrowUp) if self.is_home_library() => {
                        self.move_library_focus(
                            -(self.library_focus_columns() as isize),
                        )
                    }
                    Key::Named(NamedKey::Enter) if self.is_home_library() => {
                        if let Some(id) = self.library_focused_instance.clone()
                        {
                            if !self.selected_library_instances.is_empty() {
                                toggle_library_instance_selection(
                                    &mut self.selected_library_instances,
                                    &id,
                                );
                                self.library_selection_anchor = Some(id);
                            } else if let Some((id, name, _)) = self
                                .instances
                                .iter()
                                .find(|entry| entry.0 == id)
                                .cloned()
                            {
                                self.open_library_instance(id, name);
                            }
                        }
                    }
                    Key::Named(NamedKey::Space)
                        if self.is_home_library()
                            && !self.selected_library_instances.is_empty() =>
                    {
                        if let Some(id) = self.library_focused_instance.clone()
                        {
                            toggle_library_instance_selection(
                                &mut self.selected_library_instances,
                                &id,
                            );
                            self.library_selection_anchor = Some(id);
                        }
                    }
                    _ => {}
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && matches!(
                        &event.logical_key,
                        Key::Named(NamedKey::Escape)
                    )
                    && (self.shared_instance_unlink_confirmation
                        || self.shared_instance_pending_removal.is_some()
                        || self.shared_instance_invite_open) =>
            {
                self.shared_instance_unlink_confirmation = false;
                self.shared_instance_pending_removal = None;
                self.shared_instance_invite_open = false;
                self.shared_instance_selected_user_ids.clear();
                self.input_focus = 0;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.log_delete_confirmation
                    && event.state == ElementState::Pressed
                    && matches!(
                        &event.logical_key,
                        Key::Named(NamedKey::Escape)
                    ) =>
            {
                self.log_delete_confirmation = false;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.world_delete_confirmation.is_some()
                    && event.state == ElementState::Pressed
                    && matches!(
                        &event.logical_key,
                        Key::Named(NamedKey::Enter)
                    ) =>
            {
                self.delete_singleplayer_world();
            }
            WindowEvent::KeyboardInput { event, .. }
                if (self.world_editor_open
                    || self.world_delete_confirmation.is_some())
                    && event.state == ElementState::Pressed
                    && matches!(
                        &event.logical_key,
                        Key::Named(NamedKey::Escape)
                    ) =>
            {
                self.world_editor_open = false;
                self.world_delete_confirmation = None;
                self.world_edit_path.clear();
                self.world_edit_name.clear();
                self.input_focus = 0;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.server_remove_confirmation.is_some()
                    && event.state == ElementState::Pressed
                    && matches!(
                        &event.logical_key,
                        Key::Named(NamedKey::Escape)
                    ) =>
            {
                self.server_remove_confirmation = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.input_focus == 40
                    && event.state == ElementState::Pressed =>
            {
                if (self.modifiers.control_key() || self.modifiers.super_key())
                    && matches!(&event.logical_key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
                {
                    self.log_search_select_all = true;
                } else {
                    match event.logical_key {
                        Key::Named(NamedKey::Escape | NamedKey::Enter) => {
                            self.input_focus = 0;
                            self.log_search_select_all = false;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if self.log_search_select_all {
                                self.log_search_query.clear();
                                self.log_search_select_all = false;
                            } else {
                                self.log_search_query.pop();
                            }
                            self.log_scroll_pixels = 0.0;
                            self.log_scroll_target_pixels = 0.0;
                        }
                        Key::Character(value)
                            if value.chars().all(|ch| !ch.is_control()) =>
                        {
                            if self.log_search_select_all {
                                self.log_search_query.clear();
                                self.log_search_select_all = false;
                            }
                            if self
                                .log_search_query
                                .len()
                                .saturating_add(value.len())
                                <= 256
                            {
                                self.log_search_query.push_str(&value);
                            }
                            self.log_scroll_pixels = 0.0;
                            self.log_scroll_target_pixels = 0.0;
                        }
                        _ => {}
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                if self.instance_tab == 1 {
                    self.console_scroll = 0;
                    self.console_scroll_synced = 0;
                    self.console_scroll_pixels = 0.0;
                    self.console_scroll_target_pixels = 0.0;
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.input_focus != 0
                    && event.state == ElementState::Pressed =>
            {
                if (self.modifiers.control_key() || self.modifiers.super_key())
                    && matches!(&event.logical_key, Key::Character(value) if value.eq_ignore_ascii_case("v"))
                {
                    if self.input_focus == 6 {
                        self.paste_login_code();
                    } else if self.input_focus == 5 {
                        self.paste_browse_query();
                    }
                } else if (self.modifiers.control_key()
                    || self.modifiers.super_key())
                    && matches!(&event.logical_key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
                    && matches!(self.input_focus, 5 | 30..=34)
                {
                    if self.input_focus == 5 {
                        self.browse_query_select_all = true;
                        self.browse_query_cursor =
                            self.browse_query.chars().count();
                    } else if self.input_focus == 30 {
                        self.default_java_arguments_input.clear();
                    } else if self.input_focus == 31 {
                        self.default_environment_variables_input.clear();
                    } else if self.input_focus == 32 {
                        self.default_pre_launch_hook_input.clear();
                    } else if self.input_focus == 33 {
                        self.default_wrapper_hook_input.clear();
                    } else if self.input_focus == 34 {
                        self.default_post_exit_hook_input.clear();
                    }
                } else {
                    match event.logical_key {
                        Key::Named(NamedKey::Escape)
                            if self.input_focus == 26
                                || self.input_focus == 27
                                || (30..=34).contains(&self.input_focus) =>
                        {
                            self.input_focus = 0;
                            if let Some(settings) = self.settings.as_ref() {
                                self.default_window_width_input =
                                    settings.game_resolution.0.to_string();
                                self.default_window_height_input =
                                    settings.game_resolution.1.to_string();
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
                        }
                        Key::Character(value)
                            if value.chars().all(|ch| !ch.is_control()) =>
                        {
                            if self.input_focus == 1 {
                                self.name_input.push_str(&value);
                            } else if self.input_focus == 2 {
                                self.version_input.push_str(&value);
                            } else if self.input_focus == 6 {
                                if self
                                    .login_code_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                                {
                                    self.login_code_input.push_str(&value);
                                }
                            } else if self.input_focus == 5 {
                                let mut chars = self
                                    .browse_query
                                    .chars()
                                    .collect::<Vec<_>>();
                                if self.browse_query_select_all {
                                    chars.clear();
                                    self.browse_query_cursor = 0;
                                    self.browse_query_select_all = false;
                                }
                                let cursor =
                                    self.browse_query_cursor.min(chars.len());
                                let inserted =
                                    value.chars().collect::<Vec<_>>();
                                chars.splice(
                                    cursor..cursor,
                                    inserted.iter().copied(),
                                );
                                self.browse_query = chars.into_iter().collect();
                                self.browse_query_cursor =
                                    cursor + inserted.len();
                                self.schedule_browse_search();
                            } else if self.input_focus == 7 {
                                if self
                                    .instance_java_path
                                    .len()
                                    .saturating_add(value.len())
                                    <= 2048
                                {
                                    self.instance_java_path.push_str(&value);
                                }
                            } else if self.input_focus == 8 {
                                if self
                                    .instance_launch_args
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                                {
                                    self.instance_launch_args.push_str(&value);
                                }
                            } else if self.input_focus == 9 {
                                if value.chars().all(|ch| ch.is_ascii_digit())
                                    && self
                                        .instance_memory_mb
                                        .len()
                                        .saturating_add(value.len())
                                        <= 8
                                {
                                    self.instance_memory_mb.push_str(&value);
                                }
                            } else if self.input_focus == 10 {
                                if value.chars().all(|ch| ch.is_ascii_digit())
                                    && self
                                        .instance_resolution_width
                                        .len()
                                        .saturating_add(value.len())
                                        <= 5
                                {
                                    self.instance_resolution_width
                                        .push_str(&value);
                                }
                            } else if self.input_focus == 11 {
                                if value.chars().all(|ch| ch.is_ascii_digit())
                                    && self
                                        .instance_resolution_height
                                        .len()
                                        .saturating_add(value.len())
                                        <= 5
                                {
                                    self.instance_resolution_height
                                        .push_str(&value);
                                }
                            } else if self.input_focus == 12 {
                                if self
                                    .instance_environment_json
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                                {
                                    self.instance_environment_json
                                        .push_str(&value);
                                }
                            } else if self.input_focus == 13 {
                                if self
                                    .instance_pre_launch_hook
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                                {
                                    self.instance_pre_launch_hook
                                        .push_str(&value);
                                }
                            } else if self.input_focus == 14 {
                                if self
                                    .instance_wrapper_hook
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                                {
                                    self.instance_wrapper_hook.push_str(&value);
                                }
                            } else if self.input_focus == 15 {
                                if self
                                    .instance_post_exit_hook
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                                {
                                    self.instance_post_exit_hook
                                        .push_str(&value);
                                }
                            } else if self.input_focus == 16 {
                                if self
                                    .rename_instance_input
                                    .chars()
                                    .count()
                                    .saturating_add(value.chars().count())
                                    <= 64
                                {
                                    self.rename_instance_input.push_str(&value);
                                }
                            } else if self.input_focus == 17 {
                                if self
                                    .delete_confirmation_input
                                    .chars()
                                    .count()
                                    .saturating_add(value.chars().count())
                                    <= 512
                                {
                                    self.delete_confirmation_input
                                        .push_str(&value);
                                }
                            } else if self.input_focus == 18 {
                                if self
                                    .screenshot_group_name_input
                                    .chars()
                                    .count()
                                    .saturating_add(value.chars().count())
                                    <= 64
                                {
                                    self.screenshot_group_name_input
                                        .push_str(&value);
                                }
                            } else if self.input_focus == 19 {
                                if self
                                    .screenshot_search_query
                                    .chars()
                                    .count()
                                    .saturating_add(value.chars().count())
                                    <= 128
                                {
                                    self.screenshot_search_query
                                        .push_str(&value);
                                    self.screenshot_offset = 0;
                                    self.selected_screenshot = None;
                                    self.selected_screenshot_ids.clear();
                                }
                            } else if self.input_focus == 20 {
                                if self
                                    .shared_instance_user_query
                                    .chars()
                                    .count()
                                    < 64
                                {
                                    self.shared_instance_user_query
                                        .push_str(&value);
                                }
                            } else if self.input_focus == 21 {
                                if self
                                    .library_search_query
                                    .len()
                                    .saturating_add(value.len())
                                    <= 128
                                {
                                    self.library_search_query.push_str(&value);
                                    self.list_offset = 0;
                                }
                            } else if self.input_focus == 22 {
                                if self.server_name_input.chars().count() < 64
                                    && !value.chars().any(char::is_control)
                                {
                                    self.server_name_input.push_str(&value);
                                }
                            } else if self.input_focus == 23 {
                                if self
                                    .server_address_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 255
                                    && !value.chars().any(char::is_control)
                                {
                                    self.server_address_input.push_str(&value);
                                }
                            } else if self.input_focus == 24 {
                                if self.world_edit_name.chars().count() < 64
                                    && !value.chars().any(char::is_control)
                                {
                                    self.world_edit_name.push_str(&value);
                                }
                            } else if self.input_focus == 26
                                && value.chars().all(|ch| ch.is_ascii_digit())
                                && self
                                    .default_window_width_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 5
                            {
                                self.default_window_width_input
                                    .push_str(&value);
                            } else if self.input_focus == 27
                                && value.chars().all(|ch| ch.is_ascii_digit())
                                && self
                                    .default_window_height_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 5
                            {
                                self.default_window_height_input
                                    .push_str(&value);
                            } else if self.input_focus == 30
                                && self
                                    .default_java_arguments_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                            {
                                self.default_java_arguments_input
                                    .push_str(&value);
                            } else if self.input_focus == 31
                                && self
                                    .default_environment_variables_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                            {
                                self.default_environment_variables_input
                                    .push_str(&value);
                            } else if self.input_focus == 32
                                && self
                                    .default_pre_launch_hook_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                            {
                                self.default_pre_launch_hook_input
                                    .push_str(&value);
                            } else if self.input_focus == 33
                                && self
                                    .default_wrapper_hook_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                            {
                                self.default_wrapper_hook_input
                                    .push_str(&value);
                            } else if self.input_focus == 34
                                && self
                                    .default_post_exit_hook_input
                                    .len()
                                    .saturating_add(value.len())
                                    <= 4096
                            {
                                self.default_post_exit_hook_input
                                    .push_str(&value);
                            } else if self.input_focus == 26
                                || self.input_focus == 27
                                || (30..=34).contains(&self.input_focus)
                            {
                            } else {
                                self.group_input.push_str(&value);
                            }
                        }
                        Key::Named(NamedKey::ArrowLeft)
                            if self.input_focus == 5 =>
                        {
                            self.browse_query_cursor =
                                self.browse_query_cursor.saturating_sub(1);
                            self.browse_query_select_all = false;
                        }
                        Key::Named(NamedKey::ArrowRight)
                            if self.input_focus == 5 =>
                        {
                            self.browse_query_cursor = self
                                .browse_query_cursor
                                .saturating_add(1)
                                .min(self.browse_query.chars().count());
                            self.browse_query_select_all = false;
                        }
                        Key::Named(NamedKey::Home) if self.input_focus == 5 => {
                            self.browse_query_cursor = 0;
                            self.browse_query_select_all = false;
                        }
                        Key::Named(NamedKey::End) if self.input_focus == 5 => {
                            self.browse_query_cursor =
                                self.browse_query.chars().count();
                            self.browse_query_select_all = false;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if self.input_focus == 1 {
                                self.name_input.pop();
                            } else if self.input_focus == 2 {
                                self.version_input.pop();
                            } else if self.input_focus == 6 {
                                self.login_code_input.pop();
                            } else if self.input_focus == 5 {
                                if self.browse_query_select_all {
                                    self.browse_query.clear();
                                    self.browse_query_cursor = 0;
                                    self.browse_query_select_all = false;
                                    self.schedule_browse_search();
                                } else {
                                    let mut chars = self
                                        .browse_query
                                        .chars()
                                        .collect::<Vec<_>>();
                                    let cursor = self
                                        .browse_query_cursor
                                        .min(chars.len());
                                    if cursor > 0 {
                                        chars.remove(cursor - 1);
                                        self.browse_query_cursor = cursor - 1;
                                        self.browse_query =
                                            chars.into_iter().collect();
                                        self.schedule_browse_search();
                                    }
                                }
                            } else if self.input_focus == 7 {
                                self.instance_java_path.pop();
                            } else if self.input_focus == 8 {
                                self.instance_launch_args.pop();
                            } else if self.input_focus == 9 {
                                self.instance_memory_mb.pop();
                            } else if self.input_focus == 10 {
                                self.instance_resolution_width.pop();
                            } else if self.input_focus == 11 {
                                self.instance_resolution_height.pop();
                            } else if self.input_focus == 12 {
                                self.instance_environment_json.pop();
                            } else if self.input_focus == 13 {
                                self.instance_pre_launch_hook.pop();
                            } else if self.input_focus == 14 {
                                self.instance_wrapper_hook.pop();
                            } else if self.input_focus == 15 {
                                self.instance_post_exit_hook.pop();
                            } else if self.input_focus == 16 {
                                self.rename_instance_input.pop();
                            } else if self.input_focus == 17 {
                                self.delete_confirmation_input.pop();
                            } else if self.input_focus == 18 {
                                self.screenshot_group_name_input.pop();
                            } else if self.input_focus == 19 {
                                self.screenshot_search_query.pop();
                                self.screenshot_offset = 0;
                                self.selected_screenshot = None;
                                self.selected_screenshot_ids.clear();
                            } else if self.input_focus == 20 {
                                self.shared_instance_user_query.pop();
                            } else if self.input_focus == 21 {
                                self.library_search_query.pop();
                                self.list_offset = 0;
                            } else if self.input_focus == 22 {
                                self.server_name_input.pop();
                            } else if self.input_focus == 23 {
                                self.server_address_input.pop();
                            } else if self.input_focus == 24 {
                                self.world_edit_name.pop();
                            } else if self.input_focus == 26 {
                                self.default_window_width_input.pop();
                            } else if self.input_focus == 27 {
                                self.default_window_height_input.pop();
                            } else if self.input_focus == 30 {
                                self.default_java_arguments_input.pop();
                            } else if self.input_focus == 31 {
                                self.default_environment_variables_input.pop();
                            } else if self.input_focus == 32 {
                                self.default_pre_launch_hook_input.pop();
                            } else if self.input_focus == 33 {
                                self.default_wrapper_hook_input.pop();
                            } else if self.input_focus == 34 {
                                self.default_post_exit_hook_input.pop();
                            } else {
                                self.group_input.pop();
                            }
                        }
                        Key::Named(NamedKey::Delete)
                            if self.input_focus == 5 =>
                        {
                            let mut chars =
                                self.browse_query.chars().collect::<Vec<_>>();
                            let cursor =
                                self.browse_query_cursor.min(chars.len());
                            if self.browse_query_select_all {
                                chars.clear();
                                self.browse_query_cursor = 0;
                                self.browse_query_select_all = false;
                                self.browse_query = chars.into_iter().collect();
                                self.schedule_browse_search();
                            } else if cursor < chars.len() {
                                chars.remove(cursor);
                                self.browse_query = chars.into_iter().collect();
                                self.schedule_browse_search();
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            if self.input_focus == 3 {
                                self.create_group();
                            } else if self.input_focus == 6 {
                                self.finish_minecraft_login();
                            } else if self.input_focus == 5 {
                                self.input_focus = 0;
                                self.search_projects();
                            } else if (7..=15).contains(&self.input_focus) {
                                self.save_instance_launch_settings();
                            } else if self.input_focus == 16 {
                                self.save_instance_name();
                            } else if self.input_focus == 17 {
                                if self.bulk_delete_confirmation_open {
                                    self.remove_selected_library_instances();
                                } else {
                                    self.remove_selected_instance();
                                }
                            } else if self.input_focus == 18 {
                                self.save_screenshot_group_rename();
                            } else if self.input_focus == 19 {
                                self.input_focus = 0;
                            } else if self.input_focus == 20 {
                                self.search_shared_instance_users();
                            } else if self.input_focus == 21 {
                                self.input_focus = 0;
                            } else if self.input_focus == 22
                                || self.input_focus == 23
                            {
                                self.add_server_to_instance();
                            } else if self.input_focus == 24 {
                                self.save_singleplayer_world();
                            } else if self.input_focus == 26
                                || self.input_focus == 27
                            {
                                self.save_default_window_settings();
                            } else if (30..=34).contains(&self.input_focus) {
                                self.save_default_launch_settings();
                            } else if self.input_focus == 3 {
                                if self.create_group_open {
                                    self.create_group_open = false;
                                    self.create_group();
                                } else if self.selected_group.is_some() {
                                    self.rename_selected_group();
                                }
                            } else if self.create_instance_open {
                                self.create_instance_open = false;
                                self.create_instance();
                            } else {
                                self.create_instance();
                            }
                        }
                        Key::Named(NamedKey::Escape) => {
                            if self.world_editor_open {
                                self.world_editor_open = false;
                                self.world_edit_path.clear();
                                self.world_edit_name.clear();
                            } else if self.world_delete_confirmation.is_some() {
                                self.world_delete_confirmation = None;
                            } else if self.add_server_open {
                                self.add_server_open = false;
                                self.server_name_input.clear();
                                self.server_address_input.clear();
                                self.server_edit_index = None;
                                self.server_edit_source = None;
                            } else if self.server_remove_confirmation.is_some()
                            {
                                self.server_remove_confirmation = None;
                            } else if self.skin_delete_confirmation.is_some() {
                                self.skin_delete_confirmation = None;
                            } else if self.launcher_import_open {
                                self.launcher_import_open = false;
                                self.launcher_import_source = None;
                                self.input_focus = 0;
                            } else if self.library_sort_menu_open
                                || self.library_group_menu_open
                            {
                                self.library_sort_menu_open = false;
                                self.library_group_menu_open = false;
                            } else if self.library_filter_open {
                                self.library_filter_open = false;
                            } else if self.create_group_open {
                                self.create_group_open = false;
                                self.create_group_from_selection = false;
                                self.group_input.clear();
                            } else if self.create_instance_open {
                                self.create_instance_open = false;
                                self.name_input.clear();
                                self.version_input.clear();
                            } else if self.input_focus == 16 {
                                self.rename_instance_open = false;
                                self.rename_instance_input.clear();
                            } else if self.input_focus == 17 {
                                self.delete_confirmation_open = false;
                                self.bulk_delete_confirmation_open = false;
                                self.delete_confirmation_input.clear();
                            } else if self.input_focus == 18 {
                                self.screenshot_group_rename_id = None;
                                self.screenshot_group_name_input.clear();
                                self.screenshot_group_picker_open = false;
                            } else if self.input_focus == 19 {
                                self.input_focus = 0;
                            } else if self.input_focus == 20 {
                                self.shared_instance_invite_open = false;
                                self.shared_instance_selected_user_ids.clear();
                            }
                            self.input_focus = 0;
                        }
                        _ => {}
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
