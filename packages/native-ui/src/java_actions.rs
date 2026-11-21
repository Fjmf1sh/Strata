use super::*;

impl Launcher {
    pub(super) fn load_java_versions(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .configured_java_versions()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::JavaVersionsLoaded(result));
        });
    }

    pub(super) fn load_instance_launch_settings(&self, instance_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = async {
                let (metadata, maximum_memory) = tokio::try_join!(
                    backend.get_instance(&instance_id),
                    backend.maximum_memory_mb(),
                )
                .map_err(|error| error.to_string())?;
                let metadata = metadata
                    .ok_or_else(|| "instance no longer exists".to_string())?;
                Ok((metadata, maximum_memory))
            }
            .await;
            let _ = proxy.send_event(UiMessage::InstanceLaunchSettingsLoaded(
                instance_id,
                result,
            ));
        });
    }

    pub(super) fn save_instance_launch_settings(&mut self) {
        if self.instance_settings_saving || self.instance_settings_loading {
            return;
        }
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let memory_mb = if self.instance_memory_mb.trim().is_empty() {
            None
        } else {
            match self.instance_memory_mb.trim().parse::<u32>() {
                Ok(memory)
                    if (512..=self.instance_max_memory_mb)
                        .contains(&memory) =>
                {
                    Some(memory)
                }
                _ => {
                    self.message = format!(
                        "Memory must be between 512 and {} MB.",
                        self.instance_max_memory_mb
                    );
                    return;
                }
            }
        };
        let java_path = match self.instance_java_path.trim() {
            "" => None,
            path => Some(path.to_owned()),
        };
        let args = match parse_launch_args(&self.instance_launch_args) {
            Ok(args) => args,
            Err(error) => {
                self.message = error.into();
                return;
            }
        };
        let resolution = match parse_window_resolution(
            &self.instance_resolution_width,
            &self.instance_resolution_height,
        ) {
            Ok(resolution) => resolution,
            Err(error) => {
                self.message = error.into();
                return;
            }
        };
        let fullscreen = match self.instance_fullscreen_mode {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        };
        let environment =
            match parse_environment_json(&self.instance_environment_json) {
                Ok(environment) => environment,
                Err(error) => {
                    self.message = error;
                    return;
                }
            };
        let hooks = theseus::data::Hooks {
            pre_launch: nonempty_setting(&self.instance_pre_launch_hook),
            wrapper: nonempty_setting(&self.instance_wrapper_hook),
            post_exit: nonempty_setting(&self.instance_post_exit_hook),
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.instance_settings_saving = true;
        self.input_focus = 0;
        self.message = "Saving instance launch settings…".into();
        self.runtime.spawn(async move {
            let result = backend
                .set_instance_launch_settings(
                    &instance_id,
                    java_path,
                    args,
                    memory_mb,
                    fullscreen,
                    resolution,
                    environment,
                    hooks,
                )
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::InstanceLaunchSettingsSaved(
                instance_id,
                result,
            ));
        });
    }

    pub(super) fn pick_instance_java_path(&self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let runtime = self.runtime.clone();
        std::thread::spawn(move || {
            let path = runtime.block_on(async {
                rfd::AsyncFileDialog::new()
                    .pick_file()
                    .await
                    .map(|file| file.path().to_string_lossy().into_owned())
            });
            let _ = proxy.send_event(UiMessage::InstanceJavaPathPicked(
                instance_id,
                path,
            ));
        });
    }

    pub(super) fn install_java_runtime(&mut self, major: u32) {
        if self.java_busy {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.java_busy = true;
        self.message = format!("Installing Java {major}…");
        self.runtime.spawn(async move {
            let result = backend
                .install_java(major)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::JavaUpdated(result));
        });
    }

    pub(super) fn pick_java_runtime(&mut self) {
        if self.java_busy {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.java_busy = true;
        self.message = "Choose a Java executable to register…".into();
        std::thread::spawn(move || {
            let result = runtime.block_on(async {
                rfd::AsyncFileDialog::new()
                    .pick_file()
                    .await
                    .map(|file| file.path().to_path_buf())
            });
            if let Some(path) = result {
                let result = runtime
                    .block_on(backend.register_java(path))
                    .map_err(|error| error.to_string());
                let _ = proxy.send_event(UiMessage::JavaUpdated(result));
            } else {
                let _ = proxy.send_event(UiMessage::JavaPickerCancelled);
            }
        });
    }
}
