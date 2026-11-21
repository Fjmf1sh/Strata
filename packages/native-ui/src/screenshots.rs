use super::*;

impl Launcher {
    pub(super) fn open_instance_screenshots(&mut self, instance_id: String) {
        self.global_browse = false;
        self.global_screenshots = false;
        self.instance_tab = 8;
        self.screenshots.clear();
        self.screenshot_offset = 0;
        self.selected_screenshot = None;
        self.selected_screenshot_ids.clear();
        self.screenshot_viewer = false;
        self.screenshot_preview = None;
        self.screenshot_delete_confirmation = false;
        self.screenshot_group_picker_open = false;
        self.screenshot_search_query.clear();
        self.load_screenshot_groups();
        self.input_focus = 0;
        self.message = "Loading instance screenshots…".into();
        self.load_screenshots(Some(instance_id));
    }

    pub(super) fn load_screenshots(&mut self, instance_id: Option<String>) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.screenshot_thumbnails.clear();
        self.screenshot_thumbnails_pending.clear();
        self.screenshot_thumbnail_failures.clear();
        let event_id = instance_id
            .clone()
            .unwrap_or_else(|| GLOBAL_SCREENSHOTS_ID.to_owned());
        self.runtime.spawn(async move {
            let result = backend
                .screenshots(instance_id.as_deref())
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::ScreenshotsLoaded(event_id, result));
        });
    }

    pub(super) fn load_screenshot_groups(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let groups = backend
                .screenshot_groups()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ScreenshotGroupsLoaded(groups));
        });
    }

    pub(super) fn create_screenshot_group(&mut self) {
        if self.screenshot_group_action_pending {
            return;
        }
        let existing_names = self
            .screenshot_groups
            .iter()
            .map(|group| group.name.to_lowercase())
            .collect::<std::collections::HashSet<_>>();
        let mut number = self.screenshot_groups.len() + 1;
        let name = loop {
            let candidate = format!("Group {number}");
            if !existing_names.contains(&candidate.to_lowercase()) {
                break candidate;
            }
            number += 1;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.screenshot_group_action_pending = true;
        self.message = format!("Creating {name}…");
        self.runtime.spawn(async move {
            let result = backend
                .create_screenshot_group(name, Vec::new())
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ScreenshotGroupCreated(result));
        });
    }

    pub(super) fn begin_rename_selected_screenshot_group(&mut self) {
        let group_id = self
            .selected_screenshot
            .and_then(|index| self.screenshots.get(index))
            .and_then(|screenshot| screenshot.group_id.clone());
        let Some(group_id) = group_id else {
            self.message =
                "Select a screenshot in a custom group first.".into();
            return;
        };
        let Some(group) = self
            .screenshot_groups
            .iter()
            .find(|group| group.id == group_id)
        else {
            self.message =
                "That screenshot group is no longer available.".into();
            return;
        };
        self.screenshot_group_rename_id = Some(group_id);
        self.screenshot_group_name_input.clone_from(&group.name);
        self.input_focus = 18;
    }

    pub(super) fn save_screenshot_group_rename(&mut self) {
        if self.screenshot_group_action_pending {
            return;
        }
        let Some(group_id) = self.screenshot_group_rename_id.clone() else {
            self.input_focus = 0;
            return;
        };
        let name = self.screenshot_group_name_input.trim().to_owned();
        if name.is_empty() {
            self.message = "Group name cannot be empty.".into();
            return;
        }
        if name.chars().count() > 64 {
            self.message = "Group names must be 64 characters or fewer.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.screenshot_group_action_pending = true;
        self.message = "Renaming screenshot group…".into();
        self.runtime.spawn(async move {
            let result = backend
                .rename_screenshot_group(group_id, name)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ScreenshotGroupRenamed(result));
        });
    }

    pub(super) fn delete_selected_screenshot_group(&mut self) {
        if self.screenshot_group_action_pending {
            return;
        }
        let Some(group_id) = self.screenshot_group_delete_confirmation.clone()
        else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.screenshot_group_action_pending = true;
        self.message = "Deleting screenshot group…".into();
        self.runtime.spawn(async move {
            let result = backend
                .delete_screenshot_group(group_id.clone())
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ScreenshotGroupDeleted(
                group_id, result,
            ));
        });
    }

    pub(super) fn assign_selected_screenshot_group(
        &mut self,
        group_id: Option<String>,
    ) {
        if self.screenshot_group_action_pending {
            return;
        }
        let indices = self.selected_screenshot_indices();
        if indices.is_empty() {
            self.message = "Select a screenshot first.".into();
            return;
        }
        let updates = indices
            .into_iter()
            .filter_map(|index| {
                self.screenshots.get(index).and_then(|screenshot| {
                    (screenshot.group_id != group_id)
                        .then(|| (screenshot.id.clone(), group_id.clone()))
                })
            })
            .collect::<Vec<_>>();
        if updates.is_empty() {
            self.message = "Screenshot is already in that group.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.screenshot_group_action_pending = true;
        self.message = format!(
            "Updating {} screenshot{}…",
            updates.len(),
            if updates.len() == 1 { "" } else { "s" }
        );
        self.runtime.spawn(async move {
            let result = backend
                .set_screenshot_group_memberships(updates.clone())
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ScreenshotMembershipsUpdated(
                updates, result,
            ));
        });
    }

    pub(super) fn selected_screenshot_indices(&self) -> Vec<usize> {
        let mut indices = self
            .screenshots
            .iter()
            .enumerate()
            .filter_map(|(index, screenshot)| {
                self.selected_screenshot_ids
                    .contains(&screenshot.id)
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        if indices.is_empty() {
            if let Some(index) = self
                .selected_screenshot
                .filter(|index| *index < self.screenshots.len())
            {
                indices.push(index);
            }
        }
        indices
    }

    pub(super) fn select_screenshot(
        &mut self,
        index: usize,
        extend_selection: bool,
    ) {
        let Some(screenshot) = self.screenshots.get(index) else {
            return;
        };
        let id = screenshot.id.clone();
        if extend_selection {
            if !self.selected_screenshot_ids.insert(id.clone()) {
                self.selected_screenshot_ids.remove(&id);
                if self.selected_screenshot == Some(index) {
                    self.selected_screenshot = self
                        .screenshots
                        .iter()
                        .enumerate()
                        .find(|(_, screenshot)| {
                            self.selected_screenshot_ids
                                .contains(&screenshot.id)
                        })
                        .map(|(index, _)| index);
                }
            } else {
                self.selected_screenshot = Some(index);
            }
        } else {
            self.selected_screenshot_ids.clear();
            self.selected_screenshot_ids.insert(id);
            self.selected_screenshot = Some(index);
        }
    }

    pub(super) fn ensure_visible_screenshot_thumbnails(
        &mut self,
        order: &[usize],
        width: usize,
        height: usize,
    ) {
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let (columns, _, _, card_height) = screenshot_grid_layout(width);
        let stride = card_height.saturating_add(12);
        let visible_rows = screenshot_visible_rows(height, stride, stride - 1);
        let explicit_scroll = (self.screenshot_offset
            != self.screenshot_scroll_synced)
            .then_some((self.screenshot_offset / columns * stride) as f64);
        let current = explicit_scroll.unwrap_or(self.screenshot_scroll_pixels);
        let target =
            explicit_scroll.unwrap_or(self.screenshot_scroll_target_pixels);
        let ranges = screenshot_prefetch_ranges(
            current,
            target,
            stride,
            columns,
            visible_rows,
            order.len(),
        );
        let mut pending = Vec::new();
        for index in ranges.into_iter().flat_map(|range| range) {
            let Some(screenshot) = self.screenshots.get(order[index]) else {
                continue;
            };
            if !self.screenshot_thumbnails.contains_key(&screenshot.id)
                && !self.screenshot_thumbnail_failures.contains(&screenshot.id)
                && self
                    .screenshot_thumbnails_pending
                    .insert(screenshot.id.clone())
            {
                pending.push((screenshot.id.clone(), screenshot.path.clone()));
            }
        }
        if pending.is_empty() {
            return;
        }
        self.runtime.spawn(async move {
            let _ = spawn_image_decode(move || {
                for (id, path) in pending {
                    let result = decode_screenshot_thumbnail(path);
                    if proxy
                        .send_event(UiMessage::ScreenshotThumbnailLoaded(
                            id, result,
                        ))
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .await;
        });
    }

    pub(super) fn screenshot_order(&self) -> Vec<usize> {
        let mut order = (0..self.screenshots.len()).collect::<Vec<_>>();
        let by_sort = |left: &usize, right: &usize| {
            let left = &self.screenshots[*left];
            let right = &self.screenshots[*right];
            match self.screenshot_sort_mode % 4 {
                1 => left.created_at.cmp(&right.created_at),
                2 => left
                    .file_name
                    .to_lowercase()
                    .cmp(&right.file_name.to_lowercase()),
                3 => right
                    .file_name
                    .to_lowercase()
                    .cmp(&left.file_name.to_lowercase()),
                _ => right.created_at.cmp(&left.created_at),
            }
        };
        match self.screenshot_group_mode % 4 {
            0 => order.sort_by(|left, right| {
                self.screenshots[*right]
                    .created_at
                    .format("%Y-%m-%d")
                    .to_string()
                    .cmp(
                        &self.screenshots[*left]
                            .created_at
                            .format("%Y-%m-%d")
                            .to_string(),
                    )
                    .then_with(|| by_sort(left, right))
            }),
            1 => order.sort_by(|left, right| {
                self.screenshots[*left]
                    .instance_name
                    .to_lowercase()
                    .cmp(&self.screenshots[*right].instance_name.to_lowercase())
                    .then_with(|| by_sort(left, right))
            }),
            2 => order.sort_by(|left, right| {
                let group_order =
                    |screenshot: &theseus::instance::InstanceScreenshot| {
                        screenshot
                            .group_id
                            .as_ref()
                            .and_then(|id| {
                                self.screenshot_groups
                                    .iter()
                                    .position(|group| &group.id == id)
                            })
                            .unwrap_or(self.screenshot_groups.len())
                    };
                group_order(&self.screenshots[*left])
                    .cmp(&group_order(&self.screenshots[*right]))
                    .then_with(|| by_sort(left, right))
            }),
            _ => order.sort_by(by_sort),
        }
        let query = self.screenshot_search_query.trim().to_lowercase();
        if !query.is_empty() {
            order.retain(|index| {
                let screenshot = &self.screenshots[*index];
                let group_name = screenshot
                    .group_id
                    .as_ref()
                    .and_then(|id| {
                        self.screenshot_groups
                            .iter()
                            .find(|group| &group.id == id)
                    })
                    .map(|group| group.name.as_str())
                    .unwrap_or_default();
                screenshot.file_name.to_lowercase().contains(&query)
                    || screenshot.instance_name.to_lowercase().contains(&query)
                    || screenshot
                        .created_at
                        .format("%Y-%m-%d %H:%M")
                        .to_string()
                        .contains(&query)
                    || group_name.to_lowercase().contains(&query)
            });
        }
        order
    }

    pub(super) fn delete_selected_screenshot(&mut self) {
        let screenshots = self
            .selected_screenshot_indices()
            .into_iter()
            .filter_map(|index| self.screenshots.get(index).cloned())
            .collect::<Vec<_>>();
        if screenshots.is_empty() {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let count = screenshots.len();
        let ids = screenshots
            .iter()
            .map(|screenshot| screenshot.id.clone())
            .collect::<Vec<_>>();
        let keys = screenshots
            .into_iter()
            .map(|screenshot| theseus::instance::ScreenshotKey {
                instance_id: screenshot.instance_id,
                file_name: screenshot.file_name,
            })
            .collect();
        self.message = format!("Deleting {count} screenshot(s)…");
        self.runtime.spawn(async move {
            let result = backend
                .delete_screenshots(keys)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::ScreenshotBatchDeleted(ids, result));
        });
    }

    pub(super) fn show_screenshot_preview(
        &mut self,
        screenshot: theseus::instance::InstanceScreenshot,
    ) {
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.screenshot_viewer = true;
        self.screenshot_preview = None;
        self.message = format!("Opening {}…", screenshot.file_name);
        let screenshot_id = screenshot.id;
        let path = screenshot.path;
        self.runtime.spawn(async move {
            let result = spawn_image_decode(move || {
                let mut reader = image::ImageReader::open(path)
                    .map_err(|error| error.to_string())?
                    .with_guessed_format()
                    .map_err(|error| error.to_string())?;
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(16_384);
                limits.max_image_height = Some(16_384);
                limits.max_alloc = Some(16 * 1024 * 1024);
                reader.limits(limits);
                let decoded =
                    reader.decode().map_err(|error| error.to_string())?;
                let preview = decoded.thumbnail(1280, 960).to_rgba8();
                let (width, height) = preview.dimensions();
                let pixels = preview
                    .pixels()
                    .map(|pixel| {
                        let [red, green, blue, alpha] = pixel.0;
                        let blend = |foreground: u8, background: u8| {
                            (u16::from(foreground) * u16::from(alpha)
                                + u16::from(background)
                                    * (255 - u16::from(alpha)))
                                / 255
                        };
                        (u32::from(blend(red, 27)) << 16)
                            | (u32::from(blend(green, 29)) << 8)
                            | u32::from(blend(blue, 34))
                    })
                    .collect();
                Ok::<_, String>((width, height, pixels))
            })
            .await
            .and_then(|result| result);
            let _ = proxy.send_event(UiMessage::ScreenshotPreviewLoaded(
                screenshot_id,
                result,
            ));
        });
    }

    pub(super) fn copy_screenshot(
        &mut self,
        screenshot: theseus::instance::InstanceScreenshot,
    ) {
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.message = format!("Copying {}…", screenshot.file_name);
        self.runtime.spawn(async move {
            let result = spawn_image_decode(move || {
                copy_screenshot_image(&screenshot.path)
            })
            .await
            .and_then(|result| result);
            let _ = proxy.send_event(UiMessage::ScreenshotCopied(result));
        });
    }

    pub(super) fn export_screenshot(
        &mut self,
        screenshot: theseus::instance::InstanceScreenshot,
    ) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.message =
            format!("Choose where to export {}…", screenshot.file_name);
        self.runtime.spawn(async move {
            let result = if let Some(file) = rfd::AsyncFileDialog::new()
                .set_file_name(format!("{}.zip", screenshot.file_name))
                .save_file()
                .await
            {
                let path = file.path().to_path_buf();
                backend
                    .export_screenshot(
                        theseus::instance::ScreenshotKey {
                            instance_id: screenshot.instance_id,
                            file_name: screenshot.file_name,
                        },
                        path.clone(),
                    )
                    .await
                    .map(|()| Some(path))
                    .map_err(|error| error.to_string())
            } else {
                Ok(None)
            };
            let _ = proxy.send_event(UiMessage::ScreenshotExported(result));
        });
    }

    pub(super) fn export_selected_screenshots(&mut self) {
        let screenshots = self
            .selected_screenshot_indices()
            .into_iter()
            .filter_map(|index| self.screenshots.get(index))
            .map(|screenshot| theseus::instance::ScreenshotKey {
                instance_id: screenshot.instance_id.clone(),
                file_name: screenshot.file_name.clone(),
            })
            .collect::<Vec<_>>();
        if screenshots.is_empty() {
            self.message = "Select screenshots to export first.".into();
            return;
        }
        let count = screenshots.len();
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.message = format!("Choose where to export {count} screenshots…");
        self.runtime.spawn(async move {
            let result = if let Some(file) = rfd::AsyncFileDialog::new()
                .set_file_name("screenshots.zip")
                .save_file()
                .await
            {
                let path = file.path().to_path_buf();
                backend
                    .export_screenshots(screenshots, path.clone())
                    .await
                    .map(|()| Some(path))
                    .map_err(|error| error.to_string())
            } else {
                Ok(None)
            };
            let _ =
                proxy.send_event(UiMessage::ScreenshotBatchExported(result));
        });
    }
}
