use super::*;

pub(super) fn draw_settings_panel(
    buffer: &mut [u32],
    w: usize,
    layout_width: usize,
    h: usize,
    settings: Option<&theseus::data::Settings>,
    maximum_memory_mb: Option<u32>,
    global_screenshot_sync: Option<bool>,
    quick_instance_count: usize,
    native_color_theme: u8,
    theme_sync_across_devices: bool,
    theme_sync_saving: bool,
    system_theme_palette: u8,
    quick_instance_count_focused: bool,
    default_window_width_input: &str,
    default_window_height_input: &str,
    default_java_arguments_input: &str,
    default_environment_variables_input: &str,
    default_pre_launch_hook_input: &str,
    default_wrapper_hook_input: &str,
    default_post_exit_hook_input: &str,
    default_launch_settings_saving: bool,
    input_focus: u8,
    settings_section: usize,
    message: &str,
    settings_scroll_pixels: f64,
) {
    rect(
        buffer,
        w,
        h,
        248,
        56,
        w.saturating_sub(248),
        h.saturating_sub(56),
        0x001b1d22,
    );
    text(buffer, w, h, 292, 95, "SETTINGS", 0x00f1f3f5, 2);
    let close_button = settings_close_button(layout_width);
    rect(
        buffer,
        w,
        h,
        close_button.x,
        close_button.y,
        close_button.width,
        close_button.height,
        0x0022262c,
    );
    text(
        buffer,
        w,
        h,
        layout_width.saturating_sub(128),
        94,
        "CLOSE",
        0x00a8e66b,
        2,
    );
    match settings {
        None => text_clipped(
            buffer,
            w,
            h,
            292,
            148,
            message,
            0x00bdc2ca,
            1,
            layout_width.saturating_sub(316),
        ),
        Some(settings) => {
            let section =
                settings_section.min(SETTINGS_SECTION_ROW_INDICES.len() - 1);
            let section_rows = SETTINGS_SECTION_ROW_INDICES[section];
            let controls = settings_row_layout(layout_width);
            let settings_row_width = controls.row_width;
            text(
                buffer,
                w,
                h,
                500,
                95,
                SETTINGS_SECTION_NAMES[section],
                0x00f1f3f5,
                2,
            );
            if section == 0 {
                text(buffer, w, h, 500, 126, "COLOR THEME", 0x007b838d, 1);
                text_clipped(
                    buffer,
                    w,
                    h,
                    500,
                    137,
                    "Select your preferred color theme across Northstar.",
                    0x007b838d,
                    1,
                    layout_width.saturating_sub(516),
                );
            }
            for (y, group) in SETTINGS_SECTION_NAV_GROUPS {
                text(buffer, w, h, 296, y, group, 0x007b838d, 1);
            }
            for (nav_index, name) in SETTINGS_SECTION_NAMES.iter().enumerate() {
                let y = SETTINGS_SECTION_NAV_TOPS[nav_index];
                rounded_rect(
                    buffer,
                    w,
                    h,
                    292,
                    y,
                    184,
                    32,
                    8,
                    if nav_index == section {
                        0x00303b2b
                    } else {
                        0x0022262c
                    },
                );
                text(
                    buffer,
                    w,
                    h,
                    306,
                    y + 9,
                    name,
                    if nav_index == section {
                        0x00a8e66b
                    } else {
                        0x00bdc2ca
                    },
                    1,
                );
            }
            let all_rows = [
                (
                    "MAX PARALLEL DOWNLOADS",
                    settings.max_concurrent_downloads.to_string(),
                ),
                (
                    "MAX PARALLEL FILE WRITES",
                    settings.max_concurrent_writes.to_string(),
                ),
                (
                    "USAGE TELEMETRY",
                    if settings.telemetry {
                        "ON".into()
                    } else {
                        "OFF".into()
                    },
                ),
                (
                    "DISCORD RICH PRESENCE",
                    if settings.discord_rpc {
                        "ON".into()
                    } else {
                        "OFF".into()
                    },
                ),
                (
                    "FORCE FULLSCREEN",
                    if settings.force_fullscreen {
                        "ON".into()
                    } else {
                        "OFF".into()
                    },
                ),
                ("APPEARANCE THEME", theme_label(settings.theme).into()),
                (
                    "INSTANCE FILES TAB",
                    if settings.show_files_tab_in_instances {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "INSTANCE WORLDS TAB",
                    if settings.show_worlds_tab_in_instances {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "INSTANCE SCREENSHOTS TAB",
                    if settings.show_screenshots_tab_in_instances {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "SKIN SELECTOR SIDEBAR",
                    if settings.show_skin_selector_in_sidebar {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "SHOW PLAY TIME",
                    if settings.show_play_time { "ON" } else { "OFF" }.into(),
                ),
                (
                    "COMPACT INSTANCE CARDS",
                    if settings.compact_instance_cards {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "SHOW JUMP IN",
                    if settings.show_jump_in { "ON" } else { "OFF" }.into(),
                ),
                (
                    "HIDE RIGHT SIDEBAR",
                    if settings.toggle_sidebar { "ON" } else { "OFF" }.into(),
                ),
                (
                    "HIDE WHEN GAME STARTS",
                    if settings.hide_on_process_start {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "REFOCUS AFTER GAME CLOSES",
                    if settings.refocus_on_game_close {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "WARN ON UNKNOWN MODPACKS",
                    if settings.warn_on_unknown_modpacks {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "SKIP NON-ESSENTIAL WARNINGS",
                    if settings.skip_non_essential_warnings {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "PERSONALIZED ADS",
                    if settings.personalized_ads {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "DEFAULT MAX MEMORY",
                    format!("{} MB", settings.memory.maximum),
                ),
                (
                    "HIDE INSTALLED MODPACKS",
                    if settings.hide_installed_modpacks {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                (
                    "SYNC SCREENSHOTS BETWEEN INSTANCES",
                    global_screenshot_sync.map_or_else(
                        || "LOADING".into(),
                        |enabled| if enabled { "ON" } else { "OFF" }.into(),
                    ),
                ),
                (
                    "HIDE SKIN SELECTOR USERNAME",
                    if settings.hide_nametag_skins_page {
                        "ON"
                    } else {
                        "OFF"
                    }
                    .into(),
                ),
                ("COPY MODRINTH INSTANCES", "IMPORT".into()),
                ("JUMP IN ITEMS", format!("{quick_instance_count} ITEMS")),
                ("WINDOW WIDTH", settings.game_resolution.0.to_string()),
                ("WINDOW HEIGHT", settings.game_resolution.1.to_string()),
                ("JAVA ARGUMENTS", settings.extra_launch_args.join(" ")),
                (
                    "ENVIRONMENT VARS",
                    settings
                        .custom_env_vars
                        .iter()
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                (
                    "NORTHSTAR COLORWAY",
                    native_color_theme_label(native_color_theme).into(),
                ),
                (
                    "PRE-LAUNCH HOOK",
                    settings
                        .hooks
                        .pre_launch
                        .as_deref()
                        .unwrap_or_default()
                        .into(),
                ),
                (
                    "WRAPPER HOOK",
                    settings
                        .hooks
                        .wrapper
                        .as_deref()
                        .unwrap_or_default()
                        .into(),
                ),
                (
                    "POST-EXIT HOOK",
                    settings
                        .hooks
                        .post_exit
                        .as_deref()
                        .unwrap_or_default()
                        .into(),
                ),
                (
                    "SYNC THEME ACROSS DEVICES",
                    if theme_sync_saving {
                        "SAVING".into()
                    } else if theme_sync_across_devices {
                        "ON".into()
                    } else {
                        "OFF".into()
                    },
                ),
                (
                    "NATIVE WINDOW DECORATIONS",
                    if cfg!(target_os = "macos") {
                        "SYSTEM-MANAGED".into()
                    } else if settings.native_decorations {
                        "ON".into()
                    } else {
                        "OFF".into()
                    },
                ),
            ];
            for (position, setting_index) in section_rows.iter().enumerate() {
                let index = *setting_index;
                let y = settings_row_y(
                    section,
                    position,
                    settings_row_width,
                    settings_scroll_pixels,
                );
                let row_height = if index == 5 {
                    settings_theme_selector_extra_height(settings_row_width)
                        + 50
                } else {
                    40
                };
                if y.saturating_add(row_height) <= 146
                    || y >= h.saturating_sub(56)
                {
                    continue;
                }
                if index == 5 {
                    draw_theme_selector_cards(
                        buffer,
                        w,
                        h,
                        settings_row_width,
                        y,
                        settings.theme,
                        system_theme_palette,
                    );
                    continue;
                }
                let (label, value) = &all_rows[index];
                let row_text_visible =
                    y >= 146 && y.saturating_add(40) <= h.saturating_sub(56);
                rect(buffer, w, h, 500, y, settings_row_width, 40, 0x0022262c);
                if index == 24 && quick_instance_count_focused {
                    let row_width = settings_row_width;
                    rect(buffer, w, h, 500, y, row_width, 2, 0x00a8e66b);
                    rect(buffer, w, h, 500, y + 38, row_width, 2, 0x00a8e66b);
                    rect(buffer, w, h, 500, y, 2, 40, 0x00a8e66b);
                    rect(buffer, w, h, 498 + row_width, y, 2, 40, 0x00a8e66b);
                }
                if row_text_visible {
                    let control_x = if index < 2 || index == 19 || index == 24 {
                        controls.decrement_x
                    } else if index == 25 || index == 26 {
                        controls.dimension_x
                    } else if matches!(index, 27 | 28 | 30 | 31 | 32) {
                        controls.launch_field_x.saturating_sub(28)
                    } else {
                        controls.toggle_x
                    };
                    text_clipped(
                        buffer,
                        w,
                        h,
                        514,
                        y + 5,
                        label,
                        0x00bdc2ca,
                        if matches!(index, 27 | 28 | 30 | 31 | 32) {
                            1
                        } else {
                            2
                        },
                        control_x.saturating_sub(526),
                    );
                }
                if index < 2 || index == 19 || index == 24 {
                    rect(
                        buffer,
                        w,
                        h,
                        controls.decrement_x,
                        y + 5,
                        40,
                        30,
                        0x00303b2b,
                    );
                    if row_text_visible {
                        text(
                            buffer,
                            w,
                            h,
                            controls.decrement_x + 15,
                            y + 15,
                            "-",
                            0x00a8e66b,
                            2,
                        );
                    }
                    let value = if index == 19 {
                        maximum_memory_mb.map_or_else(
                            || "CHECKING".into(),
                            |_| {
                                format!(
                                    "{:.1} GB",
                                    settings.memory.maximum as f32 / 1024.0
                                )
                            },
                        )
                    } else if index == 24 {
                        format!("{quick_instance_count} ITEMS")
                    } else {
                        value.to_owned()
                    };
                    if row_text_visible {
                        text_clipped(
                            buffer,
                            w,
                            h,
                            controls.value_x,
                            y + 15,
                            &value,
                            0x00f1f3f5,
                            1,
                            controls
                                .increment_x
                                .saturating_sub(controls.value_x),
                        );
                    }
                    rect(
                        buffer,
                        w,
                        h,
                        controls.increment_x,
                        y + 5,
                        40,
                        30,
                        0x00303b2b,
                    );
                    if row_text_visible {
                        text(
                            buffer,
                            w,
                            h,
                            controls.increment_x + 15,
                            y + 15,
                            "+",
                            0x00a8e66b,
                            2,
                        );
                    }
                } else if index == 25 || index == 26 {
                    rect(
                        buffer,
                        w,
                        h,
                        controls.dimension_x,
                        y + 5,
                        104,
                        30,
                        if settings.force_fullscreen {
                            0x0022262c
                        } else {
                            0x001a1d21
                        },
                    );
                    let value = if index == 25 {
                        if input_focus == 26 {
                            default_window_width_input
                        } else {
                            value
                        }
                    } else if input_focus == 27 {
                        default_window_height_input
                    } else {
                        value
                    };
                    let focused =
                        input_focus == if index == 25 { 26 } else { 27 };
                    if row_text_visible {
                        let color = if settings.force_fullscreen {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        };
                        if focused {
                            text_input_clipped(
                                buffer,
                                w,
                                h,
                                controls.dimension_x + 10,
                                y + 15,
                                value,
                                color,
                                1,
                                84,
                            );
                        } else {
                            text_clipped(
                                buffer,
                                w,
                                h,
                                controls.dimension_x + 10,
                                y + 15,
                                value,
                                color,
                                1,
                                84,
                            );
                        }
                    }
                    if !settings.force_fullscreen
                        && input_focus == if index == 25 { 26 } else { 27 }
                    {
                        rect(
                            buffer,
                            w,
                            h,
                            controls.dimension_x,
                            y + 5,
                            104,
                            2,
                            0x00a8e66b,
                        );
                        rect(
                            buffer,
                            w,
                            h,
                            controls.dimension_x,
                            y + 33,
                            104,
                            2,
                            0x00a8e66b,
                        );
                        rect(
                            buffer,
                            w,
                            h,
                            controls.dimension_x,
                            y + 5,
                            2,
                            30,
                            0x00a8e66b,
                        );
                        rect(
                            buffer,
                            w,
                            h,
                            controls.dimension_x + 102,
                            y + 5,
                            2,
                            30,
                            0x00a8e66b,
                        );
                    }
                } else if matches!(index, 27 | 28 | 30 | 31 | 32) {
                    let field_width = controls.launch_field_width;
                    let field_x = controls.launch_field_x;
                    let focus_id = match index {
                        27 => 30,
                        28 => 31,
                        30 => 32,
                        31 => 33,
                        _ => 34,
                    };
                    let focused = input_focus == focus_id;
                    let value = if focused {
                        match index {
                            27 => default_java_arguments_input,
                            28 => default_environment_variables_input,
                            30 => default_pre_launch_hook_input,
                            31 => default_wrapper_hook_input,
                            _ => default_post_exit_hook_input,
                        }
                    } else {
                        value
                    };
                    rect(
                        buffer,
                        w,
                        h,
                        field_x,
                        y + 5,
                        field_width,
                        30,
                        if default_launch_settings_saving {
                            0x0022262c
                        } else {
                            0x001a1d21
                        },
                    );
                    if row_text_visible {
                        let color = if default_launch_settings_saving {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        };
                        if focused {
                            text_input_clipped(
                                buffer,
                                w,
                                h,
                                field_x + 8,
                                y + 15,
                                value,
                                color,
                                1,
                                field_width.saturating_sub(16),
                            );
                        } else {
                            text_clipped(
                                buffer,
                                w,
                                h,
                                field_x + 8,
                                y + 15,
                                value,
                                color,
                                1,
                                field_width.saturating_sub(16),
                            );
                        }
                    }
                    if focused {
                        rect(
                            buffer,
                            w,
                            h,
                            field_x,
                            y + 5,
                            field_width,
                            2,
                            0x00a8e66b,
                        );
                        rect(
                            buffer,
                            w,
                            h,
                            field_x,
                            y + 33,
                            field_width,
                            2,
                            0x00a8e66b,
                        );
                        rect(buffer, w, h, field_x, y + 5, 2, 30, 0x00a8e66b);
                        rect(
                            buffer,
                            w,
                            h,
                            field_x + field_width.saturating_sub(2),
                            y + 5,
                            2,
                            30,
                            0x00a8e66b,
                        );
                    }
                } else {
                    rect(
                        buffer,
                        w,
                        h,
                        controls.toggle_x,
                        y + 8,
                        76,
                        30,
                        0x00303b2b,
                    );
                    if row_text_visible {
                        text_clipped(
                            buffer,
                            w,
                            h,
                            controls.toggle_x + 18,
                            y + 18,
                            value,
                            0x00a8e66b,
                            2,
                            58,
                        );
                    }
                }
            }
            // Keep the scrolling rows inside their viewport and restore the
            // fixed header/footer after partially translated cards pass them.
            rect(
                buffer,
                w,
                h,
                248,
                56,
                layout_width.saturating_sub(248),
                90,
                0x001b1d22,
            );
            text(buffer, w, h, 292, 95, "SETTINGS", 0x00f1f3f5, 2);
            rect(
                buffer,
                w,
                h,
                close_button.x,
                close_button.y,
                close_button.width,
                close_button.height,
                0x0022262c,
            );
            text(
                buffer,
                w,
                h,
                layout_width.saturating_sub(128),
                94,
                "CLOSE",
                0x00a8e66b,
                2,
            );
            let footer_y = h.saturating_sub(56);
            rect(
                buffer,
                w,
                h,
                248,
                footer_y,
                layout_width.saturating_sub(248),
                56,
                0x001b1d22,
            );
            text_clipped(
                buffer,
                w,
                h,
                292,
                132,
                message,
                0x007b838d,
                1,
                layout_width.saturating_sub(316),
            );
            let runtimes_y = h.saturating_sub(56);
            rect(buffer, w, h, 292, runtimes_y, 210, 36, 0x00303b2b);
            text(
                buffer,
                w,
                h,
                305,
                runtimes_y + 12,
                "JAVA RUNTIMES",
                0x00a8e66b,
                2,
            );
        }
    }
}

fn draw_theme_selector_cards(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    row_width: usize,
    y: usize,
    selected: theseus::data::Theme,
    system_theme_palette: u8,
) {
    let active_palette = render::active_palette();
    let options = [
        (theseus::data::Theme::System, "SYSTEM", system_theme_palette),
        (theseus::data::Theme::Light, "LIGHT", 1),
        (theseus::data::Theme::Dark, "DARK", 0),
        (theseus::data::Theme::Oled, "OLED", 2),
        (theseus::data::Theme::Retro, "RETRO", 3),
    ];
    for ((theme, label, palette), bounds) in options
        .into_iter()
        .zip(settings_theme_option_bounds(row_width, y))
    {
        render::set_active_palette(palette);
        let is_selected = theme_label(theme) == theme_label(selected);
        rounded_rect(
            buffer,
            w,
            h,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            10,
            if is_selected { 0x00a8e66b } else { 0x00363b42 },
        );
        rounded_rect(
            buffer,
            w,
            h,
            bounds.x + 1,
            bounds.y + 1,
            bounds.width.saturating_sub(2),
            bounds.height.saturating_sub(2),
            9,
            0x0022262c,
        );
        let preview_x = bounds.x + 4;
        let preview_y = bounds.y + 4;
        let preview_width = bounds.width.saturating_sub(8);
        rounded_rect(
            buffer,
            w,
            h,
            preview_x,
            preview_y,
            preview_width,
            64,
            8,
            0x00363b42,
        );
        rounded_rect(
            buffer,
            w,
            h,
            preview_x + 1,
            preview_y + 1,
            preview_width.saturating_sub(2),
            62,
            7,
            0x0022262c,
        );
        rounded_rect(
            buffer,
            w,
            h,
            preview_x + 16,
            preview_y + 16,
            32,
            32,
            4,
            0x00a8e66b,
        );
        rounded_rect(
            buffer,
            w,
            h,
            preview_x + 56,
            preview_y + 20,
            preview_width.saturating_sub(72),
            8,
            4,
            0x00f1f3f5,
        );
        rounded_rect(
            buffer,
            w,
            h,
            preview_x + 56,
            preview_y + 36,
            preview_width.saturating_sub(72) * 3 / 5,
            8,
            4,
            0x007b838d,
        );
        let radio_x = bounds.x + 12;
        let radio_y = bounds.y + 84;
        rounded_rect(
            buffer,
            w,
            h,
            radio_x,
            radio_y,
            14,
            14,
            7,
            if is_selected { 0x00a8e66b } else { 0x007b838d },
        );
        rounded_rect(
            buffer,
            w,
            h,
            radio_x + 2,
            radio_y + 2,
            10,
            10,
            5,
            0x0022262c,
        );
        if is_selected {
            rounded_rect(
                buffer,
                w,
                h,
                radio_x + 5,
                radio_y + 5,
                4,
                4,
                2,
                0x00a8e66b,
            );
        }
        text_clipped(
            buffer,
            w,
            h,
            bounds.x + 34,
            bounds.y + 85,
            label,
            0x00f1f3f5,
            1,
            bounds.width.saturating_sub(42),
        );
    }
    render::set_active_palette(active_palette);
}
