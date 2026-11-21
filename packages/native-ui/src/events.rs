use super::*;
use winit::application::ApplicationHandler;
use winit::event_loop::ControlFlow;

#[cfg(target_os = "windows")]
fn ensure_initial_window_size(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetWindowPos(
            window: *mut std::ffi::c_void,
            insert_after: *mut std::ffi::c_void,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
    }

    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    unsafe {
        let result = SetWindowPos(
            handle.hwnd.get() as *mut std::ffi::c_void,
            std::ptr::null_mut(),
            0,
            0,
            1100,
            720,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = result;
    }
}

#[cfg(not(target_os = "windows"))]
fn ensure_initial_window_size(_window: &Window) {}

impl ApplicationHandler<UiMessage> for Launcher {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let mut scrolling = false;
        let now = std::time::Instant::now();
        let countdown_active = self.global_hosting
            && self.hosting_server_id.is_some()
            && self.hosting_tab == 1
            && !self.hosting_backup_options_open
            && !self.hosting_backup_create_open
            && self
                .hosting_backups
                .as_ref()
                .is_some_and(|backups| backups.next_auto_backup_at.is_some());
        let next_countdown_update = if countdown_active {
            let interval = std::time::Duration::from_secs(30);
            if now >= self.hosting_backup_countdown_refresh + interval {
                self.hosting_backup_countdown_refresh = now;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Some(self.hosting_backup_countdown_refresh + interval)
        } else {
            None
        };
        let frame_seconds = now
            .saturating_duration_since(self.scroll_frame_time)
            .as_secs_f64()
            .clamp(1.0 / 240.0, 0.05);
        self.scroll_frame_time = now;
        macro_rules! advance_scroll {
            ($current:expr, $target:expr) => {{
                if ($target - $current).abs() > 0.5 {
                    $current =
                        eased_scroll_position($current, $target, frame_seconds);
                    scrolling = true;
                } else {
                    $current = $target;
                }
            }};
        }

        if self.browse_install_choice.is_some() {
            if let Some(window) = &self.window {
                let size = window.inner_size();
                let layout = InstallInstancePickerLayout::new(
                    size.width as usize,
                    size.height as usize,
                    self.instances.len(),
                );
                let max_scroll = layout.max_scroll(self.instances.len());
                self.browse_install_scroll =
                    self.browse_install_scroll.clamp(0.0, max_scroll);
                self.browse_install_scroll_target =
                    self.browse_install_scroll_target.clamp(0.0, max_scroll);
            }
            advance_scroll!(
                self.browse_install_scroll,
                self.browse_install_scroll_target
            );
        }

        advance_scroll!(
            self.account_scroll_pixels,
            self.account_scroll_target_pixels
        );
        advance_scroll!(
            self.friend_incoming_scroll_pixels,
            self.friend_incoming_scroll_target_pixels
        );
        self.account_offset =
            (self.account_scroll_pixels / 58.0).floor() as usize;
        advance_scroll!(
            self.shared_instance_preview_scroll_pixels,
            self.shared_instance_preview_scroll_target_pixels
        );
        advance_scroll!(
            self.console_scroll_pixels,
            self.console_scroll_target_pixels
        );
        advance_scroll!(self.log_scroll_pixels, self.log_scroll_target_pixels);
        advance_scroll!(
            self.hosting_console_scroll_pixels,
            self.hosting_console_scroll_target_pixels
        );
        advance_scroll!(
            self.screenshot_scroll_pixels,
            self.screenshot_scroll_target_pixels
        );
        advance_scroll!(
            self.world_scroll_pixels,
            self.world_scroll_target_pixels
        );
        advance_scroll!(
            self.file_list_scroll_pixels,
            self.file_list_scroll_target_pixels
        );
        advance_scroll!(
            self.sidebar_scroll_pixels,
            self.sidebar_scroll_target_pixels
        );
        advance_scroll!(
            self.library_scroll_pixels,
            self.library_scroll_target_pixels
        );

        if self.browse_selected.is_some() {
            advance_scroll!(
                self.browse_detail_scroll_pixels,
                self.browse_detail_scroll_target_pixels
            );
        } else {
            advance_scroll!(
                self.browse_scroll_pixels,
                self.browse_scroll_target_pixels
            );
        }
        advance_scroll!(
            self.settings_scroll_pixels,
            self.settings_scroll_target_pixels
        );
        advance_scroll!(
            self.minecraft_skin_scroll_pixels,
            self.minecraft_skin_scroll_target_pixels
        );
        advance_scroll!(
            self.content_scroll_pixels,
            self.content_scroll_target_pixels
        );
        advance_scroll!(
            self.user_profile_scroll_pixels,
            self.user_profile_scroll_target_pixels
        );
        advance_scroll!(
            self.hosting_scroll_pixels,
            self.hosting_scroll_target_pixels
        );
        advance_scroll!(
            self.hosting_backup_scroll_pixels,
            self.hosting_backup_scroll_target_pixels
        );
        advance_scroll!(
            self.hosting_access_scroll_pixels,
            self.hosting_access_scroll_target_pixels
        );
        advance_scroll!(
            self.hosting_access_search_scroll_pixels,
            self.hosting_access_search_scroll_target_pixels
        );
        advance_scroll!(
            self.launcher_import_scroll_pixels,
            self.launcher_import_scroll_target_pixels
        );
        self.launcher_import_offset =
            (self.launcher_import_scroll_pixels / 44.0).floor() as usize;
        advance_scroll!(
            self.library_game_version_filter_scroll_pixels,
            self.library_game_version_filter_scroll_target_pixels
        );
        self.library_game_version_filter_offset =
            (self.library_game_version_filter_scroll_pixels / 24.0).floor()
                as usize;
        advance_scroll!(
            self.screenshot_group_scroll_pixels,
            self.screenshot_group_scroll_target_pixels
        );
        self.screenshot_group_offset =
            (self.screenshot_group_scroll_pixels / 34.0).floor() as usize;
        if self.file_preview.is_some() {
            advance_scroll!(
                self.file_preview_scroll_pixels,
                self.file_preview_scroll_target_pixels
            );
            self.file_offset =
                (self.file_preview_scroll_pixels / 12.0).floor() as usize;
        }
        advance_scroll!(
            self.browse_filter_scroll_pixels,
            self.browse_filter_scroll_target_pixels
        );

        let mut next_wake = next_countdown_update;
        if scrolling {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            let next_frame = now + std::time::Duration::from_millis(16);
            next_wake = Some(
                next_wake
                    .map_or(next_frame, |deadline| deadline.min(next_frame)),
            );
        }
        if let Some(deadline) = next_wake {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Northstar Launcher")
            .with_window_icon(northstar_window_icon())
            .with_decorations(cfg!(target_os = "macos"))
            .with_visible(true)
            .with_inner_size(LogicalSize::new(1100, 720))
            .with_min_inner_size(LogicalSize::new(760, 520));
        let window = match event_loop.create_window(attrs) {
            Ok(window) => window,
            Err(error) => {
                eprintln!("Could not create native launcher window: {error}");
                event_loop.exit();
                return;
            }
        };
        let window = Arc::new(window);
        let _ = window.request_inner_size(LogicalSize::new(1100, 720));
        ensure_initial_window_size(&window);
        let _ = window.focus_window();
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(context) => context,
            Err(error) => {
                eprintln!(
                    "Could not create launcher softbuffer context: {error}"
                );
                event_loop.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(surface) => surface,
            Err(error) => {
                eprintln!(
                    "Could not create launcher softbuffer surface: {error}"
                );
                event_loop.exit();
                return;
            }
        };
        self.surface = Some(surface);
        self.window = Some(window);

        let proxy = self.proxy.as_ref().unwrap().clone();
        let runtime = self.runtime.clone();
        let start_fresh = std::mem::take(&mut self.start_fresh_on_startup);
        runtime.spawn(async move {
            if start_fresh {
                let result = native_backend::remember_fresh_start_choice(
                    "org.northstar.launcher",
                )
                .await;
                let _ =
                    proxy.send_event(UiMessage::FreshStartChoiceSaved(result));
            } else {
                let result = native_backend::inspect_existing_modrinth_data(
                    "org.northstar.launcher",
                )
                .await;
                let _ =
                    proxy.send_event(UiMessage::ExistingDataChecked(result));
            }
        });
        self.message = if start_fresh {
            "Starting a separate Northstar library…".into()
        } else {
            "Checking for an existing Modrinth library…".into()
        };
        self.draw();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UiMessage) {
        if matches!(&event, UiMessage::CloseWindow) {
            event_loop.exit();
            return;
        }
        self.handle_user_event(event);
        self.schedule_message_dismissal();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                eprintln!("Native launcher received CloseRequested");
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                self.window_maximized = self
                    .window
                    .as_ref()
                    .is_some_and(|window| window.is_maximized());
                self.draw();
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::ThemeChanged(_) => self.draw(),
            WindowEvent::CursorMoved { position, .. } => {
                if self.jump_in_resize_drag.is_some() {
                    self.update_jump_in_resize_drag(position.y);
                }
                let layout_width =
                    self.window.as_ref().map_or(1024, |window| {
                        let width = window.inner_size().width as usize;
                        width.saturating_sub(
                            if self.should_show_right_sidebar() {
                                300
                            } else {
                                0
                            },
                        )
                    });
                let skin_cursor_x = if self.global_skins {
                    render::page_x_to_logical(position.x, layout_width)
                } else {
                    position.x
                };
                if self.minecraft_skin_dragging {
                    let delta =
                        skin_cursor_x - self.minecraft_skin_last_cursor_x;
                    self.minecraft_skin_rotation =
                        (self.minecraft_skin_rotation + (delta as f32 * 0.012))
                            .rem_euclid(std::f32::consts::TAU);
                }
                self.minecraft_skin_last_cursor_x = skin_cursor_x;
                self.cursor = (position.x, position.y);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(delta);
            }
            keyboard_event @ WindowEvent::KeyboardInput { .. } => {
                self.handle_keyboard_event(keyboard_event);
                self.schedule_message_dismissal();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(state, button);
                self.schedule_message_dismissal();
            }
            _ => {}
        }
    }
}
