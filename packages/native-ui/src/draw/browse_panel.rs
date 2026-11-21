use super::*;

fn draw_all_versions_toggle(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    bounds: UiBounds,
    enabled: bool,
) {
    rounded_rect(
        buffer,
        w,
        h,
        bounds.x + 12,
        bounds.y + 6,
        16,
        16,
        4,
        if enabled { 0x001bd96a } else { 0x00343a42 },
    );
    if enabled {
        text(
            buffer,
            w,
            h,
            bounds.x + 14,
            bounds.y + 7,
            "✓",
            0x001b1d22,
            1,
        );
    }
    text_clipped(
        buffer,
        w,
        h,
        bounds.x + 38,
        bounds.y + 7,
        "Show all versions",
        0x00f1f3f5,
        1,
        bounds.width.saturating_sub(46),
    );
}

pub(super) fn draw_filter_sidebar(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    cursor: (f64, f64),
    group: u8,
    group_count: usize,
    show_all_versions: bool,
    options: &[String],
    scroll: f64,
    selected: [&HashSet<String>; BROWSE_FILTER_GROUPS.len()],
    loading: bool,
    error: Option<&str>,
) {
    let layout =
        browse_filter_sidebar_layout(w, h, group, options.len(), group_count);
    let pane = layout.pane;
    let group = usize::from(group).min(group_count - 1);
    rect(
        buffer,
        w,
        h,
        pane.x,
        pane.y,
        pane.width,
        pane.height,
        0x001b1d22,
    );
    rect(buffer, w, h, pane.x, pane.y, 1, pane.height, 0x00343a42);
    text(buffer, w, h, pane.x + 16, 78, "Filters", 0x00f1f3f5, 1);
    if error.is_some() || selected.iter().any(|set| !set.is_empty()) {
        let button = layout.reset;
        let hovered = button.contains(cursor.0, cursor.1);
        rounded_rect(
            buffer,
            w,
            h,
            button.x,
            button.y,
            button.width,
            button.height,
            8,
            if hovered { 0x00343a42 } else { 0x0022262c },
        );
        text(
            buffer,
            w,
            h,
            button.x + 12,
            button.y + 10,
            if error.is_some() {
                "Retry"
            } else {
                "Clear all"
            },
            0x00f1f3f5,
            1,
        );
    }
    for (index, bounds) in layout.headings.iter().take(group_count).enumerate()
    {
        let active = group == index;
        let color = if bounds.contains(cursor.0, cursor.1) {
            0x00343a42
        } else {
            0x0022262c
        };
        rounded_rect(
            buffer,
            w,
            h,
            bounds.x,
            bounds.y + 2,
            bounds.width,
            bounds.height - 4,
            8,
            color,
        );
        let count = selected[index].len();
        let label = if count == 0 {
            BROWSE_FILTER_GROUPS[index].to_owned()
        } else {
            format!("{} ({count})", BROWSE_FILTER_GROUPS[index])
        };
        text(
            buffer,
            w,
            h,
            bounds.x + 12,
            bounds.y + 12,
            &label,
            if active { 0x00a8e66b } else { 0x00f1f3f5 },
            1,
        );
        text(
            buffer,
            w,
            h,
            bounds.x + bounds.width - 23,
            bounds.y + 12,
            if active { "▾" } else { "›" },
            0x00bdc2ca,
            1,
        );
    }
    if let Some(toggle) = layout.version_toggle {
        draw_all_versions_toggle(buffer, w, h, toggle, show_all_versions);
    }
    let viewport = layout.options;
    render::set_clip_rect(Some((
        viewport.x,
        viewport.y,
        viewport.width,
        viewport.height,
    )));
    if loading || error.is_some() || options.is_empty() {
        let label = if loading {
            "Loading filter options…"
        } else if error.is_some() {
            "Could not load filter options"
        } else {
            "No options for this project type"
        };
        text_clipped(
            buffer,
            w,
            h,
            viewport.x + 12,
            viewport.y + 12,
            label,
            0x00bdc2ca,
            1,
            viewport.width.saturating_sub(24),
        );
    } else {
        let scroll = scroll as usize;
        let row_height = BROWSE_FILTER_ROW_HEIGHT;
        let first = scroll / row_height;
        let remainder = scroll % row_height;
        let visible = (viewport.height + remainder).div_ceil(row_height);
        for (index, option) in
            options.iter().skip(first).take(visible).enumerate()
        {
            let y = viewport.y + index * row_height - remainder;
            let checked = selected[group].contains(option);
            if layout.option_at(cursor.0, cursor.1, scroll as f64)
                == Some(first + index)
            {
                rounded_rect(
                    buffer,
                    w,
                    h,
                    viewport.x,
                    y,
                    viewport.width,
                    row_height,
                    6,
                    0x0022262c,
                );
            }
            rounded_rect(
                buffer,
                w,
                h,
                viewport.x + 12,
                y + 6,
                16,
                16,
                4,
                if checked { 0x001bd96a } else { 0x00343a42 },
            );
            if checked {
                text(buffer, w, h, viewport.x + 14, y + 7, "✓", 0x001b1d22, 1);
            }
            let label = option.replace('_', " ").replace('-', " ");
            text_clipped(
                buffer,
                w,
                h,
                viewport.x + 38,
                y + 8,
                &label,
                0x00f1f3f5,
                1,
                viewport.width - 50,
            );
        }
    }
    render::set_clip_rect(None);
}

fn browse_search_placeholder(browse_filter: u8) -> &'static str {
    match browse_filter {
        1 => "Search resource packs...",
        2 => "Search datapacks...",
        3 => "Search shaders...",
        4 => "Search modpacks...",
        _ => "Search mods...",
    }
}

fn draw_browse_search_input(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    layout_width: usize,
    browse_filter: u8,
    browse_query: &str,
    browse_query_cursor: usize,
    browse_query_select_all: bool,
    focused: bool,
) {
    let layout = super::browse_search_layout(layout_width);
    if focused {
        rounded_rect(
            buffer,
            w,
            h,
            layout.query_x,
            182,
            layout.query_width,
            34,
            10,
            0x00678947,
        );
        rounded_rect(
            buffer,
            w,
            h,
            layout.query_x + 1,
            183,
            layout.query_width.saturating_sub(2),
            32,
            9,
            0x0022262c,
        );
    } else {
        rounded_rect(
            buffer,
            w,
            h,
            layout.query_x,
            182,
            layout.query_width,
            34,
            10,
            0x0022262c,
        );
    }
    render::draw_navigation_icon(
        buffer,
        w,
        h,
        layout.query_x + 20,
        199,
        1,
        0x007b838d,
    );
    let clear_search_visible =
        !browse_query.is_empty() && layout.query_width >= 44;
    let query_chars = browse_query.chars().collect::<Vec<_>>();
    let cursor = browse_query_cursor.min(query_chars.len());
    let (first_visible, text_area_width) =
        browse_query_viewport(layout.query_width, query_chars.len(), cursor);
    let visible_chars = (text_area_width / 8).max(1);
    let visible_query = query_chars
        .iter()
        .skip(first_visible)
        .take(visible_chars)
        .collect::<String>();
    if focused && browse_query_select_all && !browse_query.is_empty() {
        rounded_rect(
            buffer,
            w,
            h,
            layout.query_x + 34,
            186,
            (query_chars
                .len()
                .saturating_sub(first_visible)
                .saturating_mul(8))
            .min(text_area_width),
            23,
            4,
            0x00445e35,
        );
    }
    text_clipped(
        buffer,
        w,
        h,
        layout.query_x + 34,
        195,
        if browse_query.is_empty() {
            browse_search_placeholder(browse_filter)
        } else {
            &visible_query
        },
        if browse_query.is_empty() {
            0x007b838d
        } else {
            0x00f1f3f5
        },
        1,
        text_area_width,
    );
    if focused {
        let max_text_width = text_area_width;
        let caret_x = layout.query_x
            + 34
            + cursor
                .saturating_sub(first_visible)
                .saturating_mul(8)
                .min(max_text_width);
        rect(buffer, w, h, caret_x, 190, 1, 18, 0x00f1f3f5);
    }
    if clear_search_visible {
        text(
            buffer,
            w,
            h,
            layout.query_x + layout.query_width - 24,
            193,
            "×",
            0x00bdc2ca,
            1,
        );
    }
}

