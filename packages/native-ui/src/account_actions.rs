use super::*;

impl Launcher {
    pub(super) fn load_accounts(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = async {
                let (accounts, default, content_accounts) = tokio::try_join!(
                    backend.minecraft_accounts(),
                    backend.default_minecraft_account(),
                    backend.content_service_accounts(),
                )
                .map_err(|error| error.to_string())?;
                Ok((accounts, default, content_accounts))
            }
            .await;
            let _ = proxy.send_event(UiMessage::AccountsLoaded(result));
        });
    }

    pub(super) fn select_minecraft_account(&self, id: uuid::Uuid) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .set_default_minecraft_account(id)
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::AccountDefaultChanged(id, result));
        });
    }

    pub(super) fn remove_minecraft_account(&self, id: uuid::Uuid) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .remove_minecraft_account(id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::MinecraftAccountRemoved(id, result));
        });
    }

    pub(super) fn begin_minecraft_login(&mut self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.message = "Preparing Microsoft sign-in…".into();
        self.runtime.spawn(async move {
            let result = backend
                .begin_minecraft_login()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::LoginFlowReady(result));
        });
    }

    pub(super) fn begin_modrinth_login(&mut self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.message = "Preparing Modrinth sign-in…".into();
        self.runtime.spawn(async move {
            let result = backend
                .begin_content_service_login()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ModrinthLoginReady(result));
        });
    }

    pub(super) fn select_content_service_account(&self, user_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .set_active_content_service_account(&user_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::ContentAccountChanged(user_id, result));
        });
    }

    pub(super) fn remove_content_service_account(&self, user_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .remove_content_service_account(&user_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::ContentAccountRemoved(user_id, result));
        });
    }

    pub(super) fn paste_login_code(&mut self) {
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.message = "Reading clipboard…".into();
        self.runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(clipboard_text)
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            let _ = proxy.send_event(UiMessage::ClipboardLoaded(result));
        });
    }

    pub(super) fn paste_browse_query(&mut self) {
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(clipboard_text)
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            let _ = proxy.send_event(UiMessage::ClipboardLoaded(result));
        });
    }

    pub(super) fn paste_into_instance_setup(
        &mut self,
        value: &str,
    ) -> Result<(), String> {
        let focus = self.input_focus;
        let target = match focus {
            7 => &mut self.instance_java_path,
            8 => &mut self.instance_launch_args,
            9 => &mut self.instance_memory_mb,
            10 => &mut self.instance_resolution_width,
            11 => &mut self.instance_resolution_height,
            12 => &mut self.instance_environment_json,
            13 => &mut self.instance_pre_launch_hook,
            14 => &mut self.instance_wrapper_hook,
            15 => &mut self.instance_post_exit_hook,
            _ => return Err("No instance setup field is selected.".into()),
        };
        let limit = match focus {
            7 => 2048,
            9 => 8,
            10 | 11 => 5,
            _ => 4096,
        };
        let value = if matches!(focus, 9..=11) {
            if !value.chars().all(|character| character.is_ascii_digit()) {
                return Err("This field accepts digits only.".into());
            }
            value
        } else {
            value.trim_end_matches(['\r', '\n'])
        };
        if target.len().saturating_add(value.len()) > limit {
            return Err("The pasted text is too long for this field.".into());
        }
        target.push_str(value);
        Ok(())
    }

    pub(super) fn finish_minecraft_login(&mut self) {
        let code = match extract_minecraft_auth_code(&self.login_code_input) {
            Ok(code) => code,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(session) = self.login_flow.take() else {
            self.message =
                "Start Microsoft sign-in before submitting a code.".into();
            return;
        };
        self.login_code_input.clear();
        self.input_focus = 0;
        self.message = "Completing Microsoft sign-in…".into();
        self.runtime.spawn(async move {
            let result = backend
                .finish_minecraft_login(session, &code)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::LoginFinished(result));
        });
    }
}