fn compact_count(value: u64) -> String {
    let (scaled, suffix) = if value >= 1_000_000 {
        (value as f64 / 1_000_000.0, "M")
    } else if value >= 1_000 {
        (value as f64 / 1_000.0, "K")
    } else {
        return value.to_string();
    };
    format!("{scaled:.1}{suffix}").replace(".0", "")
}

pub(super) fn draw_browse_panel(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    layout_width: usize,
    global_browse: bool,
    instance_filter_summary: Option<&str>,
    browse_filter: u8,
    browse_query: &str,
    browse_query_cursor: usize,
    browse_query_select_all: bool,
    browse_search_focused: bool,
    browse_selected_installed: bool,
    browse_contents: &[theseus::data::ContentItem],
    browse_installing_for_instance: Option<&HashSet<String>>,
    browse_installing_modpacks: &HashSet<String>,
    browse_sort: u8,
    browse_icons: &HashMap<String, (u32, u32, Vec<u32>)>,
    browse_icon_failures: &HashSet<String>,
    message: &str,
    browse_error: Option<&str>,
    browse_search_pending: bool,
    hide_installed: bool,
    browse_selected: Option<&BrowseProject>,
    browse_details: Option<&BrowseProjectDetails>,
    browse_gallery_index: usize,
    browse_gallery_viewer_open: bool,
    browse_detail_scroll_pixels: f64,
    browse_detail_tab: u8,
    browse_description_lines: &[String],
    browse_description_links: &[BrowseDescriptionLink],
    browse_description_styles: &[BrowseDescriptionStyle],
    browse_description_images: &[BrowseDescriptionImage],
    browse_results: &[BrowseProject],
    browse_scroll_pixels: f64,
    browse_search_offset: u32,
    browse_page_size: u32,
    browse_total_hits: u32,
    browse_filter_sidebar_visible: bool,
    browse_filter_menu_open: bool,
    browse_filter_options_loading: bool,
    browse_filter_group: u8,
    browse_filter_group_count: usize,
    browse_show_all_versions: bool,
    browse_filter_menu_options: &[String],
    browse_selected_game_versions: &HashSet<String>,
    browse_selected_loaders: &HashSet<String>,
    browse_selected_categories: &HashSet<String>,
    browse_selected_environments: &HashSet<String>,
    browse_selected_licenses: &HashSet<String>,
    browse_detail_selected_channels: &HashSet<String>,
    browse_detail_selected_game_versions: &HashSet<String>,
    browse_detail_selected_loaders: &HashSet<String>,
    browse_detail_filter_menu_group: Option<u8>,
    browse_detail_filter_option_offset: usize,
    browse_project_menu_open: bool,
    browse_project_followed: bool,
    browse_project_follow_pending: bool,
    browse_project_follow_status_loading: bool,
    browse_save_menu_open: bool,
    browse_save_collections: &[native_backend::ContentServiceCollection],
    browse_save_collections_loading: bool,
    browse_save_collection_pending: &HashSet<String>,
    browse_save_scroll_pixels: f64,
    browse_filter_scroll_pixels: f64,
    browse_view_menu_open: bool,
    browse_sort_menu_open: bool,
) {
    let tabs = super::browse_project_type_tabs(layout_width);
    let (_, last_x, last_width, _) = tabs[4];
    rounded_rect(
        buffer,
        w,
        h,
        292,
        146,
        last_x + last_width - 292,
        32,
        16,
        0x0022262c,
    );
    for (label, x, width, project_filter) in tabs {
        rounded_rect(
            buffer,
            w,
            h,
            x,
            148,
            width,
            28,
            14,
            if browse_filter == project_filter {
                0x00303b2b
            } else {
                0x0022262c
            },
        );
        text_clipped(
            buffer,
            w,
            h,
            x + 6,
            157,
            label,
            if browse_filter == project_filter {
                0x00a8e66b
            } else {
                0x00bdc2ca
            },
            1,
            width.saturating_sub(12),
        );
    }
    let selected_filter_count = browse_selected_game_versions.len()
        + browse_selected_loaders.len()
        + browse_selected_categories.len()
        + browse_selected_environments.len()
        + browse_selected_licenses.len();
    let search_layout = super::browse_search_layout(layout_width);
    let (filter_x, filter_width) =
        super::browse_filter_button_layout(layout_width);
    if !browse_filter_sidebar_visible {
        rounded_rect(
            buffer,
            w,
            h,
            filter_x,
            146,
            filter_width,
            32,
            16,
            if browse_filter_menu_open || selected_filter_count > 0 {
                0x00303b2b
            } else {
                0x0022262c
            },
        );
        text_clipped(
            buffer,
            w,
            h,
            filter_x + 8,
            157,
            &if selected_filter_count == 0 {
                "Filters".to_owned()
            } else {
                format!("Filters ({selected_filter_count})")
            },
            if browse_filter_menu_open || selected_filter_count > 0 {
                0x00a8e66b
            } else {
                0x00bdc2ca
            },
            1,
            filter_width.saturating_sub(16),
        );
    }
    draw_browse_search_input(
        buffer,
        w,
        h,
        layout_width,
        browse_filter,
        browse_query,
        browse_query_cursor,
        browse_query_select_all,
        browse_search_focused,
    );
    let sort_labels =
        ["Relevance", "Downloads", "Followers", "Newest", "Updated"];
    rounded_rect(
        buffer,
        w,
        h,
        search_layout.sort_x,
        218,
        search_layout.sort_width,
        28,
        8,
        0x0022262c,
    );
    text(
        buffer,
        w,
        h,
        search_layout.sort_x + 8,
        228,
        "Sort by",
        0x007b838d,
        1,
    );
    text_clipped(
        buffer,
        w,
        h,
        search_layout.sort_x + 54,
        228,
        sort_labels[browse_sort.min(4) as usize],
        0x00f1f3f5,
        1,
        search_layout.sort_width.saturating_sub(60),
    );
    let (view_x, view_width) = super::browse_view_layout(layout_width);
    rounded_rect(buffer, w, h, view_x, 218, view_width, 28, 8, 0x0022262c);
    if view_width >= 54 {
        text(buffer, w, h, view_x + 8, 228, "View", 0x007b838d, 1);
    }
    text_clipped(
        buffer,
        w,
        h,
        view_x + 48,
        228,
        &browse_page_size.to_string(),
        0x00f1f3f5,
        1,
        view_width.saturating_sub(54),
    );
    if browse_selected.is_none()
        && let Some(summary) = instance_filter_summary
    {
        let label_x = view_x + view_width + 12;
        let right =
            super::browse_hide_installed_bounds(layout_width, browse_filter)
                .map_or(layout_width.saturating_sub(20), |bounds| {
                    bounds.x.saturating_sub(8)
                });
        let label_width = right.saturating_sub(label_x);
        if label_width >= 100 {
            text_clipped(
                buffer,
                w,
                h,
                label_x,
                228,
                summary,
                0x0096a2b0,
                1,
                label_width,
            );
        }
    }
    if let Some(bounds) =
        super::browse_hide_installed_bounds(layout_width, browse_filter)
    {
        rounded_rect(
            buffer,
            w,
            h,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            8,
            if hide_installed {
                0x00303b2b
            } else {
                0x0022262c
            },
        );
        text_clipped(
            buffer,
            w,
            h,
            bounds.x + 10,
            bounds.y + 10,
            if hide_installed {
                "[x] HIDE INSTALLED"
            } else {
                "[ ] HIDE INSTALLED"
            },
            if hide_installed {
                0x00a8e66b
            } else {
                0x00bdc2ca
            },
            1,
            bounds.width.saturating_sub(20),
        );
    }
    if let Some(project) = &browse_selected {
        let installing = if project.project_type == "modpack" {
            browse_installing_modpacks.contains(&project.id)
        } else {
            browse_installing_for_instance
                .is_some_and(|ids| ids.contains(&project.id))
        };
        rect(buffer, w, h, 292, 246, 96, 30, 0x0022262c);
        text(buffer, w, h, 304, 257, "Back", 0x00a8e66b, 1);
        if let Some(icon) = browse_icons.get(&project.id) {
            draw_scaled_project_icon(buffer, w, h, 292, 286, 96, icon);
        }
        let header_text_width = layout_width.saturating_sub(588);
        text_clipped(
            buffer,
            w,
            h,
            404,
            286,
            &project.title,
            0x00f1f3f5,
            12,
            header_text_width,
        );
        rounded_rect(
            buffer,
            w,
            h,
            layout_width.saturating_sub(132),
            286,
            112,
            40,
            10,
            if installing {
                0x00343a42
            } else if browse_selected_installed {
                0x00303b2b
            } else {
                0x00a8e66b
            },
        );
        text(
            buffer,
            w,
            h,
            layout_width.saturating_sub(116),
            299,
            if installing {
                "Installing…"
            } else if browse_selected_installed {
                "Installed"
            } else if project.project_type == "modpack" {
                "Versions"
            } else {
                "Install"
            },
            if installing {
                0x00bdc2ca
            } else if browse_selected_installed {
                0x00a8e66b
            } else {
                0x001b1d22
            },
            1,
        );
        rounded_rect(
            buffer,
            w,
            h,
            layout_width.saturating_sub(180),
            286,
            40,
            40,
            10,
            if browse_project_menu_open {
                0x00303b2b
            } else {
                0x0022262c
            },
        );
        text(
            buffer,
            w,
            h,
            layout_width.saturating_sub(168),
            295,
            "⋮",
            if browse_project_menu_open {
                0x00a8e66b
            } else {
                0x00bdc2ca
            },
            2,
        );
        text_clipped(
            buffer,
            w,
            h,
            404,
            316,
            &project.author,
            0x007b838d,
            1,
            header_text_width,
        );
        text_clipped(
            buffer,
            w,
            h,
            404,
            336,
            &project.description,
            0x00bdc2ca,
            1,
            header_text_width,
        );
        let downloads = compact_count(project.downloads);
        let followers = compact_count(project.followers);
        text(
            buffer,
            w,
            h,
            404,
            360,
            &format!("↓ {downloads}"),
            0x007b838d,
            1,
        );
        let followers_x = 404 + (downloads.chars().count() * 8) + 32;
        text(
            buffer,
            w,
            h,
            followers_x,
            360,
            &format!("♥ {followers}"),
            0x007b838d,
            1,
        );
        let mut category_x = followers_x + (followers.chars().count() * 8) + 44;
        let category_right = 404 + header_text_width;
        for category in project.categories.iter().take(4) {
            let chip_width = category.chars().count().saturating_mul(8) + 18;
            if category_x + chip_width > category_right {
                break;
            }
            rounded_rect(
                buffer, w, h, category_x, 352, chip_width, 24, 12, 0x0022262c,
            );
            text_clipped(
                buffer,
                w,
                h,
                category_x + 9,
                360,
                category,
                0x00bdc2ca,
                1,
                chip_width.saturating_sub(18),
            );
            category_x += chip_width + 8;
        }
        if let Some(details) = &browse_details {
            for (x, label, tab) in [
                (292, "Description", 0),
                (414, "Versions", 1),
                (510, "Gallery", 2),
            ] {
                let active = browse_detail_tab == tab;
                rounded_rect(
                    buffer,
                    w,
                    h,
                    x,
                    398,
                    match tab {
                        0 => 112,
                        1 => 88,
                        _ => 80,
                    },
                    28,
                    8,
                    if active { 0x00303b2b } else { 0x0022262c },
                );
                text(
                    buffer,
                    w,
                    h,
                    x + 10,
                    407,
                    label,
                    if active { 0x00a8e66b } else { 0x00bdc2ca },
                    1,
                );
            }
            if browse_detail_tab == 0 {
                render::set_clip_rect(Some((
                    292,
                    440,
                    layout_width.saturating_sub(324),
                    h.saturating_sub(458),
                )));
                let viewport_bottom = h.saturating_sub(18);
                let scroll_y = browse_detail_scroll_pixels.floor() as isize;
                let mut content_y = 0usize;
                for (line_index, line) in
                    browse_description_lines.iter().enumerate()
                {
                    let line_height = super::browse_body_line_height(line);
                    let y = 442 + content_y as isize - scroll_y;
                    content_y = content_y.saturating_add(line_height);
                    if y + line_height as isize <= 440 {
                        continue;
                    }
                    if y >= viewport_bottom as isize {
                        break;
                    }
                    if let Some(image) = browse_description_images
                        .iter()
                        .find(|image| image.line_index == line_index)
                    {
                        let image_x = 292;
                        let image_y = y.max(440) as usize + 8;
                        let image_width = layout_width.saturating_sub(324);
                        let image_height = 204;
                        if let Some((image_width_px, image_height_px, pixels)) =
                            browse_icons.get(&image.cache_key)
                        {
                            super::super::images::draw_scaled_image(
                                buffer,
                                w,
                                h,
                                *image_width_px,
                                *image_height_px,
                                pixels,
                                image_x,
                                image_y,
                                image_width,
                                image_height,
                            );
                        } else {
                            rounded_rect(
                                buffer,
                                w,
                                h,
                                image_x,
                                image_y,
                                image_width,
                                image_height,
                                8,
                                0x0022262c,
                            );
                            let label = if browse_icon_failures
                                .contains(&image.cache_key)
                            {
                                "Image unavailable"
                            } else if !image.url.starts_with("https://") {
                                "Unsupported image URL"
                            } else if image.alt.is_empty() {
                                "Loading image…"
                            } else {
                                &image.alt
                            };
                            text_clipped(
                                buffer,
                                w,
                                h,
                                image_x + 12,
                                image_y + 12,
                                label,
                                0x00bdc2ca,
                                1,
                                image_width.saturating_sub(24),
                            );
                        }
                        continue;
                    }
                    let (display, color, scale, x) = if let Some(body) =
                        line.strip_prefix("│ ")
                    {
                        rounded_rect(
                            buffer,
                            w,
                            h,
                            292,
                            y.max(440) as usize,
                            3,
                            line_height.saturating_sub(3),
                            2,
                            0x007a9e68,
                        );
                        (body, 0x00aeb8c5, 1, 304)
                    } else if let Some(body) = line.strip_prefix("▌ ") {
                        rounded_rect(
                            buffer,
                            w,
                            h,
                            292,
                            y.max(440) as usize,
                            layout_width.saturating_sub(324),
                            line_height.saturating_sub(2),
                            5,
                            0x0022262c,
                        );
                        (body, 0x00d0d5dc, 1, 304)
                    } else if let Some((hashes, body)) = line.split_once(' ')
                        && hashes.starts_with('#')
                        && hashes.chars().all(|character| character == '#')
                    {
                        (
                            body,
                            0x00f1f3f5,
                            if hashes.len() <= 2 { 2 } else { 1 },
                            292,
                        )
                    } else {
                        (line.as_str(), 0x00bdc2ca, 1, 292)
                    };
                    if !display.is_empty() {
                        text(
                            buffer,
                            w,
                            h,
                            x,
                            y.max(440) as usize + 3,
                            display,
                            color,
                            scale,
                        );
                        for style in browse_description_styles
                            .iter()
                            .filter(|style| style.line_index == line_index)
                        {
                            let prefix: String = display
                                .chars()
                                .take(style.start_char)
                                .collect();
                            let style_x =
                                x + render::measure_text_width(&prefix, scale);
                            let style_text: String = display
                                .chars()
                                .skip(style.start_char)
                                .take(
                                    style
                                        .end_char
                                        .saturating_sub(style.start_char),
                                )
                                .collect();
                            render::text_styled(
                                buffer,
                                w,
                                h,
                                style_x,
                                y.max(440) as usize + 3,
                                &style_text,
                                color,
                                scale,
                                style.bold || scale > 1,
                                style.italic,
                            );
                        }
                        for link in browse_description_links
                            .iter()
                            .filter(|link| link.line_index == line_index)
                        {
                            let prefix: String =
                                display.chars().take(link.start_char).collect();
                            let link_x = link.text_x
                                + render::measure_text_width(
                                    &prefix, link.scale,
                                );
                            let link_y = y.max(440) as usize + 3;
                            let link_text: String = display
                                .chars()
                                .skip(link.start_char)
                                .take(
                                    link.end_char
                                        .saturating_sub(link.start_char),
                                )
                                .collect();
                            let link_width = render::measure_text_width(
                                &link_text, link.scale,
                            );
                            text(
                                buffer, w, h, link_x, link_y, &link_text,
                                0x007fc8ff, link.scale,
                            );
                            render::draw_line(
                                buffer,
                                w,
                                h,
                                (
                                    link_x as isize,
                                    (link_y + 14 * link.scale) as isize,
                                ),
                                (
                                    (link_x + link_width) as isize,
                                    (link_y + 14 * link.scale) as isize,
                                ),
                                0x007fc8ff,
                            );
                        }
                    }
                }
                render::set_clip_rect(None);
            } else if browse_detail_tab == 1 {
                let filter_buttons =
                    browse_version_filter_buttons(layout_width);
                for (group, x, button_width) in &filter_buttons {
                    let (name, selected_count) = match group {
                        0 => ("Channel", browse_detail_selected_channels.len()),
                        1 => (
                            "Game version",
                            browse_detail_selected_game_versions.len(),
                        ),
                        _ => ("Loader", browse_detail_selected_loaders.len()),
                    };
                    let selected = browse_detail_filter_menu_group
                        == Some(*group)
                        || selected_count > 0;
                    let label = if selected_count == 0 {
                        name.to_owned()
                    } else {
                        format!("{name} ({selected_count})")
                    };
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        *x,
                        440,
                        *button_width,
                        30,
                        10,
                        if selected { 0x00303b2b } else { 0x0022262c },
                    );
                    text_clipped(
                        buffer,
                        w,
                        h,
                        *x + 10,
                        449,
                        &label,
                        if selected { 0x00a8e66b } else { 0x00bdc2ca },
                        1,
                        button_width.saturating_sub(20),
                    );
                }
                render::set_clip_rect(Some((
                    292,
                    476,
                    layout_width.saturating_sub(324),
                    h.saturating_sub(494),
                )));
                let filtered_indices = browse_filtered_version_indices(
                    details,
                    browse_detail_selected_channels,
                    browse_detail_selected_game_versions,
                    browse_detail_selected_loaders,
                );
                let installed_versions = browse_contents
                    .iter()
                    .filter(|item| {
                        item.project.as_ref().is_some_and(|item_project| {
                            item_project.id == project.id
                        })
                    })
                    .filter_map(|item| {
                        item.version.as_ref().map(|version| version.id.as_str())
                    })
                    .collect::<HashSet<_>>();
                let scroll = browse_detail_scroll_pixels.round() as usize;
                let row_offset = scroll / 56;
                let row_remainder = scroll % 56;
                for (index, source_index) in filtered_indices
                    .iter()
                    .skip(row_offset)
                    .take((h.saturating_sub(494) + row_remainder).div_ceil(56))
                    .enumerate()
                {
                    let Some(version) = details.versions.get(*source_index)
                    else {
                        continue;
                    };
                    let y = 476 + index * 56 - row_remainder;
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        292,
                        y,
                        layout_width.saturating_sub(324),
                        48,
                        12,
                        0x00343a42,
                    );
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        293,
                        y + 1,
                        layout_width.saturating_sub(326),
                        46,
                        11,
                        0x0022262c,
                    );
                    let channel_label = match version.version_type.as_str() {
                        "alpha" => "Alpha",
                        "beta" => "Beta",
                        _ => "Release",
                    };
                    let channel_width = channel_label.chars().count() * 8 + 16;
                    let channel_color = match version.version_type.as_str() {
                        "alpha" => 0x0033283f,
                        "beta" => 0x003c3525,
                        _ => 0x002b3b2e,
                    };
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        304,
                        y + 7,
                        channel_width,
                        20,
                        10,
                        channel_color,
                    );
                    text(
                        buffer,
                        w,
                        h,
                        312,
                        y + 12,
                        channel_label,
                        0x00bdc2ca,
                        1,
                    );
                    let title_x = 304 + channel_width + 8;
                    let title_width =
                        layout_width.saturating_sub(title_x + 152);
                    let version_title = if version.version_number.is_empty() {
                        version.name.clone()
                    } else if version.name.is_empty() {
                        version.version_number.clone()
                    } else {
                        format!("{} · {}", version.version_number, version.name)
                    };
                    let compatibility = format!(
                        "{} · {}",
                        version
                            .game_versions
                            .iter()
                            .take(3)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(","),
                        version.loaders.join(",")
                    );
                    text_clipped(
                        buffer,
                        w,
                        h,
                        title_x,
                        y + 6,
                        &version_title,
                        0x00f1f3f5,
                        1,
                        title_width,
                    );
                    text_clipped(
                        buffer,
                        w,
                        h,
                        304,
                        y + 24,
                        &compatibility,
                        0x007b838d,
                        1,
                        layout_width.saturating_sub(624),
                    );
                    let age = chrono::Utc::now()
                        .signed_duration_since(version.date_published);
                    let age_label = if age.num_days() >= 365 {
                        format!("{}y ago", age.num_days() / 365)
                    } else if age.num_days() >= 30 {
                        format!("{}mo ago", age.num_days() / 30)
                    } else if age.num_days() > 0 {
                        format!("{}d ago", age.num_days())
                    } else if age.num_hours() > 0 {
                        format!("{}h ago", age.num_hours())
                    } else {
                        format!("{}m ago", age.num_minutes().max(0))
                    };
                    let version_stats = format!(
                        "{} downloads · {}",
                        compact_count(version.downloads as u64),
                        age_label
                    );
                    text_clipped(
                        buffer,
                        w,
                        h,
                        layout_width.saturating_sub(286),
                        y + 24,
                        &version_stats,
                        0x007b838d,
                        1,
                        150,
                    );
                    let install =
                        browse_version_install_button(layout_width, y);
                    let installed =
                        installed_versions.contains(version.id.as_str());
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        install.x,
                        install.y,
                        install.width,
                        install.height,
                        7,
                        if installing {
                            0x00343a42
                        } else if installed {
                            0x00303b2b
                        } else {
                            0x00a8e66b
                        },
                    );
                    text_clipped(
                        buffer,
                        w,
                        h,
                        install.x + 9,
                        install.y + 7,
                        if installing {
                            "Installing…"
                        } else if installed {
                            "Installed"
                        } else {
                            "Install"
                        },
                        if installing {
                            0x00bdc2ca
                        } else if installed {
                            0x00a8e66b
                        } else {
                            0x001b1d22
                        },
                        1,
                        install.width.saturating_sub(18),
                    );
                }
                if filtered_indices.is_empty() {
                    text(
                        buffer,
                        w,
                        h,
                        304,
                        486,
                        "No versions match these filters.",
                        0x007b838d,
                        1,
                    );
                }
                render::set_clip_rect(None);
                if let Some(group) = browse_detail_filter_menu_group
                    && let Some((_, menu_x, _)) = filter_buttons
                        .iter()
                        .find(|(button_group, _, _)| *button_group == group)
                {
                    let options = browse_version_filter_options(details, group);
                    let visible_count = options
                        .len()
                        .saturating_sub(browse_detail_filter_option_offset)
                        .min(7);
                    let menu_height = visible_count * 28 + 12;
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        menu_x.saturating_sub(2),
                        474,
                        252,
                        menu_height,
                        12,
                        0x00343a42,
                    );
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        *menu_x,
                        476,
                        248,
                        menu_height.saturating_sub(4),
                        10,
                        0x0022262c,
                    );
                    render::set_clip_rect(Some((
                        *menu_x,
                        478,
                        248,
                        menu_height.saturating_sub(8),
                    )));
                    for (index, option) in options
                        .iter()
                        .skip(browse_detail_filter_option_offset)
                        .take(7)
                        .enumerate()
                    {
                        let y = 480 + index * 28;
                        let selected =
                            match group {
                                0 => browse_detail_selected_channels
                                    .contains(option),
                                1 => browse_detail_selected_game_versions
                                    .contains(option),
                                _ => browse_detail_selected_loaders
                                    .contains(option),
                            };
                        if selected {
                            rounded_rect(
                                buffer,
                                w,
                                h,
                                *menu_x + 4,
                                y,
                                240,
                                26,
                                7,
                                0x00303b2b,
                            );
                        }
                        text_clipped(
                            buffer,
                            w,
                            h,
                            *menu_x + 12,
                            y + 8,
                            option,
                            if selected { 0x00a8e66b } else { 0x00bdc2ca },
                            1,
                            220,
                        );
                    }
                    render::set_clip_rect(None);
                }
            } else if browse_gallery_viewer_open {
                rect(buffer, w, h, 0, 64, w, h.saturating_sub(64), 0x000b0b0d);
                if let Some(image) = details.gallery.get(browse_gallery_index) {
                    let key = format!(
                        "gallery-full:{}:{}",
                        project.id, browse_gallery_index
                    );
                    let image_x = 32;
                    let image_y = 140;
                    let image_width = w.saturating_sub(64);
                    let image_height = h.saturating_sub(232).max(64);
                    if let Some(image_data) = browse_icons.get(&key) {
                        draw_scaled_image(
                            buffer,
                            w,
                            h,
                            image_data.0,
                            image_data.1,
                            &image_data.2,
                            image_x,
                            image_y,
                            image_width,
                            image_height,
                        );
                    } else {
                        rounded_rect(
                            buffer,
                            w,
                            h,
                            image_x,
                            image_y,
                            image_width,
                            image_height,
                            12,
                            0x0022262c,
                        );
                        text(
                            buffer,
                            w,
                            h,
                            image_x + 14,
                            image_y + 16,
                            "LOADING IMAGE…",
                            0x007b838d,
                            1,
                        );
                    }
                    if let Some(title) = image.title.as_deref() {
                        text_clipped(
                            buffer,
                            w,
                            h,
                            32,
                            82,
                            title,
                            0x00f1f3f5,
                            1,
                            w.saturating_sub(64),
                        );
                    }
                    if let Some(description) = image.description.as_deref() {
                        text_clipped(
                            buffer,
                            w,
                            h,
                            32,
                            104,
                            description,
                            0x00bdc2ca,
                            1,
                            w.saturating_sub(64),
                        );
                    }
                    let toolbar_width = 396;
                    let toolbar_x = w.saturating_sub(toolbar_width) / 2;
                    let toolbar_y = h.saturating_sub(72);
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        toolbar_x,
                        toolbar_y,
                        toolbar_width,
                        48,
                        16,
                        0x0022262c,
                    );
                    let index_label = format!(
                        "{} / {}",
                        browse_gallery_index + 1,
                        details.gallery.len()
                    );
                    for (offset, width, label, enabled) in [
                        (0, 68, "Prev", details.gallery.len() > 1),
                        (76, 56, index_label.as_str(), true),
                        (140, 68, "Next", details.gallery.len() > 1),
                        (216, 72, "Open", true),
                        (296, 68, "Close", true),
                    ] {
                        let x = toolbar_x + offset;
                        rounded_rect(
                            buffer,
                            w,
                            h,
                            x,
                            toolbar_y + 8,
                            width,
                            32,
                            12,
                            if enabled { 0x00343a42 } else { 0x00202429 },
                        );
                        text_clipped(
                            buffer,
                            w,
                            h,
                            x + 6,
                            toolbar_y + 18,
                            label,
                            if enabled { 0x00bdc2ca } else { 0x007b838d },
                            1,
                            width.saturating_sub(12),
                        );
                    }
                }
            } else if details.gallery.is_empty() {
                text(
                    buffer,
                    w,
                    h,
                    292,
                    458,
                    "No gallery images",
                    0x007b838d,
                    1,
                );
            } else {
                let (columns, card_width, card_height, _) =
                    super::browse_gallery_grid_metrics(
                        layout_width,
                        details.gallery.len(),
                    );
                let image_height = card_width / 2;
                let scroll_y = browse_detail_scroll_pixels.floor() as isize;
                let viewport_bottom = h.saturating_sub(18) as isize;
                render::set_clip_rect(Some((
                    292,
                    440,
                    layout_width.saturating_sub(324),
                    h.saturating_sub(458),
                )));
                for (index, image) in details.gallery.iter().enumerate() {
                    let row = index / columns;
                    let column = index % columns;
                    let card_x = 292 + column * (card_width + 16);
                    let card_y = 442 + row * (card_height + 16);
                    let y = card_y as isize - scroll_y;
                    if y + card_height as isize <= 440 {
                        continue;
                    }
                    if y >= viewport_bottom {
                        break;
                    }
                    let draw_y = y.max(0) as usize;
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        card_x,
                        draw_y,
                        card_width,
                        card_height,
                        12,
                        0x0022262c,
                    );
                    let key = format!("gallery:{}:{index}", project.id);
                    if let Some(image_data) = browse_icons.get(&key) {
                        crate::images::draw_cover_image(
                            buffer,
                            w,
                            h,
                            image_data.0,
                            image_data.1,
                            &image_data.2,
                            card_x,
                            draw_y,
                            card_width,
                            image_height,
                        );
                    } else {
                        rounded_rect(
                            buffer,
                            w,
                            h,
                            card_x,
                            draw_y,
                            card_width,
                            image_height,
                            8,
                            0x00343a42,
                        );
                    }
                    text_clipped(
                        buffer,
                        w,
                        h,
                        card_x + 12,
                        draw_y + image_height + 12,
                        image.title.as_deref().unwrap_or("Gallery image"),
                        0x00f1f3f5,
                        1,
                        card_width.saturating_sub(24),
                    );
                    let description =
                        image.description.as_deref().unwrap_or("");
                    let description_lines = render::wrap_text_to_width(
                        description,
                        1,
                        card_width.saturating_sub(24),
                        2,
                    );
                    for (line, text_line) in
                        description_lines.lines().take(2).enumerate()
                    {
                        text_clipped(
                            buffer,
                            w,
                            h,
                            card_x + 12,
                            draw_y + image_height + 34 + line * 18,
                            text_line,
                            0x00bdc2ca,
                            1,
                            card_width.saturating_sub(24),
                        );
                    }
                    text_clipped(
                        buffer,
                        w,
                        h,
                        card_x + 12,
                        draw_y + image_height + 78,
                        image
                            .created
                            .as_deref()
                            .and_then(|created| {
                                chrono::DateTime::parse_from_rfc3339(created)
                                    .ok()
                            })
                            .map(|created| {
                                created.format("%B %-d, %Y").to_string()
                            })
                            .as_deref()
                            .unwrap_or(""),
                        0x007b838d,
                        1,
                        card_width.saturating_sub(24),
                    );
                }
                render::set_clip_rect(None);
            }
        } else {
            text_clipped(
                buffer,
                w,
                h,
                388,
                313,
                message,
                0x007b838d,
                1,
                layout_width.saturating_sub(420),
            );
        }
        if browse_project_menu_open {
            let menu_x = layout_width.saturating_sub(220);
            rounded_rect(buffer, w, h, menu_x, 334, 220, 160, 12, 0x00343a42);
            rounded_rect(
                buffer,
                w,
                h,
                menu_x + 2,
                336,
                216,
                156,
                10,
                0x0022262c,
            );
            for (index, label, enabled) in [
                (
                    0,
                    if browse_project_follow_status_loading {
                        "Checking follow…"
                    } else if browse_project_follow_pending {
                        "Updating follow…"
                    } else if browse_project_followed {
                        "Unfollow project"
                    } else {
                        "Follow project"
                    },
                    !browse_project_follow_pending
                        && !browse_project_follow_status_loading,
                ),
                (1, "Save to collection", true),
                (2, "Open in Modrinth", true),
            ] {
                let y = 342 + index * 34;
                text_clipped(
                    buffer,
                    w,
                    h,
                    menu_x + 14,
                    y + 9,
                    label,
                    if enabled { 0x00f1f3f5 } else { 0x007b838d },
                    1,
                    192,
                );
            }
            rect(buffer, w, h, menu_x + 10, 446, 200, 1, 0x00404750);
            text(
                buffer,
                w,
                h,
                menu_x + 14,
                459,
                "Report project",
                0x00ef8b8b,
                1,
            );
            if browse_save_menu_open {
                let save_x = menu_x.saturating_sub(228).max(292);
                rounded_rect(
                    buffer, w, h, save_x, 368, 220, 252, 12, 0x00343a42,
                );
                rounded_rect(
                    buffer,
                    w,
                    h,
                    save_x + 2,
                    370,
                    216,
                    248,
                    10,
                    0x0022262c,
                );
                text_clipped(
                    buffer,
                    w,
                    h,
                    save_x + 12,
                    382,
                    "Save to collection",
                    0x00f1f3f5,
                    1,
                    196,
                );
                rect(buffer, w, h, save_x + 10, 398, 200, 1, 0x00404750);
                if browse_save_collections_loading {
                    text_clipped(
                        buffer,
                        w,
                        h,
                        save_x + 12,
                        415,
                        "Loading collections…",
                        0x00bdc2ca,
                        1,
                        196,
                    );
                } else if browse_save_collections.is_empty() {
                    text_clipped(
                        buffer,
                        w,
                        h,
                        save_x + 12,
                        415,
                        "No collections yet.",
                        0x00bdc2ca,
                        1,
                        196,
                    );
                } else {
                    let scroll = browse_save_scroll_pixels.round() as usize;
                    super::render::set_clip_rect(Some((
                        save_x + 6,
                        402,
                        208,
                        154,
                    )));
                    for (index, collection) in
                        browse_save_collections.iter().enumerate()
                    {
                        let row_y = 405 + index * 34;
                        let y = row_y.saturating_sub(scroll);
                        if y + 32 < 402 || y > 556 {
                            continue;
                        }
                        rounded_rect(
                            buffer,
                            w,
                            h,
                            save_x + 8,
                            y,
                            204,
                            32,
                            8,
                            if browse_save_collection_pending
                                .contains(&collection.id)
                            {
                                0x00343a42
                            } else {
                                0x0027292e
                            },
                        );
                        let saved = browse_selected.is_some_and(|project| {
                            collection
                                .projects
                                .iter()
                                .any(|id| id == &project.id)
                        });
                        let label = if browse_save_collection_pending
                            .contains(&collection.id)
                        {
                            format!("Saving {}…", collection.name)
                        } else if saved {
                            format!("✓  {}", collection.name)
                        } else {
                            collection.name.clone()
                        };
                        text_clipped(
                            buffer,
                            w,
                            h,
                            save_x + 16,
                            y + 9,
                            &label,
                            if browse_save_collection_pending
                                .contains(&collection.id)
                            {
                                0x007b838d
                            } else {
                                0x00f1f3f5
                            },
                            1,
                            184,
                        );
                    }
                    super::render::set_clip_rect(None);
                    let viewport_height = 154usize;
                    let content_height = browse_save_collections.len() * 34;
                    let max_scroll =
                        content_height.saturating_sub(viewport_height);
                    if max_scroll > 0 {
                        let thumb_height = (viewport_height * viewport_height
                            / content_height)
                            .max(22);
                        let thumb_y = 402
                            + ((viewport_height - thumb_height)
                                * scroll.min(max_scroll)
                                / max_scroll);
                        rounded_rect(
                            buffer,
                            w,
                            h,
                            save_x + 207,
                            thumb_y,
                            3,
                            thumb_height,
                            2,
                            0x007b838d,
                        );
                    }
                }
                rounded_rect(
                    buffer,
                    w,
                    h,
                    save_x + 8,
                    562,
                    204,
                    36,
                    8,
                    0x00343a42,
                );
                text_clipped(
                    buffer,
                    w,
                    h,
                    save_x + 18,
                    573,
                    "+  Create new collection",
                    0x001bd96a,
                    1,
                    184,
                );
            }
        }
    } else {
        let layout = BrowseResultsLayout::new(layout_width, h);
        let card_width = layout.viewport.width;
        let narrow = layout.narrow;
        let card_height = layout.card_height;
        let result_stride = layout.stride();
        let visible_rows = layout.visible_rows(browse_scroll_pixels);
        let installed_ids = browse_contents
            .iter()
            .filter_map(|item| {
                item.project.as_ref().map(|project| project.id.as_str())
            })
            .collect::<HashSet<_>>();
        let browse_scroll_remainder =
            browse_scroll_pixels.round() as usize % result_stride;
        let results_viewport_height = layout.viewport.height;
        if browse_results.is_empty() {
            let viewport_bottom = h.saturating_sub(52);
            let viewport_center = 246 + results_viewport_height / 2;
            let message_width = layout_width.saturating_sub(324);
            let message_x = 292;
            let message_y =
                viewport_center.min(viewport_bottom.saturating_sub(20));
            if let Some(error) = browse_error {
                let card_width = message_width.min(520);
                let card_x =
                    message_x + message_width.saturating_sub(card_width) / 2;
                let card_y = message_y.saturating_sub(50);
                rounded_rect(
                    buffer, w, h, card_x, card_y, card_width, 100, 14,
                    0x0022262c,
                );
                text_clipped(
                    buffer,
                    w,
                    h,
                    card_x + 18,
                    card_y + 24,
                    "Could not load projects",
                    0x00ef8b8b,
                    1,
                    card_width.saturating_sub(36),
                );
                text_clipped(
                    buffer,
                    w,
                    h,
                    card_x + 18,
                    card_y + 50,
                    error.trim(),
                    0x0096a2b0,
                    1,
                    card_width.saturating_sub(36),
                );
            } else if browse_search_pending {
                let text_width = "Searching…".chars().count() * 8;
                text_clipped(
                    buffer,
                    w,
                    h,
                    message_x + message_width.saturating_sub(text_width) / 2,
                    message_y,
                    "Searching…",
                    0x0096a2b0,
                    1,
                    message_width,
                );
            } else {
                let label = if browse_total_hits > 0 {
                    "No visible projects on this page"
                } else {
                    "No results found"
                };
                let text_width = render::measure_text_width(label, 1);
                text_clipped(
                    buffer,
                    w,
                    h,
                    message_x + message_width.saturating_sub(text_width) / 2,
                    message_y,
                    label,
                    0x0096a2b0,
                    1,
                    message_width,
                );
            }
        }
        let viewport = layout.viewport;
        render::set_clip_rect(Some((
            viewport.x,
            viewport.y,
            viewport.width,
            viewport.height,
        )));
        for (index, project) in browse_results
            .iter()
            .skip(visible_rows.start)
            .take(visible_rows.len())
            .enumerate()
        {
            let y = 246 + index * result_stride - browse_scroll_remainder;
            let compact = layout.compact_icon;
            let icon_size = layout.icon_width();
            let info_x = 308 + icon_size + 12;
            let card_right = 292 + card_width;
            let action = layout.action(y);
            let installing = if project.project_type == "modpack" {
                browse_installing_modpacks.contains(&project.id)
            } else {
                browse_installing_for_instance
                    .is_some_and(|ids| ids.contains(&project.id))
            };
            let installed = project.project_type != "modpack"
                && installed_ids.contains(project.id.as_str());
            let action_disabled = browse_search_pending || installing;
            let info_width = if narrow {
                card_right.saturating_sub(info_x + 16)
            } else {
                action.x.saturating_sub(info_x + 12)
            };
            rounded_rect(
                buffer,
                w,
                h,
                292,
                y,
                card_width,
                card_height,
                16,
                0x00343a42,
            );
            rounded_rect(
                buffer,
                w,
                h,
                293,
                y + 1,
                card_width.saturating_sub(2),
                card_height.saturating_sub(2),
                15,
                0x0022262c,
            );
            if let Some(icon) = browse_icons.get(&project.id) {
                draw_scaled_project_icon(
                    buffer,
                    w,
                    h,
                    308,
                    y + 16,
                    icon_size,
                    icon,
                );
            } else {
                let (_, screen_size) =
                    render::map_page_layout_span(308, icon_size);
                rounded_rect(
                    buffer,
                    w,
                    h,
                    308,
                    y + 16,
                    icon_size,
                    screen_size,
                    10,
                    0x00343a42,
                );
            }
            let title_width = layout
                .logical_width(render::measure_text_width(&project.title, 8));
            let author_width = layout
                .logical_width(render::measure_text_width(&project.author, 1));
            let author_gap = layout.logical_width(10);
            let inline_author =
                title_width + author_gap + author_width <= info_width;
            text_clipped(
                buffer,
                w,
                h,
                info_x,
                y + 12,
                &project.title,
                0x00f1f3f5,
                8,
                info_width,
            );
            text_clipped(
                buffer,
                w,
                h,
                if inline_author {
                    info_x + title_width + author_gap
                } else {
                    info_x
                },
                y + if inline_author { 14 } else { 34 },
                &project.author,
                0x007b838d,
                1,
                if inline_author {
                    author_width
                } else {
                    info_width
                },
            );
            let (_, summary_width) =
                render::map_page_layout_span(info_x, info_width);
            let description = render::wrap_text_to_width(
                &project.description,
                1,
                summary_width,
                2,
            );
            text_clipped(
                buffer,
                w,
                h,
                info_x,
                y + if inline_author { 40 } else { 54 },
                &description,
                0x00bdc2ca,
                1,
                info_width,
            );
            let tag_right = if narrow {
                card_right.saturating_sub(16)
            } else {
                action.x.saturating_sub(12)
            };
            let mut tag_x = if compact { 308 } else { info_x };
            let tag_y = y + if narrow { 98 } else { 103 };
            for (_, category) in project
                .categories
                .iter()
                .enumerate()
                .filter(|(index, category)| {
                    !project.categories[..*index].contains(category)
                })
                .take(4)
            {
                let label = category.as_str();
                let inset = layout.logical_width(8);
                let tag_width = layout
                    .logical_width(render::measure_text_width(label, 1) + 2)
                    + inset * 2;
                if tag_x.saturating_add(tag_width) > tag_right {
                    break;
                }
                rounded_rect(
                    buffer, w, h, tag_x, tag_y, tag_width, 22, 8, 0x00343a42,
                );
                text_clipped(
                    buffer,
                    w,
                    h,
                    tag_x + inset,
                    tag_y + 3,
                    label,
                    0x00bdc2ca,
                    1,
                    tag_width.saturating_sub(inset * 2),
                );
                tag_x = tag_x.saturating_add(tag_width + 6);
            }
            let stats = format!(
                "{} ↓   {} ♥",
                compact_count(project.downloads),
                compact_count(project.followers)
            );
            text_clipped(
                buffer,
                w,
                h,
                if narrow { 308 } else { action.x },
                y + if narrow { 126 } else { 64 },
                &stats,
                0x00bdc2ca,
                1,
                action.width,
            );
            rounded_rect(
                buffer,
                w,
                h,
                action.x,
                action.y,
                action.width,
                action.height,
                10,
                if action_disabled {
                    0x00343a42
                } else if installed {
                    0x00303b2b
                } else {
                    0x00a8e66b
                },
            );
            if installed {
                text(
                    buffer,
                    w,
                    h,
                    action.x + layout.logical_width(12),
                    action.y + 8,
                    "✓",
                    if action_disabled {
                        0x007b838d
                    } else {
                        0x00a8e66b
                    },
                    1,
                );
            } else {
                render::draw_navigation_icon(
                    buffer,
                    w,
                    h,
                    action.x + layout.logical_width(18),
                    action.y + 16,
                    4,
                    if action_disabled {
                        0x007b838d
                    } else {
                        0x001b1d22
                    },
                );
            }
            text_clipped(
                buffer,
                w,
                h,
                action.x + layout.logical_width(34),
                action.y + 8,
                if installing {
                    "Installing…"
                } else if installed {
                    "Installed"
                } else if global_browse && project.project_type != "modpack" {
                    "Add to an instance"
                } else {
                    "Install"
                },
                if action_disabled {
                    0x007b838d
                } else if installed {
                    0x00a8e66b
                } else {
                    0x001b1d22
                },
                1,
                action.width.saturating_sub(layout.logical_width(44)),
            );
        }
        render::set_clip_rect(None);
        let footer_y = h.saturating_sub(42);
        rect(
            buffer,
            w,
            h,
            292,
            footer_y.saturating_sub(10),
            layout_width.saturating_sub(324),
            h.saturating_sub(footer_y.saturating_sub(10)),
            0x001b1d22,
        );
        rect(
            buffer,
            w,
            h,
            292,
            footer_y.saturating_sub(10),
            layout_width.saturating_sub(324),
            1,
            0x00363b42,
        );
        let first = if browse_total_hits == 0 {
            0
        } else {
            browse_search_offset + 1
        };
        let last = browse_search_offset
            .saturating_add(browse_page_size)
            .min(browse_total_hits);
        let [previous, next] = layout.pagination();
        let status = if browse_search_pending {
            "Searching…".to_owned()
        } else if browse_error.is_some() {
            "Search failed".to_owned()
        } else if layout_width >= 900 {
            format!("Showing {first}–{last} of {browse_total_hits}")
        } else {
            format!("{first}–{last} of {browse_total_hits}")
        };
        text_clipped(
            buffer,
            w,
            h,
            292,
            footer_y + 9,
            &status,
            0x007b838d,
            1,
            previous.x.saturating_sub(304),
        );
        let [can_previous, can_next] = browse_page_controls(
            browse_search_pending,
            browse_error.is_some(),
            browse_search_offset,
            browse_page_size,
            browse_total_hits,
        );
        for (button, label, enabled) in [
            (previous, "Previous", can_previous),
            (
                next,
                if browse_error.is_some() {
                    "Retry"
                } else {
                    "Next"
                },
                can_next,
            ),
        ] {
            rounded_rect(
                buffer,
                w,
                h,
                button.x,
                button.y,
                button.width,
                button.height,
                8,
                if enabled { 0x0022262c } else { 0x001b1d22 },
            );
            text_clipped(
                buffer,
                w,
                h,
                button.x + 10,
                button.y + 9,
                label,
                if enabled { 0x00bdc2ca } else { 0x007b838d },
                1,
                button.width.saturating_sub(20),
            );
        }
        if browse_filter_menu_open {
            let panel = browse_filter_popup_bounds(layout_width, h);
            let UiBounds {
                x: panel_x,
                y: panel_y,
                width: panel_width,
                height: panel_height,
            } = panel;
            let (version_toggle, viewport) =
                browse_popup_filter_rows(panel, browse_filter_group);
            rounded_rect(
                buffer,
                w,
                h,
                panel_x,
                panel_y,
                panel_width,
                panel_height,
                14,
                0x00343a42,
            );
            rounded_rect(
                buffer,
                w,
                h,
                panel_x + 1,
                panel_y + 1,
                panel_width.saturating_sub(2),
                panel_height.saturating_sub(2),
                13,
                0x0022262c,
            );
            for (index, label) in
                ["Versions", "Loaders", "Tags", "License", "Side"]
                    .iter()
                    .take(browse_filter_group_count)
                    .enumerate()
            {
                let bounds = browse_filter_popup_tab(
                    panel_x,
                    panel_y,
                    panel_width,
                    browse_filter_group_count,
                    index,
                );
                rounded_rect(
                    buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    7,
                    if usize::from(browse_filter_group) == index {
                        0x00303b2b
                    } else {
                        0x00202429
                    },
                );
                text_clipped(
                    buffer,
                    w,
                    h,
                    bounds.x + 5,
                    panel_y + 16,
                    label,
                    if usize::from(browse_filter_group) == index {
                        0x00a8e66b
                    } else {
                        0x00bdc2ca
                    },
                    1,
                    bounds.width.saturating_sub(10),
                );
            }
            if let Some(toggle) = version_toggle {
                draw_all_versions_toggle(
                    buffer,
                    w,
                    h,
                    toggle,
                    browse_show_all_versions,
                );
            }
            if browse_filter_options_loading {
                text(
                    buffer,
                    w,
                    h,
                    panel_x + 12,
                    viewport.y + 8,
                    "LOADING FILTER OPTIONS…",
                    0x007b838d,
                    1,
                );
            } else if browse_filter_menu_options.is_empty() {
                text(
                    buffer,
                    w,
                    h,
                    panel_x + 12,
                    viewport.y + 8,
                    "No options for this type",
                    0x007b838d,
                    1,
                );
            } else {
                let selected = match browse_filter_group {
                    1 => browse_selected_loaders,
                    2 => browse_selected_categories,
                    BROWSE_LICENSE_GROUP => browse_selected_licenses,
                    BROWSE_ENVIRONMENT_GROUP => browse_selected_environments,
                    _ => browse_selected_game_versions,
                };
                let scroll = browse_filter_scroll_pixels as usize;
                let scroll_remainder = scroll % 24;
                let visible = (viewport.height + scroll_remainder).div_ceil(24);
                render::set_clip_rect(Some((
                    viewport.x,
                    viewport.y,
                    viewport.width,
                    viewport.height,
                )));
                for (index, option) in browse_filter_menu_options
                    .iter()
                    .skip(scroll / 24)
                    .take(visible)
                    .enumerate()
                {
                    let row_y = viewport.y + index * 24 - scroll_remainder;
                    let checked = selected.contains(option);
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        panel_x + 10,
                        row_y + 2,
                        16,
                        16,
                        5,
                        if checked { 0x001bd96a } else { 0x00343a42 },
                    );
                    let label = option.replace('_', " ").replace('-', " ");
                    text_clipped(
                        buffer,
                        w,
                        h,
                        panel_x + 34,
                        row_y + 5,
                        &label,
                        if checked { 0x00f1f3f5 } else { 0x00bdc2ca },
                        1,
                        panel_width.saturating_sub(48),
                    );
                }
                render::set_clip_rect(None);
            }
        }
        if browse_sort_menu_open {
            let sort_layout = super::browse_search_layout(layout_width);
            rounded_rect(
                buffer,
                w,
                h,
                sort_layout.sort_x,
                250,
                sort_layout.sort_width,
                138,
                10,
                0x00343a42,
            );
            rounded_rect(
                buffer,
                w,
                h,
                sort_layout.sort_x + 1,
                251,
                sort_layout.sort_width.saturating_sub(2),
                136,
                9,
                0x0022262c,
            );
            for (index, label) in
                ["Relevance", "Downloads", "Followers", "Newest", "Updated"]
                    .iter()
                    .enumerate()
            {
                let row_y = 254 + index * 26;
                if index == browse_sort as usize {
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        sort_layout.sort_x + 4,
                        row_y,
                        sort_layout.sort_width.saturating_sub(8),
                        24,
                        6,
                        0x00303b2b,
                    );
                }
                text_clipped(
                    buffer,
                    w,
                    h,
                    sort_layout.sort_x + 12,
                    row_y + 7,
                    label,
                    if index == browse_sort as usize {
                        0x00a8e66b
                    } else {
                        0x00bdc2ca
                    },
                    1,
                    sort_layout.sort_width.saturating_sub(20),
                );
            }
        }
        if browse_view_menu_open {
            let (view_x, view_width) = super::browse_view_layout(layout_width);
            rounded_rect(
                buffer, w, h, view_x, 250, view_width, 164, 10, 0x00343a42,
            );
            rounded_rect(
                buffer,
                w,
                h,
                view_x + 1,
                251,
                view_width.saturating_sub(2),
                162,
                9,
                0x0022262c,
            );
            for (index, value) in [5, 10, 15, 20, 50, 100].iter().enumerate() {
                let row_y = 254 + index * 26;
                if *value == browse_page_size {
                    rounded_rect(
                        buffer,
                        w,
                        h,
                        view_x + 4,
                        row_y,
                        view_width.saturating_sub(8),
                        24,
                        6,
                        0x00303b2b,
                    );
                }
                text(
                    buffer,
                    w,
                    h,
                    view_x + 12,
                    row_y + 7,
                    &value.to_string(),
                    if *value == browse_page_size {
                        0x00a8e66b
                    } else {
                        0x00bdc2ca
                    },
                    1,
                );
            }
        }
    }
}
