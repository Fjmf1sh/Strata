#![recursion_limit = "256"]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use native_backend::{
    CreateInstance, HostingPowerAction, NativeBackend, StartupStatus,
};
use platform::{clipboard_text, copy_screenshot_image, set_clipboard_text};
use std::collections::{HashMap, HashSet, VecDeque};
use std::num::NonZeroU32;
use std::sync::Arc;
use theseus::worlds;
use tokio::runtime::Runtime;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{ResizeDirection, Window, WindowId};

mod account_actions;
mod browse_actions;
mod draw;
mod events;
mod images;
mod java_actions;
mod keyboard_events;
mod platform;
mod pointer_events;
mod render;
mod screenshots;
mod scroll_events;
mod user_events;
use images::{
    BROWSE_IMAGE_CACHE_BUDGET, INSTANCE_ICON_CACHE_BUDGET,
    SCREENSHOT_THUMBNAIL_CACHE_BUDGET, SKIN_PREVIEW_CACHE_BUDGET,
    cap_image_cache, decode_skin_texture, draw_minecraft_skin_model,
    draw_scaled_image, spawn_image_decode,
};
#[cfg(test)]
use platform::{screenshot_clipboard_bitmap, screenshot_clipboard_png};
use render::{
    blend_color, clear_text_commands, draw_line, draw_northstar_mark,
    draw_scaled_instance_icon, draw_scaled_project_icon, flush_native_text,
    northstar_window_icon, rect, rect_square, rounded_rect, text, text_clipped,
    text_input_clipped, text_regular, themed_color,
};

const CONTENT_ROW_HEIGHT: usize = 74;
const CONTENT_LIST_TOP: usize = 262;
const SIDEBAR_NEWS_CARD_HEIGHT: usize = 280;
const SIDEBAR_NEWS_CARD_STRIDE: usize = 290;
const BROWSE_PAGE_SIZES: [u32; 6] = [5, 10, 15, 20, 50, 100];
const LIBRARY_SORT_LABELS: [&str; 7] = [
    "Name",
    "Last played",
    "Hours played",
    "Date created",
    "Date modified",
    "Loader",
    "Game version",
];
const LIBRARY_GROUP_LABELS: [&str; 5] = [
    "Custom group",
    "Instance type",
    "Loader",
    "Game version",
    "No grouping",
];

#[derive(Clone, Copy)]
struct SettingsRowLayout {
    row_width: usize,
    row_right: usize,
    toggle_x: usize,
    decrement_x: usize,
    value_x: usize,
    increment_x: usize,
    dimension_x: usize,
    launch_field_x: usize,
    launch_field_width: usize,
}

fn settings_row_layout(layout_width: usize) -> SettingsRowLayout {
    let row_width = layout_width.saturating_sub(520).max(254);
    let row_right = 500 + row_width;
    let launch_field_width = row_right.saturating_sub(686).min(460);
    SettingsRowLayout {
        row_width,
        row_right,
        toggle_x: row_right.saturating_sub(76),
        decrement_x: row_right.saturating_sub(144),
        value_x: row_right.saturating_sub(96),
        increment_x: row_right.saturating_sub(40),
        dimension_x: row_right.saturating_sub(104),
        launch_field_x: row_right.saturating_sub(launch_field_width + 8),
        launch_field_width,
    }
}

const SETTINGS_THEME_OPTIONS: [theseus::data::Theme; 5] = [
    theseus::data::Theme::System,
    theseus::data::Theme::Light,
    theseus::data::Theme::Dark,
    theseus::data::Theme::Oled,
    theseus::data::Theme::Retro,
];

fn settings_theme_option_columns(row_width: usize) -> usize {
    ((row_width + 16) / 208).clamp(1, 3)
}

fn settings_theme_selector_extra_height(row_width: usize) -> usize {
    let columns = settings_theme_option_columns(row_width);
    SETTINGS_THEME_OPTIONS.len().div_ceil(columns) * 124 - 50
}

fn settings_theme_option_bounds(
    row_width: usize,
    row_y: usize,
) -> [UiBounds; 5] {
    let gap_x = 16;
    let gap_y = 12;
    let columns = settings_theme_option_columns(row_width);
    let card_width = row_width.saturating_sub(gap_x * (columns - 1)) / columns;
    std::array::from_fn(|index| UiBounds {
        x: 500 + (index % columns) * (card_width + gap_x),
        y: row_y + (index / columns) * (112 + gap_y),
        width: card_width,
        height: 112,
    })
}

fn settings_theme_option_at(
    row_width: usize,
    row_y: usize,
    x: f64,
    y: f64,
) -> Option<theseus::data::Theme> {
    SETTINGS_THEME_OPTIONS
        .into_iter()
        .zip(settings_theme_option_bounds(row_width, row_y))
        .find_map(|(theme, bounds)| bounds.contains(x, y).then_some(theme))
}

fn next_settings_theme(
    current: theseus::data::Theme,
    direction: isize,
) -> theseus::data::Theme {
    let current_index = SETTINGS_THEME_OPTIONS
        .iter()
        .position(|theme| theme_label(*theme) == theme_label(current))
        .unwrap_or(0) as isize;
    let next_index = (current_index + direction)
        .rem_euclid(SETTINGS_THEME_OPTIONS.len() as isize)
        as usize;
    SETTINGS_THEME_OPTIONS[next_index]
}

fn settings_row_y(
    section: usize,
    position: usize,
    row_width: usize,
    scroll_pixels: f64,
) -> usize {
    let theme_selector_extra_height = if section == 0 && position > 0 {
        settings_theme_selector_extra_height(row_width)
    } else {
        0
    };
    (146.0 + position as f64 * 50.0 + theme_selector_extra_height as f64
        - scroll_pixels)
        .round()
        .max(0.0) as usize
}

fn settings_row_position_at(
    section: usize,
    row_count: usize,
    row_width: usize,
    y: f64,
    scroll_pixels: f64,
) -> Option<usize> {
    (0..row_count).find(|position| {
        let row_y =
            settings_row_y(section, *position, row_width, scroll_pixels) as f64;
        let row_height = if section == 0 && *position == 0 {
            (settings_theme_selector_extra_height(row_width) + 50) as f64
        } else {
            40.0
        };
        (row_y..row_y + row_height).contains(&y)
    })
}

fn settings_close_button(layout_width: usize) -> UiBounds {
    UiBounds {
        x: layout_width.saturating_sub(142),
        y: 82,
        width: 112,
        height: 32,
    }
}

const SIDEBAR_INSTANCE_START: usize = 328;
const SIDEBAR_INSTANCE_STRIDE: usize = 44;

fn sidebar_footer_positions(height: usize) -> [usize; 3] {
    [
        height.saturating_sub(166),
        height.saturating_sub(112),
        height.saturating_sub(58),
    ]
}

fn sidebar_instance_position(index: usize) -> usize {
    SIDEBAR_INSTANCE_START + index * SIDEBAR_INSTANCE_STRIDE
}

fn navigation_rail_button(y: usize) -> UiBounds {
    UiBounds {
        x: 8,
        y,
        width: 48,
        height: 48,
    }
}

fn sidebar_instance_button(index: usize) -> UiBounds {
    UiBounds {
        x: 12,
        y: sidebar_instance_position(index),
        width: 40,
        height: 40,
    }
}

fn sidebar_footer_button(height: usize, index: usize) -> UiBounds {
    UiBounds {
        x: 8,
        y: sidebar_footer_positions(height)[index.min(2)],
        width: 48,
        height: 48,
    }
}

fn sidebar_instance_capacity(height: usize) -> usize {
    let create_y = sidebar_footer_positions(height)[0];
    let available = create_y
        .saturating_sub(8)
        .saturating_sub(SIDEBAR_INSTANCE_START);
    if available < 40 {
        0
    } else {
        (available - 40) / SIDEBAR_INSTANCE_STRIDE + 1
    }
}

#[derive(Clone, Copy)]
struct BrowseSearchLayout {
    query_x: usize,
    query_width: usize,
    sort_x: usize,
    sort_width: usize,
}

fn browse_search_layout(layout_width: usize) -> BrowseSearchLayout {
    let available = layout_width.saturating_sub(312);
    let view_width = if available >= 216 {
        (available / 3).min(112)
    } else {
        0
    };
    let gap = if view_width > 0 { 8 } else { 0 };
    BrowseSearchLayout {
        query_x: 292,
        query_width: available,
        sort_x: 292,
        sort_width: available.saturating_sub(view_width + gap).min(200),
    }
}

fn browse_view_layout(layout_width: usize) -> (usize, usize) {
    let available = layout_width.saturating_sub(312);
    let width = if available >= 216 {
        (available / 3).min(112)
    } else {
        0
    };
    let sort = browse_search_layout(layout_width);
    let gap = if width > 0 { 8 } else { 0 };
    (sort.sort_x + sort.sort_width + gap, width)
}

fn browse_hide_installed_bounds(width: usize, filter: u8) -> Option<UiBounds> {
    (width >= 850 && filter == 4).then(|| UiBounds {
        x: width - 220,
        y: 218,
        width: 200,
        height: 28,
    })
}

struct BrowseResultsLayout {
    viewport: UiBounds,
    card_height: usize,
    narrow: bool,
    content_right: usize,
    compact_icon: bool,
}

impl BrowseResultsLayout {
    fn new(width: usize, height: usize) -> Self {
        let card_width = width.saturating_sub(324);
        let (_, screen_width) = render::map_page_span(292, card_width, width);
        let narrow = screen_width < 550;
        Self {
            viewport: UiBounds {
                x: 292,
                y: 246,
                width: card_width,
                height: height.saturating_sub(298),
            },
            card_height: if narrow { 196 } else { 132 },
            narrow,
            content_right: width,
            compact_icon: screen_width < 850,
        }
    }

    fn logical_width(&self, screen_width: usize) -> usize {
        let right = self.content_right;
        if right <= 292 {
            return screen_width;
        }
        (screen_width * (right - 292)).div_ceil(right - 88)
    }

    fn icon_width(&self) -> usize {
        self.logical_width(if self.compact_icon { 64 } else { 100 })
    }

    fn stride(&self) -> usize {
        self.card_height + 12
    }

    fn max_scroll(&self, count: usize) -> f64 {
        let trailing_gap = if count == 0 { 0 } else { 12 };
        count
            .saturating_mul(self.stride())
            .saturating_sub(trailing_gap + self.viewport.height) as f64
    }

    fn visible_rows(&self, scroll: f64) -> std::ops::Range<usize> {
        let scroll = scroll.round() as usize;
        scroll / self.stride()
            ..scroll
                .saturating_add(self.viewport.height)
                .div_ceil(self.stride())
    }

    fn icon_prefetch_rows(
        &self,
        scroll: f64,
        result_count: usize,
    ) -> std::ops::Range<usize> {
        let visible = self.visible_rows(scroll);
        visible.start.saturating_sub(4)
            ..visible.end.saturating_add(6).min(result_count)
    }

    fn card_at(&self, x: f64, y: f64, scroll: f64) -> Option<(usize, f64)> {
        if !self.viewport.contains(x, y) {
            return None;
        }
        let content_y = y - self.viewport.y as f64 + scroll.round();
        let row = (content_y / self.stride() as f64).floor() as usize;
        let within_card = content_y.rem_euclid(self.stride() as f64);
        (within_card < self.card_height as f64).then_some((row, within_card))
    }

    fn action(&self, card_y: usize) -> UiBounds {
        let width = self.logical_width(176);
        UiBounds {
            x: if self.narrow {
                self.viewport.x + 16
            } else {
                self.viewport.x + self.viewport.width.saturating_sub(width + 12)
            },
            y: card_y + if self.narrow { 150 } else { 16 },
            width,
            height: 32,
        }
    }

    fn pagination(&self) -> [UiBounds; 2] {
        let right = self.viewport.x + self.viewport.width;
        let button_width = self
            .viewport
            .width
            .saturating_sub(8)
            .saturating_div(2)
            .min(96);
        let gap = if button_width > 0 { 8 } else { 0 };
        let y = self.viewport.y + self.viewport.height + 10;
        [
            right.saturating_sub(button_width * 2 + gap),
            right.saturating_sub(button_width),
        ]
        .map(|x| UiBounds {
            x,
            y,
            width: button_width,
            height: 28,
        })
    }
}

fn browse_query_viewport(
    width: usize,
    length: usize,
    cursor: usize,
) -> (usize, usize) {
    let clear_visible = length != 0 && width >= 44;
    let text_width = width.saturating_sub(if clear_visible { 66 } else { 44 });
    let visible_chars = (text_width / 8).max(1);
    let first_visible = cursor
        .min(length)
        .saturating_sub(visible_chars.saturating_sub(1));
    (first_visible, text_width)
}

fn browse_page_controls(
    pending: bool,
    failed: bool,
    offset: u32,
    limit: u32,
    total: u32,
) -> [bool; 2] {
    [
        !pending && !failed && offset > 0,
        !pending && (failed || offset.saturating_add(limit) < total),
    ]
}

fn browse_version_install_button(width: usize, row_y: usize) -> UiBounds {
    UiBounds {
        x: width.saturating_sub(128),
        y: row_y + 9,
        width: 96,
        height: 30,
    }
}

fn browse_version_install_row(
    width: usize,
    height: usize,
    x: f64,
    y: f64,
    scroll: f64,
) -> Option<usize> {
    if !(476.0..height.saturating_sub(18) as f64).contains(&y) {
        return None;
    }
    let content_y = y - 476.0 + scroll.round();
    browse_version_install_button(width, 0)
        .contains(x, content_y.rem_euclid(56.0))
        .then_some((content_y / 56.0) as usize)
}

fn browse_project_type_tabs(
    layout_width: usize,
) -> [(&'static str, usize, usize, u8); 5] {
    if layout_width >= 880 {
        return [
            ("Modpacks", 292, 88, 4),
            ("Mods", 386, 58, 0),
            ("Resource Packs", 450, 132, 1),
            ("Data Packs", 588, 106, 2),
            ("Shaders", 700, 84, 3),
        ];
    }
    let available = layout_width.saturating_sub(312);
    let filter_width = (available / 5).min(84);
    let gap = if available >= 30 { 4 } else { 0 };
    let tabs_width = available.saturating_sub(filter_width + gap);
    let labels = if layout_width < 720 {
        ["Packs", "Mods", "Textures", "Data", "Shaders"]
    } else {
        [
            "Modpacks",
            "Mods",
            "Resource Packs",
            "Data Packs",
            "Shaders",
        ]
    };
    let filters = [4, 0, 1, 2, 3];
    std::array::from_fn(|index| {
        let start = tabs_width * index / 5;
        let end = tabs_width * (index + 1) / 5;
        (labels[index], 292 + start, end - start, filters[index])
    })
}

fn browse_filter_button_layout(layout_width: usize) -> (usize, usize) {
    if layout_width >= 880 {
        return (792, layout_width.saturating_sub(792).min(120));
    }
    let tabs = browse_project_type_tabs(layout_width);
    let (_, x, width, _) = tabs[4];
    let available = layout_width.saturating_sub(312);
    let gap = if available >= 30 { 4 } else { 0 };
    let filter_x = x + width + gap;
    (
        filter_x,
        layout_width.saturating_sub(20).saturating_sub(filter_x),
    )
}

#[derive(Clone, PartialEq, Eq)]
struct BrowseProject {
    id: String,
    title: String,
    description: String,
    icon_url: Option<String>,
    author: String,
    project_type: String,
    downloads: u64,
    followers: u64,
    categories: Vec<String>,
}

impl BrowseProject {
    fn from_search_hit(
        hit: &serde_json::Value,
        requested_type: &str,
    ) -> Option<Self> {
        let string =
            |key: &str| hit.get(key).and_then(serde_json::Value::as_str);
        let number = |key: &str| {
            hit.get(key)
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default()
        };
        let project_types = hit
            .get("project_types")
            .and_then(serde_json::Value::as_array);
        let project_type = if let Some(types) = project_types {
            if types
                .iter()
                .any(|kind| kind.as_str() == Some(requested_type))
            {
                Some(requested_type)
            } else {
                types.iter().find_map(serde_json::Value::as_str)
            }
        } else {
            string("project_type")
        };
        Some(Self {
            id: string("project_id").or_else(|| string("id"))?.to_owned(),
            title: string("name").or_else(|| string("title"))?.to_owned(),
            description: string("summary")
                .or_else(|| string("description"))
                .unwrap_or_default()
                .to_owned(),
            icon_url: string("icon_url").map(str::to_owned),
            author: string("author").unwrap_or("Unknown author").to_owned(),
            project_type: project_type.unwrap_or(requested_type).to_owned(),
            downloads: number("downloads"),
            followers: number("follows"),
            categories: hit
                .get("display_categories")
                .or_else(|| hit.get("categories"))
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect(),
        })
    }
}

fn browse_search_filters(
    project_type: &str,
    versions: &HashSet<String>,
    loaders: &HashSet<String>,
    categories: &HashSet<String>,
    environments: &HashSet<String>,
    category_tags: &[BrowseCategoryTag],
    open_source: Option<bool>,
) -> String {
    let mut clauses = vec![format!("project_types = `{project_type}`")];
    let valid = |value: &str| !value.is_empty() && !value.contains(['`', '\\']);
    let mut alternatives =
        loaders.iter().map(String::as_str).collect::<Vec<_>>();
    let mut required = Vec::new();
    let category_type = browse_category_project_type(project_type);
    for category in categories
        .iter()
        .map(String::as_str)
        .filter(|value| valid(value))
    {
        if category_tags.iter().any(|tag| {
            tag.project_type == category_type
                && tag.name == category
                && tag.is_resolution
        }) {
            alternatives.push(category);
        } else {
            required.push(category);
        }
    }
    required.sort_unstable();
    for category in required {
        clauses.push(format!("categories = `{category}`"));
    }
    for (field, mut values) in [
        (
            "game_versions",
            versions.iter().map(String::as_str).collect::<Vec<_>>(),
        ),
        ("categories", alternatives),
    ] {
        values.retain(|value| valid(value));
        values.sort_unstable();
        values.dedup();
        match values.as_slice() {
            [] => {}
            [value] => clauses.push(format!("{field} = `{value}`")),
            _ => {
                let values = values
                    .into_iter()
                    .map(|value| format!("`{value}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                clauses.push(format!("{field} IN [{values}]"));
            }
        }
    }
    if matches!(project_type, "mod" | "modpack") {
        let values: &[&str] = match (
            environments.contains("Client"),
            environments.contains("Server"),
        ) {
            (true, true) => &[
                "client_only_server_optional",
                "server_only_client_optional",
                "client_and_server",
                "client_or_server",
                "client_or_server_prefers_both",
            ],
            (true, false) => &[
                "client_only",
                "client_only_server_optional",
                "client_or_server_prefers_both",
                "client_or_server",
            ],
            (false, true) => &[
                "server_only",
                "dedicated_server_only",
                "server_only_client_optional",
                "client_or_server_prefers_both",
                "client_or_server",
            ],
            (false, false) => &[],
        };
        if !values.is_empty() {
            let values = values
                .iter()
                .map(|value| format!("`{value}`"))
                .collect::<Vec<_>>()
                .join(", ");
            clauses.push(format!("environment IN [{values}]"));
        }
    }
    match open_source {
        Some(true) => clauses.push("open_source = true".into()),
        Some(false) => clauses.push("open_source NOT IN [true]".into()),
        None => {}
    }
    clauses.join(" AND ")
}

fn browse_excluded_project_ids(ids: &HashSet<String>) -> Option<String> {
    let mut ids: Vec<_> = ids
        .iter()
        .filter(|id| {
            !id.is_empty() && id.bytes().all(|c| c.is_ascii_alphanumeric())
        })
        .collect();
    ids.sort_unstable();
    (!ids.is_empty()).then(|| {
        format!(
            "project_id NOT IN [{}]",
            ids.iter()
                .map(|id| format!("`{id}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn browse_instance_constraints(
    project_type: &str,
    game_version: &str,
    loader: &str,
) -> Vec<String> {
    if project_type == "resourcepack" {
        return Vec::new();
    }
    let loader = loader
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "");
    let vanilla_shader = project_type == "shader" && loader == "vanilla";
    let mut clauses = Vec::new();
    if !vanilla_shader
        && !game_version.is_empty()
        && !game_version.contains(['`', '\\'])
    {
        clauses.push(format!("game_versions = `{game_version}`"));
    }
    if project_type == "mod"
        && matches!(loader.as_str(), "fabric" | "forge" | "quilt" | "neoforge")
    {
        clauses.push(format!("categories = `{loader}`"));
    } else if vanilla_shader {
        clauses.push("categories = `vanilla`".into());
    }
    clauses
}

fn browse_instance_filter_summary(
    project_type: &str,
    game_version: &str,
    loader: &str,
) -> Option<String> {
    let constraints =
        browse_instance_constraints(project_type, game_version, loader);
    let labels: Vec<_> = constraints
        .iter()
        .filter_map(|clause| {
            let value = clause.split_once('`')?.1.strip_suffix('`')?;
            Some(value)
        })
        .collect();
    (!labels.is_empty()).then(|| format!("Instance: {}", labels.join(" · ")))
}

fn browse_category_project_type(project_type: &str) -> &str {
    if matches!(project_type, "datapack" | "plugin") {
        "mod"
    } else {
        project_type
    }
}

#[cfg(test)]
mod browse_search_tests {
    use super::browse_page_controls;
    use super::{
        BrowseCategoryTag, BrowseProject, BrowseResultsLayout,
        browse_filter_sidebar_layout, browse_query_viewport,
        browse_search_filters, hosting_access_row_layout,
        hosting_backup_action_bounds, hosting_backup_countdown_label,
        hosting_backup_creator_name,
        hosting_backup_manual_slots_after_auto_toggle,
        hosting_backup_search_matches, hosting_console_log_bounds,
        hosting_console_max_scroll, hosting_console_visible_range,
        hosting_deleted_backup_matches, hosting_deleted_backup_row_controls,
        hosting_deleted_backup_sort_order, hosting_servers_refresh_button,
        instance_console_clear_bounds, instance_console_search_bounds,
        instance_console_share_bounds, instance_log_action_bounds,
        instance_log_filter_bottom, instance_log_filter_bounds,
        instance_log_search_bounds, log_insight_lines, log_search_line_indices,
        native_console_line_level, native_log_line_matches_filter,
        native_log_match_count, native_log_present_levels,
        toggle_native_log_filter, visible_browse_versions,
    };
    use std::collections::HashSet;

    #[test]
    fn browse_controls_fit_narrow_widths_without_overlapping() {
        use super::{
            browse_filter_button_layout, browse_project_type_tabs,
            browse_search_layout, browse_view_layout,
        };
        for width in [360, 500, 620, 719, 800, 879, 880, 1100] {
            let tabs = browse_project_type_tabs(width);
            let layout = super::BrowseResultsLayout::new(width, 720);
            let [previous, next] = layout.pagination();
            assert!(previous.x >= layout.viewport.x);
            assert!(previous.x + previous.width <= next.x);
            assert!(
                next.x + next.width
                    <= layout.viewport.x + layout.viewport.width
            );
            let popup = super::browse_filter_popup_bounds(width, 720);
            assert!(popup.x >= 292);
            assert!(popup.x + popup.width <= width);
            let screen_x =
                super::render::map_page_span(popup.x, popup.width, width);
            assert!(screen_x.0 >= 64 && screen_x.0 + screen_x.1 <= width);
            let (filter_x, filter_width) = browse_filter_button_layout(width);
            for pair in tabs.windows(2) {
                assert!(pair[0].1 + pair[0].2 <= pair[1].1);
            }
            assert!(tabs[4].1 + tabs[4].2 <= filter_x);
            assert!(filter_x + filter_width <= width);
            let sort = browse_search_layout(width);
            let (view_x, view_width) = browse_view_layout(width);
            assert!(sort.sort_x + sort.sort_width <= view_x);
            assert!(view_x + view_width <= width);
            if let Some(hide) = super::browse_hide_installed_bounds(width, 4) {
                assert!(view_x + view_width <= hide.x);
                assert!(hide.contains(
                    (hide.x + hide.width - 1) as f64,
                    (hide.y + hide.height - 1) as f64
                ));
                assert!(
                    !hide.contains((hide.x + hide.width) as f64, hide.y as f64)
                );
            }
        }
    }

    #[test]
    fn installed_project_exclusions_are_sorted_and_safe() {
        let ids = ["second", "first", "bad`id", "", "bad\\id"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            super::browse_excluded_project_ids(&ids).as_deref(),
            Some("project_id NOT IN [`first`, `second`]")
        );
        assert_eq!(
            super::browse_excluded_project_ids(
                &std::collections::HashSet::new()
            ),
            None
        );
    }

    #[test]
    fn instance_filter_summary_matches_active_constraints() {
        assert_eq!(
            super::browse_instance_filter_summary("mod", "1.21.1", "Fabric")
                .as_deref(),
            Some("Instance: 1.21.1 · fabric")
        );
        assert_eq!(
            super::browse_instance_filter_summary(
                "shader", "1.21.1", "vanilla"
            )
            .as_deref(),
            Some("Instance: vanilla")
        );
        assert_eq!(
            super::browse_instance_filter_summary(
                "resourcepack",
                "1.21.1",
                "fabric"
            ),
            None
        );
    }

    #[test]
    fn instance_browse_applies_version_and_loader_constraints() {
        use super::browse_instance_constraints;
        assert_eq!(
            browse_instance_constraints("mod", "1.21.1", "Neo-Forge"),
            ["game_versions = `1.21.1`", "categories = `neoforge`"]
        );
        assert_eq!(
            browse_instance_constraints("shader", "1.21.1", "vanilla"),
            ["categories = `vanilla`"]
        );
        assert_eq!(
            browse_instance_constraints("shader", "1.21.1", "fabric"),
            ["game_versions = `1.21.1`"]
        );
        assert!(
            browse_instance_constraints("resourcepack", "1.21.1", "fabric")
                .is_empty()
        );
        assert!(
            browse_instance_constraints("mod", "bad`value", "unknown")
                .is_empty()
        );
    }

    #[test]
    fn explicit_install_checks_target_version_and_mod_loader() {
        use super::{BrowseVersion, browse_version_matches_target};
        let mut version = BrowseVersion {
            id: "version".into(),
            name: "Example".into(),
            version_number: "1.0".into(),
            version_type: "release".into(),
            downloads: 0,
            date_published: chrono::DateTime::UNIX_EPOCH,
            game_versions: vec!["1.21.1".into()],
            loaders: vec!["neoforge".into()],
        };
        assert!(browse_version_matches_target(
            "mod",
            &version,
            "1.21.1",
            "Neo-Forge"
        ));
        assert!(!browse_version_matches_target(
            "mod", &version, "1.21.1", "fabric"
        ));
        assert!(!browse_version_matches_target(
            "mod", &version, "1.20.1", "neoforge"
        ));
        assert!(browse_version_matches_target(
            "resourcepack",
            &version,
            "1.21.1",
            "fabric"
        ));
        version.loaders = vec!["datapack".into()];
        assert!(browse_version_matches_target(
            "mod", &version, "1.21.1", "fabric"
        ));

        use super::{BrowseInstallChoice, NativeLibraryCard};
        let instance = NativeLibraryCard {
            icon_path: None,
            game_version: "1.21.1".into(),
            loader: "fabric".into(),
            instance_type: "normal".into(),
            install_state: "installed".into(),
            content_locked: false,
            launch_blocked: false,
        };
        let mut incompatible = version.clone();
        incompatible.id = "older".into();
        incompatible.game_versions = vec!["1.20.1".into()];
        let project = BrowseProject::from_search_hit(&serde_json::json!({
            "project_id":"example", "name":"Example", "project_types":["mod"]
        }), "mod").unwrap();
        let mut choice = BrowseInstallChoice {
            project,
            version_id: None,
            versions: None,
            generation: 1,
            installed: Default::default(),
        };
        // A completed lookup must not wait for another instance's lookup.
        assert!(choice.presence("first").is_none());
        choice.installed.insert("first".into(), Ok(false));
        choice
            .installed
            .insert("failed".into(), Err("unavailable".into()));
        assert_eq!(choice.presence("first"), Some(&Ok(false)));
        assert!(matches!(choice.presence("failed"), Some(Err(_))));
        assert!(choice.presence("pending").is_none());
        assert_eq!(choice.compatibility(&instance), None);
        choice.versions = Some(Ok(vec![version, incompatible]));
        assert_eq!(choice.compatibility(&instance), Some(true));
        choice.version_id = Some("older".into());
        assert_eq!(choice.compatibility(&instance), Some(false));
    }

    #[test]
    fn version_install_uses_the_visible_button_after_scrolling() {
        use super::browse_version_install_row;
        let x = 1000.0;
        assert_eq!(
            browse_version_install_row(1100, 720, x, 485.0, 0.0),
            Some(0)
        );
        assert_eq!(
            browse_version_install_row(1100, 720, x, 485.0, 58.0),
            Some(1)
        );
        assert_eq!(browse_version_install_row(1100, 720, x, 516.0, 0.0), None);
        assert_eq!(browse_version_install_row(1100, 720, x, 475.0, 20.0), None);
    }

    #[test]
    fn install_instance_picker_reaches_rows_beyond_the_first_screen() {
        use super::{INSTALL_INSTANCE_ROW_HEIGHT, InstallInstancePickerLayout};
        let layout = InstallInstancePickerLayout::new(1100, 520, 15);
        let x = (layout.list.x + 1) as f64;
        let top = layout.list.y as f64;
        assert_eq!(layout.row_at(x, top, 0.0), Some(0));
        // During animation the visible row boundary follows the floored draw offset.
        assert_eq!(layout.row_at(x, top + 28.0, 23.7), None);
        assert_eq!(layout.row_at(x, top + 29.0, 23.7), Some(1));
        assert_eq!(
            layout.row_at(
                x,
                top + (INSTALL_INSTANCE_ROW_HEIGHT - 1) as f64,
                0.0
            ),
            None
        );
        assert_eq!(
            layout.row_at(
                x,
                (layout.list.y + layout.list.height - 8) as f64,
                layout.max_scroll(15)
            ),
            Some(14)
        );
        assert_eq!(
            layout.row_at(
                (layout.list.x + layout.list.width - 2) as f64,
                top,
                0.0
            ),
            None
        );
    }

    #[test]
    fn pagination_disables_during_search_and_offers_retry_after_failure() {
        assert_eq!(
            browse_page_controls(true, false, 20, 20, 100),
            [false, false]
        );
        assert_eq!(browse_page_controls(false, true, 20, 20, 0), [false, true]);
        assert_eq!(
            browse_page_controls(false, false, 0, 20, 100),
            [false, true]
        );
        assert_eq!(
            browse_page_controls(false, false, 80, 20, 100),
            [true, false]
        );
    }

    #[test]
    fn decodes_v3_hit_and_prefers_the_selected_project_type() {
        let hit = serde_json::json!({
            "project_id": "project123",
            "name": "Example project",
            "summary": "Example summary",
            "project_types": ["mod", "datapack"],
            "downloads": 42
        });
        let project = BrowseProject::from_search_hit(&hit, "datapack").unwrap();
        assert_eq!(project.id, "project123");
        assert_eq!(project.title, "Example project");
        assert_eq!(project.description, "Example summary");
        assert_eq!(project.project_type, "datapack");
        assert_eq!(project.downloads, 42);
    }

    #[test]
    fn version_toggle_retains_selected_preview_versions() {
        let versions = [
            ("26.3 snapshot".into(), false),
            ("26.2".into(), true),
            ("26.2 rc".into(), false),
        ];
        assert_eq!(
            visible_browse_versions(&versions, false, &HashSet::new()),
            ["26.2"]
        );
        let selected = HashSet::from(["26.2 rc".into()]);
        assert_eq!(
            visible_browse_versions(&versions, false, &selected),
            ["26.2", "26.2 rc"]
        );
        assert_eq!(
            visible_browse_versions(&versions, true, &selected),
            ["26.3 snapshot", "26.2", "26.2 rc"]
        );
    }

    #[test]
    fn builds_v3_filter_groups() {
        assert_eq!(
            browse_search_filters(
                "mod",
                &HashSet::from(["1.21".into(), "1.20.1".into()]),
                &HashSet::from(["fabric".into()]),
                &HashSet::from(["optimization".into()]),
                &HashSet::new(),
                &[],
                None,
            ),
            "project_types = `mod` AND categories = `optimization` AND game_versions IN [`1.20.1`, `1.21`] AND categories = `fabric`"
        );
    }

    #[test]
    fn categories_require_all_tags_while_resolutions_allow_alternatives() {
        let tags = ["16x", "32x"].map(|name| BrowseCategoryTag {
            name: name.into(),
            project_type: "resourcepack".into(),
            is_resolution: true,
        });
        assert_eq!(
            browse_search_filters(
                "resourcepack",
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::from([
                    "32x".into(),
                    "16x".into(),
                    "realistic".into(),
                    "utility".into()
                ]),
                &HashSet::new(),
                &tags,
                None,
            ),
            "project_types = `resourcepack` AND categories = `realistic` AND categories = `utility` AND categories IN [`16x`, `32x`]"
        );
    }

    #[test]
    fn environment_groups_follow_upstream_client_and_server_rules() {
        let query = |kind, sides| {
            browse_search_filters(
                kind,
                &HashSet::new(),
                &HashSet::new(),
                &HashSet::new(),
                &sides,
                &[],
                None,
            )
        };
        assert!(
            query("mod", HashSet::from(["Client".into()]))
                .contains("environment IN [`client_only`,")
        );
        let server = query("modpack", HashSet::from(["Server".into()]));
        assert!(server.contains("`dedicated_server_only`"));
        let both =
            query("mod", HashSet::from(["Client".into(), "Server".into()]));
        assert!(both.contains("`client_and_server`"));
        assert!(!both.contains("`client_only`"));
        assert_eq!(
            query("shader", HashSet::from(["Client".into()])),
            "project_types = `shader`"
        );
    }

    #[test]
    fn open_source_filter_supports_inclusion_and_exclusion() {
        let empty = HashSet::new();
        let query = |license| {
            browse_search_filters(
                "shader",
                &empty,
                &empty,
                &empty,
                &empty,
                &[],
                license,
            )
        };
        assert_eq!(query(None), "project_types = `shader`");
        assert_eq!(
            query(Some(true)),
            "project_types = `shader` AND open_source = true"
        );
        assert_eq!(
            query(Some(false)),
            "project_types = `shader` AND open_source NOT IN [true]"
        );
    }

    #[test]
    fn long_search_query_uses_the_same_visible_start_for_draw_and_click() {
        assert_eq!(browse_query_viewport(146, 30, 30), (21, 80));
        assert_eq!(browse_query_viewport(146, 30, 3), (0, 80));
    }

    #[test]
    fn filter_sidebar_hit_testing_stays_inside_the_scrolling_options() {
        let layout = browse_filter_sidebar_layout(1100, 520, 1, 100, 4);
        let options = layout.options;
        let x = options.x as f64 + 20.0;
        let y = options.y as f64;
        assert_eq!(layout.option_at(x, y - 1.0, 89.9), None);
        assert_eq!(layout.option_at(x, y, 89.9), Some(2));
        assert_eq!(layout.option_at(x, y + 1.0, 89.9), Some(3));
        assert_eq!(
            layout.option_at(x, (options.y + options.height) as f64, 89.9),
            None
        );
        assert_eq!(layout.headings[2].y, options.y + options.height);
        assert_eq!(layout.max_scroll(100), 3000.0 - options.height as f64);
        let short = browse_filter_sidebar_layout(1100, 720, 0, 3, 3);
        assert_eq!(short.options.height, 90);
        let toggle = short.version_toggle.unwrap();
        assert_eq!(toggle.y + toggle.height, short.options.y);
        assert_eq!(
            short.option_at(toggle.x as f64, toggle.y as f64, 0.0),
            None
        );
        assert_eq!(short.max_scroll(3), 0.0);
    }

    #[test]
    fn browse_layout_draws_partial_cards_and_reaches_the_last_card() {
        let narrow = BrowseResultsLayout::new(620, 720);
        assert_eq!(narrow.stride(), 208);
        assert_eq!(narrow.visible_rows(0.0), 0..3);
        assert_eq!(narrow.visible_rows(208.0), 1..4);
        let scroll = narrow.max_scroll(10);
        assert_eq!(scroll, 1646.0);
        assert_eq!(narrow.card_at(320.0, 667.0, scroll), Some((9, 195.0)));
        assert_eq!(narrow.card_at(320.0, 668.0, scroll), None);
        assert_eq!(narrow.card_at(320.0, 445.0, 0.0), None);
        let [previous, next] = narrow.pagination();
        assert!(previous.contains((previous.x + 1) as f64, 705.0));
        assert!(next.contains((next.x + 1) as f64, 705.0));
        assert!(!next.contains((next.x + 1) as f64, 706.0));
        let action = narrow.action(0);
        assert!(action.contains((action.x + action.width - 1) as f64, 181.0));
        assert!(!action.contains((action.x + action.width) as f64, 181.0));
        assert_eq!(BrowseResultsLayout::new(800, 720).stride(), 144);
        assert_eq!(BrowseResultsLayout::new(1100, 720).stride(), 144);
    }

    #[test]
    fn browse_icon_prefetch_stays_ahead_of_the_viewport_and_is_bounded() {
        let layout = BrowseResultsLayout::new(1100, 720);
        assert_eq!(layout.icon_prefetch_rows(0.0, 20), 0..9);
        assert_eq!(layout.icon_prefetch_rows(500.0, 10), 0..10);
        assert_eq!(layout.icon_prefetch_rows(layout.max_scroll(10), 10), 2..10);
        assert_eq!(layout.icon_prefetch_rows(0.0, 0), 0..0);
    }

    #[test]
    fn hosting_refresh_hit_target_covers_the_full_visible_button() {
        for width in [460, 760, 1100] {
            let button = hosting_servers_refresh_button(width);
            assert!(button.x + button.width <= width);
            assert!(
                button.contains(button.x as f64 + 1.0, button.y as f64 + 1.0)
            );
            assert!(button.contains(
                (button.x + button.width - 1) as f64,
                (button.y + button.height - 1) as f64,
            ));
            assert!(!button.contains(
                (button.x + button.width) as f64,
                (button.y + button.height - 1) as f64,
            ));
        }
    }

    #[test]
    fn hosting_backup_action_hit_targets_match_the_drawn_row_buttons() {
        let row_right = 812.0;
        let bounds = hosting_backup_action_bounds(row_right);
        let widths = [80.0, 72.0, 64.0, 56.0];
        assert_eq!(bounds[0].0, row_right - 284.0);
        assert_eq!(bounds[3].1, row_right);
        for (index, (left, right)) in bounds.iter().enumerate() {
            assert_eq!(right - left, widths[index]);
            assert!(left < right);
            if index > 0 {
                assert!(bounds[index - 1].1 < *left);
            }
        }
    }

    #[test]
    fn deleted_backup_selection_copy_and_recovery_targets_are_aligned() {
        let [selection, copy_id, recover] =
            hosting_deleted_backup_row_controls(396, 300);
        assert_eq!(
            (selection.x, selection.y, selection.width, selection.height),
            (120, 319, 18, 18)
        );
        assert_eq!(
            (copy_id.x, copy_id.y, copy_id.width, copy_id.height),
            (142, 317, 24, 20)
        );
        assert_eq!(
            (recover.x, recover.y, recover.width, recover.height),
            (284, 311, 100, 32)
        );
        assert!(selection.contains(120.0, 319.0));
        assert!(copy_id.contains(165.0, 336.0));
        assert!(recover.contains(383.0, 342.0));
        assert!(!recover.contains(384.0, 342.0));
    }

    #[test]
    fn hosting_backup_search_matches_names_without_case_sensitivity() {
        assert!(hosting_backup_search_matches("Before restore", "restore"));
        assert!(hosting_backup_search_matches("World Backup", "world"));
        assert!(hosting_backup_search_matches("World Backup", ""));
        assert!(!hosting_backup_search_matches("World Backup", "server"));
    }

    #[test]
    fn backup_manual_slots_survive_disabling_automatic_backups() {
        assert_eq!(
            hosting_backup_manual_slots_after_auto_toggle(false, 3, 10),
            3
        );
        assert_eq!(
            hosting_backup_manual_slots_after_auto_toggle(true, 3, 10),
            3
        );
        assert_eq!(
            hosting_backup_manual_slots_after_auto_toggle(true, 10, 10),
            5
        );
        assert_eq!(
            hosting_backup_manual_slots_after_auto_toggle(true, 4, 4),
            0
        );
    }

    #[test]
    fn deleted_backup_filters_cover_reason_type_recoverability_and_id_search() {
        let backup = native_backend::HostingDeletedBackup {
            id: "backup-17".into(),
            name: "World snapshot".into(),
            created_at: String::new(),
            status: "done".into(),
            automated: true,
            created_by: None,
            scheduled_delete: String::new(),
            undelete_deadline: String::new(),
            deleted_suspension: true,
            can_undelete: true,
            permanently_deleted: false,
        };
        assert!(hosting_deleted_backup_matches(
            &backup,
            "backup-17",
            0,
            0,
            0
        ));
        assert!(hosting_deleted_backup_matches(&backup, "", 1, 1, 1));
        assert!(!hosting_deleted_backup_matches(&backup, "", 2, 0, 0));
        assert!(!hosting_deleted_backup_matches(&backup, "", 0, 2, 0));
        assert!(!hosting_deleted_backup_matches(&backup, "", 0, 0, 2));
    }

    #[test]
    fn deleted_backup_sort_modes_cover_time_name_reason_and_deadline() {
        let newer = native_backend::HostingDeletedBackup {
            id: "new".into(),
            name: "Zebra".into(),
            created_at: "2026-11-02T00:00:00Z".into(),
            status: "done".into(),
            automated: false,
            created_by: None,
            scheduled_delete: String::new(),
            undelete_deadline: "2026-11-12T00:00:00Z".into(),
            deleted_suspension: false,
            can_undelete: true,
            permanently_deleted: false,
        };
        let older = native_backend::HostingDeletedBackup {
            id: "old".into(),
            name: "Apple".into(),
            created_at: "2026-11-01T00:00:00Z".into(),
            status: "done".into(),
            automated: true,
            created_by: None,
            scheduled_delete: String::new(),
            undelete_deadline: "2026-11-10T00:00:00Z".into(),
            deleted_suspension: true,
            can_undelete: true,
            permanently_deleted: false,
        };
        assert_eq!(
            hosting_deleted_backup_sort_order(&newer, &older, 0),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            hosting_deleted_backup_sort_order(&newer, &older, 1),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            hosting_deleted_backup_sort_order(&newer, &older, 2),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            hosting_deleted_backup_sort_order(&newer, &older, 3),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            hosting_deleted_backup_sort_order(&newer, &older, 4),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            hosting_deleted_backup_sort_order(&newer, &older, 5),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn backup_countdown_labels_distinguish_creation_and_rotation() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-21T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(
            hosting_backup_countdown_label(
                Some("2026-11-21T13:05:00Z"),
                now,
                false,
            ),
            "Next in 1h 5m"
        );
        assert_eq!(
            hosting_backup_countdown_label(
                Some("2026-11-21T13:05:00Z"),
                now,
                true,
            ),
            "Replacing oldest in 1h 5m"
        );
        assert_eq!(
            hosting_backup_countdown_label(
                Some("2026-11-21T12:00:30Z"),
                now,
                false,
            ),
            "Next backup starting soon"
        );
        assert_eq!(
            hosting_backup_countdown_label(None, now, false),
            "Next scheduled"
        );
    }

    #[test]
    fn backup_creator_labels_cover_users_support_and_automatic_backups() {
        let user = native_backend::HostingBackupUserInfo {
            id: "user-1".into(),
            username: "Frank".into(),
            avatar_url: None,
        };
        let support = native_backend::HostingBackupUserInfo {
            id: "support".into(),
            username: "service".into(),
            avatar_url: None,
        };
        assert_eq!(
            hosting_backup_creator_name(Some(&user), false).as_deref(),
            Some("Frank")
        );
        assert_eq!(
            hosting_backup_creator_name(Some(&support), false).as_deref(),
            Some("Support")
        );
        assert_eq!(
            hosting_backup_creator_name(None, true).as_deref(),
            Some("Automatic")
        );
        assert_eq!(hosting_backup_creator_name(None, false), None);
    }

    #[test]
    fn hosting_console_scroll_exposes_older_and_latest_lines_without_overflow()
    {
        assert_eq!(hosting_console_max_scroll(0, 180), 0.0);
        assert_eq!(hosting_console_max_scroll(5, 180), 0.0);
        assert_eq!(hosting_console_max_scroll(20, 180), 140.0);
        assert_eq!(hosting_console_visible_range(20, 64, 0.0), 0..6);
        assert_eq!(hosting_console_visible_range(20, 64, 256.0), 16..20);
        assert_eq!(hosting_console_visible_range(3, 180, 0.0), 0..3);
    }

    #[test]
    fn hosting_console_scroll_bounds_match_the_console_viewport() {
        let compact = hosting_console_log_bounds(1100, 720, false);
        assert_eq!(
            (compact.x, compact.y, compact.width, compact.height),
            (112, 484, 908, 126)
        );
        let sidebar = hosting_console_log_bounds(1100, 720, true);
        assert_eq!(
            (sidebar.x, sidebar.y, sidebar.width, sidebar.height),
            (112, 484, 624, 126)
        );
    }

    #[test]
    fn crash_insights_summarize_problems_and_first_solution() {
        let response = serde_json::json!({
            "analysis": {
                "problems": [{
                    "message": "A mod failed to load",
                    "solutions": [{ "message": "Remove the incompatible mod" }]
                }]
            }
        });
        assert_eq!(
            log_insight_lines(&response),
            [
                "A mod failed to load".to_owned(),
                "Fix: Remove the incompatible mod".to_owned()
            ]
        );
        assert!(log_insight_lines(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn log_action_hit_bounds_match_the_visible_buttons() {
        assert!(instance_log_action_bounds(779, false).is_none());
        let [copy, share, delete, analyse] =
            instance_log_action_bounds(1100, false).unwrap();
        assert_eq!(
            (copy.x, copy.y, copy.width, copy.height),
            (726, 146, 78, 28)
        );
        assert_eq!(
            (share.x, share.y, share.width, share.height),
            (810, 146, 78, 28)
        );
        assert_eq!(
            (delete.x, delete.y, delete.width, delete.height),
            (894, 146, 78, 28)
        );
        assert_eq!(
            (analyse.x, analyse.y, analyse.width, analyse.height),
            (978, 146, 90, 28)
        );
        assert!(copy.contains(726.0, 146.0));
        assert!(!copy.contains(804.0, 160.0));
        assert!(share.contains(887.9, 173.9));
        assert!(!share.contains(888.0, 160.0));
        assert!(delete.contains(971.9, 173.9));
        assert!(!delete.contains(972.0, 160.0));
        assert!(analyse.contains(1067.9, 173.9));
        assert!(!analyse.contains(1068.0, 160.0));
        let compact = instance_log_action_bounds(900, false).unwrap();
        assert_eq!(compact[0].x, 292);
        assert_eq!(compact[0].y, 196);
        assert_eq!(compact[3].x + compact[3].width, 634);
        let compact_with_insights =
            instance_log_action_bounds(900, true).unwrap();
        assert_eq!(compact_with_insights[0].y, 296);
    }

    #[test]
    fn log_search_filters_case_insensitively_and_tracks_responsive_bounds() {
        let log = "Starting game\nERROR: missing mod\nerror: bad config";
        assert_eq!(log_search_line_indices(log, "error"), [1, 2]);
        assert_eq!(log_search_line_indices(log, " "), [0, 1, 2]);
        assert!(log_search_line_indices(log, "absent").is_empty());
        assert_eq!(native_log_match_count(log, "ERROR", 0), 2);
        assert_eq!(native_log_match_count(log, " absent ", 0), 0);
        for (width, analysis, expected_y) in [
            (1100, false, 196),
            (900, false, 234),
            (900, true, 334),
            (700, true, 300),
        ] {
            let bounds = instance_log_search_bounds(width, analysis);
            assert_eq!(bounds.y, expected_y);
            assert_eq!(bounds.height, 30);
        }
        let console = instance_console_search_bounds(1100);
        assert_eq!(
            (console.x, console.y, console.width, console.height),
            (292, 194, 776, 30)
        );
        let clear = instance_console_clear_bounds(1100);
        assert_eq!(
            (clear.x, clear.y, clear.width, clear.height),
            (872, 154, 64, 28)
        );
        let share = instance_console_share_bounds(1100);
        assert_eq!(
            (share.x, share.y, share.width, share.height),
            (786, 154, 72, 28)
        );
        let filter_y = instance_log_filter_bounds(1100, 232, 0);
        assert_eq!(filter_y.len(), 4);
        assert!(filter_y.iter().all(|(_, bounds)| bounds.y == 232));
        let wrapped_filters = instance_log_filter_bounds(
            500,
            232,
            super::LOG_LEVEL_DEBUG | super::LOG_LEVEL_TRACE,
        );
        assert!(wrapped_filters.iter().any(|(_, bounds)| bounds.y > 232));
        assert_eq!(
            instance_log_filter_bottom(
                500,
                232,
                super::LOG_LEVEL_DEBUG | super::LOG_LEVEL_TRACE
            ),
            328
        );
    }

    #[test]
    fn native_log_level_detection_and_pills_match_upstream_rules() {
        let lines = [
            "[main/INFO] Launching",
            "[main/WARN] Deprecated option",
            "[main/ERROR] Failure",
            "[main/DEBUG] Details",
            "[main/TRACE] Trace details",
            "[System] [CHAT] hello",
            "An exception occurred",
            "unclassified output",
            "\tat net.minecraft.Main.run(Main.java:1)",
        ];
        assert_eq!(
            native_console_line_level(lines[0]),
            Some(super::LOG_LEVEL_INFO)
        );
        assert_eq!(
            native_console_line_level(lines[1]),
            Some(super::LOG_LEVEL_WARN)
        );
        assert_eq!(
            native_console_line_level(lines[2]),
            Some(super::LOG_LEVEL_ERROR)
        );
        assert_eq!(
            native_console_line_level(lines[3]),
            Some(super::LOG_LEVEL_DEBUG)
        );
        assert_eq!(
            native_console_line_level(lines[4]),
            Some(super::LOG_LEVEL_TRACE)
        );
        assert_eq!(
            native_console_line_level(lines[5]),
            Some(super::LOG_LEVEL_INFO)
        );
        assert_eq!(native_console_line_level(lines[7]), None);
        assert_eq!(
            native_console_line_level(lines[8]),
            Some(super::LOG_LEVEL_ERROR)
        );
        assert_eq!(
            native_log_present_levels(&lines.join("\n")),
            super::LOG_LEVEL_DEBUG | super::LOG_LEVEL_TRACE
        );
        assert!(native_log_line_matches_filter(
            lines[7],
            super::LOG_LEVEL_INFO
        ));
        assert_eq!(
            native_log_match_count(
                &lines.join("\n"),
                "",
                super::LOG_LEVEL_ERROR | super::LOG_LEVEL_WARN
            ),
            3
        );
        assert_eq!(toggle_native_log_filter(0, super::LOG_LEVEL_ERROR), 1);
        assert_eq!(
            toggle_native_log_filter(
                super::LOG_LEVEL_ERROR | super::LOG_LEVEL_WARN,
                super::LOG_LEVEL_ERROR
            ),
            super::LOG_LEVEL_WARN
        );
        assert_eq!(
            toggle_native_log_filter(
                super::LOG_LEVEL_INFO,
                super::LOG_LEVEL_INFO
            ),
            0
        );
    }

    #[test]
    fn hosting_access_row_targets_match_the_visible_role_and_remove_buttons() {
        for panel_width in [316, 460, 560, 616, 940] {
            let row = hosting_access_row_layout(panel_width, 264);
            let viewport_right = panel_width + 80;
            assert!(row.role.x >= 112);
            assert!(row.remove.x + row.remove.width <= viewport_right);
            assert!(row.user.x + row.user.width <= row.role.x);
            assert!(row.cancel_invite.x >= row.user.x);
            assert!(
                row.cancel_invite.x + row.cancel_invite.width
                    <= row.user.x + row.user.width
            );
            assert_eq!(row.cancel_invite.width, 24);
            assert_eq!(row.cancel_invite.height, row.user.height);
            assert!(row.role.contains(
                row.role.x as f64 + 1.0,
                row.role.y as f64 + 1.0,
            ));
            assert!(!row.role.contains(
                (row.role.x + row.role.width) as f64,
                row.role.y as f64 + 1.0,
            ));
            assert!(row.remove.contains(
                row.remove.x as f64 + 1.0,
                row.remove.y as f64 + 1.0,
            ));
            assert!(!row.remove.contains(
                (row.remove.x + row.remove.width) as f64,
                row.remove.y as f64 + 1.0,
            ));
            if row.joined.width > 0 {
                assert!(row.user.x + row.user.width <= row.joined.x);
                assert!(row.joined.x + row.joined.width <= row.role.x);
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum NativeRoute {
    Library { group: Option<String> },
    Hosting { server_id: Option<String> },
    Discover { project: Option<BrowseProject> },
    Skins,
    Screenshots,
    Instance { id: String, name: String, tab: u8 },
    UserProfile { user_id: String },
    Settings { java: bool, section: usize },
    Accounts { tab: u8 },
    CreateInstance,
}

#[derive(Clone)]
struct BrowseVersion {
    id: String,
    name: String,
    version_number: String,
    version_type: String,
    date_published: chrono::DateTime<chrono::Utc>,
    downloads: u32,
    game_versions: Vec<String>,
    loaders: Vec<String>,
}

impl From<theseus::data::Version> for BrowseVersion {
    fn from(version: theseus::data::Version) -> Self {
        Self {
            id: version.id,
            name: version.name,
            version_number: version.version_number,
            version_type: version.version_type,
            date_published: version.date_published,
            downloads: version.downloads,
            game_versions: version.game_versions,
            loaders: version.loaders,
        }
    }
}

struct BrowseInstallChoice {
    project: BrowseProject,
    version_id: Option<String>,
    versions: Option<Result<Vec<BrowseVersion>, String>>,
    generation: u64,
    installed: HashMap<String, Result<bool, String>>,
}

impl BrowseInstallChoice {
    fn presence(&self, id: &str) -> Option<&Result<bool, String>> {
        self.installed.get(id)
    }

    fn compatibility(&self, card: &NativeLibraryCard) -> Option<bool> {
        self.versions.as_ref()?.as_ref().ok().map(|versions| {
            versions.iter().any(|version| {
                self.version_id.as_deref().is_none_or(|id| id == version.id)
                    && browse_version_matches_target(
                        &self.project.project_type,
                        version,
                        &card.game_version,
                        &card.loader,
                    )
            })
        })
    }

    fn compatibility_label(&self, card: &NativeLibraryCard) -> &'static str {
        match self.compatibility(card) {
            Some(true) => " · Compatible",
            Some(false) if self.version_id.is_some() => {
                " · Selected version differs"
            }
            Some(false) => " · Review versions",
            None if self.versions.is_none() => " · Checking compatibility…",
            None => " · Compatibility unavailable",
        }
    }
}

struct BrowseInstallOverride {
    project: BrowseProject,
    version_id: String,
    version_label: String,
    instance_id: String,
    target_label: String,
}

fn browse_version_matches_target(
    project_type: &str,
    version: &BrowseVersion,
    game_version: &str,
    loader: &str,
) -> bool {
    if !version
        .game_versions
        .iter()
        .any(|value| value == game_version)
    {
        return false;
    }
    if project_type != "mod" {
        return true;
    }
    let normalize =
        |value: &str| value.to_ascii_lowercase().replace([' ', '-', '_'], "");
    let loader = normalize(loader);
    version.loaders.iter().any(|candidate| {
        let candidate = normalize(candidate);
        candidate == loader
            || candidate == "datapack"
            || matches!(
                (loader.as_str(), candidate.as_str()),
                ("neo", "neoforge") | ("neoforge", "neo")
            )
    })
}

struct BrowseProjectDetails {
    slug: String,
    body: String,
    categories: Vec<String>,
    additional_categories: Vec<String>,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    client_side: String,
    server_side: String,
    license_name: String,
    license_url: Option<String>,
    published: String,
    updated: String,
    links: Vec<(String, String)>,
    gallery: Vec<BrowseGalleryImage>,
    versions: Vec<BrowseVersion>,
}

#[derive(Clone, Debug)]
struct BrowseDescriptionLink {
    line_index: usize,
    start_char: usize,
    end_char: usize,
    text_x: usize,
    scale: usize,
    url: String,
}

#[derive(Clone, Debug)]
struct BrowseDescriptionStyle {
    line_index: usize,
    start_char: usize,
    end_char: usize,
    bold: bool,
    italic: bool,
}

#[derive(Clone, Debug)]
struct BrowseDescriptionImage {
    line_index: usize,
    cache_key: String,
    url: String,
    alt: String,
}

fn browse_version_filter_buttons(
    layout_width: usize,
) -> Vec<(u8, usize, usize)> {
    let right = layout_width.saturating_sub(20);
    let mut x = 292usize;
    let mut buttons = Vec::new();
    for (group, width) in [(0, 110), (1, 152), (2, 104)] {
        if x + width > right {
            break;
        }
        buttons.push((group, x, width));
        x += width + 8;
    }
    buttons
}

fn browse_version_filter_options(
    details: &BrowseProjectDetails,
    group: u8,
) -> Vec<String> {
    let mut values = Vec::new();
    match group {
        0 => {
            for channel in ["release", "beta", "alpha"] {
                if details
                    .versions
                    .iter()
                    .any(|version| version.version_type == channel)
                {
                    values.push(channel.to_owned());
                }
            }
        }
        1 => {
            for version in &details.versions {
                for game_version in &version.game_versions {
                    if !values.contains(game_version) {
                        values.push(game_version.clone());
                    }
                }
            }
        }
        _ => {
            for version in &details.versions {
                for loader in &version.loaders {
                    if !values.contains(loader) {
                        values.push(loader.clone());
                    }
                }
            }
        }
    }
    values
}

fn browse_filtered_version_indices(
    details: &BrowseProjectDetails,
    channels: &HashSet<String>,
    game_versions: &HashSet<String>,
    loaders: &HashSet<String>,
) -> Vec<usize> {
    details
        .versions
        .iter()
        .enumerate()
        .filter(|(_, version)| {
            (channels.is_empty() || channels.contains(&version.version_type))
                && (game_versions.is_empty()
                    || version
                        .game_versions
                        .iter()
                        .any(|value| game_versions.contains(value)))
                && (loaders.is_empty()
                    || version
                        .loaders
                        .iter()
                        .any(|value| loaders.contains(value)))
        })
        .map(|(index, _)| index)
        .collect()
}

#[derive(Clone)]
struct BrowseGalleryImage {
    url: String,
    raw_url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    created: Option<String>,
}

#[derive(Default)]
struct BrowseSearchPage {
    projects: Vec<BrowseProject>,
    total_hits: u32,
}

struct BrowseInstallNotice {
    text: String,
    failed: bool,
    expires: std::time::Instant,
}

#[derive(Clone, Default)]
struct BrowseFilterOptions {
    game_versions: Vec<(String, bool)>,
    loaders: Vec<(String, Vec<String>)>,
    categories: Vec<BrowseCategoryTag>,
}

fn visible_browse_versions(
    versions: &[(String, bool)],
    show_all: bool,
    selected: &HashSet<String>,
) -> Vec<String> {
    versions
        .iter()
        .filter(|(name, release)| {
            show_all || *release || selected.contains(name)
        })
        .map(|(name, _)| name.clone())
        .collect()
}

#[derive(Clone)]
struct BrowseCategoryTag {
    project_type: String,
    name: String,
    is_resolution: bool,
}

#[derive(Clone)]
struct NativeImportSource {
    launcher: theseus::pack::import::ImportLauncherType,
    path: std::path::PathBuf,
    folders: Vec<String>,
}

fn import_launcher_options()
-> [(&'static str, theseus::pack::import::ImportLauncherType); 5] {
    use theseus::pack::import::ImportLauncherType as T;
    [
        ("Prism Launcher", T::PrismLauncher),
        ("MultiMC", T::MultiMC),
        ("ATLauncher", T::ATLauncher),
        ("GDLauncher", T::GDLauncher),
        ("CurseForge", T::Curseforge),
    ]
}

#[derive(Clone)]
struct NativeLibraryCard {
    icon_path: Option<String>,
    game_version: String,
    loader: String,
    instance_type: String,
    install_state: String,
    content_locked: bool,
    launch_blocked: bool,
}

fn can_pick_install_instance(card: Option<&NativeLibraryCard>) -> bool {
    card.is_none_or(|card| {
        !card.content_locked && card.install_state == "installed"
    })
}

fn browse_install_override_layout(
    width: usize,
    height: usize,
) -> (UiBounds, [UiBounds; 2]) {
    let panel_width = width.saturating_sub(32).min(550);
    let x = width.saturating_sub(panel_width) / 2;
    let y = height.saturating_sub(212) / 2;
    (
        UiBounds {
            x,
            y,
            width: panel_width,
            height: 212,
        },
        [
            UiBounds {
                x: x + panel_width.saturating_sub(244),
                y: y + 158,
                width: 96,
                height: 36,
            },
            UiBounds {
                x: x + panel_width.saturating_sub(136),
                y: y + 158,
                width: 120,
                height: 36,
            },
        ],
    )
}

fn hosting_backup_prompt_layout(
    width: usize,
    height: usize,
) -> (UiBounds, UiBounds, UiBounds, UiBounds) {
    let panel_width = width.saturating_sub(32).min(520);
    let panel_height = 252usize.min(height.saturating_sub(24));
    let x = width.saturating_sub(panel_width) / 2;
    let y = height.saturating_sub(panel_height) / 2;
    (
        UiBounds {
            x,
            y,
            width: panel_width,
            height: panel_height,
        },
        UiBounds {
            x: x + 20,
            y: y + 96,
            width: panel_width.saturating_sub(40),
            height: 40,
        },
        UiBounds {
            x: x + panel_width.saturating_sub(240),
            y: y + 196,
            width: 100,
            height: 36,
        },
        UiBounds {
            x: x + panel_width.saturating_sub(132),
            y: y + 196,
            width: 112,
            height: 36,
        },
    )
}

fn hosting_backup_rotation_layout(
    width: usize,
    height: usize,
) -> (UiBounds, UiBounds, UiBounds) {
    let panel_width = width.saturating_sub(32).min(520);
    let panel_height = 224usize.min(height.saturating_sub(24));
    let x = width.saturating_sub(panel_width) / 2;
    let y = height.saturating_sub(panel_height) / 2;
    (
        UiBounds {
            x,
            y,
            width: panel_width,
            height: panel_height,
        },
        UiBounds {
            x: x + panel_width.saturating_sub(240),
            y: y + 168,
            width: 100,
            height: 36,
        },
        UiBounds {
            x: x + panel_width.saturating_sub(132),
            y: y + 168,
            width: 112,
            height: 36,
        },
    )
}

fn hosting_backup_action_bounds(row_right: f64) -> [(f64, f64); 4] {
    let x = row_right - 284.0;
    [
        (x, x + 80.0),
        (x + 84.0, x + 156.0),
        (x + 160.0, x + 224.0),
        (x + 228.0, x + 284.0),
    ]
}

fn hosting_deleted_backup_row_controls(
    row_right: usize,
    row_y: usize,
) -> [UiBounds; 3] {
    [
        UiBounds {
            x: 120,
            y: row_y + 19,
            width: 18,
            height: 18,
        },
        UiBounds {
            x: 142,
            y: row_y + 17,
            width: 24,
            height: 20,
        },
        UiBounds {
            x: row_right.saturating_sub(112),
            y: row_y + 11,
            width: 100,
            height: 32,
        },
    ]
}

fn hosting_backup_search_matches(name: &str, query: &str) -> bool {
    query.is_empty() || name.to_lowercase().contains(query)
}

fn hosting_deleted_backup_matches(
    backup: &native_backend::HostingDeletedBackup,
    query: &str,
    reason_filter: u8,
    type_filter: u8,
    recoverability_filter: u8,
) -> bool {
    let query_matches = query.is_empty()
        || backup.name.to_lowercase().contains(query)
        || backup.id.to_lowercase().contains(query);
    let reason_matches = match reason_filter {
        1 => backup.deleted_suspension,
        2 => !backup.deleted_suspension,
        _ => true,
    };
    let type_matches = match type_filter {
        1 => backup.automated,
        2 => !backup.automated,
        _ => true,
    };
    let recovery_matches = match recoverability_filter {
        1 => backup.can_undelete,
        2 => !backup.can_undelete,
        _ => true,
    };
    query_matches && reason_matches && type_matches && recovery_matches
}

fn hosting_deleted_backup_sort_order(
    left: &native_backend::HostingDeletedBackup,
    right: &native_backend::HostingDeletedBackup,
    sort: u8,
) -> std::cmp::Ordering {
    match sort {
        1 => left.created_at.cmp(&right.created_at),
        2 => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        3 => right.name.to_lowercase().cmp(&left.name.to_lowercase()),
        4 => left.deleted_suspension.cmp(&right.deleted_suspension),
        5 => match (left.can_undelete, right.can_undelete) {
            (true, true) => {
                left.undelete_deadline.cmp(&right.undelete_deadline)
            }
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => left.created_at.cmp(&right.created_at),
        },
        _ => right.created_at.cmp(&left.created_at),
    }
}

fn hosting_backup_interval_choices(saved: u32) -> Vec<u32> {
    let mut choices = vec![6, 12, 24, 48, 168];
    if saved > 0 && !choices.contains(&saved) {
        choices.push(saved);
        choices.sort_unstable();
    }
    choices
}

fn hosting_backup_manual_slots_after_auto_toggle(
    auto_enabled: bool,
    manual_slots: usize,
    slot_limit: usize,
) -> usize {
    if auto_enabled && manual_slots >= slot_limit {
        slot_limit.saturating_sub(5)
    } else {
        manual_slots.min(slot_limit)
    }
}

fn hosting_backup_interval_label(hours: u32) -> String {
    match hours {
        168 => "7d".into(),
        hours if hours % 24 == 0 => format!("{}d", hours / 24),
        _ => format!("{hours}h"),
    }
}

fn hosting_backup_countdown_label(
    next_at: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
    slots_full: bool,
) -> String {
    let Some(next_at) = next_at
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&chrono::Utc))
    else {
        return "Next scheduled".into();
    };
    let seconds = (next_at - now).num_seconds();
    if seconds < 60 {
        return if slots_full {
            "Replacing oldest soon".into()
        } else {
            "Next backup starting soon".into()
        };
    }
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let remainder = minutes % 60;
    let duration = if hours > 0 {
        format!("{hours}h {remainder}m")
    } else {
        format!("{minutes}m")
    };
    if slots_full {
        format!("Replacing oldest in {duration}")
    } else {
        format!("Next in {duration}")
    }
}

fn hosting_backup_creator_name(
    user: Option<&native_backend::HostingBackupUserInfo>,
    automated: bool,
) -> Option<String> {
    user.map(|user| {
        if user.id == "support" {
            "Support".to_owned()
        } else {
            user.username.clone()
        }
    })
    .or_else(|| automated.then(|| "Automatic".to_owned()))
}

const HOSTING_CONSOLE_ROW_HEIGHT: usize = 16;

fn hosting_console_max_scroll(
    line_count: usize,
    viewport_height: usize,
) -> f64 {
    line_count
        .saturating_mul(HOSTING_CONSOLE_ROW_HEIGHT)
        .saturating_sub(viewport_height) as f64
}

fn hosting_console_visible_range(
    line_count: usize,
    viewport_height: usize,
    scroll_pixels: f64,
) -> std::ops::Range<usize> {
    let first = (scroll_pixels / HOSTING_CONSOLE_ROW_HEIGHT as f64)
        .floor()
        .max(0.0) as usize;
    let visible = viewport_height / HOSTING_CONSOLE_ROW_HEIGHT + 2;
    first.min(line_count)..first.saturating_add(visible).min(line_count)
}

fn hosting_console_log_bounds(
    window_width: usize,
    window_height: usize,
    sidebar_visible: bool,
) -> UiBounds {
    let library_width =
        window_width.saturating_sub(if sidebar_visible { 300 } else { 0 });
    let panel_width = library_width.saturating_sub(144).min(940);
    let console_bottom = window_height.saturating_sub(66);
    UiBounds {
        x: 112,
        y: 484,
        width: panel_width.saturating_sub(32),
        height: console_bottom.saturating_sub(528),
    }
}

fn log_insight_lines(analysis: &serde_json::Value) -> Vec<String> {
    let Some(problems) = analysis
        .get("analysis")
        .and_then(|value| value.get("problems"))
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for problem in problems.iter().take(3) {
        if let Some(message) =
            problem.get("message").and_then(serde_json::Value::as_str)
        {
            lines.push(message.to_owned());
        }
        if let Some(solutions) = problem
            .get("solutions")
            .and_then(serde_json::Value::as_array)
        {
            if let Some(message) = solutions
                .first()
                .and_then(|solution| solution.get("message"))
                .and_then(serde_json::Value::as_str)
            {
                lines.push(format!("Fix: {message}"));
            }
        }
    }
    lines
}

fn instance_log_action_bounds(
    window_width: usize,
    analysis_visible: bool,
) -> Option<[UiBounds; 4]> {
    if window_width < 780 {
        return None;
    }
    let (top, left) = if window_width >= 1000 {
        (146, window_width.saturating_sub(374))
    } else if analysis_visible {
        (296, 292)
    } else {
        (196, 292)
    };
    Some([
        UiBounds {
            x: left,
            y: top,
            width: 78,
            height: 28,
        },
        UiBounds {
            x: left + 84,
            y: top,
            width: 78,
            height: 28,
        },
        UiBounds {
            x: left + 168,
            y: top,
            width: 78,
            height: 28,
        },
        UiBounds {
            x: left + 252,
            y: top,
            width: 90,
            height: 28,
        },
    ])
}

fn instance_log_search_bounds(
    window_width: usize,
    analysis_visible: bool,
) -> UiBounds {
    let compact_action_row = (780..1000).contains(&window_width);
    let y = match (analysis_visible, compact_action_row) {
        (false, false) => 196,
        (false, true) => 234,
        (true, false) => 300,
        (true, true) => 334,
    };
    UiBounds {
        x: 292,
        y,
        width: window_width.saturating_sub(324),
        height: 30,
    }
}

fn instance_console_search_bounds(window_width: usize) -> UiBounds {
    UiBounds {
        x: 292,
        y: 194,
        width: window_width.saturating_sub(324),
        height: 30,
    }
}

fn instance_console_clear_bounds(window_width: usize) -> UiBounds {
    UiBounds {
        x: window_width.saturating_sub(228),
        y: 154,
        width: 64,
        height: 28,
    }
}

fn instance_console_share_bounds(window_width: usize) -> UiBounds {
    UiBounds {
        x: window_width.saturating_sub(314),
        y: 154,
        width: 72,
        height: 28,
    }
}

#[cfg(test)]
fn log_search_line_indices(text: &str, query: &str) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        return (0..text.lines().count()).collect();
    }
    let query = query.to_lowercase();
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            line.to_lowercase().contains(&query).then_some(index)
        })
        .collect()
}

const LOG_LEVEL_ERROR: u8 = 1;
const LOG_LEVEL_WARN: u8 = 2;
const LOG_LEVEL_INFO: u8 = 4;
const LOG_LEVEL_DEBUG: u8 = 8;
const LOG_LEVEL_TRACE: u8 = 16;

fn native_console_line_level(line: &str) -> Option<u8> {
    if line.contains("/INFO") || line.contains("[System] [CHAT]") {
        Some(LOG_LEVEL_INFO)
    } else if line.contains("/WARN") {
        Some(LOG_LEVEL_WARN)
    } else if line.contains("/DEBUG") {
        Some(LOG_LEVEL_DEBUG)
    } else if line.contains("/TRACE") {
        Some(LOG_LEVEL_TRACE)
    } else if ["/ERROR", "Exception:", ":?]", "Error", "[thread", "\tat"]
        .iter()
        .any(|trigger| line.contains(trigger))
    {
        Some(LOG_LEVEL_ERROR)
    } else {
        None
    }
}

fn native_log_present_levels(text: &str) -> u8 {
    text.lines().fold(0, |levels, line| {
        levels
            | native_console_line_level(line)
                .filter(|level| {
                    matches!(*level, LOG_LEVEL_DEBUG | LOG_LEVEL_TRACE)
                })
                .unwrap_or(0)
    })
}

fn native_log_line_matches_filter(line: &str, active_filter: u8) -> bool {
    active_filter == 0
        || active_filter
            & native_console_line_level(line).unwrap_or(LOG_LEVEL_INFO)
            != 0
}

fn native_log_match_count(text: &str, query: &str, active_filter: u8) -> usize {
    let query = query.trim().to_lowercase();
    text.lines()
        .filter(|line| native_log_line_matches_filter(line, active_filter))
        .filter(|line| query.is_empty() || line.to_lowercase().contains(&query))
        .count()
}

fn toggle_native_log_filter(active_filter: u8, clicked_filter: u8) -> u8 {
    if clicked_filter == 0 {
        0
    } else if active_filter == 0 {
        clicked_filter
    } else {
        active_filter ^ clicked_filter
    }
}

fn instance_log_filter_bounds(
    window_width: usize,
    top: usize,
    present_levels: u8,
) -> Vec<(u8, UiBounds)> {
    let options = [
        (0, 48),
        (LOG_LEVEL_ERROR, 62),
        (LOG_LEVEL_WARN, 56),
        (LOG_LEVEL_INFO, 54),
        (LOG_LEVEL_DEBUG, 64),
        (LOG_LEVEL_TRACE, 60),
    ];
    let right = window_width.saturating_sub(24);
    let mut x = 292usize;
    let mut y = top;
    let mut bounds = Vec::new();
    for (level, width) in options.into_iter().filter(|(level, _)| {
        *level == 0
            || *level & (LOG_LEVEL_ERROR | LOG_LEVEL_WARN | LOG_LEVEL_INFO) != 0
            || present_levels & *level != 0
    }) {
        if x > 292 && x.saturating_add(width) > right {
            x = 292;
            y = y.saturating_add(34);
        }
        bounds.push((
            level,
            UiBounds {
                x,
                y,
                width,
                height: 28,
            },
        ));
        x = x.saturating_add(width + 6);
    }
    bounds
}

fn instance_log_filter_bottom(
    window_width: usize,
    top: usize,
    present_levels: u8,
) -> usize {
    instance_log_filter_bounds(window_width, top, present_levels)
        .last()
        .map_or(top, |(_, bounds)| bounds.y + bounds.height)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeLibraryCardAction {
    Stop,
    Repair,
    Play,
    Installing,
    None,
}

fn native_library_card_action(
    running: bool,
    installing: bool,
    installed: bool,
    content_locked: bool,
    launch_blocked: bool,
    selection_active: bool,
    hovered_or_focused: bool,
) -> NativeLibraryCardAction {
    if running {
        NativeLibraryCardAction::Stop
    } else if installing {
        NativeLibraryCardAction::Installing
    } else if selection_active {
        NativeLibraryCardAction::None
    } else if !installed {
        if hovered_or_focused && !content_locked {
            NativeLibraryCardAction::Repair
        } else {
            NativeLibraryCardAction::None
        }
    } else if hovered_or_focused && !launch_blocked {
        NativeLibraryCardAction::Play
    } else {
        NativeLibraryCardAction::None
    }
}

fn library_membership_toggle_visible(
    membership_mode: bool,
    action: NativeLibraryCardAction,
) -> bool {
    membership_mode && action != NativeLibraryCardAction::Stop
}

#[derive(Clone, Copy, Default)]
struct NativeInstanceMetrics {
    created: i64,
    modified: i64,
    last_played: Option<i64>,
    playtime: u64,
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum NativeInstanceSortKey {
    Timestamp(Option<i64>),
    Number(u64),
    Text(String),
}

type NativeInstanceEntry = (
    String,
    String,
    Vec<String>,
    Option<String>,
    String,
    String,
    NativeInstanceMetrics,
    String,
    String,
    bool,
    bool,
);

fn native_instance_entry(
    item: theseus::data::InstanceMetadata,
) -> NativeInstanceEntry {
    let instance_type = match &item.link {
        theseus::data::InstanceLink::ServerProject { .. }
        | theseus::data::InstanceLink::ServerProjectModpack { .. }
        | theseus::data::InstanceLink::ModrinthHosting { .. } => "Server",
        theseus::data::InstanceLink::ModrinthModpack { .. }
        | theseus::data::InstanceLink::ImportedModpack { .. }
        | theseus::data::InstanceLink::SharedInstance { .. } => "Modpack",
        theseus::data::InstanceLink::Unmanaged => "Custom",
    }
    .to_owned();
    let metrics = NativeInstanceMetrics {
        created: item.instance.created.timestamp(),
        modified: item.instance.modified.timestamp(),
        last_played: item.instance.last_played.map(|time| time.timestamp()),
        playtime: item.instance.submitted_time_played,
    };
    let install_state = format!("{:?}", item.instance.install_stage);
    let install_state = match install_state.as_str() {
        "Installed" | "PackInstalled" => "installed",
        "MinecraftInstalling" | "PackInstalling" => "installing",
        _ => "not_installed",
    }
    .to_owned();
    (
        item.instance.id,
        item.instance.name,
        item.group_ids,
        item.instance.icon_path,
        item.applied_content_set.game_version,
        item.applied_content_set.loader.as_str().to_owned(),
        metrics,
        instance_type,
        install_state,
        item.content_locked,
        item.launch_blocked,
    )
}

fn native_installed_modpack_ids(
    items: &[theseus::data::InstanceMetadata],
) -> HashSet<String> {
    items
        .iter()
        .filter_map(|item| match &item.link {
            theseus::data::InstanceLink::ModrinthModpack {
                project_id, ..
            } => Some(project_id.clone()),
            theseus::data::InstanceLink::ImportedModpack {
                project_id: Some(project_id),
                ..
            } => Some(project_id.clone()),
            theseus::data::InstanceLink::SharedInstance {
                modpack_project_id: Some(project_id),
                ..
            } => Some(project_id.clone()),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Copy)]
struct UiBounds {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl UiBounds {
    fn contains(&self, x: f64, y: f64) -> bool {
        (self.x as f64..(self.x + self.width) as f64).contains(&x)
            && (self.y as f64..(self.y + self.height) as f64).contains(&y)
    }

    fn on_screen(self, content_right: usize, scroll: usize) -> Self {
        let (x, width) =
            render::map_page_span(self.x, self.width, content_right);
        Self {
            x,
            width,
            y: self.y.saturating_sub(scroll),
            ..self
        }
    }
}

fn hosting_servers_refresh_button(width: usize) -> UiBounds {
    UiBounds {
        x: width.saturating_sub(136),
        y: 78,
        width: 104,
        height: 32,
    }
}

struct HostingAccessRowLayout {
    user: UiBounds,
    cancel_invite: UiBounds,
    role: UiBounds,
    joined: UiBounds,
    remove: UiBounds,
}

fn hosting_access_row_layout(
    panel_width: usize,
    row_y: usize,
) -> HostingAccessRowLayout {
    let remove_x = panel_width.saturating_add(4);
    let role_x = remove_x.saturating_sub(96);
    let joined_width = if panel_width >= 560 { 70 } else { 0 };
    let joined_x = role_x.saturating_sub(100);
    let user_width = if joined_width > 0 {
        joined_x.saturating_sub(144)
    } else {
        role_x.saturating_sub(144)
    };
    HostingAccessRowLayout {
        user: UiBounds {
            x: 128,
            y: row_y + 8,
            width: user_width,
            height: 32,
        },
        cancel_invite: UiBounds {
            x: 128 + user_width.saturating_sub(28),
            y: row_y + 8,
            width: user_width.min(24),
            height: 32,
        },
        role: UiBounds {
            x: role_x,
            y: row_y + 8,
            width: 80,
            height: 32,
        },
        joined: UiBounds {
            x: joined_x,
            y: row_y + 8,
            width: joined_width,
            height: 32,
        },
        remove: UiBounds {
            x: remove_x,
            y: row_y + 8,
            width: 72,
            height: 32,
        },
    }
}

const INSTALL_INSTANCE_ROW_HEIGHT: usize = 52;

struct InstallInstancePickerLayout {
    panel: UiBounds,
    list: UiBounds,
    close: UiBounds,
}

impl InstallInstancePickerLayout {
    fn new(width: usize, height: usize, count: usize) -> Self {
        let panel_width = width.saturating_sub(32).min(500);
        let list_height = count
            .max(1)
            .saturating_mul(INSTALL_INSTANCE_ROW_HEIGHT)
            .min(height.saturating_sub(128))
            .min(8 * INSTALL_INSTANCE_ROW_HEIGHT);
        let panel_height = list_height + 96;
        let x = width.saturating_sub(panel_width) / 2;
        let y = height.saturating_sub(panel_height) / 2;
        Self {
            panel: UiBounds {
                x,
                y,
                width: panel_width,
                height: panel_height,
            },
            list: UiBounds {
                x: x + 16,
                y: y + 80,
                width: panel_width.saturating_sub(32),
                height: list_height,
            },
            close: UiBounds {
                x: x + panel_width.saturating_sub(46),
                y: y + 12,
                width: 34,
                height: 32,
            },
        }
    }

    fn max_scroll(&self, count: usize) -> f64 {
        count
            .saturating_mul(INSTALL_INSTANCE_ROW_HEIGHT)
            .saturating_sub(self.list.height) as f64
    }

    fn row_at(&self, x: f64, y: f64, scroll: f64) -> Option<usize> {
        if !self.list.contains(x, y) {
            return None;
        }
        if x >= (self.list.x + self.list.width.saturating_sub(8)) as f64 {
            return None;
        }
        let local_y = (y - self.list.y as f64 + scroll.floor()) as usize;
        (local_y % INSTALL_INSTANCE_ROW_HEIGHT
            < INSTALL_INSTANCE_ROW_HEIGHT - 6)
            .then_some(local_y / INSTALL_INSTANCE_ROW_HEIGHT)
    }
}

fn instance_back_bounds(wide: bool) -> UiBounds {
    let (x, width) = if wide { (248, 38) } else { (292, 78) };
    UiBounds {
        x,
        y: 80,
        width,
        height: 30,
    }
}

fn instance_primary_bounds(width: usize) -> UiBounds {
    UiBounds {
        x: width.saturating_sub(382),
        y: 82,
        width: 112,
        height: 28,
    }
}

const BROWSE_LICENSE_GROUP: u8 = 3;
const BROWSE_ENVIRONMENT_GROUP: u8 = 4;
const BROWSE_LICENSE_OPTIONS: [&str; 2] =
    ["Open source", "Exclude open source"];
const BROWSE_FILTER_GROUPS: [&str; 5] = [
    "Game versions",
    "Loaders",
    "Categories",
    "License",
    "Environment",
];
const BROWSE_FILTER_ROW_HEIGHT: usize = 30;

struct BrowseFilterSidebarLayout {
    pane: UiBounds,
    reset: UiBounds,
    headings: [UiBounds; BROWSE_FILTER_GROUPS.len()],
    options: UiBounds,
    version_toggle: Option<UiBounds>,
}

fn browse_filter_sidebar_layout(
    width: usize,
    height: usize,
    group: u8,
    count: usize,
    group_count: usize,
) -> BrowseFilterSidebarLayout {
    let x = width.saturating_sub(300);
    let headings_top = 112;
    let heading_height = 38;
    let toggle_height = if group == 0 {
        BROWSE_FILTER_ROW_HEIGHT
    } else {
        0
    };
    let options_height = height
        .saturating_sub(
            headings_top + group_count * heading_height + 12 + toggle_height,
        )
        .min(count.saturating_mul(BROWSE_FILTER_ROW_HEIGHT).max(60));
    let group = usize::from(group).min(group_count - 1);
    let headings = std::array::from_fn(|index| UiBounds {
        x: x + 12,
        y: headings_top
            + index * heading_height
            + if index > group {
                options_height + toggle_height
            } else {
                0
            },
        width: 276,
        height: if index < group_count {
            heading_height
        } else {
            0
        },
    });
    BrowseFilterSidebarLayout {
        pane: UiBounds {
            x,
            y: 56,
            width: 300,
            height: height.saturating_sub(56),
        },
        reset: UiBounds {
            x: x + 210,
            y: 68,
            width: 76,
            height: 32,
        },
        headings,
        options: UiBounds {
            x: x + 12,
            y: headings[group].y + heading_height + toggle_height,
            width: 276,
            height: options_height,
        },
        version_toggle: (toggle_height > 0).then_some(UiBounds {
            x: x + 12,
            y: headings[group].y + heading_height,
            width: 276,
            height: toggle_height,
        }),
    }
}

fn browse_popup_filter_rows(
    panel: UiBounds,
    group: u8,
) -> (Option<UiBounds>, UiBounds) {
    let mut options = UiBounds {
        x: panel.x + 8,
        y: panel.y + 40,
        width: panel.width.saturating_sub(16),
        height: panel.height.saturating_sub(48),
    };
    let version_toggle = (group == 0).then(|| {
        let toggle = UiBounds {
            height: 28,
            ..options
        };
        options.y += toggle.height;
        options.height = options.height.saturating_sub(toggle.height);
        toggle
    });
    (version_toggle, options)
}

fn browse_filter_popup_bounds(width: usize, height: usize) -> UiBounds {
    let panel_width = width.saturating_sub(304).min(300);
    let x = width.saturating_sub(12 + panel_width).max(292);
    UiBounds {
        x,
        y: 178,
        width: panel_width,
        height: height.saturating_sub(190).min(440),
    }
}

fn browse_filter_popup_tab(
    x: usize,
    y: usize,
    width: usize,
    count: usize,
    index: usize,
) -> UiBounds {
    let stride = width.saturating_sub(16) / count;
    UiBounds {
        x: x + 8 + index * stride,
        y: y + 8,
        width: stride.saturating_sub(4),
        height: 24,
    }
}

impl BrowseFilterSidebarLayout {
    fn max_scroll(&self, count: usize) -> f64 {
        count
            .saturating_mul(BROWSE_FILTER_ROW_HEIGHT)
            .saturating_sub(self.options.height) as f64
    }

    fn option_at(&self, x: f64, y: f64, scroll: f64) -> Option<usize> {
        self.options.contains(x, y).then(|| {
            ((y - self.options.y as f64 + scroll.floor())
                / BROWSE_FILTER_ROW_HEIGHT as f64) as usize
        })
    }
}

struct LibraryFilterPopupLayout {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    hidden_toggle_y: usize,
    loader_rows_y: usize,
    type_header_y: usize,
    type_rows_y: usize,
    game_version_header_y: usize,
    game_version_rows_y: usize,
    state_header_y: usize,
    state_rows_y: usize,
    clear_y: usize,
}

fn library_filter_popup_layout(
    anchor: UiBounds,
    width: usize,
    height: usize,
    loader_count: usize,
) -> LibraryFilterPopupLayout {
    let panel_width = 236usize.min(width.saturating_sub(24));
    let panel_height = 422 + loader_count * 24;
    let x = anchor.x.min(width.saturating_sub(panel_width + 12));
    let y = (anchor.y + anchor.height + 6)
        .min(height.saturating_sub(panel_height + 12))
        .max(56);
    LibraryFilterPopupLayout {
        x,
        y,
        width: panel_width,
        height: panel_height,
        hidden_toggle_y: y + 36,
        loader_rows_y: y + 60,
        type_header_y: y + 68 + loader_count * 24,
        type_rows_y: y + 92 + loader_count * 24,
        game_version_header_y: y + 172 + loader_count * 24,
        game_version_rows_y: y + 196 + loader_count * 24,
        state_header_y: y + 276 + loader_count * 24,
        state_rows_y: y + 300 + loader_count * 24,
        clear_y: y + panel_height - 34,
    }
}

fn library_visibility_matches(
    instance_id: &str,
    hidden_instance_ids: &HashSet<String>,
    show_hidden_instances: bool,
) -> bool {
    show_hidden_instances || !hidden_instance_ids.contains(instance_id)
}

fn library_icon_prefetch_ranges(
    current_scroll: usize,
    target_scroll: usize,
    viewport_height: usize,
    lead: usize,
) -> [(usize, usize); 2] {
    [current_scroll, target_scroll].map(|scroll| {
        (
            scroll.saturating_sub(lead),
            scroll.saturating_add(viewport_height).saturating_add(lead),
        )
    })
}

fn library_dropdown_layout(
    anchor: UiBounds,
    window_width: usize,
    window_height: usize,
    option_count: usize,
    menu_width: usize,
) -> UiBounds {
    let width = menu_width.min(window_width.saturating_sub(24));
    let height = option_count.saturating_mul(32).saturating_add(12);
    let x = anchor.x.min(window_width.saturating_sub(width + 12));
    let below = anchor.y.saturating_add(anchor.height + 6);
    let y = if below.saturating_add(height) <= window_height {
        below
    } else {
        anchor.y.saturating_sub(height + 6)
    };
    UiBounds {
        x,
        y: y.min(window_height.saturating_sub(height + 12)),
        width,
        height,
    }
}

fn library_dropdown_row(bounds: UiBounds, index: usize) -> UiBounds {
    UiBounds {
        x: bounds.x + 6,
        y: bounds.y + 6 + index * 32,
        width: bounds.width.saturating_sub(12),
        height: 28,
    }
}

struct LibraryToolbarLayout {
    search: UiBounds,
    new_group: UiBounds,
    new_instance: UiBounds,
    import_pack: UiBounds,
    sort: UiBounds,
    filter: UiBounds,
    group: UiBounds,
    cards_top: usize,
}

impl LibraryToolbarLayout {
    fn title_y(&self) -> usize {
        self.search.y.saturating_sub(if self.search.width < 216 {
            124
        } else {
            60
        })
    }

    fn view_tabs(&self) -> [UiBounds; 2] {
        if self.search.width < 216 {
            return [
                UiBounds {
                    x: 292,
                    y: self.search.y.saturating_sub(64),
                    width: self.search.width,
                    height: 28,
                },
                UiBounds {
                    x: 292,
                    y: self.search.y.saturating_sub(32),
                    width: self.search.width,
                    height: 28,
                },
            ];
        }
        [(292, 112), (412, 88)].map(|(x, width)| UiBounds {
            x,
            y: self.search.y.saturating_sub(36),
            width,
            height: 28,
        })
    }

    fn refresh(&self, width: usize) -> UiBounds {
        UiBounds {
            x: width.saturating_sub(112),
            y: self.title_y().saturating_sub(5),
            width: 88,
            height: 28,
        }
    }

    fn membership(&self, width: usize) -> UiBounds {
        if self.search.width < 216 {
            return UiBounds {
                x: 292,
                y: self.title_y() + 28,
                width: self.search.width,
                height: 28,
            };
        }
        UiBounds {
            x: self.refresh(width).x.saturating_sub(146),
            width: 138,
            ..self.refresh(width)
        }
    }
}

struct WelcomeScreenLayout {
    center_x: usize,
    top: usize,
    title_y: usize,
    create_y: usize,
    import_y: usize,
    options_y: usize,
    show_mark: bool,
}

#[derive(Clone, Copy)]
struct LibraryGridLayout {
    columns: usize,
    card_width: usize,
    art_height: usize,
    card_height: usize,
    gap: usize,
    cards_top: usize,
    jump_in: bool,
}

enum NativeLibraryGridRow {
    Header(String, String, usize),
    Cards(Vec<usize>),
}

fn library_grid_row_height(
    row: &NativeLibraryGridRow,
    grid: LibraryGridLayout,
) -> usize {
    match row {
        NativeLibraryGridRow::Header(..) => 40,
        NativeLibraryGridRow::Cards(..) => grid.card_height + grid.gap,
    }
}

fn eased_scroll_position(current: f64, target: f64, frame_seconds: f64) -> f64 {
    let difference = target - current;
    if difference.abs() <= 0.5 {
        target
    } else {
        let easing = 1.0 - (-frame_seconds / 0.05).exp();
        current + difference * easing
    }
}

fn relative_play_time(timestamp: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let elapsed = now.saturating_sub(timestamp.max(0) as u64);
    if elapsed < 60 {
        "Just now".into()
    } else if elapsed < 3600 {
        format!("{}m ago", elapsed / 60)
    } else if elapsed < 86_400 {
        format!("{}h ago", elapsed / 3600)
    } else {
        format!("{}d ago", elapsed / 86_400)
    }
}

fn append_server_motd_text(value: &serde_json::Value, output: &mut String) {
    if output.len() >= 512 {
        return;
    }
    match value {
        serde_json::Value::String(text) => output.push_str(text),
        serde_json::Value::Array(values) => {
            for value in values {
                append_server_motd_text(value, output);
            }
        }
        serde_json::Value::Object(properties) => {
            if let Some(text) = properties
                .get("text")
                .or_else(|| properties.get("translate"))
                .and_then(serde_json::Value::as_str)
            {
                output.push_str(text);
            }
            if let Some(extra) = properties.get("extra") {
                append_server_motd_text(extra, output);
            }
        }
        _ => {}
    }
}

fn server_motd_text(status: &worlds::ServerStatus) -> Option<String> {
    let raw = status.description.as_ref()?;
    let value = serde_json::from_str::<serde_json::Value>(raw.get()).ok()?;
    let mut output = String::new();
    append_server_motd_text(&value, &mut output);
    let text = output.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

fn gallery_zoom_after_scroll(current_zoom: f32, scroll_delta: f64) -> f32 {
    (current_zoom * 1.1_f32.powf((-scroll_delta / 58.0) as f32)).clamp(1.0, 5.0)
}

fn content_scroll_max(item_count: usize, viewport_height: usize) -> f64 {
    let visible_rows = (viewport_height / CONTENT_ROW_HEIGHT).max(1);
    item_count
        .saturating_sub(visible_rows)
        .saturating_mul(CONTENT_ROW_HEIGHT) as f64
}

fn profile_projects_max_scroll(
    project_count: usize,
    window_height: usize,
) -> f64 {
    project_count
        .saturating_mul(62)
        .saturating_sub(window_height.saturating_sub(384)) as f64
}

fn profile_project_types(projects: &[serde_json::Value]) -> Vec<String> {
    let mut types = projects
        .iter()
        .flat_map(|project| {
            project
                .get("project_types")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| {
                    project
                        .get("project_type")
                        .and_then(serde_json::Value::as_str)
                        .map(|value| vec![value.to_owned()])
                        .unwrap_or_default()
                })
        })
        .collect::<Vec<_>>();
    types.sort();
    types.dedup();
    types
}

fn profile_badge_labels(
    profile: Option<&serde_json::Value>,
    projects: &[serde_json::Value],
) -> Vec<&'static str> {
    let Some(profile) = profile else {
        return Vec::new();
    };
    let role = profile
        .get("role")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let flags = profile
        .get("badges")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let created = profile
        .get("created")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let mut labels = Vec::new();
    if matches!(role, "admin" | "moderator") {
        labels.push("Modrinth Team");
    }
    if role == "moderator" {
        labels.push("Content Moderator");
    }
    if flags & (1 << 4) != 0
        || (!created.is_empty() && created < "2020-11-30T08:00:00")
    {
        labels.push("Alpha Tester");
    }
    if !created.is_empty() && created < "2022-02-27T08:00:00" {
        labels.push("Beta Tester");
    }
    if flags & 1 != 0 {
        labels.push("Modrinth+ Member");
    }
    let downloads = projects
        .iter()
        .filter_map(|p| p.get("downloads").and_then(serde_json::Value::as_u64))
        .sum::<u64>();
    for (threshold, label) in [
        (500_000_000, "500m Downloads"),
        (250_000_000, "250m Downloads"),
        (100_000_000, "100m Downloads"),
        (50_000_000, "50m Downloads"),
        (25_000_000, "25m Downloads"),
        (10_000_000, "10m Downloads"),
        (1_000_000, "1m Downloads"),
    ] {
        if downloads >= threshold {
            labels.push(label);
            break;
        }
    }
    for (kind, cutoff, flag, label) in [
        (
            "modpack",
            "2022-05-23T00:57:00",
            1 << 1,
            "Early Modpack Adopter",
        ),
        (
            "resourcepack",
            "2022-08-27T23:03:00",
            1 << 2,
            "Early Resource Pack Adopter",
        ),
        (
            "plugin",
            "2022-08-27T23:03:00",
            1 << 3,
            "Early Plugin Adopter",
        ),
        (
            "datapack",
            "2023-01-08T02:00:00",
            0,
            "Early Data Pack Adopter",
        ),
        ("shader", "2023-01-08T02:00:00", 0, "Early Shader Adopter"),
        ("server", "2026-03-04T01:33:00", 0, "Early Server Adopter"),
    ] {
        let earliest = projects
            .iter()
            .filter(|p| {
                p.get("project_types")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|ts| {
                        ts.iter().any(|t| t.as_str() == Some(kind))
                    })
                    || p.get("project_type").and_then(serde_json::Value::as_str)
                        == Some(kind)
            })
            .filter_map(|p| {
                p.get("published")
                    .or_else(|| p.get("created"))
                    .and_then(serde_json::Value::as_str)
            })
            .min();
        if (flag != 0 && flags & flag != 0)
            || earliest.is_some_and(|date| date < cutoff)
        {
            labels.push(label);
        }
    }
    labels
}

fn profile_project_matches(
    project: &serde_json::Value,
    project_type: &str,
) -> bool {
    project
        .get("project_types")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|types| {
            types
                .iter()
                .any(|value| value.as_str() == Some(project_type))
        })
        || project
            .get("project_type")
            .and_then(serde_json::Value::as_str)
            == Some(project_type)
}

fn filtered_profile_project_indices(
    projects: &[serde_json::Value],
    project_type: Option<&str>,
) -> Vec<usize> {
    projects
        .iter()
        .enumerate()
        .filter_map(|(index, project)| {
            project_type
                .is_none_or(|kind| profile_project_matches(project, kind))
                .then_some(index)
        })
        .collect()
}

fn adjust_memory_setting(
    current: u32,
    delta_steps: isize,
    maximum_mb: u32,
) -> u32 {
    let maximum_mb = maximum_mb.max(512);
    (i64::from(current) + delta_steps.saturating_mul(64) as i64)
        .clamp(512, i64::from(maximum_mb)) as u32
}

fn visible_jump_in_count(
    selected_count: usize,
    available_count: usize,
    columns: usize,
) -> usize {
    let maximum = available_count.min(MAX_JUMP_IN_ITEMS);
    if maximum == 0 {
        0
    } else {
        selected_count.clamp(columns.min(maximum).max(1), maximum)
    }
}

#[derive(Clone)]
enum NativeJumpInItem {
    Instance { index: usize, newly_added: bool },
    World(theseus::worlds::WorldWithInstance),
}

fn library_grid_layout(
    width: usize,
    compact_cards: bool,
    jump_in_rows: usize,
) -> LibraryGridLayout {
    let toolbar = if jump_in_rows > 0 {
        home_library_toolbar_layout(width, jump_in_rows)
    } else {
        library_toolbar_layout(width)
    };
    let content_width = width.saturating_sub(324);
    let gap = 12;
    let minimum_width = if compact_cards {
        240
    } else if width < 1280 {
        128
    } else {
        160
    };
    let columns = ((content_width + gap) / (minimum_width + gap)).max(1);
    let card_width =
        content_width.saturating_sub(gap * columns.saturating_sub(1)) / columns;
    let art_height = if compact_cards {
        40
    } else {
        card_width.saturating_sub(24)
    };
    LibraryGridLayout {
        columns,
        card_width,
        art_height,
        card_height: if compact_cards { 64 } else { art_height + 78 },
        gap,
        cards_top: toolbar.cards_top,
        jump_in: jump_in_rows > 0,
    }
}

fn minecraft_skin_grid_layout(width: usize) -> (usize, usize, usize, usize) {
    let content_width = width.saturating_sub(324);
    let gap = 12;
    let columns = ((content_width + gap) / 232).max(1);
    let card_width =
        content_width.saturating_sub(gap * columns.saturating_sub(1)) / columns;
    (columns, card_width, 224, gap)
}

fn updated_library_selection_range(
    visible_order: &[String],
    anchor_id: Option<&str>,
    target_id: &str,
    selected: &HashSet<String>,
) -> HashSet<String> {
    let mut updated = selected.clone();
    let positions = anchor_id.and_then(|anchor| {
        Some((
            visible_order.iter().position(|id| id == anchor)?,
            visible_order.iter().position(|id| id == target_id)?,
        ))
    });
    let Some((anchor_index, target_index)) = positions else {
        if !updated.remove(target_id) {
            updated.insert(target_id.to_owned());
        }
        return updated;
    };

    let start = anchor_index.min(target_index);
    let end = anchor_index.max(target_index);
    let range = &visible_order[start..=end];
    if updated.contains(target_id) {
        for instance_id in range {
            updated.remove(instance_id);
        }
    } else {
        updated.extend(range.iter().cloned());
    }
    updated
}

fn toggle_library_instance_selection(
    selected: &mut HashSet<String>,
    instance_id: &str,
) {
    if !selected.remove(instance_id) {
        selected.insert(instance_id.to_owned());
    }
}

fn library_selection_button_bounds(
    card_x: usize,
    card_y: usize,
    card_width: usize,
    compact: bool,
) -> UiBounds {
    let size = if compact { 20 } else { 24 };
    UiBounds {
        x: card_x + card_width.saturating_sub(if compact { 29 } else { 45 }),
        y: card_y + if compact { 15 } else { 22 },
        width: size,
        height: size,
    }
}

fn library_toolbar_layout(width: usize) -> LibraryToolbarLayout {
    let left = 292;
    let right = width.saturating_sub(20).max(left + 1);
    let available = right.saturating_sub(left);
    let group_width = 118;
    let instance_width = 134;
    let import_width = 112;
    let gap = 8;
    let narrow = available < 614;
    if narrow {
        if available < group_width + instance_width + import_width + gap * 2 {
            let search = UiBounds {
                x: left,
                y: 204,
                width: available,
                height: 36,
            };
            let new_group = UiBounds {
                x: left,
                y: 248,
                width: available,
                height: 36,
            };
            let new_instance = UiBounds {
                x: left,
                y: 292,
                width: available,
                height: 36,
            };
            let import_pack = UiBounds {
                x: left,
                y: 336,
                width: available,
                height: 36,
            };
            let sort = UiBounds {
                x: left,
                y: 380,
                width: available,
                height: 30,
            };
            let filter = UiBounds {
                x: left,
                y: 416,
                width: available,
                height: 30,
            };
            let group = UiBounds {
                x: left,
                y: 452,
                width: available,
                height: 30,
            };
            return LibraryToolbarLayout {
                search,
                new_group,
                new_instance,
                import_pack,
                sort,
                filter,
                group,
                cards_top: 488,
            };
        }
        let buttons_y = 188;
        let search = UiBounds {
            x: left,
            y: 144,
            width: available,
            height: 36,
        };
        let new_group = UiBounds {
            x: left,
            y: buttons_y,
            width: group_width,
            height: 36,
        };
        let new_instance = UiBounds {
            x: new_group.x + group_width + gap,
            y: buttons_y,
            width: instance_width,
            height: 36,
        };
        let import_pack = UiBounds {
            x: new_instance.x + instance_width + gap,
            y: buttons_y,
            width: import_width,
            height: 36,
        };
        let sort = UiBounds {
            x: left,
            y: 232,
            width: 196,
            height: 30,
        };
        let filter = UiBounds {
            x: sort.x + sort.width + gap,
            y: sort.y,
            width: 104,
            height: 30,
        };
        let inline_group_width = available.saturating_sub(316).min(196);
        let group_wraps = inline_group_width < 112;
        let group = UiBounds {
            x: if group_wraps {
                left
            } else {
                filter.x + filter.width + gap
            },
            y: if group_wraps { sort.y + 36 } else { sort.y },
            width: if group_wraps {
                available
            } else {
                inline_group_width
            },
            height: 30,
        };
        LibraryToolbarLayout {
            search,
            new_group,
            new_instance,
            import_pack,
            sort,
            filter,
            group,
            cards_top: if group_wraps { 336 } else { 300 },
        }
    } else {
        let fixed = group_width + instance_width + import_width + gap * 3;
        let search = UiBounds {
            x: left,
            y: 144,
            width: available.saturating_sub(fixed),
            height: 36,
        };
        let new_group = UiBounds {
            x: search.x + search.width + gap,
            y: 144,
            width: group_width,
            height: 36,
        };
        let new_instance = UiBounds {
            x: new_group.x + group_width + gap,
            y: 144,
            width: instance_width,
            height: 36,
        };
        let import_pack = UiBounds {
            x: new_instance.x + instance_width + gap,
            y: 144,
            width: import_width,
            height: 36,
        };
        let sort = UiBounds {
            x: left,
            y: 192,
            width: 196,
            height: 30,
        };
        let filter = UiBounds {
            x: sort.x + sort.width + gap,
            y: sort.y,
            width: 104,
            height: 30,
        };
        let group = UiBounds {
            x: filter.x + filter.width + gap,
            y: sort.y,
            width: available.saturating_sub(316).min(196),
            height: 30,
        };
        LibraryToolbarLayout {
            search,
            new_group,
            new_instance,
            import_pack,
            sort,
            filter,
            group,
            cards_top: 262,
        }
    }
}

fn home_library_toolbar_layout(
    width: usize,
    jump_in_rows: usize,
) -> LibraryToolbarLayout {
    let mut toolbar = library_toolbar_layout(width);
    if jump_in_rows == 0 {
        return toolbar;
    }
    let offset = 36 + jump_in_rows * HOME_JUMP_IN_ROW_HEIGHT;
    toolbar.search.y += offset;
    toolbar.new_group.y += offset;
    toolbar.new_instance.y += offset;
    toolbar.import_pack.y += offset;
    toolbar.sort.y += offset;
    toolbar.filter.y += offset;
    toolbar.group.y += offset;
    toolbar.cards_top += offset;
    toolbar
}

fn welcome_screen_layout(width: usize, height: usize) -> WelcomeScreenLayout {
    let center_x = 292 + width.saturating_sub(324) / 2;
    let import_footer_top = height.saturating_sub(104);
    let content_top = 112;
    let content_height = 288;
    let top = content_top
        + import_footer_top.saturating_sub(content_top + content_height) / 2;
    WelcomeScreenLayout {
        center_x,
        top,
        title_y: top + 124,
        create_y: top + 209,
        import_y: height.saturating_sub(64),
        options_y: top + 253,
        show_mark: true,
    }
}

const GLOBAL_DISCOVER_ID: &str = "__northstar_global_discover__";
const GLOBAL_SCREENSHOTS_ID: &str = "__northstar_global_screenshots__";
const NATIVE_TITLE_BAR_HEIGHT: usize = 48;
const MAX_JUMP_IN_ITEMS: usize = 5;
const WELCOME_SPLIT_BUTTON_WIDTH: usize = 206;
const WELCOME_PRIMARY_BUTTON_WIDTH: usize = 170;
const WELCOME_IMPORT_BUTTON_WIDTH: usize = 160;
const HOME_JUMP_IN_TITLE_Y: usize = 84;
const HOME_JUMP_IN_TILES_Y: usize = 108;
const HOME_JUMP_IN_ROW_HEIGHT: usize = 92;
const HOME_JUMP_IN_TILE_HEIGHT: usize = 80;
const MIGRATION_SPACE_RESERVE: u64 = 256 * 1024 * 1024;
const SETTINGS_APPEARANCE_ROW_INDICES: &[usize] = if cfg!(target_os = "macos") {
    &[5, 33, 13, 22, 29]
} else {
    &[5, 33, 34, 13, 22, 29]
};
const SETTINGS_SECTION_ROW_INDICES: [&[usize]; 6] = [
    SETTINGS_APPEARANCE_ROW_INDICES,
    &[6, 7, 8, 9, 12, 20, 21, 24],
    &[10, 11, 14, 15, 16, 17],
    &[2, 3, 18],
    &[0, 1, 23],
    &[4, 19, 25, 26, 27, 28, 30, 31, 32],
];
const SETTINGS_SECTION_NAMES: [&str; 6] = [
    "Appearance",
    "Features",
    "Behavior",
    "Privacy",
    "System",
    "Instance Defaults",
];
const SETTINGS_SECTION_NAV_TOPS: [usize; 6] = [164, 200, 236, 294, 352, 410];
const SETTINGS_SECTION_NAV_GROUPS: [(usize, &str); 4] = [
    (146, "DISPLAY"),
    (276, "ACCOUNT"),
    (334, "SYSTEM"),
    (392, "INSTANCE DEFAULTS"),
];

fn settings_section_at_y(y: f64) -> Option<usize> {
    SETTINGS_SECTION_NAV_TOPS
        .iter()
        .position(|top| (*top as f64..(*top + 32) as f64).contains(&y))
}

fn uses_custom_window_controls(
    is_macos: bool,
    native_decorations: bool,
) -> bool {
    !is_macos && !native_decorations
}

#[derive(Clone, Copy)]
enum SettingsChange {
    DownloadConcurrency(isize),
    WriteConcurrency(isize),
    Telemetry,
    DiscordRpc,
    Fullscreen,
    Theme(theseus::data::Theme),
    ThemeSync(bool),
    NativeDecorations,
    FilesTab,
    WorldsTab,
    ScreenshotsTab,
    SkinSelector,
    PlayTime,
    CompactCards,
    JumpIn,
    HideInstalledModpacks,
    HideRightSidebar,
    HideOnProcessStart,
    RefocusOnGameClose,
    WarnOnUnknownModpacks,
    SkipNonEssentialWarnings,
    PersonalizedAds,
    DefaultMemory(isize),
    ScreenshotSync(bool),
    HideNametag,
    QuickInstanceCount(isize),
    ColorTheme,
    FriendsActiveCollapsed,
    FriendsOnlineCollapsed,
    FriendsOfflineCollapsed,
    FriendsPendingCollapsed,
}

#[derive(Clone, Copy)]
enum FriendRemovalKind {
    Friend,
    IncomingRequest,
    SentRequest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostingBackupPromptAction {
    Restore,
    Rename,
    Delete,
}

#[derive(Clone, Debug)]
struct HostingBackupPrompt {
    action: HostingBackupPromptAction,
    backup: native_backend::HostingBackup,
}

fn hosting_permissions_allow_backups(permissions: &serde_json::Value) -> bool {
    const BACKUPS: u64 = 1 << 58;
    const SERVER_ADMIN: u64 = u64::MAX ^ ((1 << 15) - 1);
    let mask = permissions
        .as_u64()
        .or_else(|| permissions.as_i64().map(|value| value as u64))
        .or_else(|| {
            permissions.as_str().and_then(|value| {
                value.parse::<u64>().ok().or_else(|| {
                    let scopes =
                        value.split('|').map(str::trim).collect::<HashSet<_>>();
                    if scopes.contains("SERVER_ADMIN") {
                        Some(SERVER_ADMIN)
                    } else if scopes.contains("BACKUPS") {
                        Some(BACKUPS)
                    } else {
                        None
                    }
                })
            })
        })
        .unwrap_or_default();
    mask & BACKUPS == BACKUPS || mask & SERVER_ADMIN == SERVER_ADMIN
}

enum UiMessage {
    ExistingDataChecked(
        Result<Option<native_backend::ExistingModrinthData>, String>,
    ),
    ExistingDataCopied(
        Result<native_backend::ExistingModrinthMigration, String>,
    ),
    FreshStartChoiceSaved(Result<(), String>),
    BackendStarted(
        Result<
            (
                NativeBackend,
                tokio::sync::mpsc::UnboundedReceiver<
                    native_backend::NativeEvent,
                >,
            ),
            String,
        >,
    ),
    BackendReady,
    BackendFailed(String),
    OnboardingChecklistLoaded(
        Result<theseus::data::OnboardingChecklist, String>,
    ),
    OnboardingSkipped(Result<(), String>),
    Instances(Result<Vec<NativeInstanceEntry>, String>),
    HostingServersLoaded(Result<Vec<native_backend::HostingServer>, String>),
    HostingServerLoaded(
        String,
        Result<native_backend::HostingServerDetails, String>,
    ),
    HostingBackupsLoaded(
        String,
        Result<native_backend::HostingBackups, String>,
    ),
    HostingBackupCreated(String, Result<(), String>),
    HostingBackupMutationFinished(String, String, Result<(), String>),
    HostingBackupBulkUndeleteFinished(
        String,
        Vec<String>,
        Vec<(String, String)>,
    ),
    HostingBackupOptionsFinished(
        String,
        bool,
        Result<native_backend::HostingBackupOptionsUpdate, String>,
    ),
    HostingBackupDownloadFinished(
        String,
        Result<Option<std::path::PathBuf>, String>,
    ),
    HostingServerConnected(
        String,
        Result<
            (
                native_backend::HostingServerConnection,
                tokio::sync::mpsc::UnboundedReceiver<
                    native_backend::HostingServerEvent,
                >,
            ),
            String,
        >,
    ),
    HostingServerEvent(String, native_backend::HostingServerEvent),
    HostingPowerActionFinished(String, Result<(), String>),
    HostingAddressCopyFeedbackClear(String),
    HostingAccessLoaded(
        String,
        Result<Vec<native_backend::HostingServerAccessUser>, String>,
    ),
    HostingAccessSearchLoaded(
        String,
        String,
        Result<Vec<theseus::users::SearchUser>, String>,
    ),
    HostingAccessInviteFinished(String, String, Result<(), String>),
    HostingAccessReinviteFinished(
        String,
        String,
        Result<native_backend::HostingServerReinviteResponse, String>,
    ),
    HostingAccessRoleFinished(
        String,
        String,
        String,
        native_backend::HostingServerAccessRole,
        Result<(), String>,
    ),
    HostingAccessRemoveFinished(String, String, String, Result<(), String>),
    InstalledModpackIds(HashSet<String>),
    InstanceIconLoaded(String, Result<(u32, u32, Vec<u32>), String>),
    BrowseIconLoaded(String, Result<(u32, u32, Vec<u32>), String>),
    NewsArticlesLoaded(Result<Vec<native_backend::NewsArticle>, String>),
    SidebarFriendsLoaded(
        u64,
        Result<Vec<native_backend::NativeFriend>, String>,
    ),
    RecentWorldsLoaded(Result<Vec<theseus::worlds::WorldWithInstance>, String>),
    RefreshRecentWorlds,
    RefreshSidebarFriends,
    FriendSearchLoaded(Result<Vec<theseus::users::SearchUser>, String>),
    FriendActionFinished(String, bool, Result<(), String>),
    FriendRemoveFinished(String, FriendRemovalKind, Result<(), String>),
    UserProfileLoaded(
        String,
        Result<
            (
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
            ),
            String,
        >,
    ),
    Groups(Result<Vec<theseus::instance::InstanceGroup>, String>),
    HiddenInstancesLoaded(u64, u64, Result<Vec<String>, String>),
    InstanceVisibilityChanged(String, bool, u64, Result<(), String>),
    GroupCreated(Result<theseus::instance::InstanceGroup, String>),
    GroupRenamed(Result<theseus::instance::InstanceGroup, String>),
    GroupDeleted(String, Result<(), String>),
    GroupOrderChanged(Result<Vec<theseus::instance::InstanceGroup>, String>),
    MembershipAction(String, Vec<String>, Result<(), String>),
    BatchMembershipAction(
        Vec<theseus::instance::InstanceGroupMembershipUpdate>,
        Result<(), String>,
    ),
    BulkInstancesRemoved(Vec<String>, Vec<String>),
    InstanceContent(String, Result<Vec<theseus::data::ContentItem>, String>),
    WorldsLoaded(String, Result<Vec<worlds::World>, String>),
    ServerPingLoaded(String, Result<worlds::ServerStatus, String>),
    WorldStatusUpdated(String, Result<(), String>),
    ServerAdded(String, String, Result<usize, String>),
    ServerSaved(String, String, Result<(), String>),
    ServerRemoved(String, String, Result<(), String>),
    WorldRenamed(String, String, Result<(), String>),
    WorldDeleted(String, String, Result<(), String>),
    ScreenshotsLoaded(
        String,
        Result<Vec<theseus::instance::InstanceScreenshot>, String>,
    ),
    ScreenshotGroupsLoaded(
        Result<Vec<theseus::instance::ScreenshotGroup>, String>,
    ),
    ScreenshotGroupCreated(Result<theseus::instance::ScreenshotGroup, String>),
    ScreenshotGroupRenamed(Result<theseus::instance::ScreenshotGroup, String>),
    ScreenshotGroupDeleted(String, Result<(), String>),
    ScreenshotMembershipsUpdated(
        Vec<(String, Option<String>)>,
        Result<(), String>,
    ),
    ScreenshotBatchDeleted(Vec<String>, Result<(), String>),
    ScreenshotBatchExported(Result<Option<std::path::PathBuf>, String>),
    ScreenshotThumbnailLoaded(String, Result<(u32, u32, Vec<u32>), String>),
    ScreenshotExported(Result<Option<std::path::PathBuf>, String>),
    ScreenshotCopied(Result<(), String>),
    ClipboardTextCopied(Result<(), String>),
    ScreenshotPreviewLoaded(String, Result<(u32, u32, Vec<u32>), String>),
    SharedInstanceLoaded(
        String,
        Result<
            (
                Option<theseus::data::InstanceMetadata>,
                bool,
                Option<theseus::instance::SharedInstanceUsers>,
                Vec<theseus::instance::SharedInstanceInvite>,
            ),
            String,
        >,
    ),
    SharedInstanceInviteCreated(
        Result<theseus::instance::SharedInstanceInviteLink, String>,
    ),
    SharedInstanceInviteCopied(String, Result<(), String>),
    SharedInstanceInviteRevoked(String, Result<(), String>),
    SharedInstancePublishPreview(
        Result<Option<theseus::instance::SharedInstancePublishPreview>, String>,
    ),
    SharedInstancePublishProgress(u64, u64),
    SharedInstancePublished(
        Result<theseus::data::SharedInstanceAttachment, String>,
    ),
    SharedUsersSearched(Result<Vec<theseus::users::SearchUser>, String>),
    SharedUsersInvited(Result<theseus::instance::SharedInstanceUsers, String>),
    SharedUserRemoved(
        String,
        Result<theseus::instance::SharedInstanceUsers, String>,
    ),
    SharedModpackUnlinked(Result<theseus::data::InstanceMetadata, String>),
    ContentAction(String, Result<(), String>),
    ContentFileAdded(String, Result<Option<String>, String>),
    LiveLogChanged(String),
    LiveLogLoaded(String, Result<String, String>),
    InstanceLogsLoaded(String, Result<Vec<(String, bool, u64)>, String>),
    ClearMessage(u64),
    InstanceLogContentLoaded(String, String, bool, Result<String, String>),
    LogAnalysisFinished(String, String, Result<serde_json::Value, String>),
    LogShareFinished(String, String, Result<String, String>),
    LogDeleted(String, String, Result<(), String>),
    RefreshProcess(String),
    ProcessStateLoaded(
        String,
        Result<Option<theseus::data::ProcessMetadata>, String>,
    ),
    StopProcessDone(String, Result<(), String>),
    RefreshRunningProcesses,
    RunningProcessesLoaded(Result<Vec<theseus::data::ProcessMetadata>, String>),
    CardStopDone(String, Result<(), String>),
    FilesLoaded(
        String,
        String,
        Result<Vec<(String, String, String, Option<u64>, bool)>, String>,
    ),
    FileRead(String, String, Result<String, String>),
    BrowseSearchDebounced(u64, String),
    BrowseResults(u64, String, Result<BrowseSearchPage, String>),
    BrowseFilterOptionsLoaded(Result<BrowseFilterOptions, String>),
    BrowseDetails(String, Result<BrowseProjectDetails, String>),
    BrowseFollowStatus(String, Result<bool, String>),
    BrowseFollowChanged(String, bool, Result<(), String>),
    BrowseSaveCollectionsLoaded(
        Result<Vec<native_backend::ContentServiceCollection>, String>,
    ),
    BrowseCollectionSaved(String, String, Vec<String>, Result<(), String>),
    BrowseCollectionCreated(
        Result<native_backend::ContentServiceCollection, String>,
    ),
    BrowseInstallDone(String, String, String, Result<usize, String>),
    BrowseModpackInstallDone(String, Result<String, String>),
    BrowseInstallVersionsLoaded(u64, Result<Vec<BrowseVersion>, String>),
    BrowseInstallPresenceLoaded(u64, String, Result<bool, String>),
    ClearBrowseInstallNotice(std::time::Instant),
    ExportFinished(String, Result<Option<std::path::PathBuf>, String>),
    DuplicateFinished(String, Result<(), String>),
    InstanceRenamed(String, Result<String, String>),
    InstanceRemoved(String, Result<(), String>),
    SettingsLoaded(Result<theseus::data::Settings, String>),
    WindowDefaultsSaved(Result<theseus::data::Settings, String>),
    DefaultLaunchSettingsSaved(Result<theseus::data::Settings, String>),
    ScreenshotSyncLoaded(Result<bool, String>),
    ScreenshotSyncSaved(Result<bool, String>),
    QuickInstanceCountLoaded(Result<usize, String>),
    QuickInstanceCountSaved(Result<usize, String>),
    ColorThemeLoaded(Result<u8, String>),
    ColorThemeSaved(Result<u8, String>),
    PreferredDarkThemeLoaded(Result<theseus::data::Theme, String>),
    PreferredDarkThemeSaved(Result<(), String>),
    ThemeSyncLoaded(u64, Result<bool, String>),
    ThemeSyncSaved(u64, Result<bool, String>),
    ThemeSyncThemeSaved(Result<(), String>),
    ThemePreferencesLoaded(String, Result<serde_json::Value, String>),
    SettingsSaved(Result<theseus::data::Settings, String>),
    MaximumMemoryLoaded(Result<u32, String>),
    JavaVersionsLoaded(Result<Vec<theseus::data::JavaVersion>, String>),
    JavaUpdated(Result<theseus::data::JavaVersion, String>),
    JavaPickerCancelled,
    InstanceLaunchSettingsLoaded(
        String,
        Result<(theseus::data::InstanceMetadata, u32), String>,
    ),
    InstanceLaunchSettingsSaved(
        String,
        Result<theseus::data::InstanceMetadata, String>,
    ),
    InstanceJavaPathPicked(String, Option<String>),
    AccountsLoaded(
        Result<
            (
                Vec<native_backend::MinecraftAccount>,
                Option<uuid::Uuid>,
                Vec<native_backend::ContentServiceAccount>,
            ),
            String,
        >,
    ),
    AccountDefaultChanged(uuid::Uuid, Result<(), String>),
    MinecraftAccountRemoved(uuid::Uuid, Result<(), String>),
    LoginFlowReady(Result<native_backend::MinecraftLoginSession, String>),
    ModrinthLoginReady(Result<native_backend::ModrinthLoginSession, String>),
    ModrinthLoginFinished(
        Result<native_backend::ContentServiceAccount, String>,
    ),
    ContentAccountChanged(String, Result<(), String>),
    ContentAccountRemoved(String, Result<(), String>),
    ClipboardLoaded(Result<String, String>),
    LoginFinished(Result<native_backend::MinecraftAccount, String>),
    EventsLagged(u64),
    RefreshInstances,
    Notice(String),
    Launch(String, Option<String>, Option<String>),
    Import(std::path::PathBuf),
    LauncherInstances(Result<NativeImportSource, String>),
    LauncherImportFinished(String, Result<(), String>),
    MinecraftSkinsLoaded(Result<Vec<theseus::minecraft_skins::Skin>, String>),
    MinecraftSkinPreviewLoaded(String, Result<(u32, u32, Vec<u32>), String>),
    MinecraftSkinActionFinished(String, Result<(), String>),
    MinecraftSkinRemoved(String, Result<(), String>),
    Minimize,
    Restore,
    CloseWindow,
    InstallJob(std::sync::Arc<theseus::install::InstallJobSnapshot>),
    JobActionResult(String),
}

struct Launcher {
    runtime: Arc<Runtime>,
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    window_maximized: bool,
    scroll_frame_time: std::time::Instant,
    navigation_back: Vec<NativeRoute>,
    navigation_forward: Vec<NativeRoute>,
    navigation_last_route: Option<NativeRoute>,
    navigation_restoring: bool,
    backend: Option<NativeBackend>,
    proxy: Option<winit::event_loop::EventLoopProxy<UiMessage>>,
    ready: bool,
    instances: Vec<(String, String, Vec<String>)>,
    instance_cards: HashMap<String, NativeLibraryCard>,
    instance_metrics: HashMap<String, NativeInstanceMetrics>,
    instance_icons: HashMap<String, (u32, u32, Vec<u32>)>,
    instance_icons_pending: HashSet<String>,
    instance_icon_failures: HashSet<String>,
    groups: Vec<theseus::instance::InstanceGroup>,
    selected_group: Option<String>,
    membership_mode: bool,
    selected_library_instances: HashSet<String>,
    hidden_instance_ids: HashSet<String>,
    show_hidden_instances: bool,
    hidden_instance_save_pending: HashMap<String, u64>,
    hidden_instance_generation: u64,
    hidden_instance_load_generation: u64,
    library_selection_anchor: Option<String>,
    library_focused_instance: Option<String>,
    group_input: String,
    create_group_open: bool,
    create_group_from_selection: bool,
    migration_plan: Option<native_backend::ExistingModrinthData>,
    migration_prompt_open: bool,
    migration_settings_entry: bool,
    migration_busy: bool,
    migration_saving_fresh_choice: bool,
    selected_instance: Option<(String, String)>,
    rename_instance_open: bool,
    rename_instance_input: String,
    delete_confirmation_open: bool,
    bulk_delete_confirmation_open: bool,
    delete_confirmation_input: String,
    instance_deletion_pending: bool,
    instance_tab: u8,
    contents: Vec<theseus::data::ContentItem>,
    worlds: Vec<worlds::World>,
    server_statuses: HashMap<String, String>,
    server_motds: HashMap<String, String>,
    world_offset: usize,
    world_scroll_pixels: f64,
    world_scroll_target_pixels: f64,
    world_scroll_synced: usize,
    add_server_open: bool,
    add_server_busy: bool,
    server_edit_index: Option<usize>,
    server_edit_source: Option<theseus::instance::ServerSource>,
    server_remove_confirmation: Option<(usize, String, bool)>,
    world_editor_open: bool,
    world_edit_path: String,
    world_edit_name: String,
    world_edit_reset_icon: bool,
    world_delete_confirmation: Option<(String, String)>,
    server_name_input: String,
    server_address_input: String,
    server_pack_status: worlds::ServerPackStatus,
    screenshots: Vec<theseus::instance::InstanceScreenshot>,
    screenshot_thumbnails: HashMap<String, (u32, u32, Vec<u32>)>,
    screenshot_thumbnails_pending: HashSet<String>,
    screenshot_thumbnail_failures: HashSet<String>,
    screenshot_groups: Vec<theseus::instance::ScreenshotGroup>,
    screenshot_group_mode: u8,
    screenshot_sort_mode: u8,
    screenshot_group_action_pending: bool,
    screenshot_group_picker_open: bool,
    screenshot_group_offset: usize,
    screenshot_group_scroll_pixels: f64,
    screenshot_group_scroll_target_pixels: f64,
    screenshot_search_query: String,
    screenshot_group_rename_id: Option<String>,
    screenshot_group_name_input: String,
    screenshot_group_delete_confirmation: Option<String>,
    screenshot_offset: usize,
    screenshot_scroll_pixels: f64,
    screenshot_scroll_target_pixels: f64,
    screenshot_scroll_synced: usize,
    selected_screenshot: Option<usize>,
    selected_screenshot_ids: HashSet<String>,
    screenshot_viewer: bool,
    screenshot_preview: Option<(u32, u32, Vec<u32>)>,
    screenshot_delete_confirmation: bool,
    shared_instance_metadata: Option<theseus::data::InstanceMetadata>,
    shared_instance_can_use: Option<bool>,
    shared_instance_users: Vec<theseus::instance::SharedInstanceUser>,
    shared_instance_invites: Vec<theseus::instance::SharedInstanceInvite>,
    shared_instance_invite_link: Option<String>,
    shared_instance_busy: bool,
    shared_instance_preview:
        Option<theseus::instance::SharedInstancePublishPreview>,
    shared_instance_selected_files: HashSet<String>,
    shared_instance_preview_scroll_pixels: f64,
    shared_instance_preview_scroll_target_pixels: f64,
    shared_instance_progress: Option<(u64, u64)>,
    shared_instance_invite_open: bool,
    shared_instance_user_query: String,
    shared_instance_user_results: Vec<theseus::users::SearchUser>,
    shared_instance_selected_user_ids: HashSet<String>,
    shared_instance_pending_removal: Option<String>,
    shared_instance_unlink_confirmation: bool,
    console_text: String,
    console_scroll: usize,
    console_scroll_synced: usize,
    console_scroll_pixels: f64,
    console_scroll_target_pixels: f64,
    instance_logs: Vec<(String, bool, u64)>,
    selected_log: Option<(String, bool)>,
    log_text: String,
    log_search_query: String,
    log_search_select_all: bool,
    log_level_filter: u8,
    log_analysis: Option<serde_json::Value>,
    log_analysis_pending: bool,
    log_share_pending: bool,
    log_delete_confirmation: bool,
    log_delete_pending: bool,
    log_scroll: usize,
    log_scroll_synced: usize,
    log_scroll_pixels: f64,
    log_scroll_target_pixels: f64,
    process_state: Option<theseus::data::ProcessMetadata>,
    running_processes: HashMap<String, theseus::data::ProcessMetadata>,
    running_processes_pending: bool,
    running_processes_refresh_dirty: bool,
    file_path: String,
    file_items: Vec<(String, String, String, Option<u64>, bool)>,
    file_preview: Option<(String, String)>,
    file_preview_scroll_pixels: f64,
    file_preview_scroll_target_pixels: f64,
    file_offset: usize,
    file_list_scroll_pixels: f64,
    file_list_scroll_target_pixels: f64,
    file_list_scroll_synced: usize,
    browse_query: String,
    browse_query_cursor: usize,
    browse_query_select_all: bool,
    browse_results: Vec<BrowseProject>,
    browse_hidden_ids_dirty: bool,
    browse_icons: HashMap<String, (u32, u32, Vec<u32>)>,
    browse_icons_pending: HashSet<String>,
    browse_icon_failures: HashSet<String>,
    news_articles: Vec<native_backend::NewsArticle>,
    news_error: Option<String>,
    news_loaded: bool,
    sidebar_scroll_pixels: f64,
    sidebar_scroll_target_pixels: f64,
    onboarding_checklist: Option<theseus::data::OnboardingChecklist>,
    welcome_options_open: bool,
    sidebar_friends: Vec<native_backend::NativeFriend>,
    sidebar_friends_error: Option<String>,
    sidebar_friends_loaded: bool,
    sidebar_friends_request_generation: u64,
    sidebar_friend_query: String,
    recent_worlds: Vec<theseus::worlds::WorldWithInstance>,
    jump_in_visible_items: usize,
    quick_instance_count_saving: bool,
    native_color_theme: u8,
    native_color_theme_saving: bool,
    preferred_dark_theme: theseus::data::Theme,
    theme_sync_across_devices: bool,
    theme_sync_saving: bool,
    theme_sync_generation: u64,
    jump_in_resize_drag: Option<(f64, usize, usize)>,
    jump_in_overflow: Option<usize>,
    jump_in_server_pings_pending: HashSet<String>,
    friend_request_open: bool,
    friend_incoming_open: bool,
    friend_incoming_scroll_pixels: f64,
    friend_incoming_scroll_target_pixels: f64,
    friend_search_query: String,
    friend_search_results: Vec<theseus::users::SearchUser>,
    friend_search_error: Option<String>,
    friend_search_loading: bool,
    friend_request_pending: HashSet<String>,
    friend_remove_confirmation: Option<native_backend::NativeFriend>,
    friend_remove_error: Option<String>,
    friend_remove_pending: HashSet<String>,
    browse_selected: Option<BrowseProject>,
    browse_details: Option<BrowseProjectDetails>,
    browse_install_choice: Option<BrowseInstallChoice>,
    browse_install_choice_generation: u64,
    browse_install_override: Option<BrowseInstallOverride>,
    browse_installing_projects: HashMap<String, HashSet<String>>,
    browse_installing_modpacks: HashSet<String>,
    browse_install_notice: Option<BrowseInstallNotice>,
    browse_install_scroll: f64,
    browse_install_scroll_target: f64,
    launcher_import_open: bool,
    launcher_import_busy: bool,
    launcher_import_source: Option<NativeImportSource>,
    launcher_import_offset: usize,
    launcher_import_scroll_pixels: f64,
    launcher_import_scroll_target_pixels: f64,
    launcher_import_focus: usize,
    global_skins: bool,
    global_user_profile: bool,
    user_profile_id: Option<String>,
    user_profile: Option<serde_json::Value>,
    user_profile_projects: Vec<serde_json::Value>,
    user_profile_filtered_project_indices: Vec<usize>,
    user_profile_organizations: Vec<serde_json::Value>,
    user_profile_collections: Vec<serde_json::Value>,
    user_profile_type_filter: Option<String>,
    user_profile_show_collections: bool,
    user_profile_loading: bool,
    user_profile_error: Option<String>,
    user_profile_scroll_pixels: f64,
    user_profile_scroll_target_pixels: f64,
    minecraft_skins: Vec<theseus::minecraft_skins::Skin>,
    minecraft_skin_previews: HashMap<String, (u32, u32, Vec<u32>)>,
    minecraft_skin_previews_pending: HashSet<String>,
    minecraft_skin_preview_failures: HashSet<String>,
    minecraft_skin_scroll_pixels: f64,
    minecraft_skin_scroll_target_pixels: f64,
    minecraft_skin_variant: theseus::minecraft_skins::MinecraftSkinVariant,
    skin_delete_confirmation: Option<theseus::minecraft_skins::Skin>,
    minecraft_skin_rotation: f32,
    minecraft_skin_dragging: bool,
    minecraft_skin_last_cursor_x: f64,
    global_browse: bool,
    global_screenshots: bool,
    global_hosting: bool,
    hosting_servers: Vec<native_backend::HostingServer>,
    hosting_servers_loading: bool,
    hosting_servers_refreshing: bool,
    hosting_servers_error: Option<String>,
    hosting_server_id: Option<String>,
    hosting_server_details: Option<native_backend::HostingServerDetails>,
    hosting_server_details_loading: bool,
    hosting_server_details_error: Option<String>,
    hosting_tab: u8,
    hosting_backups: Option<native_backend::HostingBackups>,
    hosting_backups_loading: bool,
    hosting_backups_error: Option<String>,
    hosting_backup_create_open: bool,
    hosting_backup_create_name: String,
    hosting_backup_search: String,
    hosting_backup_show_deleted: bool,
    hosting_backup_deleted_reason_filter: u8,
    hosting_backup_deleted_type_filter: u8,
    hosting_backup_deleted_recoverability_filter: u8,
    hosting_backup_deleted_sort: u8,
    hosting_backup_countdown_refresh: std::time::Instant,
    hosting_backup_selected: HashSet<String>,
    hosting_backup_creating: bool,
    hosting_backup_prompt: Option<HostingBackupPrompt>,
    hosting_backup_mutating: bool,
    hosting_backup_downloading: bool,
    hosting_backup_options_open: bool,
    hosting_backup_auto_enabled: bool,
    hosting_backup_manual_slots: usize,
    hosting_backup_interval_hours: u32,
    hosting_backup_options_saving: bool,
    hosting_backup_rotation_confirmation: bool,
    hosting_backup_rotation_count: usize,
    hosting_backup_scroll_pixels: f64,
    hosting_backup_scroll_target_pixels: f64,
    hosting_access_users: Vec<native_backend::HostingServerAccessUser>,
    hosting_access_loading: bool,
    hosting_access_loaded: bool,
    hosting_access_error: Option<String>,
    hosting_access_remove_confirmation: Option<String>,
    hosting_access_scroll_pixels: f64,
    hosting_access_scroll_target_pixels: f64,
    hosting_access_invite_open: bool,
    hosting_access_search_query: String,
    hosting_access_search_results: Vec<theseus::users::SearchUser>,
    hosting_access_search_loading: bool,
    hosting_access_search_complete: bool,
    hosting_access_search_scroll_pixels: f64,
    hosting_access_search_scroll_target_pixels: f64,
    hosting_access_invite_error: Option<String>,
    hosting_access_role: u8,
    hosting_connection: Option<native_backend::HostingServerConnection>,
    hosting_socket_connecting: bool,
    hosting_socket_connected: bool,
    hosting_socket_error: Option<String>,
    hosting_power_state: Option<String>,
    hosting_live_stats: Option<(f64, u64, u64, u64, u64)>,
    hosting_console_lines: VecDeque<String>,
    hosting_console_input: String,
    hosting_console_scroll_pixels: f64,
    hosting_console_scroll_target_pixels: f64,
    hosting_console_follow_tail: bool,
    hosting_power_pending: bool,
    hosting_power_error: Option<String>,
    hosting_address_copied: bool,
    hosting_scroll_pixels: f64,
    hosting_scroll_target_pixels: f64,
    library_search_query: String,
    library_sort_mode: u8,
    library_group_mode: u8,
    library_collapsed_groups: HashSet<String>,
    library_sort_menu_open: bool,
    library_group_menu_open: bool,
    library_filter_open: bool,
    library_loader_filters: HashSet<String>,
    library_instance_type_filters: HashSet<String>,
    library_install_state_filters: HashSet<String>,
    library_game_version_filters: HashSet<String>,
    library_game_version_filter_offset: usize,
    library_game_version_filter_scroll_pixels: f64,
    library_game_version_filter_scroll_target_pixels: f64,
    create_instance_open: bool,
    browse_filter: u8,
    browse_sort: u8,
    browse_search_generation: u64,
    browse_request_generation: u64,
    browse_search_pending: bool,
    installed_modpack_ids: HashSet<String>,

    browse_scroll_pixels: f64,
    browse_scroll_target_pixels: f64,
    browse_search_offset: u32,
    browse_page_size: u32,
    browse_view_menu_open: bool,
    browse_sort_menu_open: bool,
    browse_total_hits: u32,
    browse_error: Option<String>,
    browse_filter_options: Option<BrowseFilterOptions>,
    browse_show_all_versions: bool,
    browse_filter_options_error: Option<String>,
    browse_filter_options_loading: bool,
    browse_filter_menu_open: bool,
    browse_filter_group: u8,
    browse_filter_scroll_pixels: f64,
    browse_filter_scroll_target_pixels: f64,
    browse_selected_game_versions: HashSet<String>,
    browse_selected_loaders: HashSet<String>,
    browse_selected_categories: HashSet<String>,
    browse_selected_environments: HashSet<String>,
    browse_selected_licenses: HashSet<String>,
    browse_detail_selected_channels: HashSet<String>,
    browse_detail_selected_game_versions: HashSet<String>,
    browse_detail_selected_loaders: HashSet<String>,
    browse_detail_filter_menu_group: Option<u8>,
    browse_detail_filter_option_offset: usize,
    browse_project_menu_open: bool,
    browse_project_followed: bool,
    browse_project_follow_pending: bool,
    browse_project_follow_status_loaded: bool,
    browse_project_follow_status_loading: bool,
    browse_save_menu_open: bool,
    browse_save_collections_loaded: bool,
    browse_save_collections_loading: bool,
    browse_save_collections: Vec<native_backend::ContentServiceCollection>,
    browse_save_collection_pending: HashSet<String>,
    browse_save_scroll_pixels: f64,
    browse_collection_create_open: bool,
    browse_collection_create_name: String,
    browse_collection_creating: bool,
    browse_detail_scroll_pixels: f64,
    browse_detail_scroll_target_pixels: f64,
    browse_gallery_index: usize,
    browse_gallery_viewer_open: bool,
    browse_gallery_zoom: f32,
    browse_detail_tab: u8,
    browse_description_lines: Option<(
        String,
        usize,
        Vec<String>,
        Vec<BrowseDescriptionLink>,
        Vec<BrowseDescriptionStyle>,
        Vec<BrowseDescriptionImage>,
    )>,
    settings_open: bool,
    settings_section: usize,
    settings_jump_in_focused: bool,
    settings_scroll_pixels: f64,
    settings_scroll_target_pixels: f64,
    global_screenshot_sync: Option<bool>,
    default_window_width_input: String,
    default_window_height_input: String,
    window_defaults_saving: bool,
    default_java_arguments_input: String,
    default_environment_variables_input: String,
    default_pre_launch_hook_input: String,
    default_wrapper_hook_input: String,
    default_post_exit_hook_input: String,
    default_launch_settings_saving: bool,
    java_open: bool,
    settings: Option<theseus::data::Settings>,
    maximum_memory_mb: Option<u32>,
    java_versions: Vec<theseus::data::JavaVersion>,
    java_busy: bool,
    instance_java_path: String,
    instance_launch_args: String,
    instance_memory_mb: String,
    instance_max_memory_mb: u32,
    instance_fullscreen_mode: u8,
    instance_resolution_width: String,
    instance_resolution_height: String,
    instance_settings_loading: bool,
    instance_settings_saving: bool,
    instance_setup_page: u8,
    instance_environment_json: String,
    instance_pre_launch_hook: String,
    instance_wrapper_hook: String,
    instance_post_exit_hook: String,
    accounts_open: bool,
    account_tab: u8,
    minecraft_accounts: Vec<native_backend::MinecraftAccount>,
    content_accounts: Vec<native_backend::ContentServiceAccount>,
    modrinth_login_task: Option<tokio::task::JoinHandle<()>>,
    default_account: Option<uuid::Uuid>,
    account_offset: usize,
    account_scroll_pixels: f64,
    account_scroll_target_pixels: f64,
    login_flow: Option<native_backend::MinecraftLoginSession>,
    login_code_input: String,
    cursor: (f64, f64),
    modifiers: winit::keyboard::ModifiersState,
    list_offset: usize,
    library_scroll_pixels: f64,
    library_scroll_target_pixels: f64,
    library_scroll_offset_synced: usize,
    content_scroll_pixels: f64,
    content_scroll_target_pixels: f64,
    content_filter: u8,
    filtered_content_cache: Vec<usize>,
    name_input: String,
    version_input: String,
    loader_choice: usize,
    input_focus: u8,
    message: String,
    last_timed_message: String,
    message_generation: u64,
    pack_on_start: Option<std::path::PathBuf>,
    start_fresh_on_startup: bool,
    install_progress: Option<f32>,
    active_job: Option<theseus::install::InstallJobSnapshot>,
    job_control_pending: bool,
    refresh_pending: Arc<std::sync::atomic::AtomicBool>,
    groups_refresh_pending: Arc<std::sync::atomic::AtomicBool>,
    console_refresh_pending: Arc<std::sync::atomic::AtomicBool>,
    console_refresh_dirty: Arc<std::sync::atomic::AtomicBool>,
    active_log_instance: Arc<std::sync::Mutex<Option<String>>>,
}

impl Launcher {
    fn library_scroll_prefix(
        rows: &[NativeLibraryGridRow],
        grid: LibraryGridLayout,
        row_count: usize,
    ) -> usize {
        rows.iter()
            .take(row_count)
            .map(|row| library_grid_row_height(row, grid))
            .sum()
    }

    fn schedule_message_dismissal(&mut self) {
        let message = self.message.clone();
        if message.is_empty() || message == self.last_timed_message {
            return;
        }
        self.last_timed_message = message.clone();
        self.message_generation = self.message_generation.wrapping_add(1);
        let generation = self.message_generation;
        let Some(delay) = message_dismissal_delay(&message) else {
            return;
        };
        let (Some(proxy), runtime) = (self.proxy.clone(), self.runtime.clone())
        else {
            return;
        };
        runtime.spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = proxy.send_event(UiMessage::ClearMessage(generation));
        });
    }

    fn current_navigation_route(&self) -> NativeRoute {
        if self.global_user_profile {
            return NativeRoute::UserProfile {
                user_id: self.user_profile_id.clone().unwrap_or_default(),
            };
        }
        if self.accounts_open {
            return NativeRoute::Accounts {
                tab: self.account_tab,
            };
        }
        if self.settings_open || self.java_open {
            return NativeRoute::Settings {
                java: self.java_open,
                section: self.settings_section,
            };
        }
        if self.create_instance_open {
            return NativeRoute::CreateInstance;
        }
        if self.global_hosting {
            return NativeRoute::Hosting {
                server_id: self.hosting_server_id.clone(),
            };
        }
        if self.global_skins {
            return NativeRoute::Skins;
        }
        if self.global_screenshots {
            return NativeRoute::Screenshots;
        }
        if self.global_browse {
            return NativeRoute::Discover {
                project: self.browse_selected.clone(),
            };
        }
        if let Some((id, name)) = &self.selected_instance {
            return NativeRoute::Instance {
                id: id.clone(),
                name: name.clone(),
                tab: self.instance_tab,
            };
        }
        NativeRoute::Library {
            group: self.selected_group.clone(),
        }
    }

    fn sync_navigation_history(&mut self) {
        let current = self.current_navigation_route();
        if !self.navigation_restoring {
            if let Some(previous) = self.navigation_last_route.as_ref()
                && previous != &current
            {
                self.jump_in_overflow = None;
                self.welcome_options_open = false;
                if !self.message.is_empty()
                    && self.message == self.last_timed_message
                {
                    self.message.clear();
                    self.last_timed_message.clear();
                    self.message_generation =
                        self.message_generation.wrapping_add(1);
                }
                self.navigation_back.push(previous.clone());
                if self.navigation_back.len() > 64 {
                    self.navigation_back.remove(0);
                }
                self.navigation_forward.clear();
            }
        }
        self.navigation_last_route = Some(current);
    }

    fn restore_navigation_route(&mut self, route: NativeRoute) {
        self.navigation_restoring = true;
        self.global_browse = false;
        self.global_screenshots = false;
        self.global_hosting = false;
        self.global_skins = false;
        self.global_user_profile = false;
        self.settings_open = false;
        self.java_open = false;
        self.accounts_open = false;
        self.create_instance_open = false;
        self.selected_instance = None;
        self.hosting_server_id = None;
        match route {
            NativeRoute::Library { group } => {
                self.accounts_open = false;
                self.settings_open = false;
                self.java_open = false;
                self.create_instance_open = false;
                self.return_to_library(group);
            }
            NativeRoute::Discover { project } => {
                self.open_global_browse();
                if let Some(project) = project {
                    self.open_browse_project(project);
                }
            }
            NativeRoute::Hosting { server_id } => {
                self.open_global_hosting();
                if let Some(server_id) = server_id {
                    self.open_hosting_server(server_id);
                }
            }
            NativeRoute::Skins => self.open_minecraft_skins(),
            NativeRoute::Screenshots => self.open_global_screenshots(),
            NativeRoute::Instance { id, name, tab } => {
                self.accounts_open = false;
                self.settings_open = false;
                self.java_open = false;
                self.create_instance_open = false;
                self.open_library_instance(id.clone(), name);
                self.instance_tab = tab;
                match tab {
                    1 => {
                        if let Ok(mut active) = self.active_log_instance.lock()
                        {
                            *active = Some(id.clone());
                        }
                        self.refresh_live_log(id.clone());
                        self.refresh_process_state(id);
                    }
                    2 => self.load_instance_files(id, String::new()),
                    3 => self.search_projects(),
                    4 => self.load_instance_launch_settings(id),
                    5 => self.refresh_instance_logs(id),
                    7 => self.load_instance_worlds(id),
                    8 => self.open_instance_screenshots(id),
                    9 => self.load_shared_instance(id),
                    _ => {}
                }
            }
            NativeRoute::UserProfile { user_id } => {
                self.open_user_profile(user_id);
            }
            NativeRoute::Settings { java, section } => {
                self.global_browse = false;
                self.global_screenshots = false;
                self.global_skins = false;
                self.global_user_profile = false;
                self.selected_instance = None;
                self.accounts_open = false;
                self.create_instance_open = false;
                self.settings_open = !java;
                self.java_open = java;
                self.settings_section = section;
                self.load_settings();
            }
            NativeRoute::Accounts { tab } => {
                self.global_browse = false;
                self.global_screenshots = false;
                self.global_skins = false;
                self.global_user_profile = false;
                self.selected_instance = None;
                self.settings_open = false;
                self.java_open = false;
                self.create_instance_open = false;
                self.accounts_open = true;
                self.account_tab = tab;
                self.account_offset = 0;
                self.account_scroll_pixels = 0.0;
                self.account_scroll_target_pixels = 0.0;
                self.load_accounts();
            }
            NativeRoute::CreateInstance => {
                self.return_to_library(None);
                self.create_instance_open = true;
            }
        }
        self.navigation_restoring = false;
        self.navigation_last_route = Some(self.current_navigation_route());
    }

    fn navigate_history(&mut self, backwards: bool) {
        let current = self.current_navigation_route();
        let target = if backwards {
            self.navigation_back.pop().map(|route| {
                self.navigation_forward.push(current);
                route
            })
        } else {
            self.navigation_forward.pop().map(|route| {
                self.navigation_back.push(current);
                route
            })
        };
        if let Some(route) = target {
            self.restore_navigation_route(route);
            self.draw();
        }
    }

    fn should_show_right_sidebar(&self) -> bool {
        let has_sidebar_width = self.window.as_ref().is_none_or(|window| {
            window.inner_size().width as usize >= 64 + 300 + 670
        });
        let discover_route = self.global_browse;
        let home_route = self.selected_instance.is_none()
            && !self.global_user_profile
            && !self.global_hosting
            && !self.accounts_open
            && !self.java_open
            && !self.settings_open;
        has_sidebar_width
            && (discover_route || self.global_screenshots || home_route)
            && (discover_route
                || self
                    .settings
                    .as_ref()
                    .is_none_or(|settings| !settings.toggle_sidebar))
            && !self.accounts_open
            && !self.java_open
            && !self.create_group_open
            && !self.create_instance_open
            && !self.migration_prompt_open
            && !self.launcher_import_open
            && !self.delete_confirmation_open
            && !self.bulk_delete_confirmation_open
            && self.server_remove_confirmation.is_none()
            && self.world_delete_confirmation.is_none()
    }

    fn browse_filter_sidebar_visible(&self) -> bool {
        self.is_browse_results_page()
            && self.global_browse
            && self.should_show_right_sidebar()
    }

    fn sidebar_instances(&self, height: usize) -> Vec<(String, String)> {
        let mut instances: Vec<_> = self
            .instances
            .iter()
            .map(|(id, name, _)| {
                let metrics =
                    self.instance_metrics.get(id).copied().unwrap_or_default();
                (
                    id.clone(),
                    name.clone(),
                    metrics.created.max(metrics.last_played.unwrap_or(0)),
                )
            })
            .collect();
        instances.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
        instances
            .into_iter()
            .take(
                sidebar_instance_capacity(height)
                    .min(self.jump_in_visible_items),
            )
            .map(|(id, name, _)| (id, name))
            .collect()
    }

    fn native_instance_tabs(
        &self,
        wide: bool,
    ) -> Vec<(usize, usize, &'static str, u8)> {
        let show_files = self
            .settings
            .as_ref()
            .is_none_or(|settings| settings.show_files_tab_in_instances);
        let show_screenshots = self
            .settings
            .as_ref()
            .is_none_or(|settings| settings.show_screenshots_tab_in_instances);
        let show_worlds = self
            .settings
            .as_ref()
            .is_none_or(|settings| settings.show_worlds_tab_in_instances);
        if !wide {
            return [
                (506, 50, "Content", 0),
                (556, 50, "Console", 1),
                (606, 38, "Files", 2),
                (644, 44, "Browse", 3),
                (688, 38, "Setup", 4),
                (726, 32, "More", 6),
            ]
            .into_iter()
            .filter(|(_, _, _, tab)| *tab != 2 || show_files)
            .collect();
        }
        let mut x = 506;
        let mut tabs = vec![(x, 72, "Content", 0)];
        x += 72;
        if show_files {
            tabs.push((x, 48, "Files", 2));
            x += 48;
        }
        if show_screenshots {
            tabs.push((x, 96, "Screenshots", 8));
            x += 96;
        }
        if show_worlds {
            tabs.push((x, 64, "Worlds", 7));
            x += 64;
        }
        tabs.push((x, 48, "Logs", 5));
        x += 48;
        tabs.push((x, 56, "Share", 9));
        x += 56;
        tabs.push((x, 48, "More", 6));
        tabs
    }

    fn uses_wide_instance_header(&self, width: usize) -> bool {
        width >= 980
            && !self.global_browse
            && !self.global_hosting
            && !self.global_skins
            && !self.global_screenshots
            && self.instance_tab != 8
    }

    fn instance_more_rows(
        &self,
        wide: bool,
        width: usize,
    ) -> impl Iterator<Item = (UiBounds, &'static str, &'static str, u8)> + use<>
    {
        let mut rows: Vec<(&'static str, &'static str, u8)> = if wide {
            vec![
                ("Console", "View live game output", 1),
                ("Browse", "Install mods and content", 3),
                ("Setup", "Edit Java and launch settings", 4),
            ]
        } else {
            vec![
                ("Worlds", "Open and manage saves and servers", 7),
                ("Logs", "View game logs and crash reports", 5),
                ("Screenshots", "View and export instance screenshots", 8),
                ("Share", "Invite players and manage access", 9),
            ]
        };
        let is_hidden = self
            .selected_instance
            .as_ref()
            .is_some_and(|(id, _)| self.hidden_instance_ids.contains(id));
        rows.push((
            if is_hidden {
                "Show in Library"
            } else {
                "Hide from Library"
            },
            "Change whether this instance appears on Home",
            10,
        ));
        let show_worlds = self
            .settings
            .as_ref()
            .is_none_or(|settings| settings.show_worlds_tab_in_instances);
        let show_screenshots = self
            .settings
            .as_ref()
            .is_none_or(|settings| settings.show_screenshots_tab_in_instances);
        rows.into_iter()
            .filter(move |&(_, _, tab)| {
                (tab != 7 || show_worlds) && (tab != 8 || show_screenshots)
            })
            .enumerate()
            .map(move |(index, (label, description, tab))| {
                (
                    UiBounds {
                        x: 292,
                        y: 180 + index * 62,
                        width: width.saturating_sub(324),
                        height: 50,
                    },
                    label,
                    description,
                    tab,
                )
            })
    }

    fn activate_instance_tab(&mut self, tab: u8) {
        let Some((id, _)) = self.selected_instance.clone() else {
            return;
        };
        if tab == 8 {
            self.open_instance_screenshots(id);
            return;
        }
        if tab == 10 {
            self.toggle_selected_instance_visibility();
            return;
        }
        self.instance_tab = tab;
        self.input_focus = 0;
        if let Ok(mut active) = self.active_log_instance.lock() {
            *active = (tab == 1).then(|| id.clone());
        }
        match tab {
            1 => {
                self.console_scroll = 0;
                self.refresh_live_log(id.clone());
                self.refresh_process_state(id);
            }
            2 => {
                self.file_path.clear();
                self.file_items.clear();
                self.file_preview = None;
                self.file_preview_scroll_pixels = 0.0;
                self.file_preview_scroll_target_pixels = 0.0;
                self.file_offset = 0;
                self.load_instance_files(id, String::new());
            }
            3 => self.search_projects(),
            4 => {
                self.instance_setup_page = 0;
                self.instance_settings_loading = true;
                self.instance_java_path.clear();
                self.instance_launch_args.clear();
                self.instance_memory_mb.clear();
                self.instance_resolution_width.clear();
                self.instance_resolution_height.clear();
                self.instance_fullscreen_mode = 0;
                self.instance_environment_json.clear();
                self.instance_pre_launch_hook.clear();
                self.instance_wrapper_hook.clear();
                self.instance_post_exit_hook.clear();
                self.message = "Loading instance launch settings…".into();
                self.load_instance_launch_settings(id);
            }
            5 => {
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
                self.message = "Loading logs and crash reports…".into();
                self.refresh_instance_logs(id);
            }
            7 => {
                self.worlds.clear();
                self.world_offset = 0;
                self.world_scroll_pixels = 0.0;
                self.world_scroll_target_pixels = 0.0;
                self.world_scroll_synced = 0;
                self.message = "Loading worlds and servers…".into();
                self.load_instance_worlds(id);
            }
            9 => {
                self.shared_instance_metadata = None;
                self.shared_instance_users.clear();
                self.shared_instance_invites.clear();
                self.load_shared_instance(id);
            }
            _ => {}
        }
    }

    fn is_home_library(&self) -> bool {
        self.selected_instance.is_none()
            && !self.global_browse
            && !self.global_hosting
            && !self.global_skins
            && !self.global_screenshots
            && !self.global_user_profile
            && !self.settings_open
            && !self.accounts_open
            && !self.java_open
            && !self.create_instance_open
    }

    fn is_browse_results_page(&self) -> bool {
        self.selected_instance.is_some()
            && self.instance_tab == 3
            && self.browse_selected.is_none()
            && !self.global_hosting
            && !self.global_skins
            && !self.global_screenshots
            && !self.global_user_profile
            && !self.settings_open
            && !self.accounts_open
            && !self.java_open
            && !self.create_instance_open
    }

    fn home_page_scroll_active(&self) -> bool {
        self.is_home_library()
    }

    fn home_page_scroll_input_active(&self) -> bool {
        self.home_page_scroll_active()
            && !self.create_group_open
            && !self.migration_prompt_open
            && !self.launcher_import_open
            && !self.delete_confirmation_open
            && !self.bulk_delete_confirmation_open
            && self.server_remove_confirmation.is_none()
            && self.world_delete_confirmation.is_none()
            && self.friend_remove_confirmation.is_none()
            && !self.friend_request_open
    }

    fn library_card_action(
        &self,
        instance_id: &str,
        hovered_or_focused: bool,
    ) -> NativeLibraryCardAction {
        let card = self.instance_cards.get(instance_id);
        native_library_card_action(
            self.running_processes.contains_key(instance_id),
            card.is_some_and(|card| card.install_state == "installing"),
            card.is_none_or(|card| card.install_state == "installed"),
            card.is_some_and(|card| card.content_locked),
            card.is_some_and(|card| card.launch_blocked),
            !self.selected_library_instances.is_empty(),
            hovered_or_focused,
        )
    }

    fn repair_instance(&mut self, instance_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.message = "Repairing instance…".into();
        self.draw();
        self.runtime.spawn(async move {
            match backend.repair_instance(&instance_id).await {
                Ok(snapshot) => {
                    let _ = proxy.send_event(UiMessage::InstallJob(
                        std::sync::Arc::new(snapshot),
                    ));
                    let _ = proxy.send_event(UiMessage::RefreshInstances);
                }
                Err(error) => {
                    let _ = proxy.send_event(UiMessage::Notice(format!(
                        "Could not repair instance: {error}"
                    )));
                }
            }
        });
    }

    fn open_library_instance(&mut self, id: String, name: String) {
        self.selected_instance = Some((id.clone(), name));
        self.rename_instance_open = false;
        self.rename_instance_input.clear();
        self.instance_tab = 0;
        self.process_state = None;
        self.file_path.clear();
        self.file_items.clear();
        self.file_preview = None;
        self.file_preview_scroll_pixels = 0.0;
        self.file_preview_scroll_target_pixels = 0.0;
        self.file_offset = 0;
        self.console_text.clear();
        self.console_scroll = 0;
        self.console_scroll_synced = 0;
        self.console_scroll_pixels = 0.0;
        self.console_scroll_target_pixels = 0.0;
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
        self.shared_instance_preview_scroll_pixels = 0.0;
        self.shared_instance_preview_scroll_target_pixels = 0.0;
        self.load_instance_content(id);
    }

    fn library_focus_columns(&self) -> usize {
        let width = self
            .window
            .as_ref()
            .map_or(1024, |window| window.inner_size().width as usize);
        self.library_grid_layout(width).columns.max(1)
    }

    fn library_grid_layout(&self, width: usize) -> LibraryGridLayout {
        let items = self.jump_in_items();
        let visible_count = self.jump_in_visible_count(width, items.len());
        let rows = if visible_count == 0 {
            0
        } else {
            visible_count.div_ceil(Self::jump_in_columns(width))
        };
        library_grid_layout(
            width,
            self.settings
                .as_ref()
                .is_some_and(|settings| settings.compact_instance_cards),
            rows,
        )
    }

    fn current_library_toolbar_layout(
        &self,
        width: usize,
    ) -> LibraryToolbarLayout {
        if !self.is_home_library() {
            return library_toolbar_layout(width);
        }
        let visible_count =
            self.jump_in_visible_count(width, self.jump_in_items().len());
        let rows = if visible_count == 0 {
            0
        } else {
            visible_count.div_ceil(Self::jump_in_columns(width))
        };
        home_library_toolbar_layout(width, rows)
    }

    fn jump_in_columns(width: usize) -> usize {
        ((width.saturating_sub(112) + 12) / 682).clamp(1, 5)
    }

    fn jump_in_visible_count(&self, _width: usize, item_count: usize) -> usize {
        visible_jump_in_count(
            self.jump_in_visible_items,
            item_count,
            Self::jump_in_columns(_width),
        )
    }

    fn jump_in_resize_handle(&self, width: usize) -> Option<(usize, usize)> {
        let item_count = self.jump_in_items().len().min(MAX_JUMP_IN_ITEMS);
        let columns = Self::jump_in_columns(width);
        if item_count <= columns {
            return None;
        }
        let visible_count = self.jump_in_visible_count(width, item_count);
        let rows = visible_count.div_ceil(columns);
        let content_width = width.saturating_sub(324);
        let x = 292 + content_width.saturating_sub(48) / 2;
        let y = HOME_JUMP_IN_TILES_Y + rows * HOME_JUMP_IN_ROW_HEIGHT - 14;
        Some((x, y))
    }

    fn update_jump_in_resize_drag(&mut self, cursor_y: f64) {
        let Some((start_y, start_count, _)) = self.jump_in_resize_drag else {
            return;
        };
        let window_width = self
            .window
            .as_ref()
            .map_or(1024, |window| window.inner_size().width as usize);
        let library_width =
            window_width.saturating_sub(if self.should_show_right_sidebar() {
                300
            } else {
                0
            });
        let items = self.jump_in_items();
        let max_visible = items.len().min(MAX_JUMP_IN_ITEMS);
        let columns = Self::jump_in_columns(library_width);
        if max_visible <= 1 {
            return;
        }
        let row_delta = ((cursor_y - start_y) / HOME_JUMP_IN_ROW_HEIGHT as f64)
            .round() as isize;
        if row_delta == 0 {
            return;
        }
        self.jump_in_visible_items =
            (start_count as isize + row_delta * columns as isize).clamp(
                columns.min(max_visible).max(1) as isize,
                max_visible as isize,
            ) as usize;
    }

    fn jump_in_items(&self) -> Vec<NativeJumpInItem> {
        let enabled = self
            .settings
            .as_ref()
            .is_none_or(|settings| settings.show_jump_in);
        if !enabled
            || self.global_browse
            || self.global_screenshots
            || self.global_skins
            || self.settings_open
            || self.accounts_open
            || self.selected_instance.is_some()
            || self.selected_group.is_some()
            || !self.library_search_query.trim().is_empty()
            || !self.library_loader_filters.is_empty()
            || !self.library_instance_type_filters.is_empty()
            || !self.library_install_state_filters.is_empty()
            || !self.library_game_version_filters.is_empty()
            || self.show_hidden_instances
            || self.membership_mode
        {
            return Vec::new();
        }
        let worlds = self.recent_worlds_for_jump_in();
        let mut items = Vec::<(i64, NativeJumpInItem)>::new();
        let mut consumed_worlds = HashSet::new();
        for (index, (instance_id, _, _)) in self.instances.iter().enumerate() {
            if self.hidden_instance_ids.contains(instance_id) {
                continue;
            }
            let Some(metrics) = self.instance_metrics.get(instance_id) else {
                continue;
            };
            let matching_world = worlds
                .iter()
                .find(|world| world.instance_id == *instance_id);
            let world_time = matching_world
                .and_then(|world| world.world.last_played)
                .map(|time| time.timestamp());
            if let (Some(world), Some(world_time)) =
                (matching_world, world_time)
            {
                consumed_worlds.insert(world.instance_id.clone());
                if metrics.last_played.is_none_or(|last| world_time >= last) {
                    items.push((
                        world_time,
                        NativeJumpInItem::World(world.clone()),
                    ));
                    continue;
                }
            }
            items.push((
                metrics.last_played.unwrap_or(metrics.created),
                NativeJumpInItem::Instance {
                    index,
                    newly_added: metrics.last_played.is_none(),
                },
            ));
        }
        for world in worlds {
            if self.hidden_instance_ids.contains(&world.instance_id) {
                continue;
            }
            if !consumed_worlds.contains(&world.instance_id)
                && let Some(last_played) = world.world.last_played
            {
                items.push((
                    last_played.timestamp(),
                    NativeJumpInItem::World(world),
                ));
            }
        }
        items.sort_by(|left, right| right.0.cmp(&left.0));
        let mut newly_added_count = 0;
        items
            .into_iter()
            .filter_map(|(_, item)| match &item {
                NativeJumpInItem::Instance {
                    newly_added: true, ..
                } => {
                    if newly_added_count >= 3 {
                        None
                    } else {
                        newly_added_count += 1;
                        Some(item)
                    }
                }
                _ => Some(item),
            })
            .take(5)
            .collect()
    }

    fn recent_worlds_for_jump_in(
        &self,
    ) -> Vec<theseus::worlds::WorldWithInstance> {
        let enabled = self
            .settings
            .as_ref()
            .is_none_or(|settings| settings.show_jump_in);
        if !enabled
            || self.global_browse
            || self.global_screenshots
            || self.global_skins
            || self.settings_open
            || self.accounts_open
            || self.selected_instance.is_some()
            || self.selected_group.is_some()
            || !self.library_search_query.trim().is_empty()
            || !self.library_loader_filters.is_empty()
            || !self.library_instance_type_filters.is_empty()
            || !self.library_install_state_filters.is_empty()
            || !self.library_game_version_filters.is_empty()
            || self.membership_mode
        {
            return Vec::new();
        }
        self.recent_worlds
            .iter()
            .filter(|world| {
                world.world.last_played.is_some()
                    && self.instances.iter().any(|instance| {
                        instance.0 == world.instance_id
                            && !self
                                .instance_cards
                                .get(&instance.0)
                                .is_some_and(|card| card.launch_blocked)
                    })
            })
            .take(5)
            .cloned()
            .collect()
    }

    fn activate_jump_in_item(&mut self, item: NativeJumpInItem) {
        let launch_target = match item {
            NativeJumpInItem::Instance { index, .. } => self
                .instances
                .get(index)
                .map(|(id, name, _)| (id.clone(), name.clone(), None, None)),
            NativeJumpInItem::World(recent_world) => {
                let Some((_, instance_name, _)) = self
                    .instances
                    .iter()
                    .find(|instance| instance.0 == recent_world.instance_id)
                else {
                    return;
                };
                let (server, world) = match &recent_world.world.details {
                    theseus::worlds::WorldDetails::Server {
                        address, ..
                    } => (Some(address.clone()), None),
                    theseus::worlds::WorldDetails::Singleplayer {
                        path,
                        ..
                    } => (None, Some(path.clone())),
                };
                Some((
                    recent_world.instance_id,
                    instance_name.clone(),
                    server,
                    world,
                ))
            }
        };
        if let Some((instance_id, instance_name, server, world)) = launch_target
        {
            if self.running_processes.contains_key(&instance_id) {
                self.stop_card_process(instance_id);
            } else {
                self.launch_instance(instance_id, instance_name, server, world);
            }
        }
    }

    fn view_jump_in_item(&mut self, item: NativeJumpInItem) {
        let (instance_id, instance_name, tab) = match item {
            NativeJumpInItem::Instance { index, .. } => {
                let Some((id, name, _)) = self.instances.get(index) else {
                    return;
                };
                (id.clone(), name.clone(), 0)
            }
            NativeJumpInItem::World(world) => {
                let Some((_, name, _)) = self
                    .instances
                    .iter()
                    .find(|instance| instance.0 == world.instance_id)
                else {
                    return;
                };
                (world.instance_id, name.clone(), 7)
            }
        };
        self.open_library_instance(instance_id.clone(), instance_name);
        self.instance_tab = tab;
        if tab == 7 {
            self.load_instance_worlds(instance_id);
        }
    }

    fn open_jump_in_instance_folder(&mut self, item: &NativeJumpInItem) {
        let instance_id = match item {
            NativeJumpInItem::Instance { index, .. } => {
                self.instances.get(*index).map(|(id, _, _)| id.clone())
            }
            NativeJumpInItem::World(world) => Some(world.instance_id.clone()),
        };
        let Some(instance_id) = instance_id else {
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            self.message = "Launcher services are not ready.".into();
            self.draw();
            return;
        };
        let runtime = self.runtime.clone();
        self.message = "Opening instance folder…".into();
        self.draw();
        runtime.spawn(async move {
            let result = async {
                let path = theseus::instance::get_full_path(&instance_id)
                    .await
                    .map_err(|error| error.to_string())?;
                open::that_detached(path).map_err(|error| error.to_string())
            }
            .await;
            if let Err(error) = result {
                let _ = proxy.send_event(UiMessage::Notice(format!(
                    "Could not open instance folder: {error}"
                )));
            }
        });
    }

    fn move_library_focus(&mut self, delta: isize) {
        let width = self
            .window
            .as_ref()
            .map_or(1024, |window| window.inner_size().width as usize);
        let grid = self.library_grid_layout(width);
        let rows = self.library_grid_rows(grid.columns);
        let ids: Vec<String> = rows
            .iter()
            .filter_map(|row| match row {
                NativeLibraryGridRow::Cards(indices) => Some(
                    indices
                        .iter()
                        .map(|index| self.instances[*index].0.clone())
                        .collect::<Vec<_>>(),
                ),
                NativeLibraryGridRow::Header(_, _, _) => None,
            })
            .flatten()
            .collect();
        if ids.is_empty() {
            return;
        }
        let current = self
            .library_focused_instance
            .as_ref()
            .and_then(|id| ids.iter().position(|candidate| candidate == id));
        let next = current.map_or(0, |index| {
            (index as isize + delta).clamp(0, ids.len() as isize - 1) as usize
        });
        let id = ids[next].clone();
        if self.modifiers.shift_key()
            && self.selected_library_instances.is_empty()
        {
            if let Some(anchor) =
                current.and_then(|index| ids.get(index)).cloned()
            {
                self.selected_library_instances.insert(anchor.clone());
                self.library_selection_anchor = Some(anchor);
            }
        }
        self.library_focused_instance = Some(id.clone());
        if !self.selected_library_instances.is_empty()
            && self.modifiers.shift_key()
        {
            self.select_library_instance_range(&id);
        }
        if let Some(row_index) = rows.iter().position(|row| matches!(row, NativeLibraryGridRow::Cards(indices) if indices.iter().any(|index| self.instances[*index].0 == id))) {
            let height = self.window.as_ref().map_or(768, |window| window.inner_size().height as usize);
            let visible = height.saturating_sub(grid.cards_top) / (grid.card_height + grid.gap).max(1);
            if row_index < self.list_offset { self.list_offset = row_index; }
            else if row_index >= self.list_offset + visible.max(1) { self.list_offset = row_index + 1 - visible.max(1); }
        }
    }
}

fn message_dismissal_delay(message: &str) -> Option<std::time::Duration> {
    let lower = message.to_ascii_lowercase();
    let is_error = ["failed", "error", "could not", "unable"]
        .iter()
        .any(|word| lower.contains(word));
    // Keep a short safety timeout so a lost completion event cannot leave
    // stale in-flight text visible while the user moves between screens.
    let is_in_flight = message.ends_with('…') || message.ends_with("...");
    if is_in_flight && !is_error {
        return Some(std::time::Duration::from_secs(15));
    }
    Some(if is_error {
        std::time::Duration::from_secs(12)
    } else {
        std::time::Duration::from_secs(5)
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarFriendGroup {
    Active,
    Online,
    Offline,
    Pending,
}

impl SidebarFriendGroup {
    fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Online => "Online",
            Self::Offline => "Offline",
            Self::Pending => "Pending",
        }
    }
}

struct SidebarFriendSection {
    group: SidebarFriendGroup,
    header_y: usize,
    friend_indices: Vec<usize>,
    collapsed: bool,
}

fn sidebar_collapsed_groups(
    settings: Option<&theseus::data::Settings>,
) -> [bool; 4] {
    settings.map_or([false, false, true, true], |settings| {
        [
            settings.friends_active_collapsed,
            settings.friends_online_collapsed,
            settings.friends_offline_collapsed,
            settings.friends_pending_collapsed,
        ]
    })
}

fn sidebar_friend_header_y(
    onboarding_checklist: Option<&theseus::data::OnboardingChecklist>,
) -> usize {
    if onboarding_checklist.is_some_and(|checklist| {
        checklist.show_checklist && checklist.has_logged_into_minecraft
    }) {
        372
    } else {
        244
    }
}

fn sidebar_incoming_friend_indices(
    friends: &[native_backend::NativeFriend],
) -> Vec<usize> {
    let mut indices = friends
        .iter()
        .enumerate()
        .filter_map(|(index, friend)| friend.incoming_request.then_some(index))
        .collect::<Vec<_>>();
    indices.sort_by_key(|index| std::cmp::Reverse(friends[*index].created));
    indices
}

fn friend_incoming_modal_max_scroll(count: usize, height: usize) -> f64 {
    let panel_height = 500usize.min(height.saturating_sub(32));
    let viewport_height = panel_height.saturating_sub(134);
    count.saturating_mul(64).saturating_sub(viewport_height) as f64
}

fn sidebar_visible_incoming_friend_indices(
    friends: &[native_backend::NativeFriend],
    scroll_pixels: f64,
    height: usize,
) -> Vec<usize> {
    let incoming = sidebar_incoming_friend_indices(friends);
    let panel_height = 500usize.min(height.saturating_sub(32));
    let viewport_height = panel_height.saturating_sub(134);
    let first = (scroll_pixels.max(0.0) as usize / 64)
        .min(incoming.len().saturating_sub(1));
    let visible = viewport_height.div_ceil(64) + 1;
    incoming.into_iter().skip(first).take(visible).collect()
}

fn sidebar_friend_start_y(
    onboarding_checklist: Option<&theseus::data::OnboardingChecklist>,
) -> usize {
    sidebar_friend_header_y(onboarding_checklist) + 30
}

fn sidebar_friend_sections(
    friends: &[native_backend::NativeFriend],
    onboarding_checklist: Option<&theseus::data::OnboardingChecklist>,
    query: &str,
    collapsed_groups: [bool; 4],
) -> Vec<SidebarFriendSection> {
    let mut header_y = sidebar_friend_start_y(onboarding_checklist);
    let query = query.trim().to_lowercase();
    let mut sections = Vec::new();
    for (group_index, group) in [
        SidebarFriendGroup::Active,
        SidebarFriendGroup::Online,
        SidebarFriendGroup::Offline,
        SidebarFriendGroup::Pending,
    ]
    .into_iter()
    .enumerate()
    {
        let mut friend_indices = friends
            .iter()
            .enumerate()
            .filter_map(|(index, friend)| {
                if !query.is_empty()
                    && !friend.username.to_lowercase().contains(&query)
                {
                    return None;
                }
                let matches = match group {
                    SidebarFriendGroup::Active => {
                        friend.accepted
                            && friend.online
                            && friend.playing_on.is_some()
                    }
                    SidebarFriendGroup::Online => {
                        friend.accepted
                            && friend.online
                            && friend.playing_on.is_none()
                    }
                    SidebarFriendGroup::Offline => {
                        friend.accepted && !friend.online
                    }
                    SidebarFriendGroup::Pending => {
                        !friend.accepted && !friend.incoming_request
                    }
                };
                matches.then_some(index)
            })
            .collect::<Vec<_>>();
        if group == SidebarFriendGroup::Pending {
            friend_indices.sort_by_key(|index| {
                std::cmp::Reverse(friends[*index].created)
            });
        }
        if friend_indices.is_empty() {
            continue;
        }
        if !sections.is_empty() {
            header_y += 8;
        }
        let friend_count = friend_indices.len();
        let collapsed = query.is_empty() && collapsed_groups[group_index];
        sections.push(SidebarFriendSection {
            group,
            header_y,
            friend_indices,
            collapsed,
        });
        header_y += 24 + if collapsed { 0 } else { friend_count * 38 };
    }
    sections
}

fn sidebar_news_top(
    friends: &[native_backend::NativeFriend],
    onboarding_checklist: Option<&theseus::data::OnboardingChecklist>,
    query: &str,
    collapsed_groups: [bool; 4],
) -> usize {
    sidebar_friend_sections(
        friends,
        onboarding_checklist,
        query,
        collapsed_groups,
    )
    .last()
    .map_or(360, |section| {
        (section.header_y + 24 + section.friend_indices.len() * 38 + 16)
            .max(406)
    })
    .max(sidebar_friend_start_y(onboarding_checklist) + 132)
    .max(406)
}

fn sidebar_visible_friend_indices(
    friends: &[native_backend::NativeFriend],
    onboarding_checklist: Option<&theseus::data::OnboardingChecklist>,
    query: &str,
    collapsed_groups: [bool; 4],
    scroll_pixels: f64,
    height: usize,
) -> Vec<usize> {
    let viewport_top = scroll_pixels as usize + 48;
    let viewport_bottom = scroll_pixels as usize + height;
    sidebar_friend_sections(
        friends,
        onboarding_checklist,
        query,
        collapsed_groups,
    )
    .into_iter()
    .flat_map(|section| {
        section.friend_indices.into_iter().enumerate().filter_map(
            move |(row_index, friend_index)| {
                if section.collapsed {
                    return None;
                }
                let row_top = section.header_y + 24 + row_index * 38;
                (row_top + 34 > viewport_top && row_top < viewport_bottom)
                    .then_some(friend_index)
            },
        )
    })
    .collect()
}

fn sidebar_content_height(
    article_count: usize,
    friends: &[native_backend::NativeFriend],
    onboarding_checklist: Option<&theseus::data::OnboardingChecklist>,
    query: &str,
    collapsed_groups: [bool; 4],
) -> usize {
    let news_top = sidebar_news_top(
        friends,
        onboarding_checklist,
        query,
        collapsed_groups,
    );
    let article_count = article_count.min(4);
    if article_count == 0 {
        news_top + 68
    } else {
        news_top + 36 + article_count * SIDEBAR_NEWS_CARD_STRIDE
    }
}

fn sidebar_scroll_max(
    article_count: usize,
    friends: &[native_backend::NativeFriend],
    height: usize,
    onboarding_checklist: Option<&theseus::data::OnboardingChecklist>,
    query: &str,
    collapsed_groups: [bool; 4],
) -> usize {
    sidebar_content_height(
        article_count,
        friends,
        onboarding_checklist,
        query,
        collapsed_groups,
    )
    .saturating_sub(height)
}

impl Launcher {
    fn inspect_existing_modrinth_instances(&mut self) {
        if self.migration_busy || self.migration_prompt_open {
            return;
        }
        let Some(proxy) = self.proxy.clone() else {
            self.message = "Launcher window is not ready.".into();
            return;
        };
        self.migration_settings_entry = true;
        self.migration_busy = true;
        self.migration_plan = None;
        self.message = "Checking for an existing Modrinth library…".into();
        self.runtime.spawn(async move {
            let result = native_backend::inspect_existing_modrinth_data(
                "org.northstar.launcher",
            )
            .await;
            let _ = proxy.send_event(UiMessage::ExistingDataChecked(result));
        });
    }

    fn start_backend(&mut self) {
        let proxy = self.proxy.as_ref().unwrap().clone();
        self.runtime.spawn(async move {
            let result = NativeBackend::start("org.northstar.launcher".into())
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::BackendStarted(result));
        });
        self.message = "Starting launcher services…".into();
        self.draw();
    }

    fn attach_backend(
        &mut self,
        backend: NativeBackend,
        mut events: tokio::sync::mpsc::UnboundedReceiver<
            native_backend::NativeEvent,
        >,
    ) {
        let proxy = self.proxy.as_ref().unwrap().clone();
        let runtime = self.runtime.clone();
        let active_log_instance = self.active_log_instance.clone();
        let status = backend.startup_status();
        self.backend = Some(backend);
        self.load_hidden_instance_ids();
        self.load_news_articles();
        std::thread::spawn(move || {
            runtime.block_on(async move {
            let mut status = status;
            loop {
                match status.borrow_and_update().clone() {
                    StartupStatus::Ready => {
                        let _ = proxy.send_event(UiMessage::BackendReady);
                        while let Some(event) = events.recv().await {
                            match event {
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::Instance(_)
                                    | theseus::AppEvent::InstanceGroupsChanged(_),
                                ) => {
                                    let _ = proxy.send_event(
                                        UiMessage::RefreshInstances,
                                    );
                                }
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::Friend(_),
                                ) => {
                                    let _ = proxy.send_event(
                                        UiMessage::RefreshSidebarFriends,
                                    );
                                }
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::Command(
                                        theseus::CommandPayload::RunMRPack {
                                            path,
                                        },
                                    ),
                                ) => {
                                    let _ = proxy.send_event(
                                        UiMessage::Import(path.into()),
                                    );
                                }
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::Command(
                                        theseus::CommandPayload::LaunchInstance {
                                            id,
                                            server,
                                            singleplayer_world,
                                        },
                                    ),
                                ) => {
                                    let _ = proxy.send_event(UiMessage::Launch(
                                        id,
                                        server,
                                        singleplayer_world,
                                    ));
                                }
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::InstallJob(snapshot),
                                ) => {
                                    let _ = proxy.send_event(
                                        UiMessage::InstallJob(snapshot),
                                    );
                                }
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::Log(payload),
                                ) => {
                                    if active_log_instance
                                        .lock()
                                        .map(|id| {
                                            id.as_deref()
                                                == Some(
                                                    payload.instance_id.as_str(),
                                                )
                                        })
                                        .unwrap_or(false)
                                    {
                                        let _ = proxy.send_event(
                                            UiMessage::LiveLogChanged(
                                                payload.instance_id,
                                            ),
                                        );
                                    }
                                }
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::Process(payload),
                                ) => {
                                    let _ = proxy.send_event(
                                        UiMessage::RefreshRunningProcesses,
                                    );
                                    if active_log_instance
                                        .lock()
                                        .map(|id| {
                                            id.as_deref()
                                                == Some(
                                                    payload.instance_id.as_str(),
                                                )
                                        })
                                        .unwrap_or(false)
                                    {
                                        let _ = proxy.send_event(
                                            UiMessage::RefreshProcess(
                                                payload.instance_id,
                                            ),
                                        );
                                    }
                                }
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::MinimizeWindow,
                                ) => {
                                    let _ = proxy.send_event(UiMessage::Minimize);
                                }
                                native_backend::NativeEvent::Event(
                                    theseus::AppEvent::RestoreAndFocusWindow,
                                ) => {
                                    let _ = proxy.send_event(UiMessage::Restore);
                                }
                                native_backend::NativeEvent::Lagged(count) => {
                                    let _ = proxy.send_event(
                                        UiMessage::EventsLagged(count),
                                    );
                                    let _ = proxy.send_event(
                                        UiMessage::RefreshInstances,
                                    );
                                    let _ = proxy.send_event(
                                        UiMessage::RefreshSidebarFriends,
                                    );
                                }
                                native_backend::NativeEvent::Event(_) => {}
                            }
                        }
                        break;
                    }
                    StartupStatus::Failed(error) => { let _ = proxy.send_event(UiMessage::BackendFailed(error)); break; }
                    StartupStatus::Starting => {}
                }
                tokio::select! { changed = status.changed() => if changed.is_err() { break }, }
            }
        })
        });
    }
}

impl Launcher {
    fn load_news_articles(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend.news_articles().await;
            let _ = proxy.send_event(UiMessage::NewsArticlesLoaded(result));
        });
    }

    fn load_onboarding_checklist(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .onboarding_checklist()
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::OnboardingChecklistLoaded(result));
        });
    }

    fn skip_create_instance_onboarding(&mut self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher services are not ready.".into();
            return;
        };
        self.message = "Saving your choice…".into();
        self.runtime.spawn(async move {
            let result = backend
                .skip_create_instance_onboarding()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::OnboardingSkipped(result));
        });
    }

    fn load_sidebar_friends(&mut self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.sidebar_friends_request_generation =
            self.sidebar_friends_request_generation.wrapping_add(1);
        let generation = self.sidebar_friends_request_generation;
        self.runtime.spawn(async move {
            let result = match tokio::time::timeout(
                std::time::Duration::from_secs(8),
                backend.friends(),
            )
            .await
            {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(_) => Err("Timed out loading Modrinth friends.".into()),
            };
            let _ = proxy.send_event(UiMessage::SidebarFriendsLoaded(
                generation, result,
            ));
        });
    }

    fn load_recent_worlds(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .recent_worlds(5)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::RecentWorldsLoaded(result));
        });
    }

    fn search_friend_users(&mut self) {
        let query = self.friend_search_query.trim().to_owned();
        if query.chars().count() < 2 {
            self.friend_search_error =
                Some("Enter at least two characters to search.".into());
            self.friend_search_results.clear();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.friend_search_error =
                Some("Launcher services are not ready.".into());
            return;
        };
        self.friend_search_loading = true;
        self.friend_search_error = None;
        self.friend_search_results.clear();
        self.runtime.spawn(async move {
            let result = backend
                .search_users(&query)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::FriendSearchLoaded(result));
        });
    }

    fn send_friend_request(&mut self, user_id: String) {
        if self.friend_request_pending.contains(&user_id) {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.friend_search_error =
                Some("Launcher services are not ready.".into());
            return;
        };
        let is_accepting = self
            .sidebar_friends
            .iter()
            .any(|friend| friend.user_id == user_id && friend.incoming_request);
        self.friend_request_pending.insert(user_id.clone());
        self.runtime.spawn(async move {
            let result = backend
                .add_friend(&user_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::FriendActionFinished(
                user_id,
                is_accepting,
                result,
            ));
        });
    }

    fn remove_sidebar_friend(&mut self, user_id: String) {
        if !self.friend_remove_pending.insert(user_id.clone()) {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.friend_remove_pending.remove(&user_id);
            self.friend_remove_error =
                Some("Launcher services are not ready.".into());
            return;
        };
        let removal_kind = self
            .sidebar_friends
            .iter()
            .find(|friend| friend.user_id == user_id)
            .map_or(FriendRemovalKind::Friend, |friend| {
                if friend.accepted {
                    FriendRemovalKind::Friend
                } else if friend.incoming_request {
                    FriendRemovalKind::IncomingRequest
                } else {
                    FriendRemovalKind::SentRequest
                }
            });
        self.runtime.spawn(async move {
            let result = backend
                .remove_friend(&user_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::FriendRemoveFinished(
                user_id,
                removal_kind,
                result,
            ));
        });
    }

    fn request_instance_icon(&mut self, instance_id: &str, icon_path: &str) {
        if self.instance_icons.contains_key(instance_id)
            || self.instance_icons_pending.contains(instance_id)
            || self.instance_icon_failures.contains(instance_id)
            || icon_path.starts_with("http://")
            || icon_path.starts_with("https://")
            || icon_path.to_ascii_lowercase().ends_with(".svg")
        {
            return;
        }
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let runtime = self.runtime.clone();
        self.instance_icons_pending.insert(instance_id.to_owned());
        let id = instance_id.to_owned();
        let path = std::path::PathBuf::from(icon_path);
        runtime.spawn(async move {
            let result = spawn_image_decode(move || {
                let mut reader = image::ImageReader::open(path)
                    .map_err(|error| error.to_string())?
                    .with_guessed_format()
                    .map_err(|error| error.to_string())?;
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(4096);
                limits.max_image_height = Some(4096);
                limits.max_alloc = Some(16 * 1024 * 1024);
                reader.limits(limits);
                let image = reader
                    .decode()
                    .map_err(|error| error.to_string())?
                    .thumbnail(128, 128)
                    .to_rgba8();
                let (width, height) = image.dimensions();
                let pixels = image
                    .pixels()
                    .map(|pixel| {
                        let [red, green, blue, alpha] = pixel.0;
                        (u32::from(alpha) << 24)
                            | (u32::from(red) << 16)
                            | (u32::from(green) << 8)
                            | u32::from(blue)
                    })
                    .collect();
                Ok::<_, String>((width, height, pixels))
            })
            .await
            .and_then(|result| result);
            let _ = proxy.send_event(UiMessage::InstanceIconLoaded(id, result));
        });
    }

    fn request_browse_icon(&mut self, project_id: &str, icon_url: &str) {
        if self.browse_icons.contains_key(project_id)
            || self.browse_icons_pending.contains(project_id)
            || self.browse_icon_failures.contains(project_id)
            || !icon_url.starts_with("https://")
        {
            return;
        }
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.browse_icons_pending.insert(project_id.to_owned());
        let runtime = self.runtime.clone();
        let id = project_id.to_owned();
        let url = icon_url.to_owned();
        let gallery_image = project_id.starts_with("gallery:")
            || project_id.starts_with("gallery-full:")
            || project_id.starts_with("description-image:");
        runtime.spawn(async move {
            let result = match theseus::instance::cache_remote_icon(&url).await
            {
                Ok(path) => spawn_image_decode(move || {
                    let mut reader = image::ImageReader::open(path)
                        .map_err(|error| error.to_string())?
                        .with_guessed_format()
                        .map_err(|error| error.to_string())?;
                    let mut limits = image::Limits::default();
                    limits.max_image_width = Some(4096);
                    limits.max_image_height = Some(4096);
                    limits.max_alloc = Some(16 * 1024 * 1024);
                    reader.limits(limits);
                    let image = reader
                        .decode()
                        .map_err(|error| error.to_string())?
                        .thumbnail(
                            if gallery_image { 960 } else { 128 },
                            if gallery_image { 640 } else { 128 },
                        )
                        .to_rgba8();
                    let (width, height) = image.dimensions();
                    let pixels = image
                        .pixels()
                        .map(|pixel| {
                            let [red, green, blue, alpha] = pixel.0;
                            (u32::from(alpha) << 24)
                                | (u32::from(red) << 16)
                                | (u32::from(green) << 8)
                                | u32::from(blue)
                        })
                        .collect();
                    Ok::<_, String>((width, height, pixels))
                })
                .await
                .and_then(|result| result),
                Err(error) => Err(error.to_string()),
            };
            let _ = proxy.send_event(UiMessage::BrowseIconLoaded(id, result));
        });
    }

    fn filtered_content_indices(&self) -> Vec<usize> {
        self.contents
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let matches = match self.content_filter {
                    1 => item.project_type == theseus::data::ProjectType::Mod,
                    2 => {
                        item.project_type
                            == theseus::data::ProjectType::ResourcePack
                    }
                    3 => {
                        item.project_type
                            == theseus::data::ProjectType::DataPack
                    }
                    4 => {
                        item.project_type
                            == theseus::data::ProjectType::ShaderPack
                    }
                    _ => true,
                };
                matches.then_some(index)
            })
            .collect()
    }

    fn rebuild_filtered_content_cache(&mut self) {
        self.filtered_content_cache = self.filtered_content_indices();
    }

    fn library_game_version_options(&self) -> Vec<String> {
        let mut versions: Vec<String> = self
            .instance_cards
            .values()
            .map(|card| card.game_version.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        versions.sort_by(|a, b| {
            b.split('.')
                .map(|part| part.parse::<u32>().unwrap_or(0))
                .cmp(a.split('.').map(|part| part.parse::<u32>().unwrap_or(0)))
        });
        versions
    }

    fn filtered_instance_indices(&self) -> Vec<usize> {
        let query = self.library_search_query.trim().to_lowercase();
        let mut indices: Vec<usize> = self
            .instances
            .iter()
            .enumerate()
            .filter_map(|(index, (instance_id, name, group_ids))| {
                if self.membership_mode {
                    return Some(index);
                }
                let visibility_matches = library_visibility_matches(
                    instance_id,
                    &self.hidden_instance_ids,
                    self.show_hidden_instances,
                );
                let included = match self.selected_group.as_deref() {
                    None => true,
                    Some(group_id) => group_ids.iter().any(|id| id == group_id),
                };
                let matches_name = name.to_lowercase().contains(&query);
                let matches_metadata =
                    self.instance_cards.get(instance_id).is_some_and(|card| {
                        card.loader.to_lowercase().contains(&query)
                            || card.game_version.to_lowercase().contains(&query)
                            || card
                                .instance_type
                                .to_lowercase()
                                .contains(&query)
                    });
                let matches_group = group_ids.iter().any(|group_id| {
                    self.groups
                        .iter()
                        .find(|group| &group.id == group_id)
                        .is_some_and(|group| {
                            group.name.to_lowercase().contains(&query)
                        })
                });
                let matches_query = query.is_empty()
                    || matches_name
                    || matches_metadata
                    || matches_group;
                let loader_matches = self.library_loader_filters.is_empty()
                    || self.instance_cards.get(instance_id).is_some_and(
                        |card| {
                            self.library_loader_filters
                                .contains(&card.loader.to_lowercase())
                        },
                    );
                let type_matches = self
                    .library_instance_type_filters
                    .is_empty()
                    || self.instance_cards.get(instance_id).is_some_and(
                        |card| {
                            self.library_instance_type_filters
                                .contains(&card.instance_type.to_lowercase())
                        },
                    );
                let state_matches =
                    self.library_install_state_filters.is_empty()
                        || self.instance_cards.get(instance_id).is_some_and(
                            |card| {
                                self.library_install_state_filters
                                    .contains(&card.install_state)
                            },
                        );
                let game_version_matches =
                    self.library_game_version_filters.is_empty()
                        || self.instance_cards.get(instance_id).is_some_and(
                            |card| {
                                self.library_game_version_filters
                                    .contains(&card.game_version)
                            },
                        );
                (visibility_matches
                    && included
                    && matches_query
                    && loader_matches
                    && type_matches
                    && state_matches
                    && game_version_matches)
                    .then_some(index)
            })
            .collect();
        let sort_kind = self.library_sort_mode / 2;
        let descending = self.library_sort_mode % 2 != 0;
        indices.sort_by_cached_key(|index| {
            let instance_id = &self.instances[*index].0;
            match sort_kind {
                1 => NativeInstanceSortKey::Timestamp(
                    self.instance_metrics
                        .get(instance_id)
                        .and_then(|metrics| metrics.last_played),
                ),
                2 => NativeInstanceSortKey::Number(
                    self.instance_metrics
                        .get(instance_id)
                        .map_or(0, |metrics| metrics.playtime),
                ),
                3 => NativeInstanceSortKey::Timestamp(Some(
                    self.instance_metrics
                        .get(instance_id)
                        .map_or(0, |metrics| metrics.created),
                )),
                4 => NativeInstanceSortKey::Timestamp(Some(
                    self.instance_metrics
                        .get(instance_id)
                        .map_or(0, |metrics| metrics.modified),
                )),
                5 => NativeInstanceSortKey::Text(
                    self.instance_cards
                        .get(instance_id)
                        .map_or("", |card| card.loader.as_str())
                        .to_lowercase(),
                ),
                6 => NativeInstanceSortKey::Text(
                    self.instance_cards
                        .get(instance_id)
                        .map_or("", |card| card.game_version.as_str())
                        .to_lowercase(),
                ),
                _ => NativeInstanceSortKey::Text(
                    self.instances[*index].1.to_lowercase(),
                ),
            }
        });
        if descending {
            indices.reverse();
        }
        indices
    }

    fn library_grid_rows(&self, columns: usize) -> Vec<NativeLibraryGridRow> {
        let indices = self.filtered_instance_indices();
        let mut sections: Vec<(String, String, Vec<usize>)> = Vec::new();
        match self.library_group_mode % 5 {
            0 => {
                if let Some(selected_group) = self.selected_group.as_ref() {
                    let name = if selected_group == "group:favorites" {
                        "Favorites".to_owned()
                    } else {
                        self.groups
                            .iter()
                            .find(|group| &group.id == selected_group)
                            .map_or_else(
                                || "Group".to_owned(),
                                |group| group.name.clone(),
                            )
                    };
                    let members = if self.membership_mode {
                        indices
                    } else {
                        indices
                            .into_iter()
                            .filter(|index| {
                                self.instances[*index]
                                    .2
                                    .iter()
                                    .any(|id| id == selected_group)
                            })
                            .collect()
                    };
                    sections.push((selected_group.clone(), name, members));
                } else {
                    let favorite_ids: Vec<usize> = indices
                        .iter()
                        .copied()
                        .filter(|index| {
                            self.instances[*index]
                                .2
                                .iter()
                                .any(|id| id == "group:favorites")
                        })
                        .collect();
                    sections.push((
                        "group:favorites".into(),
                        "Favorites".into(),
                        favorite_ids,
                    ));
                    for group in self
                        .groups
                        .iter()
                        .filter(|group| group.id != "group:favorites")
                    {
                        let members = indices
                            .iter()
                            .copied()
                            .filter(|index| {
                                self.instances[*index]
                                    .2
                                    .iter()
                                    .any(|id| id == &group.id)
                            })
                            .collect();
                        sections.push((
                            group.id.clone(),
                            group.name.clone(),
                            members,
                        ));
                    }
                    let ungrouped = indices
                        .into_iter()
                        .filter(|index| {
                            !self.instances[*index]
                                .2
                                .iter()
                                .any(|id| id != "group:favorites")
                        })
                        .collect();
                    sections.push((
                        "group:none".into(),
                        "None".into(),
                        ungrouped,
                    ));
                }
            }
            mode => {
                let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
                for index in indices {
                    let (instance_id, _, _) = &self.instances[index];
                    let key = self.instance_cards.get(instance_id).map_or_else(
                        || "Unknown".to_owned(),
                        |card| match mode {
                            1 => card.instance_type.clone(),
                            2 => card.loader.clone(),
                            3 => card.game_version.clone(),
                            _ => "None".to_owned(),
                        },
                    );
                    buckets.entry(key).or_default().push(index);
                }
                let mut keys: Vec<String> = buckets.keys().cloned().collect();
                if mode == 3 {
                    keys.sort_by(|a, b| {
                        b.split('.')
                            .map(|part| part.parse::<u32>().unwrap_or(0))
                            .cmp(
                                a.split('.').map(|part| {
                                    part.parse::<u32>().unwrap_or(0)
                                }),
                            )
                    });
                } else {
                    keys.sort_by_key(|key| key.to_lowercase());
                }
                sections.extend(keys.into_iter().filter_map(|key| {
                    buckets.remove(&key).map(|instances| {
                        (format!("{mode}:{key}"), key, instances)
                    })
                }));
            }
        }

        let mut rows = Vec::new();
        for (key, title, section) in sections {
            if self.library_group_mode % 5 != 4 {
                if section.is_empty()
                    && self.selected_group.is_none()
                    && self.library_group_mode % 5 == 0
                {
                    continue;
                }
                rows.push(NativeLibraryGridRow::Header(
                    key.clone(),
                    title,
                    section.len(),
                ));
            }
            if self.library_collapsed_groups.contains(&key) {
                continue;
            }
            for chunk in section.chunks(columns.max(1)) {
                rows.push(NativeLibraryGridRow::Cards(chunk.to_vec()));
            }
        }
        rows
    }

    fn select_library_instance_range(&mut self, target_id: &str) {
        let width = self
            .window
            .as_ref()
            .map_or(1024, |window| window.inner_size().width as usize);
        let grid = self.library_grid_layout(width);
        let mut visible_order = Vec::new();
        for row in self.library_grid_rows(grid.columns) {
            if let NativeLibraryGridRow::Cards(indices) = row {
                visible_order.extend(
                    indices
                        .into_iter()
                        .map(|index| self.instances[index].0.clone()),
                );
            }
        }

        self.selected_library_instances = updated_library_selection_range(
            &visible_order,
            self.library_selection_anchor.as_deref(),
            target_id,
            &self.selected_library_instances,
        );
        self.library_selection_anchor = Some(target_id.to_owned());
    }

    fn selected_loader(&self) -> theseus::data::ModLoader {
        match self.loader_choice % 5 {
            0 => theseus::data::ModLoader::Vanilla,
            1 => theseus::data::ModLoader::Fabric,
            2 => theseus::data::ModLoader::Forge,
            3 => theseus::data::ModLoader::NeoForge,
            _ => theseus::data::ModLoader::Quilt,
        }
    }

    fn update_group_membership(
        &mut self,
        instance_id: String,
        group_ids: Vec<String>,
    ) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = "Updating group membership…".into();
        std::thread::spawn(move || {
            let update = theseus::instance::InstanceGroupMembershipUpdate {
                instance_id: instance_id.clone(),
                group_ids: group_ids.clone(),
            };
            let result = runtime
                .block_on(backend.set_group_memberships(vec![update]))
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::MembershipAction(
                instance_id,
                group_ids,
                result,
            ));
        });
    }

    fn update_selected_group_membership(
        &mut self,
        group_id: String,
        remove_membership: bool,
    ) {
        let updates: Vec<theseus::instance::InstanceGroupMembershipUpdate> =
            self.instances
                .iter()
                .filter(|(instance_id, _, _)| {
                    self.selected_library_instances.contains(instance_id)
                })
                .filter_map(|(instance_id, _, group_ids)| {
                    let is_member = group_ids.iter().any(|id| id == &group_id);
                    if is_member == !remove_membership {
                        return None;
                    }
                    let mut next_group_ids = group_ids.clone();
                    if remove_membership {
                        next_group_ids.retain(|id| id != &group_id);
                    } else {
                        next_group_ids.push(group_id.clone());
                    }
                    Some(theseus::instance::InstanceGroupMembershipUpdate {
                        instance_id: instance_id.clone(),
                        group_ids: next_group_ids,
                    })
                })
                .collect();
        if updates.is_empty() {
            self.selected_library_instances.clear();
            self.library_selection_anchor = None;
            self.message = if remove_membership {
                "Selected instances are not in this group.".into()
            } else {
                "Selected instances are already in this group.".into()
            };
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.message = "Updating selected group membership…".into();
        self.runtime.spawn(async move {
            let request_updates = updates.clone();
            let result = backend
                .set_group_memberships(request_updates)
                .await
                .map(|_| ())
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::BatchMembershipAction(updates, result));
        });
    }

    fn selected_group_action_removes_membership(&self, group_id: &str) -> bool {
        let mut selected_count = 0;
        let all_selected_are_members = self
            .instances
            .iter()
            .filter(|(instance_id, _, _)| {
                self.selected_library_instances.contains(instance_id)
            })
            .all(|(_, _, group_ids)| {
                selected_count += 1;
                group_ids.iter().any(|id| id == group_id)
            });
        selected_count > 0 && all_selected_are_members
    }

    fn create_group(&mut self) {
        let name = self.group_input.trim().to_string();
        if name.is_empty() {
            self.message = "Enter a name for the group.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = format!("Creating group {name}…");
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.create_group(name))
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::GroupCreated(result));
        });
    }

    fn rename_selected_group(&mut self) {
        let Some(id) = self.selected_group.clone() else {
            return;
        };
        if id == "group:favorites" {
            self.message = "Favorites cannot be renamed.".into();
            return;
        }
        let name = self.group_input.trim().to_string();
        if name.is_empty() {
            self.message = "Enter a new group name first.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = format!("Renaming group to {name}…");
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.rename_group(id, name))
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::GroupRenamed(result));
        });
    }

    fn move_selected_group(&mut self, offset: isize) {
        let Some(selected_id) = self.selected_group.clone() else {
            return;
        };
        let mut ordered_ids: Vec<String> = self
            .groups
            .iter()
            .filter(|group| group.id != "group:favorites")
            .map(|group| group.id.clone())
            .collect();
        let Some(index) = ordered_ids.iter().position(|id| id == &selected_id)
        else {
            return;
        };
        let target = index as isize + offset;
        if target < 0 || target >= ordered_ids.len() as isize {
            self.message = "Group is already at that end of the list.".into();
            return;
        }
        ordered_ids.swap(index, target as usize);
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = "Updating group order…".into();
        std::thread::spawn(move || {
            let result = runtime.block_on(async {
                backend
                    .set_group_order(ordered_ids)
                    .await
                    .map_err(|error| error.to_string())?;
                backend
                    .list_groups()
                    .await
                    .map_err(|error| error.to_string())
            });
            let _ = proxy.send_event(UiMessage::GroupOrderChanged(result));
        });
    }

    fn delete_selected_group(&mut self) {
        let Some(id) = self.selected_group.clone() else {
            return;
        };
        if id == "group:favorites" {
            self.message = "Favorites cannot be deleted.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = "Deleting group…".into();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.delete_group(id.clone()))
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::GroupDeleted(id, result));
        });
    }

    fn load_instance_content(&self, instance_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.instance_content(&instance_id, None))
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::InstanceContent(instance_id, result));
        });
    }

    fn load_instance_worlds(&self, instance_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .instance_worlds(&instance_id)
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::WorldsLoaded(instance_id, result));
        });
    }

    fn load_server_status(&self, address: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .server_status(&address)
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::ServerPingLoaded(address, result));
        });
    }

    fn request_jump_in_server_status(&mut self, address: &str) {
        if self.server_statuses.contains_key(address)
            || !self.jump_in_server_pings_pending.insert(address.to_owned())
        {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.jump_in_server_pings_pending.remove(address);
            return;
        };
        let address = address.to_owned();
        self.runtime.spawn(async move {
            let result = backend
                .server_status(&address)
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::ServerPingLoaded(address, result));
        });
    }

    fn load_shared_instance(&mut self, instance_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.message = "Loading sharing details…".into();
        self.runtime.spawn(async move {
            let result = async {
                let metadata = backend
                    .get_instance(&instance_id)
                    .await
                    .map_err(|error| error.to_string())?;
                let can_use = backend
                    .can_use_shared_instances()
                    .await
                    .map_err(|error| error.to_string())?;
                let (users, invites) = if can_use {
                    if metadata
                        .as_ref()
                        .and_then(|metadata| metadata.shared_instance.as_ref())
                        .is_some()
                    {
                        let users = backend
                            .shared_instance_users(&instance_id)
                            .await
                            .map_err(|error| error.to_string())?;
                        let invites = if metadata
                            .as_ref()
                            .and_then(|metadata| {
                                metadata.shared_instance.as_ref()
                            })
                            .is_some_and(|attachment| {
                                attachment.role
                                    == theseus::data::SharedInstanceRole::Owner
                            }) {
                            backend
                                .shared_instance_invites(&instance_id)
                                .await
                                .map_err(|error| error.to_string())?
                        } else {
                            Vec::new()
                        };
                        (Some(users), invites)
                    } else {
                        (None, Vec::new())
                    }
                } else {
                    (None, Vec::new())
                };
                Ok((metadata, can_use, users, invites))
            }
            .await;
            let _ = proxy.send_event(UiMessage::SharedInstanceLoaded(
                instance_id,
                result,
            ));
        });
    }

    fn preview_shared_instance_publish(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.shared_instance_busy = true;
        self.shared_instance_preview = None;
        self.message = "Reviewing files and shared content…".into();
        self.runtime.spawn(async move {
            let result = backend
                .shared_instance_publish_preview(&instance_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::SharedInstancePublishPreview(result));
        });
    }

    fn unlink_imported_modpack_for_sharing(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.shared_instance_busy = true;
        self.message = "Removing the imported modpack link…".into();
        self.runtime.spawn(async move {
            let result = backend
                .edit_instance(
                    &instance_id,
                    theseus::data::EditInstance {
                        link: Some(theseus::data::InstanceLink::Unmanaged),
                        ..Default::default()
                    },
                )
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::SharedModpackUnlinked(result));
        });
    }

    fn search_shared_instance_users(&mut self) {
        let query = self.shared_instance_user_query.trim().to_owned();
        if query.chars().count() < 2 {
            self.message = "Enter at least two characters to search.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.shared_instance_busy = true;
        self.message = format!("Searching users for {query}…");
        self.runtime.spawn(async move {
            let result = backend
                .search_users(&query)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::SharedUsersSearched(result));
        });
    }

    fn invite_selected_shared_instance_users(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let user_ids = self
            .shared_instance_selected_user_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if user_ids.is_empty() {
            self.message = "Select at least one user to invite.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.shared_instance_busy = true;
        self.message = format!("Inviting {} player(s)…", user_ids.len());
        self.runtime.spawn(async move {
            let result = backend
                .invite_shared_instance_users(&instance_id, user_ids)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::SharedUsersInvited(result));
        });
    }

    fn remove_shared_instance_user(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(user_id) = self.shared_instance_pending_removal.clone() else {
            return;
        };
        let has_pending_recipients = self
            .shared_instance_users
            .iter()
            .any(|user| user.id != user_id && user.joined_at.is_none());
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.shared_instance_busy = true;
        self.message = "Revoking shared-instance access…".into();
        self.runtime.spawn(async move {
            let result = backend
                .remove_shared_instance_users(
                    &instance_id,
                    vec![user_id.clone()],
                    has_pending_recipients,
                )
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::SharedUserRemoved(user_id, result));
        });
    }

    fn publish_shared_instance_selection(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(preview) = self.shared_instance_preview.as_ref() else {
            return;
        };
        let file_paths = preview
            .files
            .iter()
            .filter(|file| {
                !file.blocked
                    && self.shared_instance_selected_files.contains(&file.path)
            })
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.shared_instance_busy = true;
        self.shared_instance_progress = Some((0, 0));
        self.message = "Publishing shared-instance update…".into();
        self.runtime.spawn(async move {
            let progress_proxy = proxy.clone();
            let result = backend
                .publish_shared_instance(
                    &instance_id,
                    file_paths,
                    move |uploaded, total| {
                        let _ = progress_proxy.send_event(
                            UiMessage::SharedInstancePublishProgress(
                                uploaded, total,
                            ),
                        );
                    },
                )
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::SharedInstancePublished(result));
        });
    }

    fn create_shared_instance_invite(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.shared_instance_busy = true;
        self.message = "Creating an invite link…".into();
        self.runtime.spawn(async move {
            let result = backend
                .create_shared_instance_invite_link(
                    &instance_id,
                    Some(7 * 24 * 60 * 60),
                    Some(10),
                    None,
                )
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::SharedInstanceInviteCreated(result));
        });
    }

    fn revoke_shared_instance_invite(&mut self, invite_id: String) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.shared_instance_busy = true;
        self.message = "Revoking invite link…".into();
        self.runtime.spawn(async move {
            let result = backend
                .revoke_shared_instance_invite(&instance_id, invite_id.clone())
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::SharedInstanceInviteRevoked(
                invite_id, result,
            ));
        });
    }

    fn copy_shared_instance_invite(&mut self, url: String) {
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.message = "Copying invite link…".into();
        self.runtime.spawn(async move {
            let clipboard_url = url.clone();
            let result = tokio::task::spawn_blocking(move || {
                set_clipboard_text(&clipboard_url)
            })
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result);
            let _ = proxy
                .send_event(UiMessage::SharedInstanceInviteCopied(url, result));
        });
    }

    fn open_minecraft_skins(&mut self) {
        self.global_hosting = false;
        self.global_browse = false;
        self.global_screenshots = false;
        self.global_skins = true;
        self.global_user_profile = false;
        self.settings_open = false;
        self.accounts_open = false;
        self.create_instance_open = false;
        self.selected_instance = None;
        self.selected_group = None;
        self.minecraft_skin_scroll_pixels = 0.0;
        self.minecraft_skin_scroll_target_pixels = 0.0;
        self.input_focus = 0;
        self.load_minecraft_skins();
    }

    fn open_user_profile(&mut self, user_id: String) {
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.global_browse = false;
        self.global_screenshots = false;
        self.global_hosting = false;
        self.global_skins = false;
        self.global_user_profile = true;
        self.user_profile_id = Some(user_id.clone());
        self.user_profile = None;
        self.user_profile_projects.clear();
        self.user_profile_filtered_project_indices.clear();
        self.user_profile_organizations.clear();
        self.user_profile_collections.clear();
        self.user_profile_type_filter = None;
        self.user_profile_show_collections = false;
        self.user_profile_error = None;
        self.user_profile_loading = true;
        self.user_profile_scroll_pixels = 0.0;
        self.user_profile_scroll_target_pixels = 0.0;
        self.selected_instance = None;
        self.selected_group = None;
        self.accounts_open = false;
        self.settings_open = false;
        self.java_open = false;
        self.input_focus = 0;
        self.runtime.spawn(async move {
            let result = async {
                let (profile, projects, organizations, collections) =
                    tokio::try_join!(
                        backend.user_profile(&user_id),
                        backend.user_projects(&user_id),
                        backend.user_organizations(&user_id),
                        backend.user_collections(&user_id),
                    )
                    .map_err(|error| error.to_string())?;
                Ok((profile, projects, organizations, collections))
            }
            .await;
            let _ =
                proxy.send_event(UiMessage::UserProfileLoaded(user_id, result));
        });
    }

    fn load_minecraft_skins(&mut self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.message = "Loading Minecraft skins…".into();
        self.runtime.spawn(async move {
            let result = backend
                .minecraft_skins()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::MinecraftSkinsLoaded(result));
        });
    }

    fn request_minecraft_skin_preview(
        &mut self,
        skin: theseus::minecraft_skins::Skin,
    ) {
        let key = skin.texture_key.to_string();
        if self.minecraft_skin_previews.contains_key(&key)
            || self.minecraft_skin_preview_failures.contains(&key)
            || !self.minecraft_skin_previews_pending.insert(key.clone())
        {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.minecraft_skin_previews_pending.remove(&key);
            return;
        };
        self.runtime.spawn(async move {
            let result = match backend.minecraft_skin_texture(&skin).await {
                Ok(bytes) => match spawn_image_decode(move || {
                    decode_skin_texture(&bytes)
                })
                .await
                {
                    Ok(result) => result,
                    Err(error) => Err(error.to_string()),
                },
                Err(error) => Err(error.to_string()),
            };
            let _ = proxy
                .send_event(UiMessage::MinecraftSkinPreviewLoaded(key, result));
        });
    }

    fn equip_minecraft_skin(&mut self, skin: theseus::minecraft_skins::Skin) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.message = format!(
            "Equipping {}…",
            skin.name.as_deref().unwrap_or("Minecraft skin")
        );
        let label =
            skin.name.as_deref().unwrap_or("Minecraft skin").to_string();
        self.runtime.spawn(async move {
            let result = backend
                .equip_minecraft_skin(skin)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::MinecraftSkinActionFinished(
                label, result,
            ));
        });
    }

    fn remove_minecraft_skin(&mut self, skin: theseus::minecraft_skins::Skin) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let name = skin.name.as_deref().unwrap_or("Minecraft skin").to_string();
        self.skin_delete_confirmation = None;
        self.message = format!("Removing saved skin {name}…");
        self.runtime.spawn(async move {
            let result = backend
                .remove_custom_minecraft_skin(skin)
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::MinecraftSkinRemoved(name, result));
        });
    }

    fn pick_minecraft_skin(&mut self) {
        if !self.ready {
            self.message = "Launcher services are still starting.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        let variant = self.minecraft_skin_variant;
        self.message = "Choose a Minecraft skin PNG…".into();
        std::thread::spawn(move || {
            let selected = runtime.block_on(async {
                rfd::AsyncFileDialog::new()
                    .add_filter("Minecraft skin PNG", &["png"])
                    .pick_file()
                    .await
                    .map(|file| file.path().to_path_buf())
            });
            let result = if let Some(path) = selected {
                match std::fs::read(&path) {
                    Ok(texture) => runtime
                        .block_on(
                            backend
                                .add_and_equip_minecraft_skin(texture, variant),
                        )
                        .map_err(|error| error.to_string()),
                    Err(error) => Err(error.to_string()),
                }
            } else {
                Err("Skin selection canceled.".to_string())
            };
            let _ = proxy.send_event(UiMessage::MinecraftSkinActionFinished(
                "custom skin".into(),
                result,
            ));
        });
    }

    fn open_global_screenshots(&mut self) {
        self.global_hosting = false;
        self.global_browse = false;
        self.global_screenshots = true;
        self.global_skins = false;
        self.global_user_profile = false;
        self.settings_open = false;
        self.java_open = false;
        self.accounts_open = false;
        self.create_instance_open = false;
        self.selected_group = None;
        self.selected_instance =
            Some((GLOBAL_SCREENSHOTS_ID.to_owned(), "SCREENSHOTS".to_owned()));
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
        self.message = "Loading screenshots…".into();
        self.load_screenshots(None);
    }

    fn open_global_hosting(&mut self) {
        self.global_browse = false;
        self.global_screenshots = false;
        self.global_skins = false;
        self.global_hosting = true;
        self.global_user_profile = false;
        self.settings_open = false;
        self.java_open = false;
        self.accounts_open = false;
        self.create_instance_open = false;
        self.selected_group = None;
        self.selected_instance = None;
        self.input_focus = 0;
        self.hosting_servers.clear();
        self.hosting_servers_error = None;
        self.hosting_servers_loading = true;
        self.hosting_servers_refreshing = false;
        self.hosting_scroll_pixels = 0.0;
        self.hosting_scroll_target_pixels = 0.0;
        self.hosting_server_id = None;
        self.hosting_server_details = None;
        self.hosting_server_details_loading = false;
        self.hosting_server_details_error = None;
        self.hosting_tab = 0;
        self.hosting_backups = None;
        self.hosting_backups_loading = false;
        self.hosting_backups_error = None;
        self.hosting_backup_create_open = false;
        self.hosting_backup_create_name.clear();
        self.hosting_backup_search.clear();
        self.hosting_backup_show_deleted = false;
        self.hosting_backup_deleted_reason_filter = 0;
        self.hosting_backup_deleted_type_filter = 0;
        self.hosting_backup_deleted_recoverability_filter = 0;
        self.hosting_backup_deleted_sort = 0;
        self.hosting_backup_options_open = false;
        self.hosting_backup_auto_enabled = false;
        self.hosting_backup_manual_slots = 0;
        self.hosting_backup_interval_hours = 24;
        self.hosting_backup_options_saving = false;
        self.hosting_backup_rotation_confirmation = false;
        self.hosting_backup_rotation_count = 0;
        self.hosting_backup_selected.clear();
        self.hosting_backup_creating = false;
        self.hosting_backup_prompt = None;
        self.hosting_backup_mutating = false;
        self.hosting_backup_downloading = false;
        self.hosting_backup_scroll_pixels = 0.0;
        self.hosting_backup_scroll_target_pixels = 0.0;
        self.hosting_access_users.clear();
        self.hosting_access_loading = false;
        self.hosting_access_loaded = false;
        self.hosting_access_error = None;
        self.hosting_access_remove_confirmation = None;
        self.hosting_access_scroll_pixels = 0.0;
        self.hosting_access_scroll_target_pixels = 0.0;
        self.hosting_access_invite_open = false;
        self.hosting_access_search_query.clear();
        self.hosting_access_search_results.clear();
        self.hosting_access_search_loading = false;
        self.hosting_access_search_complete = false;
        self.hosting_access_search_scroll_pixels = 0.0;
        self.hosting_access_search_scroll_target_pixels = 0.0;
        self.hosting_access_invite_error = None;
        self.hosting_access_role = 0;
        self.hosting_connection = None;
        self.hosting_socket_connecting = false;
        self.hosting_socket_connected = false;
        self.hosting_socket_error = None;
        self.hosting_power_state = None;
        self.hosting_live_stats = None;
        self.hosting_console_lines.clear();
        self.hosting_console_input.clear();
        self.hosting_console_scroll_pixels = 0.0;
        self.hosting_console_scroll_target_pixels = 0.0;
        self.hosting_console_follow_tail = true;
        self.hosting_power_pending = false;
        self.hosting_power_error = None;
        self.hosting_address_copied = false;
        self.load_hosting_servers();
    }

    fn open_hosting_server(&mut self, server_id: String) {
        self.global_browse = false;
        self.global_screenshots = false;
        self.global_skins = false;
        self.global_hosting = true;
        self.global_user_profile = false;
        self.settings_open = false;
        self.java_open = false;
        self.accounts_open = false;
        self.create_instance_open = false;
        self.selected_instance = None;
        self.hosting_server_id = Some(server_id.clone());
        self.hosting_server_details = None;
        self.hosting_server_details_error = None;
        self.hosting_server_details_loading = true;
        self.hosting_tab = 0;
        self.hosting_backups = None;
        self.hosting_backups_loading = false;
        self.hosting_backups_error = None;
        self.hosting_backup_create_open = false;
        self.hosting_backup_create_name.clear();
        self.hosting_backup_search.clear();
        self.hosting_backup_show_deleted = false;
        self.hosting_backup_deleted_reason_filter = 0;
        self.hosting_backup_deleted_type_filter = 0;
        self.hosting_backup_deleted_recoverability_filter = 0;
        self.hosting_backup_deleted_sort = 0;
        self.hosting_backup_options_open = false;
        self.hosting_backup_auto_enabled = false;
        self.hosting_backup_manual_slots = 0;
        self.hosting_backup_interval_hours = 24;
        self.hosting_backup_options_saving = false;
        self.hosting_backup_rotation_confirmation = false;
        self.hosting_backup_rotation_count = 0;
        self.hosting_backup_selected.clear();
        self.hosting_backup_creating = false;
        self.hosting_backup_prompt = None;
        self.hosting_backup_mutating = false;
        self.hosting_backup_downloading = false;
        self.hosting_backup_scroll_pixels = 0.0;
        self.hosting_backup_scroll_target_pixels = 0.0;
        self.hosting_access_users.clear();
        self.hosting_access_loading = false;
        self.hosting_access_loaded = false;
        self.hosting_access_error = None;
        self.hosting_access_remove_confirmation = None;
        self.hosting_access_scroll_pixels = 0.0;
        self.hosting_access_scroll_target_pixels = 0.0;
        self.hosting_access_invite_open = false;
        self.hosting_access_search_query.clear();
        self.hosting_access_search_results.clear();
        self.hosting_access_search_loading = false;
        self.hosting_access_search_complete = false;
        self.hosting_access_search_scroll_pixels = 0.0;
        self.hosting_access_search_scroll_target_pixels = 0.0;
        self.hosting_access_invite_error = None;
        self.hosting_access_role = 0;
        self.hosting_connection = None;
        self.hosting_socket_connecting = true;
        self.hosting_socket_connected = false;
        self.hosting_socket_error = None;
        self.hosting_power_state = None;
        self.hosting_live_stats = None;
        self.hosting_console_lines.clear();
        self.hosting_console_input.clear();
        self.hosting_console_scroll_pixels = 0.0;
        self.hosting_console_scroll_target_pixels = 0.0;
        self.hosting_console_follow_tail = true;
        self.hosting_power_pending = false;
        self.hosting_power_error = None;
        self.hosting_address_copied = false;
        self.load_hosting_server_details(server_id.clone());
        self.connect_hosting_server(server_id);
    }

    fn connect_hosting_server(&self, server_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .connect_hosting_server(&server_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingServerConnected(
                server_id, result,
            ));
        });
    }

    fn request_hosting_power(&mut self, action: HostingPowerAction) {
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        if self.hosting_power_pending {
            return;
        }
        self.hosting_power_pending = true;
        self.hosting_power_error = None;
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_power(&server_id, action)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingPowerActionFinished(
                server_id, result,
            ));
        });
    }

    fn send_hosting_console_command(&mut self) {
        let command = std::mem::take(&mut self.hosting_console_input);
        if let Some(connection) = self.hosting_connection.as_ref() {
            if let Err(error) = connection.send_command(command.clone()) {
                self.hosting_socket_error = Some(error);
            } else if !command.trim().is_empty() {
                self.hosting_console_lines.push_back(format!("> {command}"));
            }
        } else if !command.trim().is_empty() {
            self.hosting_socket_error =
                Some("The server console is disconnected.".into());
        }
        self.input_focus = 0;
        self.draw();
    }

    fn load_hosting_server_details(&self, server_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_details(&server_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::HostingServerLoaded(server_id, result));
        });
    }

    fn load_hosting_backups(&mut self) {
        if self.hosting_backups_loading || self.hosting_backups.is_some() {
            return;
        }
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        self.hosting_backups_loading = true;
        self.hosting_backups_error = None;
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_backups(&server_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::HostingBackupsLoaded(server_id, result));
        });
    }

    fn open_hosting_backup_options(&mut self) {
        if !self.hosting_can_manage_backups() {
            self.message =
                "You do not have permission to change backup settings.".into();
            self.draw();
            return;
        }
        let Some(options) = self
            .hosting_backups
            .as_ref()
            .map(|backups| backups.options.clone())
        else {
            return;
        };
        let slot_limit = options.limit.min(15);
        self.hosting_backup_auto_enabled = options.split_automatic > 0;
        self.hosting_backup_interval_hours = options.interval.max(1);
        self.hosting_backup_manual_slots =
            options
                .split_manual
                .min(if self.hosting_backup_auto_enabled {
                    slot_limit.saturating_sub(1)
                } else {
                    slot_limit
                });
        self.hosting_backup_create_open = false;
        self.hosting_backup_selected.clear();
        self.hosting_backup_options_open = true;
        self.hosting_backup_rotation_confirmation = false;
        self.input_focus = 0;
        self.draw();
    }

    fn cancel_hosting_backup_options(&mut self) {
        self.hosting_backup_options_open = false;
        self.hosting_backup_rotation_confirmation = false;
        self.input_focus = 0;
        self.draw();
    }

    fn save_hosting_backup_options(&mut self, acknowledge_rotation: bool) {
        if self.hosting_backup_options_saving {
            return;
        }
        if !self.hosting_can_manage_backups() {
            self.message =
                "You do not have permission to change backup settings.".into();
            self.draw();
            return;
        }
        let Some((world_id, options)) = self
            .hosting_backups
            .as_ref()
            .map(|backups| (backups.world_id.clone(), backups.options.clone()))
        else {
            return;
        };
        let slot_limit = options.limit.min(15);
        if self.hosting_backup_auto_enabled && slot_limit == 0 {
            self.message = "This server has no backup slots available.".into();
            self.draw();
            return;
        }
        let patch = if self.hosting_backup_auto_enabled {
            let manual = self
                .hosting_backup_manual_slots
                .min(slot_limit.saturating_sub(1));
            native_backend::HostingBackupOptionsPatch {
                split_automatic: slot_limit.saturating_sub(manual),
                split_manual: manual,
                interval: Some(self.hosting_backup_interval_hours.max(1)),
            }
        } else {
            native_backend::HostingBackupOptionsPatch {
                split_automatic: 0,
                split_manual: slot_limit,
                interval: None,
            }
        };
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        self.hosting_backup_options_saving = true;
        self.message = "Saving backup settings…".into();
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_update_backup_options(
                    &server_id,
                    &world_id,
                    patch,
                    acknowledge_rotation,
                )
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingBackupOptionsFinished(
                server_id,
                acknowledge_rotation,
                result,
            ));
        });
    }

    fn download_hosting_backup(
        &mut self,
        backup: native_backend::HostingBackup,
    ) {
        if self.hosting_backup_downloading || backup.status != "done" {
            return;
        }
        let (Some(server_id), Some(world_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.hosting_backups
                .as_ref()
                .map(|backups| backups.world_id.clone()),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        self.hosting_backup_downloading = true;
        self.message = format!("Choose where to save {}…", backup.name);
        self.draw();
        let runtime = self.runtime.clone();
        runtime.spawn(async move {
            let file_name =
                format!("{}.zip", safe_export_filename(&backup.name));
            let result = if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("ZIP archive", &["zip"])
                .set_file_name(&file_name)
                .save_file()
                .await
            {
                let destination = file.path().to_path_buf();
                backend
                    .hosting_server_download_backup(
                        &server_id,
                        &world_id,
                        &backup.id,
                        destination.clone(),
                    )
                    .await
                    .map(|()| Some(destination))
                    .map_err(|error| error.to_string())
            } else {
                Ok(None)
            };
            let _ = proxy.send_event(UiMessage::HostingBackupDownloadFinished(
                server_id, result,
            ));
        });
    }

    fn undelete_hosting_backup(
        &mut self,
        backup: native_backend::HostingDeletedBackup,
    ) {
        if self.hosting_backup_mutating || !backup.can_undelete {
            return;
        }
        let (Some(server_id), Some(world_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.hosting_backups
                .as_ref()
                .map(|backups| backups.world_id.clone()),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        self.hosting_backup_mutating = true;
        self.message = format!("Recovering {}…", backup.name);
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_undelete_backup(
                    &server_id, &world_id, &backup.id,
                )
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingBackupMutationFinished(
                server_id,
                "Backup recovered.".into(),
                result,
            ));
        });
    }

    fn undelete_selected_hosting_backups(&mut self) {
        if self.hosting_backup_mutating {
            return;
        }
        let backup_ids =
            self.hosting_backups
                .as_ref()
                .map_or_else(Vec::new, |backups| {
                    backups
                        .deleted_backups
                        .iter()
                        .filter(|backup| {
                            backup.can_undelete
                                && self
                                    .hosting_backup_selected
                                    .contains(&backup.id)
                        })
                        .map(|backup| backup.id.clone())
                        .collect()
                });
        if backup_ids.is_empty() {
            self.message =
                "None of the selected backups can be recovered.".into();
            self.draw();
            return;
        }
        let (Some(server_id), Some(world_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.hosting_backups
                .as_ref()
                .map(|backups| backups.world_id.clone()),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        self.hosting_backup_mutating = true;
        self.message = format!("Recovering {} backups…", backup_ids.len());
        self.draw();
        self.runtime.spawn(async move {
            let outcomes = backend
                .hosting_server_undelete_backups(
                    &server_id,
                    &world_id,
                    &backup_ids,
                )
                .await;
            let mut recovered = Vec::new();
            let mut failures = Vec::new();
            for (id, outcome) in outcomes {
                match outcome {
                    Ok(()) => recovered.push(id),
                    Err(error) => failures.push((id, error)),
                }
            }
            let _ =
                proxy.send_event(UiMessage::HostingBackupBulkUndeleteFinished(
                    server_id, recovered, failures,
                ));
        });
    }

    fn create_hosting_backup(&mut self) {
        if self.hosting_backup_creating {
            return;
        }
        if !self.hosting_can_manage_backups() {
            self.message =
                "You do not have permission to manage backups.".into();
            self.draw();
            return;
        }
        let Some(world_id) = self
            .hosting_backups
            .as_ref()
            .map(|backups| backups.world_id.clone())
        else {
            return;
        };
        if self.hosting_backups.as_ref().is_some_and(|backups| {
            backups.manual_count >= backups.options.split_manual
        }) {
            self.message = "All manual backup slots are in use.".into();
            self.draw();
            return;
        }
        let Some(server_id) = self.hosting_server_id.clone() else {
            return;
        };
        let name = {
            let entered = self.hosting_backup_create_name.trim();
            if entered.is_empty() {
                format!(
                    "Backup #{}",
                    self.hosting_backups
                        .as_ref()
                        .map_or(1, |backups| backups.backups.len() + 1)
                )
            } else {
                entered.to_owned()
            }
        };
        if name.chars().count() > 64 {
            self.message =
                "Backup names must be 64 characters or fewer.".into();
            self.draw();
            return;
        }
        if self.hosting_backups.as_ref().is_some_and(|backups| {
            backups.backups.iter().any(|backup| {
                backup.name.trim().eq_ignore_ascii_case(name.trim())
            })
        }) {
            self.message = "A backup with that name already exists.".into();
            self.draw();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.hosting_backup_creating = true;
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_create_backup(&server_id, &world_id, &name)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::HostingBackupCreated(server_id, result));
        });
    }

    fn open_hosting_backup_prompt(
        &mut self,
        action: HostingBackupPromptAction,
        backup: native_backend::HostingBackup,
    ) {
        if !self.hosting_can_manage_backups() {
            self.message =
                "You do not have permission to manage backups.".into();
            self.draw();
            return;
        }
        if action == HostingBackupPromptAction::Restore
            && self.hosting_server_is_running()
        {
            self.message = "Stop the server before restoring a backup.".into();
            self.draw();
            return;
        }
        self.hosting_backup_create_open = false;
        self.hosting_backup_prompt = Some(HostingBackupPrompt {
            action,
            backup: backup.clone(),
        });
        if action == HostingBackupPromptAction::Rename {
            self.hosting_backup_create_name = backup.name;
            self.input_focus = 39;
        } else {
            self.hosting_backup_create_name.clear();
            self.input_focus = 0;
        }
        self.draw();
    }

    fn open_hosting_backup_bulk_delete_prompt(&mut self) {
        if !self.hosting_can_manage_backups()
            || self.hosting_backup_selected.is_empty()
        {
            return;
        }
        let count = self.hosting_backup_selected.len();
        self.hosting_backup_create_open = false;
        self.hosting_backup_prompt = Some(HostingBackupPrompt {
            action: HostingBackupPromptAction::Delete,
            backup: native_backend::HostingBackup {
                id: String::new(),
                name: format!("{count} selected backups"),
                created_at: String::new(),
                status: "done".into(),
                automated: false,
                history: Vec::new(),
            },
        });
        self.input_focus = 0;
        self.draw();
    }

    fn hosting_server_is_running(&self) -> bool {
        self.hosting_power_state.as_deref().or_else(|| {
            self.hosting_server_details
                .as_ref()
                .map(|details| details.server.status.as_str())
        }) == Some("running")
    }

    fn hosting_can_manage_backups(&self) -> bool {
        self.hosting_server_details.as_ref().is_some_and(|details| {
            hosting_permissions_allow_backups(&details.current_user_permissions)
        })
    }

    fn submit_hosting_backup_prompt(&mut self) {
        if self.hosting_backup_mutating {
            return;
        }
        let Some(prompt) = self.hosting_backup_prompt.clone() else {
            return;
        };
        if !self.hosting_can_manage_backups() {
            self.message =
                "You do not have permission to manage backups.".into();
            self.draw();
            return;
        }
        if prompt.action == HostingBackupPromptAction::Restore
            && self.hosting_server_is_running()
        {
            self.message = "Stop the server before restoring a backup.".into();
            self.draw();
            return;
        }
        let Some((world_id, backups)) = self
            .hosting_backups
            .as_ref()
            .map(|backups| (backups.world_id.clone(), backups.backups.clone()))
        else {
            return;
        };
        let server_id = self.hosting_server_id.clone();
        let Some(server_id) = server_id else {
            return;
        };
        let name = match prompt.action {
            HostingBackupPromptAction::Rename => {
                let name = self.hosting_backup_create_name.trim().to_owned();
                if name.is_empty() || name.chars().count() > 64 {
                    self.message =
                        "Backup names must be 1–64 characters.".into();
                    self.draw();
                    return;
                }
                if backups.iter().any(|backup| {
                    backup.id != prompt.backup.id
                        && backup.name.trim().eq_ignore_ascii_case(&name)
                }) {
                    self.message =
                        "A backup with that name already exists.".into();
                    self.draw();
                    return;
                }
                name
            }
            HostingBackupPromptAction::Restore => {
                format!("Before restoring \"{}\"", prompt.backup.name)
                    .chars()
                    .take(92)
                    .collect()
            }
            HostingBackupPromptAction::Delete => String::new(),
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let backup_id = prompt.backup.id.clone();
        let delete_ids = if backup_id.is_empty() {
            self.hosting_backup_selected
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        } else {
            vec![backup_id.clone()]
        };
        if prompt.action == HostingBackupPromptAction::Delete
            && delete_ids.is_empty()
        {
            return;
        }
        let (operation, success_message) = match prompt.action {
            HostingBackupPromptAction::Restore => {
                ("restore", "Backup restore queued.")
            }
            HostingBackupPromptAction::Rename => ("rename", "Backup renamed."),
            HostingBackupPromptAction::Delete => (
                "delete",
                if delete_ids.len() == 1 {
                    "Backup deleted."
                } else {
                    "Backups deleted."
                },
            ),
        };
        let operation = operation.to_owned();
        let success_message = success_message.to_owned();
        self.hosting_backup_mutating = true;
        self.draw();
        self.runtime.spawn(async move {
            let result = match operation.as_str() {
                "restore" => {
                    backend
                        .hosting_server_restore_backup(
                            &server_id, &world_id, &backup_id, &name,
                        )
                        .await
                }
                "rename" => {
                    backend
                        .hosting_server_rename_backup(
                            &server_id, &world_id, &backup_id, &name,
                        )
                        .await
                }
                _ => {
                    if delete_ids.len() == 1 {
                        backend
                            .hosting_server_delete_backup(
                                &server_id,
                                &world_id,
                                &delete_ids[0],
                            )
                            .await
                    } else {
                        backend
                            .hosting_server_delete_backups(
                                &server_id,
                                &world_id,
                                &delete_ids,
                            )
                            .await
                    }
                }
            }
            .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingBackupMutationFinished(
                server_id,
                success_message,
                result,
            ));
        });
    }

    fn load_hosting_server_access(&mut self) {
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        if self.hosting_access_loading || self.hosting_access_loaded {
            return;
        }
        self.hosting_access_loading = true;
        self.hosting_access_error = None;
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_access(&server_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::HostingAccessLoaded(server_id, result));
        });
    }

    fn search_hosting_access_users(&mut self) {
        let query = self.hosting_access_search_query.trim().to_owned();
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        if query.is_empty() || self.hosting_access_search_loading {
            return;
        }
        self.hosting_access_search_loading = true;
        self.hosting_access_search_complete = false;
        self.hosting_access_search_scroll_pixels = 0.0;
        self.hosting_access_search_scroll_target_pixels = 0.0;
        self.hosting_access_invite_error = None;
        self.runtime.spawn(async move {
            let result = backend
                .search_users(&query)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingAccessSearchLoaded(
                server_id, query, result,
            ));
        });
        self.draw();
    }

    fn invite_hosting_access_user(&mut self, user: theseus::users::SearchUser) {
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        if self.hosting_access_loading
            || self
                .hosting_access_users
                .iter()
                .any(|member| member.user.id.eq_ignore_ascii_case(&user.id))
        {
            return;
        }
        self.hosting_access_loading = true;
        self.hosting_access_invite_error = None;
        let role = if self.hosting_access_role == 0 {
            native_backend::HostingServerAccessRole::Viewer
        } else {
            native_backend::HostingServerAccessRole::Editor
        };
        let username = user.username;
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_add_user(&server_id, &user.id, role)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingAccessInviteFinished(
                server_id, username, result,
            ));
        });
        self.draw();
    }

    fn update_hosting_access_role(
        &mut self,
        user: &native_backend::HostingServerAccessUser,
        role: native_backend::HostingServerAccessRole,
    ) {
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        if self.hosting_access_loading {
            return;
        }
        let user_id = user.user.id.clone();
        let username = user.user.username.clone();
        self.hosting_access_loading = true;
        self.hosting_access_error = None;
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_update_user_role(&server_id, &user_id, role)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingAccessRoleFinished(
                server_id, user_id, username, role, result,
            ));
        });
    }

    fn remove_hosting_access_user(
        &mut self,
        user: &native_backend::HostingServerAccessUser,
    ) {
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        if self.hosting_access_loading {
            return;
        }
        let user_id = user.user.id.clone();
        let username = user.user.username.clone();
        self.hosting_access_loading = true;
        self.hosting_access_error = None;
        self.message = format!("Removing {username} from server access…");
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_remove_user(&server_id, &user_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingAccessRemoveFinished(
                server_id, user_id, username, result,
            ));
        });
    }

    fn reinvite_hosting_access_user(
        &mut self,
        user: &native_backend::HostingServerAccessUser,
    ) {
        let (Some(server_id), Some(backend), Some(proxy)) = (
            self.hosting_server_id.clone(),
            self.backend.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        if self.hosting_access_loading || user.added_on.is_some() {
            return;
        }
        let user_id = user.user.id.clone();
        let username = user.user.username.clone();
        self.hosting_access_loading = true;
        self.hosting_access_error = None;
        self.message = format!("Resending invitation to {username}…");
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .hosting_server_reinvite_user(&server_id, &user_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingAccessReinviteFinished(
                server_id, username, result,
            ));
        });
    }

    fn load_hosting_servers(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .hosting_servers(50, 0)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HostingServersLoaded(result));
        });
    }

    fn add_server_to_instance(&mut self) {
        if self.add_server_busy {
            return;
        }
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let address = self.server_address_input.trim().to_owned();
        if address.is_empty()
            || address.chars().any(char::is_control)
            || address.len() > 255
        {
            self.message = "Enter a valid server address.".into();
            return;
        }
        let name = self.server_name_input.trim();
        if name.chars().count() > 64 || name.chars().any(char::is_control) {
            self.message =
                "Server names must be 64 characters or fewer.".into();
            return;
        }
        let server_name = if name.is_empty() {
            address.clone()
        } else {
            name.to_owned()
        };
        let pack_status = self.server_pack_status;
        let edit_index = self.server_edit_index;
        self.add_server_busy = true;
        self.message = if edit_index.is_some() {
            "Saving server…".into()
        } else {
            "Adding server…".into()
        };
        self.runtime.spawn(async move {
            if let Some(index) = edit_index {
                let result = backend
                    .edit_server_in_instance(
                        &instance_id,
                        index,
                        server_name.clone(),
                        address,
                        pack_status,
                    )
                    .await
                    .map_err(|error| error.to_string());
                let _ = proxy.send_event(UiMessage::ServerSaved(
                    instance_id,
                    server_name,
                    result,
                ));
            } else {
                let result = backend
                    .add_server_to_instance(
                        &instance_id,
                        server_name.clone(),
                        address,
                        pack_status,
                    )
                    .await
                    .map_err(|error| error.to_string());
                let _ = proxy.send_event(UiMessage::ServerAdded(
                    instance_id,
                    server_name,
                    result,
                ));
            }
        });
    }

    fn remove_server_from_instance(&mut self) {
        if self.add_server_busy {
            return;
        }
        let Some((index, name, _)) = self.server_remove_confirmation.clone()
        else {
            return;
        };
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.add_server_busy = true;
        self.message = format!("Removing server {name}…");
        self.runtime.spawn(async move {
            let result = backend
                .remove_server_from_instance(&instance_id, index)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ServerRemoved(
                instance_id,
                name,
                result,
            ));
        });
    }

    fn save_singleplayer_world(&mut self) {
        if self.add_server_busy {
            return;
        }
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let name = self.world_edit_name.trim().to_owned();
        if name.is_empty()
            || name.chars().count() > 64
            || name.chars().any(char::is_control)
        {
            self.message =
                "World names must contain 1 to 64 characters.".into();
            return;
        }
        let world_id = self.world_edit_path.clone();
        let reset_icon = self.world_edit_reset_icon;
        self.add_server_busy = true;
        self.message = format!("Saving world {name}…");
        self.runtime.spawn(async move {
            let result = backend
                .rename_singleplayer_world(
                    &instance_id,
                    &world_id,
                    &name,
                    reset_icon,
                )
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::WorldRenamed(
                instance_id,
                name,
                result,
            ));
        });
    }

    fn delete_singleplayer_world(&mut self) {
        if self.add_server_busy {
            return;
        }
        let Some((world_id, world_name)) =
            self.world_delete_confirmation.clone()
        else {
            return;
        };
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.add_server_busy = true;
        self.message = format!("Deleting world {world_name}…");
        self.runtime.spawn(async move {
            let result = backend
                .delete_singleplayer_world(&instance_id, &world_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::WorldDeleted(
                instance_id,
                world_name,
                result,
            ));
        });
    }

    fn update_world_status(
        &mut self,
        world: &worlds::World,
        status: worlds::DisplayStatus,
    ) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(backend) = self.backend.clone() else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let world_type = world.world_type();
        let world_id = world.world_id().to_owned();
        self.message = "Updating world visibility…".into();
        self.runtime.spawn(async move {
            let result = backend
                .set_world_display_status(
                    &instance_id,
                    world_type,
                    &world_id,
                    status,
                )
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::WorldStatusUpdated(instance_id, result));
        });
    }

    fn clear_live_console(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.as_ref() else {
            return;
        };
        if let Some(backend) = self.backend.as_ref() {
            backend.clear_live_log_buffer(instance_id);
        }
        self.console_text.clear();
        self.console_scroll = 0;
        self.console_scroll_synced = 0;
        self.console_scroll_pixels = 0.0;
        self.console_scroll_target_pixels = 0.0;
        self.draw();
    }

    fn toggle_log_level_filter(&mut self, level: u8) {
        self.log_level_filter =
            toggle_native_log_filter(self.log_level_filter, level);
        self.console_scroll = 0;
        self.console_scroll_synced = 0;
        self.console_scroll_pixels = 0.0;
        self.console_scroll_target_pixels = 0.0;
        self.log_scroll = 0;
        self.log_scroll_synced = 0;
        self.log_scroll_pixels = 0.0;
        self.log_scroll_target_pixels = 0.0;
    }

    fn copy_text_to_clipboard(&self, text: String) {
        let (Some(proxy), runtime) = (self.proxy.clone(), self.runtime.clone())
        else {
            return;
        };
        runtime.spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || set_clipboard_text(&text))
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result);
            let _ = proxy.send_event(UiMessage::ClipboardTextCopied(result));
        });
    }

    fn refresh_live_log(&self, instance_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        let pending = self.console_refresh_pending.clone();
        let dirty = self.console_refresh_dirty.clone();
        if pending.swap(true, std::sync::atomic::Ordering::AcqRel) {
            dirty.store(true, std::sync::atomic::Ordering::Release);
            return;
        }
        let instance_id_for_retry = instance_id.clone();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.live_log(&instance_id))
                .map(|log| log.as_str().to_owned())
                .map_err(|error| error.to_string());
            pending.store(false, std::sync::atomic::Ordering::Release);
            let _ = proxy.send_event(UiMessage::LiveLogLoaded(
                instance_id.clone(),
                result,
            ));
            if dirty.swap(false, std::sync::atomic::Ordering::AcqRel) {
                let _ = proxy.send_event(UiMessage::LiveLogChanged(
                    instance_id_for_retry,
                ));
            }
        });
    }

    fn refresh_instance_logs(&self, instance_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.instance_logs(&instance_id))
                .map(|logs| {
                    logs.into_iter()
                        .map(|log| {
                            (
                                log.filename,
                                log.log_type
                                    == theseus::logs::LogType::CrashReport,
                                log.age,
                            )
                        })
                        .collect()
                })
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::InstanceLogsLoaded(instance_id, result));
        });
    }

    fn read_instance_log(&self, filename: String, is_crash: bool) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        std::thread::spawn(move || {
            let log_type = if is_crash {
                theseus::logs::LogType::CrashReport
            } else {
                theseus::logs::LogType::InfoLog
            };
            let result = runtime
                .block_on(backend.instance_log_content(
                    &instance_id,
                    log_type,
                    &filename,
                ))
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::InstanceLogContentLoaded(
                instance_id,
                filename,
                is_crash,
                result,
            ));
        });
    }

    fn analyse_selected_log(&mut self) {
        if self.log_analysis_pending || self.log_text.trim().is_empty() {
            return;
        }
        let (Some((instance_id, _)), Some((filename, _))) =
            (self.selected_instance.clone(), self.selected_log.clone())
        else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let content = self.log_text.clone();
        let runtime = self.runtime.clone();
        self.log_analysis_pending = true;
        self.log_analysis = None;
        self.message = "Analyzing log with mclo.gs…".into();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.analyse_minecraft_log(&content))
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::LogAnalysisFinished(
                instance_id,
                filename,
                result,
            ));
        });
    }

    fn share_selected_log(&mut self) {
        if self.log_share_pending || self.log_text.trim().is_empty() {
            return;
        }
        let Some((filename, _)) = self.selected_log.clone() else {
            return;
        };
        self.share_log_content(filename, self.log_text.clone());
    }

    fn share_live_console(&mut self) {
        if self.log_share_pending || self.console_text.trim().is_empty() {
            return;
        }
        self.share_log_content(
            "__live_console__".into(),
            self.console_text.clone(),
        );
    }

    fn share_log_content(&mut self, filename: String, content: String) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.log_share_pending = true;
        self.message = "Sending log text to mclo.gs…".into();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.share_minecraft_log(&content))
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::LogShareFinished(
                instance_id,
                filename,
                result,
            ));
        });
    }

    fn selected_log_delete_disabled(&self) -> bool {
        self.selected_log
            .as_ref()
            .is_some_and(|(filename, _)| filename == "latest.log")
            && self.selected_instance.as_ref().is_some_and(|selected| {
                self.process_state
                    .as_ref()
                    .is_some_and(|process| process.instance_id == selected.0)
            })
    }

    fn delete_selected_log(&mut self) {
        if self.log_delete_pending {
            return;
        }
        if self.selected_log_delete_disabled() {
            self.log_delete_confirmation = false;
            self.message =
                "Cannot delete latest.log while Minecraft is running.".into();
            return;
        }
        let (Some((instance_id, _)), Some((filename, is_crash))) =
            (self.selected_instance.clone(), self.selected_log.clone())
        else {
            self.log_delete_confirmation = false;
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.log_delete_confirmation = false;
            return;
        };
        let log_type = if is_crash {
            theseus::logs::LogType::CrashReport
        } else {
            theseus::logs::LogType::InfoLog
        };
        let runtime = self.runtime.clone();
        self.log_delete_confirmation = false;
        self.log_delete_pending = true;
        self.message = format!("Deleting {filename}…");
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.delete_instance_log(
                    &instance_id,
                    log_type,
                    &filename,
                ))
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::LogDeleted(
                instance_id,
                filename,
                result,
            ));
        });
    }

    fn refresh_process_state(&self, instance_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.running_processes())
                .map(|processes| {
                    processes
                        .into_iter()
                        .find(|process| process.instance_id == instance_id)
                })
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::ProcessStateLoaded(instance_id, result));
        });
    }

    fn refresh_running_processes(&mut self) {
        if !self.ready {
            return;
        }
        if self.running_processes_pending {
            self.running_processes_refresh_dirty = true;
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.running_processes_pending = true;
        let runtime = self.runtime.clone();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.running_processes())
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::RunningProcessesLoaded(result));
        });
    }

    fn stop_card_process(&mut self, instance_id: String) {
        let Some(process) = self.running_processes.get(&instance_id) else {
            self.refresh_running_processes();
            return;
        };
        let process_id = process.uuid;
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.message = "Stopping Minecraft…".into();
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .stop_process(process_id)
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::CardStopDone(instance_id, result));
        });
    }

    fn load_instance_files(&self, instance_id: String, path: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.list_instance_files(&instance_id, &path))
                .map(|items| {
                    items
                        .into_iter()
                        .map(|item| {
                            (
                                item.name,
                                item.kind,
                                item.path,
                                item.size,
                                item.read_only,
                            )
                        })
                        .collect()
                })
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::FilesLoaded(
                instance_id,
                path,
                result,
            ));
        });
    }

    fn load_settings(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result =
                backend.settings().await.map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::SettingsLoaded(result));
        });
    }

    fn save_default_window_settings(&mut self) {
        if self.window_defaults_saving {
            return;
        }
        let resolution = match parse_window_resolution(
            &self.default_window_width_input,
            &self.default_window_height_input,
        ) {
            Ok(Some(resolution)) => resolution,
            Ok(None) => {
                self.message =
                    "Enter both default window dimensions before saving."
                        .into();
                self.draw();
                return;
            }
            Err(error) => {
                self.message = error.to_string();
                self.draw();
                return;
            }
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            self.draw();
            return;
        };
        self.window_defaults_saving = true;
        self.message = "Saving default game window size…".into();
        self.draw();
        self.runtime.spawn(async move {
            let saved = backend
                .update_settings(move |settings| {
                    settings.game_resolution =
                        theseus::data::WindowSize(resolution.0, resolution.1);
                })
                .await;
            let result = match saved {
                Ok(()) => backend.settings().await,
                Err(error) => Err(error),
            }
            .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::WindowDefaultsSaved(result));
        });
    }

    fn save_default_launch_settings(&mut self) {
        if self.default_launch_settings_saving {
            return;
        }
        let arguments =
            match parse_launch_args(&self.default_java_arguments_input) {
                Ok(arguments) => arguments.unwrap_or_default(),
                Err(error) => {
                    self.message = error.into();
                    self.draw();
                    return;
                }
            };
        let environment = match parse_environment_variables(
            &self.default_environment_variables_input,
        ) {
            Ok(environment) => environment,
            Err(error) => {
                self.message = error;
                self.draw();
                return;
            }
        };
        let hooks = default_launch_hooks(
            &self.default_pre_launch_hook_input,
            &self.default_wrapper_hook_input,
            &self.default_post_exit_hook_input,
        );
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            self.draw();
            return;
        };
        self.default_launch_settings_saving = true;
        self.message = "Saving default launch settings…".into();
        self.draw();
        self.runtime.spawn(async move {
            let saved = backend
                .update_settings(move |settings| {
                    settings.extra_launch_args = arguments;
                    settings.custom_env_vars = environment;
                    settings.hooks = hooks;
                })
                .await;
            let result = match saved {
                Ok(()) => backend.settings().await,
                Err(error) => Err(error),
            }
            .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::DefaultLaunchSettingsSaved(result));
        });
    }

    fn load_quick_instance_count(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .quick_instance_count()
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::QuickInstanceCountLoaded(result));
        });
    }

    fn load_native_color_theme(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .native_color_theme()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ColorThemeLoaded(result));
        });
    }

    fn load_preferred_dark_theme(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .preferred_dark_theme()
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::PreferredDarkThemeLoaded(result));
        });
    }

    fn persist_preferred_dark_theme(&self, theme: theseus::data::Theme) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .set_preferred_dark_theme(theme)
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::PreferredDarkThemeSaved(result));
        });
    }

    fn load_theme_sync_setting(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let generation = self.theme_sync_generation;
        self.runtime.spawn(async move {
            let result = backend
                .theme_sync_across_devices()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::ThemeSyncLoaded(generation, result));
        });
    }

    fn load_account_theme_preferences(&self, user_id: String) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .user_preferences(&user_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::ThemePreferencesLoaded(user_id, result));
        });
    }

    fn sync_current_theme_to_account(&self) {
        let (Some(backend), Some(proxy), Some(settings), Some(account)) = (
            self.backend.clone(),
            self.proxy.clone(),
            self.settings.as_ref(),
            self.content_accounts.iter().find(|account| account.active),
        ) else {
            return;
        };
        let user_id = account.user_id.clone();
        let theme = settings.theme;
        self.runtime.spawn(async move {
            let result = backend
                .set_user_appearance_theme(&user_id, theme)
                .await
                .map(|_| ())
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ThemeSyncThemeSaved(result));
        });
    }

    fn apply_account_theme_preferences(
        &mut self,
        user_id: String,
        preferences: serde_json::Value,
    ) {
        if !self.theme_sync_across_devices
            || !self
                .content_accounts
                .iter()
                .any(|account| account.active && account.user_id == user_id)
        {
            return;
        }
        let Some(theme) = theme_from_user_preferences(&preferences) else {
            return;
        };
        if self.settings.as_ref().is_some_and(|settings| {
            theme_label(settings.theme) == theme_label(theme)
        }) {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.message = "Applying synced appearance theme…".into();
        self.draw();
        self.runtime.spawn(async move {
            let saved = backend
                .update_settings(move |settings| settings.theme = theme)
                .await;
            let result = match saved {
                Ok(()) => backend.settings().await,
                Err(error) => Err(error),
            }
            .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::SettingsSaved(result));
        });
    }

    fn load_hidden_instance_ids(&mut self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.hidden_instance_generation =
            self.hidden_instance_generation.wrapping_add(1);
        self.hidden_instance_load_generation =
            self.hidden_instance_load_generation.wrapping_add(1);
        let generation = self.hidden_instance_generation;
        let load_generation = self.hidden_instance_load_generation;
        self.hidden_instance_save_pending.clear();
        self.runtime.spawn(async move {
            let result = backend
                .hidden_instance_ids()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::HiddenInstancesLoaded(
                generation,
                load_generation,
                result,
            ));
        });
    }

    fn toggle_selected_instance_visibility(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        if self.hidden_instance_save_pending.contains_key(&instance_id) {
            return;
        }
        let hidden = !self.hidden_instance_ids.contains(&instance_id);
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.hidden_instance_load_generation =
            self.hidden_instance_load_generation.wrapping_add(1);
        let generation = self.hidden_instance_generation;
        self.hidden_instance_save_pending
            .insert(instance_id.clone(), generation);
        self.message = if hidden {
            "Hiding instance from your library…".into()
        } else {
            "Showing instance in your library…".into()
        };
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .set_instance_hidden(&instance_id, hidden)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::InstanceVisibilityChanged(
                instance_id,
                hidden,
                generation,
                result,
            ));
        });
    }

    fn load_maximum_memory(&self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.runtime.spawn(async move {
            let result = backend
                .maximum_memory_mb()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::MaximumMemoryLoaded(result));
        });
    }

    fn change_setting(&mut self, change: SettingsChange) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        if let SettingsChange::ScreenshotSync(enabled) = change {
            self.message = "Saving setting…".into();
            self.runtime.spawn(async move {
                let result = backend
                    .set_global_screenshot_sync(enabled)
                    .await
                    .map_err(|error| error.to_string());
                let _ =
                    proxy.send_event(UiMessage::ScreenshotSyncSaved(result));
            });
            return;
        }
        if let SettingsChange::QuickInstanceCount(delta) = change {
            let next = (self.jump_in_visible_items as isize + delta)
                .clamp(1, MAX_JUMP_IN_ITEMS as isize)
                as usize;
            if next != self.jump_in_visible_items {
                self.persist_quick_instance_count(next);
            }
            return;
        }
        if let SettingsChange::ThemeSync(enabled) = change {
            if self.theme_sync_saving {
                return;
            }
            let has_active_account =
                self.content_accounts.iter().any(|account| account.active);
            if enabled && !has_active_account {
                self.message =
                    "Sign in to Modrinth to sync your theme across devices."
                        .into();
                self.draw();
                return;
            }
            self.theme_sync_saving = true;
            self.theme_sync_generation =
                self.theme_sync_generation.wrapping_add(1);
            let generation = self.theme_sync_generation;
            self.message = "Saving theme sync preference…".into();
            self.draw();
            self.runtime.spawn(async move {
                let result = backend
                    .set_theme_sync_across_devices(enabled)
                    .await
                    .map_err(|error| error.to_string());
                let _ = proxy
                    .send_event(UiMessage::ThemeSyncSaved(generation, result));
            });
            return;
        }
        if matches!(change, SettingsChange::ColorTheme) {
            if self.native_color_theme_saving {
                return;
            }
            let next = (self.native_color_theme + 1) % 9;
            self.native_color_theme_saving = true;
            self.message = "Saving Northstar colorway…".into();
            self.draw();
            self.runtime.spawn(async move {
                let result = backend
                    .set_native_color_theme(next)
                    .await
                    .map_err(|error| error.to_string());
                let _ = proxy.send_event(UiMessage::ColorThemeSaved(result));
            });
            return;
        }
        let theme_to_sync = match change {
            SettingsChange::Theme(theme) if self.theme_sync_across_devices => {
                self.content_accounts
                    .iter()
                    .any(|account| account.active)
                    .then_some(theme)
            }
            _ => None,
        };
        let theme_sync_user_id = theme_to_sync.and_then(|_| {
            self.content_accounts
                .iter()
                .find(|account| account.active)
                .map(|account| account.user_id.clone())
        });
        let memory_limit = self.maximum_memory_mb;
        if matches!(change, SettingsChange::DefaultMemory(_))
            && memory_limit.is_none()
        {
            self.message = "Checking available memory…".into();
            self.load_maximum_memory();
            return;
        }
        self.message = "Saving setting…".into();
        self.runtime.spawn(async move {
            let result = backend
                .update_settings(move |settings| match change {
                    SettingsChange::DownloadConcurrency(delta) => {
                        settings.max_concurrent_downloads = adjust_count(
                            settings.max_concurrent_downloads,
                            delta,
                        );
                    }
                    SettingsChange::WriteConcurrency(delta) => {
                        settings.max_concurrent_writes =
                            adjust_count(settings.max_concurrent_writes, delta);
                    }
                    SettingsChange::Telemetry => {
                        settings.telemetry = !settings.telemetry
                    }
                    SettingsChange::DiscordRpc => {
                        settings.discord_rpc = !settings.discord_rpc
                    }
                    SettingsChange::Fullscreen => {
                        settings.force_fullscreen = !settings.force_fullscreen
                    }
                    SettingsChange::Theme(theme) => settings.theme = theme,
                    SettingsChange::NativeDecorations => {
                        settings.native_decorations =
                            !settings.native_decorations
                    }
                    SettingsChange::ColorTheme => {}
                    SettingsChange::FilesTab => {
                        settings.show_files_tab_in_instances =
                            !settings.show_files_tab_in_instances
                    }
                    SettingsChange::WorldsTab => {
                        settings.show_worlds_tab_in_instances =
                            !settings.show_worlds_tab_in_instances
                    }
                    SettingsChange::ScreenshotsTab => {
                        settings.show_screenshots_tab_in_instances =
                            !settings.show_screenshots_tab_in_instances
                    }
                    SettingsChange::SkinSelector => {
                        settings.show_skin_selector_in_sidebar =
                            !settings.show_skin_selector_in_sidebar
                    }
                    SettingsChange::PlayTime => {
                        settings.show_play_time = !settings.show_play_time
                    }
                    SettingsChange::CompactCards => {
                        settings.compact_instance_cards =
                            !settings.compact_instance_cards
                    }
                    SettingsChange::JumpIn => {
                        settings.show_jump_in = !settings.show_jump_in
                    }
                    SettingsChange::HideInstalledModpacks => {
                        settings.hide_installed_modpacks =
                            !settings.hide_installed_modpacks
                    }
                    SettingsChange::HideRightSidebar => {
                        settings.toggle_sidebar = !settings.toggle_sidebar
                    }
                    SettingsChange::HideOnProcessStart => {
                        settings.hide_on_process_start =
                            !settings.hide_on_process_start
                    }
                    SettingsChange::RefocusOnGameClose => {
                        settings.refocus_on_game_close =
                            !settings.refocus_on_game_close
                    }
                    SettingsChange::WarnOnUnknownModpacks => {
                        settings.warn_on_unknown_modpacks =
                            !settings.warn_on_unknown_modpacks
                    }
                    SettingsChange::SkipNonEssentialWarnings => {
                        settings.skip_non_essential_warnings =
                            !settings.skip_non_essential_warnings
                    }
                    SettingsChange::PersonalizedAds => {
                        settings.personalized_ads = !settings.personalized_ads
                    }
                    SettingsChange::DefaultMemory(delta) => {
                        let maximum = memory_limit
                            .unwrap_or(512)
                            .checked_div(1024)
                            .unwrap_or(0)
                            .saturating_mul(1024)
                            .max(512);
                        settings.memory.maximum = adjust_memory_setting(
                            settings.memory.maximum,
                            delta,
                            maximum,
                        );
                    }
                    SettingsChange::ScreenshotSync(_) => {}
                    SettingsChange::ThemeSync(_) => {}
                    SettingsChange::HideNametag => {
                        settings.hide_nametag_skins_page =
                            !settings.hide_nametag_skins_page
                    }
                    SettingsChange::QuickInstanceCount(_) => {}
                    SettingsChange::FriendsActiveCollapsed => {
                        settings.friends_active_collapsed =
                            !settings.friends_active_collapsed
                    }
                    SettingsChange::FriendsOnlineCollapsed => {
                        settings.friends_online_collapsed =
                            !settings.friends_online_collapsed
                    }
                    SettingsChange::FriendsOfflineCollapsed => {
                        settings.friends_offline_collapsed =
                            !settings.friends_offline_collapsed
                    }
                    SettingsChange::FriendsPendingCollapsed => {
                        settings.friends_pending_collapsed =
                            !settings.friends_pending_collapsed
                    }
                })
                .await
                .map_err(|error| error.to_string());
            let result = match result {
                Ok(()) => {
                    backend.settings().await.map_err(|error| error.to_string())
                }
                Err(error) => Err(error),
            };
            let local_theme_saved = result.is_ok();
            let _ = proxy.send_event(UiMessage::SettingsSaved(result));
            if local_theme_saved
                && let (Some(user_id), Some(theme)) =
                    (theme_sync_user_id, theme_to_sync)
            {
                let result = backend
                    .set_user_appearance_theme(&user_id, theme)
                    .await
                    .map(|_| ())
                    .map_err(|error| error.to_string());
                let _ =
                    proxy.send_event(UiMessage::ThemeSyncThemeSaved(result));
            }
        });
    }

    fn persist_quick_instance_count(&mut self, count: usize) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        if self.quick_instance_count_saving {
            return;
        }
        self.quick_instance_count_saving = true;
        self.message = "Saving Jump In item count…".into();
        self.draw();
        self.runtime.spawn(async move {
            let result = backend
                .set_quick_instance_count(count.clamp(1, MAX_JUMP_IN_ITEMS))
                .await
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::QuickInstanceCountSaved(result));
        });
    }

    fn load_global_screenshot_sync(&mut self) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        self.global_screenshot_sync = None;
        self.runtime.spawn(async move {
            let result = backend
                .global_screenshot_sync()
                .await
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::ScreenshotSyncLoaded(result));
        });
    }

    fn open_global_browse(&mut self) {
        self.global_hosting = false;
        self.global_browse = true;
        self.global_screenshots = false;
        self.global_skins = false;
        self.global_user_profile = false;
        self.settings_open = false;
        self.java_open = false;
        self.accounts_open = false;
        self.create_instance_open = false;
        self.screenshot_group_picker_open = false;
        self.selected_group = None;
        self.selected_instance =
            Some((GLOBAL_DISCOVER_ID.to_owned(), "DISCOVER".to_owned()));
        self.instance_tab = 3;
        self.browse_selected = None;
        self.browse_details = None;

        self.reset_browse_scroll();
        self.browse_search_offset = 0;
        self.browse_filter_menu_open = false;
        self.reset_browse_filter_scroll();
        self.browse_view_menu_open = false;
        self.browse_sort_menu_open = false;
        self.browse_detail_scroll_pixels = 0.0;
        self.browse_detail_scroll_target_pixels = 0.0;
        self.browse_detail_selected_channels.clear();
        self.browse_detail_selected_game_versions.clear();
        self.browse_detail_selected_loaders.clear();
        self.browse_detail_filter_menu_group = None;
        self.browse_detail_filter_option_offset = 0;
        self.browse_project_menu_open = false;
        self.browse_gallery_index = 0;
        self.browse_gallery_viewer_open = false;
        self.browse_gallery_zoom = 1.0;
        self.browse_detail_tab = 0;
        self.browse_description_lines = None;
        self.input_focus = 0;
        if let Ok(mut active) = self.active_log_instance.lock() {
            *active = None;
        }
        self.search_projects();
    }

    fn return_to_library(&mut self, selected_group: Option<String>) {
        self.global_hosting = false;
        self.global_browse = false;
        self.global_screenshots = false;
        self.global_skins = false;
        self.global_user_profile = false;
        self.user_profile_id = None;
        self.user_profile = None;
        self.user_profile_projects.clear();
        self.user_profile_filtered_project_indices.clear();
        self.user_profile_organizations.clear();
        self.user_profile_collections.clear();
        self.user_profile_type_filter = None;
        self.user_profile_show_collections = false;
        self.user_profile_error = None;
        self.user_profile_loading = false;
        self.user_profile_scroll_pixels = 0.0;
        self.user_profile_scroll_target_pixels = 0.0;
        self.screenshot_group_picker_open = false;
        self.browse_install_choice = None;
        self.selected_instance = None;
        self.selected_group = selected_group;
        self.membership_mode = false;
        self.list_offset = 0;
        self.input_focus = 0;
        self.browse_view_menu_open = false;
        self.browse_sort_menu_open = false;
    }

    fn search_projects(&mut self) {
        self.browse_search_generation =
            self.browse_search_generation.wrapping_add(1);
        self.browse_search_offset = 0;

        self.reset_browse_scroll();
        self.search_projects_at_offset();
    }

    fn browse_project_is_installed(&self, project: &BrowseProject) -> bool {
        if self.global_browse || project.project_type == "modpack" {
            return false;
        }
        self.contents.iter().any(|item| {
            item.project
                .as_ref()
                .is_some_and(|installed| installed.id == project.id)
        })
    }

    fn browse_project_is_installing(&self, project: &BrowseProject) -> bool {
        if project.project_type == "modpack" {
            self.browse_installing_modpacks.contains(&project.id)
        } else {
            self.selected_instance
                .as_ref()
                .and_then(|(id, _)| self.browse_installing_projects.get(id))
                .is_some_and(|projects| projects.contains(&project.id))
        }
    }

    fn browse_version_is_installed(
        &self,
        project_id: &str,
        version_id: &str,
    ) -> bool {
        !self.global_browse
            && self.contents.iter().any(|item| {
                item.project
                    .as_ref()
                    .is_some_and(|project| project.id == project_id)
                    && item
                        .version
                        .as_ref()
                        .is_some_and(|version| version.id == version_id)
            })
    }

    fn show_browse_install_notice(&mut self, text: String, failed: bool) {
        let duration =
            std::time::Duration::from_secs(if failed { 12 } else { 8 });
        let expires = std::time::Instant::now() + duration;
        self.browse_install_notice = Some(BrowseInstallNotice {
            text,
            failed,
            expires,
        });
        if let Some(proxy) = self.proxy.clone() {
            self.runtime.spawn(async move {
                tokio::time::sleep_until(expires.into()).await;
                let _ = proxy
                    .send_event(UiMessage::ClearBrowseInstallNotice(expires));
            });
        }
    }

    fn schedule_browse_search(&mut self) {
        self.browse_search_generation =
            self.browse_search_generation.wrapping_add(1);
        let generation = self.browse_search_generation;
        let query = self.browse_query.clone();
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.browse_request_generation =
            self.browse_request_generation.wrapping_add(1);
        self.browse_search_pending = true;
        self.browse_error = None;
        self.runtime.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let _ = proxy.send_event(UiMessage::BrowseSearchDebounced(
                generation, query,
            ));
        });
    }

    fn search_previous_projects_page(&mut self) {
        if !self.browse_page_controls()[0] {
            return;
        }
        self.browse_search_offset = self
            .browse_search_offset
            .saturating_sub(self.browse_page_size);

        self.reset_browse_scroll();
        self.search_projects_at_offset();
    }

    fn search_next_projects_page(&mut self) {
        if !self.browse_page_controls()[1] {
            return;
        }
        if self.browse_error.is_some() {
            self.search_projects_at_offset();
            return;
        }
        self.browse_search_offset = self
            .browse_search_offset
            .saturating_add(self.browse_page_size);

        self.reset_browse_scroll();
        self.search_projects_at_offset();
    }

    fn reset_browse_scroll(&mut self) {
        self.browse_scroll_pixels = 0.0;
        self.browse_scroll_target_pixels = 0.0;
    }

    fn browse_page_controls(&self) -> [bool; 2] {
        browse_page_controls(
            self.browse_search_pending,
            self.browse_error.is_some(),
            self.browse_search_offset,
            self.browse_page_size,
            self.browse_total_hits,
        )
    }

    fn load_browse_filter_options(&mut self) {
        if self.browse_filter_options.is_some()
            || self.browse_filter_options_loading
        {
            return;
        }
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        self.browse_filter_options_loading = true;
        self.browse_filter_options_error = None;
        self.runtime.spawn(async move {
            let (game_versions, loaders, categories) = tokio::join!(
                theseus::tags::get_game_version_tags(),
                theseus::tags::get_loader_tags(),
                theseus::tags::get_category_tags(),
            );
            let result = match (game_versions, loaders, categories) {
                (Ok(game_versions), Ok(loaders), Ok(categories)) => {
                    Ok(BrowseFilterOptions {
                        game_versions: game_versions
                            .into_iter()
                            .map(|version| {
                                (
                                    version.version,
                                    version.version_type == "release",
                                )
                            })
                            .collect(),
                        loaders: loaders
                            .into_iter()
                            .map(|loader| {
                                (loader.name, loader.supported_project_types)
                            })
                            .collect(),
                        categories: categories
                            .into_iter()
                            .map(|category| BrowseCategoryTag {
                                project_type: category.project_type,
                                name: category.name,
                                is_resolution: category.header == "resolutions",
                            })
                            .collect(),
                    })
                }
                (Err(error), _, _)
                | (_, Err(error), _)
                | (_, _, Err(error)) => Err(error.to_string()),
            };
            let _ =
                proxy.send_event(UiMessage::BrowseFilterOptionsLoaded(result));
        });
    }

    fn browse_filter_group_count(&self) -> usize {
        if matches!(self.browse_filter, 0 | 4) {
            BROWSE_FILTER_GROUPS.len()
        } else {
            BROWSE_FILTER_GROUPS.len() - 1
        }
    }

    fn browse_filter_menu_options(&self) -> Vec<String> {
        if self.browse_filter_group == BROWSE_LICENSE_GROUP {
            return BROWSE_LICENSE_OPTIONS.map(str::to_owned).to_vec();
        }
        if self.browse_filter_group == BROWSE_ENVIRONMENT_GROUP {
            return if self.browse_filter_group_count()
                == BROWSE_FILTER_GROUPS.len()
            {
                vec!["Client".into(), "Server".into()]
            } else {
                Vec::new()
            };
        }
        let Some(options) = &self.browse_filter_options else {
            return Vec::new();
        };
        let project_type = match self.browse_filter {
            1 => "resourcepack",
            2 => "datapack",
            3 => "shader",
            4 => "modpack",
            _ => "mod",
        };
        match self.browse_filter_group {
            1 => options
                .loaders
                .iter()
                .filter(|(_, supported)| {
                    supported.iter().any(|value| value == project_type)
                        && (project_type != "mod"
                            || !supported.iter().any(|value| {
                                matches!(value.as_str(), "plugin" | "datapack")
                            }))
                })
                .map(|(name, _)| name.clone())
                .collect(),
            2 => {
                let category_project_type =
                    browse_category_project_type(project_type);
                let mut categories = options
                    .categories
                    .iter()
                    .filter(|tag| tag.project_type == category_project_type)
                    .map(|tag| tag.name.clone())
                    .collect::<Vec<_>>();
                categories.sort_unstable();
                categories.dedup();
                categories
            }
            _ => visible_browse_versions(
                &options.game_versions,
                self.browse_show_all_versions,
                &self.browse_selected_game_versions,
            ),
        }
    }

    fn reset_browse_filter_scroll(&mut self) {
        self.browse_filter_scroll_pixels = 0.0;
        self.browse_filter_scroll_target_pixels = 0.0;
    }

    fn toggle_browse_all_versions(&mut self) {
        self.browse_show_all_versions = !self.browse_show_all_versions;
        self.reset_browse_filter_scroll();
    }

    fn toggle_browse_filter_option(&mut self, option: String) {
        let selected = match self.browse_filter_group {
            1 => &mut self.browse_selected_loaders,
            2 => &mut self.browse_selected_categories,
            BROWSE_LICENSE_GROUP => &mut self.browse_selected_licenses,
            BROWSE_ENVIRONMENT_GROUP => &mut self.browse_selected_environments,
            _ => &mut self.browse_selected_game_versions,
        };
        if !selected.remove(&option) {
            if self.browse_filter_group == BROWSE_LICENSE_GROUP {
                selected.clear();
            }
            selected.insert(option);
        }
        self.search_projects();
    }

    fn search_projects_at_offset(&mut self) {
        let Some(backend) = self.backend.clone() else {
            self.browse_search_pending = false;
            self.message = "Launcher backend is not ready.".into();
            self.browse_error = Some(self.message.clone());
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            self.browse_search_pending = false;
            return;
        };
        self.browse_search_pending = true;
        self.browse_request_generation =
            self.browse_request_generation.wrapping_add(1);
        let request_generation = self.browse_request_generation;
        let query = self.browse_query.trim().to_owned();
        let request_query = query.clone();
        let project_type = match self.browse_filter {
            1 => "resourcepack",
            2 => "datapack",
            3 => "shader",
            4 => "modpack",
            _ => "mod",
        };
        let sort_index = match self.browse_sort {
            1 => "downloads",
            2 => "follows",
            3 => "newest",
            4 => "updated",
            _ => "relevance",
        };
        let offset = self.browse_search_offset;
        let limit = self.browse_page_size;
        let mut filters = browse_search_filters(
            project_type,
            &self.browse_selected_game_versions,
            &self.browse_selected_loaders,
            &self.browse_selected_categories,
            &self.browse_selected_environments,
            self.browse_filter_options
                .as_ref()
                .map_or(&[], |options| options.categories.as_slice()),
            if self
                .browse_selected_licenses
                .contains(BROWSE_LICENSE_OPTIONS[0])
            {
                Some(true)
            } else if self
                .browse_selected_licenses
                .contains(BROWSE_LICENSE_OPTIONS[1])
            {
                Some(false)
            } else {
                None
            },
        );
        if !self.global_browse
            && let Some(card) = self
                .selected_instance
                .as_ref()
                .and_then(|(id, _)| self.instance_cards.get(id))
        {
            for constraint in browse_instance_constraints(
                project_type,
                &card.game_version,
                &card.loader,
            ) {
                filters.push_str(" AND ");
                filters.push_str(&constraint);
            }
        }
        if project_type == "modpack"
            && self
                .settings
                .as_ref()
                .is_some_and(|settings| settings.hide_installed_modpacks)
            && let Some(excluded) =
                browse_excluded_project_ids(&self.installed_modpack_ids)
        {
            filters.push_str(" AND ");
            filters.push_str(&excluded);
        }
        self.browse_hidden_ids_dirty = false;
        self.browse_error = None;
        self.message = "Searching projects…".into();
        self.runtime.spawn(async move {
            let filters = urlencoding::encode(&filters);
            let query_encoded = urlencoding::encode(&query).into_owned();
            let result = backend
                .search_projects(
                    &format!("?new_filters={filters}&index={sort_index}&limit={limit}&offset={offset}&query={query_encoded}"),
                    None,
                )
                .await
                .map_err(|error| error.to_string())
                .map(|results| {
                    results
                        .map(|results| {
                            let total_hits = results.result.total_hits;
                            let projects = results
                                .result
                                .hits
                                .into_iter()
                                .filter_map(|hit| BrowseProject::from_search_hit(&hit, project_type))
                                .collect();
                            BrowseSearchPage {
                                projects,
                                total_hits,
                            }
                        })
                        .unwrap_or_default()
                });
            let _ = proxy.send_event(UiMessage::BrowseResults(
                request_generation,
                request_query,
                result,
            ));
        });
    }

    fn read_instance_file(&mut self, path: String, size: Option<u64>) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        if size.is_some_and(|size| size > 256 * 1024) {
            self.message = "Preview is limited to files up to 256 KiB.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = format!("Reading {path}…");
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.read_instance_file(&instance_id, &path))
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    if bytes.contains(&0) {
                        Err("binary files cannot be shown as text".into())
                    } else {
                        Ok(String::from_utf8_lossy(&bytes).into_owned())
                    }
                });
            let _ = proxy.send_event(UiMessage::FileRead(
                instance_id,
                path,
                result,
            ));
        });
    }

    fn stop_selected_process(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let Some(process) = self
            .process_state
            .as_ref()
            .filter(|process| process.instance_id == instance_id)
        else {
            return;
        };
        let process_id = process.uuid;
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = "Stopping Minecraft…".into();
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.stop_process(process_id))
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::StopProcessDone(instance_id, result));
        });
    }

    fn toggle_content(&mut self, item: theseus::data::ContentItem) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        if item.locked {
            self.message = "This content is managed by its modpack and cannot be toggled here.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = format!("Updating {}…", item.file_name);
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.set_content_enabled(
                    &instance_id,
                    &item.file_path,
                    !item.enabled,
                ))
                .map(|_| ())
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::ContentAction(instance_id, result));
        });
    }

    fn remove_content(&mut self, item: theseus::data::ContentItem) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        if item.locked {
            self.message = "This content is managed by its modpack and cannot be removed here.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = format!("Removing {}…", item.file_name);
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.remove_content(&instance_id, &item.file_path))
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::ContentAction(instance_id, result));
        });
    }

    fn update_content(&mut self, item: theseus::data::ContentItem) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        if item.locked {
            self.message = "This content is managed by its modpack and cannot be updated here.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = format!("Updating {}…", item.file_name);
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.update_content(&instance_id, &item.file_path))
                .map(|_| ())
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::ContentAction(instance_id, result));
        });
    }

    fn pick_content_file(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        self.message = "Choose a .jar or .zip content file…".into();
        std::thread::spawn(move || {
            let result = runtime.block_on(async {
                let selected = rfd::AsyncFileDialog::new()
                    .add_filter("Minecraft content", &["jar", "zip"])
                    .pick_file()
                    .await;
                match selected {
                    Some(file) => backend
                        .add_content_file(&instance_id, file.path(), None)
                        .await
                        .map(Some)
                        .map_err(|error| error.to_string()),
                    None => Ok(None),
                }
            });
            let _ = proxy
                .send_event(UiMessage::ContentFileAdded(instance_id, result));
        });
    }

    fn export_selected_instance(&mut self) {
        let Some((instance_id, instance_name)) = self.selected_instance.clone()
        else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let runtime = self.runtime.clone();
        self.message = "Preparing .mrpack export…".into();
        std::thread::spawn(move || {
            let result = runtime.block_on(async {
                let candidates = backend
                    .export_candidates(&instance_id)
                    .await
                    .map_err(|error| error.to_string())?;
                let included_paths = candidates
                    .into_iter()
                    .filter(|candidate| candidate.default_selected)
                    .map(|candidate| candidate.path.as_str().to_owned())
                    .collect();
                let file_name =
                    format!("{}.mrpack", safe_export_filename(&instance_name));
                let selected = rfd::AsyncFileDialog::new()
                    .add_filter("Modrinth modpack", &["mrpack"])
                    .set_file_name(&file_name)
                    .save_file()
                    .await;
                let Some(selected) = selected else {
                    return Ok(None);
                };
                let destination = selected.path().to_path_buf();
                backend
                    .export_pack(native_backend::ExportPack {
                        instance_id: instance_id.clone(),
                        destination: destination.clone(),
                        included_paths,
                        excluded_paths: Vec::new(),
                        version_id: None,
                        description: None,
                        name: Some(instance_name),
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(Some(destination))
            });
            let _ = proxy
                .send_event(UiMessage::ExportFinished(instance_id, result));
        });
    }

    fn duplicate_selected_instance(&mut self) {
        let Some((instance_id, instance_name)) = self.selected_instance.clone()
        else {
            return;
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let runtime = self.runtime.clone();
        self.message = format!("Duplicating {instance_name}…");
        std::thread::spawn(move || {
            let result = runtime.block_on(async {
                let job = backend
                    .duplicate_instance(instance_id.clone())
                    .await
                    .map_err(|error| error.to_string())?;
                let job_id = uuid::Uuid::parse_str(&job.job_id)
                    .map_err(|error| error.to_string())?;
                loop {
                    let snapshot = backend
                        .get_install_job(job_id)
                        .await
                        .map_err(|error| error.to_string())?;
                    if snapshot.status.is_finished() {
                        if matches!(
                            snapshot.status,
                            theseus::install::InstallJobStatus::Succeeded
                        ) {
                            return Ok(());
                        }
                        return Err(snapshot
                            .error
                            .map(|error| error.message)
                            .unwrap_or_else(|| {
                                format!(
                                    "Instance duplication {}.",
                                    snapshot.status.as_str()
                                )
                            }));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(300))
                        .await;
                }
            });
            let _ = proxy
                .send_event(UiMessage::DuplicateFinished(instance_id, result));
        });
    }

    fn save_instance_name(&mut self) {
        let Some((instance_id, _)) = self.selected_instance.clone() else {
            return;
        };
        let name = match validate_instance_name(&self.rename_instance_input) {
            Ok(name) => name,
            Err(error) => {
                self.message = error.into();
                return;
            }
        };
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.message = "Renaming instance…".into();
        self.runtime.spawn(async move {
            let result = backend
                .edit_instance(
                    &instance_id,
                    theseus::data::EditInstance {
                        name: Some(name),
                        ..Default::default()
                    },
                )
                .await
                .map(|metadata| metadata.instance.name)
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::InstanceRenamed(instance_id, result));
        });
    }

    fn remove_selected_instance(&mut self) {
        if self.instance_deletion_pending {
            return;
        }
        let Some((instance_id, instance_name)) = self.selected_instance.clone()
        else {
            self.delete_confirmation_open = false;
            return;
        };
        if !delete_confirmation_matches(
            &self.delete_confirmation_input,
            &instance_name,
        ) {
            self.message =
                "Type the instance name exactly to confirm removal.".into();
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.instance_deletion_pending = true;
        self.input_focus = 0;
        self.message = format!("Removing {instance_name} and its files…");
        self.runtime.spawn(async move {
            let result = backend
                .remove_instance(&instance_id)
                .await
                .map_err(|error| error.to_string());
            let _ = proxy
                .send_event(UiMessage::InstanceRemoved(instance_id, result));
        });
    }

    fn remove_selected_library_instances(&mut self) {
        if self.instance_deletion_pending
            || self.selected_library_instances.is_empty()
        {
            return;
        }
        let expected =
            format!("DELETE {}", self.selected_library_instances.len());
        if !self
            .delete_confirmation_input
            .trim()
            .eq_ignore_ascii_case(&expected)
        {
            self.message =
                format!("Type {expected} exactly to confirm removal.");
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let instance_ids: Vec<String> =
            self.selected_library_instances.iter().cloned().collect();
        self.instance_deletion_pending = true;
        self.input_focus = 0;
        self.message =
            format!("Removing {} selected instances…", instance_ids.len());
        self.runtime.spawn(async move {
            let mut removed = Vec::new();
            let mut errors = Vec::new();
            for instance_id in instance_ids {
                match backend.remove_instance(&instance_id).await {
                    Ok(()) => removed.push(instance_id),
                    Err(error) => {
                        errors.push(format!("{instance_id}: {error}"))
                    }
                }
            }
            let _ = proxy
                .send_event(UiMessage::BulkInstancesRemoved(removed, errors));
        });
    }

    fn control_active_job(&mut self, cancel: bool) {
        if self.job_control_pending {
            return;
        }
        let Some(job) = self.active_job.as_ref() else {
            return;
        };
        if job.status.is_finished() {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let Ok(id) = uuid::Uuid::parse_str(&job.job_id) else {
            return;
        };
        let paused = job.paused;
        self.job_control_pending = true;
        self.message = if cancel {
            "Canceling install…".into()
        } else if paused {
            "Resuming install…".into()
        } else {
            "Pausing install…".into()
        };
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        let runtime = self.runtime.clone();
        std::thread::spawn(move || {
            let result = runtime.block_on(async {
                if cancel {
                    backend.cancel_install_job(id).await
                } else if paused {
                    backend.resume_install_job(id).await
                } else {
                    backend.pause_install_job(id).await
                }
            });
            let message = result
                .map(|job| format!("Install {}.", job.status.as_str()))
                .unwrap_or_else(|error| {
                    format!("Could not update install: {error}")
                });
            let _ = proxy.send_event(UiMessage::JobActionResult(message));
        });
    }

    fn import_pack(&mut self, path: std::path::PathBuf) {
        if !self.ready {
            self.message = "Launcher services are still starting.".into();
            return;
        }
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mrpack"))
        {
            self.message = "Choose an existing .mrpack file.".into();
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        let backend_for_refresh = backend.clone();
        self.message = format!(
            "Importing {}…",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("modpack")
        );
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        std::thread::spawn(move || {
            let result = runtime.block_on(async move {
                let job = backend
                    .import_pack_file(path, None)
                    .await
                    .map_err(|error| error.to_string())?;
                let job_id = uuid::Uuid::parse_str(&job.job_id)
                    .map_err(|error| error.to_string())?;
                loop {
                    let snapshot = backend
                        .get_install_job(job_id)
                        .await
                        .map_err(|error| error.to_string())?;
                    if snapshot.status.is_finished() {
                        if matches!(
                            snapshot.status,
                            theseus::install::InstallJobStatus::Succeeded
                        ) {
                            return Ok(
                                "Modpack imported successfully.".to_string()
                            );
                        }
                        return Err(snapshot
                            .error
                            .map(|error| error.message)
                            .unwrap_or_else(|| {
                                format!(
                                    "Modpack import {}.",
                                    snapshot.status.as_str()
                                )
                            }));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(300))
                        .await;
                }
            });
            let message = result.unwrap_or_else(|error| {
                format!("Could not import modpack: {error}")
            });
            let _ = proxy.send_event(UiMessage::Notice(message));
            let result = runtime
                .block_on(backend_for_refresh.list_instances())
                .map(|items| {
                    let _ = proxy.send_event(UiMessage::InstalledModpackIds(
                        native_installed_modpack_ids(&items),
                    ));
                    items.into_iter().map(native_instance_entry).collect()
                })
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::Instances(result));
        });
    }

    fn pick_modpack_file(&mut self) {
        if !self.ready {
            self.message = "Launcher services are still starting.".into();
            return;
        }
        let Some(proxy) = self.proxy.clone() else {
            self.message = "Launcher window is not ready.".into();
            return;
        };
        let runtime = self.runtime.clone();
        self.message = "Choose a Modrinth .mrpack file…".into();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        std::thread::spawn(move || {
            let selected = runtime.block_on(async {
                rfd::AsyncFileDialog::new()
                    .add_filter("Modrinth modpack", &["mrpack"])
                    .pick_file()
                    .await
                    .map(|file| file.path().to_path_buf())
            });
            if let Some(path) = selected {
                let _ = proxy.send_event(UiMessage::Import(path));
            } else {
                let _ = proxy.send_event(UiMessage::Notice(
                    "Modpack selection canceled.".into(),
                ));
            }
        });
    }

    fn open_launcher_import(&mut self) {
        if !self.ready {
            self.message = "Launcher services are still starting.".into();
            return;
        }
        self.launcher_import_open = true;
        self.launcher_import_busy = false;
        self.launcher_import_source = None;
        self.launcher_import_offset = 0;
        self.launcher_import_scroll_pixels = 0.0;
        self.launcher_import_scroll_target_pixels = 0.0;
        self.launcher_import_focus = 0;
        self.draw();
    }

    fn move_launcher_import_focus(&mut self, delta: isize) {
        if self.launcher_import_busy {
            return;
        }
        let row_count = self
            .launcher_import_source
            .as_ref()
            .map_or(import_launcher_options().len() + 1, |source| {
                source.folders.len()
            });
        if row_count == 0 {
            return;
        }
        self.launcher_import_focus =
            (self.launcher_import_focus as isize + delta)
                .clamp(0, row_count as isize - 1) as usize;
        if let Some(source) = &self.launcher_import_source {
            let visible = 7;
            if self.launcher_import_focus < self.launcher_import_offset {
                self.launcher_import_offset = self.launcher_import_focus;
            } else if self.launcher_import_focus
                >= self.launcher_import_offset + visible
            {
                self.launcher_import_offset =
                    self.launcher_import_focus + 1 - visible;
            }
            self.launcher_import_offset = self
                .launcher_import_offset
                .min(source.folders.len().saturating_sub(visible));
            self.launcher_import_scroll_target_pixels =
                self.launcher_import_offset as f64 * 44.0;
        } else {
            self.launcher_import_scroll_target_pixels = 0.0;
        }
    }

    fn activate_launcher_import_focus(&mut self) {
        if self.launcher_import_busy {
            return;
        }
        if let Some(source) = self.launcher_import_source.clone() {
            if let Some(folder) =
                source.folders.get(self.launcher_import_focus).cloned()
            {
                self.import_launcher_folder(source, folder);
            }
        } else if self.launcher_import_focus == 0 {
            self.launcher_import_open = false;
            self.pick_modpack_file();
        } else if let Some((_, launcher)) = import_launcher_options()
            .get(self.launcher_import_focus - 1)
            .copied()
        {
            self.load_launcher_instances(launcher);
        }
    }

    fn load_launcher_instances(
        &mut self,
        launcher: theseus::pack::import::ImportLauncherType,
    ) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        self.launcher_import_busy = true;
        let default_path = NativeBackend::default_launcher_path(launcher);
        self.message = default_path.as_ref().map_or_else(
            || "Choose the launcher’s instances folder…".to_string(),
            |path| format!("Searching {}…", path.display()),
        );
        self.runtime.spawn(async move {
            let path = match default_path {
                Some(path) => Some(path),
                None => rfd::AsyncFileDialog::new()
                    .pick_folder()
                    .await
                    .map(|folder| folder.path().to_path_buf()),
            };
            let result = if let Some(path) = path {
                backend
                    .importable_instances(launcher, path.clone())
                    .await
                    .map(|folders| NativeImportSource {
                        launcher,
                        path,
                        folders,
                    })
                    .map_err(|error| error.to_string())
            } else {
                Err("launcher folder selection canceled".to_string())
            };
            let _ = proxy.send_event(UiMessage::LauncherInstances(result));
        });
    }

    fn import_launcher_folder(
        &mut self,
        source: NativeImportSource,
        folder: String,
    ) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            self.message = "Launcher backend is not ready.".into();
            return;
        };
        let runtime = self.runtime.clone();
        let name = folder.clone();
        self.launcher_import_open = false;
        self.launcher_import_busy = true;
        self.message = format!("Importing {name}…");
        runtime.spawn(async move {
            let result = async {
                let job = backend
                    .import_instance(source.launcher, source.path, folder)
                    .await
                    .map_err(|error| error.to_string())?;
                let job_id = uuid::Uuid::parse_str(&job.job_id)
                    .map_err(|error| error.to_string())?;
                loop {
                    let snapshot = backend
                        .get_install_job(job_id)
                        .await
                        .map_err(|error| error.to_string())?;
                    if snapshot.status.is_finished() {
                        if matches!(
                            snapshot.status,
                            theseus::install::InstallJobStatus::Succeeded
                        ) {
                            return Ok(());
                        }
                        return Err(snapshot
                            .error
                            .map(|error| error.message)
                            .unwrap_or_else(|| {
                                format!("Import {}.", snapshot.status.as_str())
                            }));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(300))
                        .await;
                }
            }
            .await;
            let _ = proxy
                .send_event(UiMessage::LauncherImportFinished(name, result));
        });
    }

    fn create_instance(&mut self) {
        if !self.ready {
            self.message = "Launcher services are still starting.".into();
            return;
        }
        let name = self.name_input.trim().to_string();
        if name.is_empty() {
            self.message = "Enter a name for the new instance.".into();
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        let requested_version = self.version_input.trim().to_string();
        let loader = self.selected_loader();
        let backend_for_refresh = backend.clone();
        self.message = if requested_version.is_empty() {
            format!("Loading latest Minecraft with {}…", loader.as_str())
        } else {
            format!(
                "Preparing Minecraft {requested_version} with {}…",
                loader.as_str()
            )
        };
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        std::thread::spawn(move || {
            let result = runtime.block_on(async move {
                let catalog = backend
                    .minecraft_versions()
                    .await
                    .map_err(|error| error.to_string())?;
                let game_version = if requested_version.is_empty() { catalog.latest.release.0 } else { requested_version };
                if !catalog.versions.iter().any(|version| version.id.0 == game_version) {
                    return Err(format!("Minecraft version {game_version} is not in the version catalog"));
                }
                let loader = loader;
                let job = backend
                    .create_instance(CreateInstance {
                        name,
                        game_version,
                        loader,
                        loader_version: None,
                        icon_path: None,
                        icon_config: None,
                        link: theseus::data::InstanceLink::Unmanaged,
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                let job_id = uuid::Uuid::parse_str(&job.job_id)
                    .map_err(|error| error.to_string())?;
                loop {
                    let snapshot = backend
                        .get_install_job(job_id)
                        .await
                        .map_err(|error| error.to_string())?;
                    if snapshot.status.is_finished() {
                        if matches!(
                            snapshot.status,
                            theseus::install::InstallJobStatus::Succeeded
                        ) {
                            return Ok(
                                "Instance created successfully.".to_string()
                            );
                        }
                        return Err(snapshot
                            .error
                            .map(|error| error.message)
                            .unwrap_or_else(|| {
                                format!(
                                    "Instance creation {}.",
                                    snapshot.status.as_str()
                                )
                            }));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(300))
                        .await;
                }
            });
            let message = result.unwrap_or_else(|error| {
                format!("Could not create instance: {error}")
            });
            let _ = proxy.send_event(UiMessage::Notice(message));
            let result = runtime
                .block_on(backend_for_refresh.list_instances())
                .map(|items| {
                    let _ = proxy.send_event(UiMessage::InstalledModpackIds(
                        native_installed_modpack_ids(&items),
                    ));
                    items.into_iter().map(native_instance_entry).collect()
                })
                .map_err(|error| error.to_string());
            let _ = proxy.send_event(UiMessage::Instances(result));
        });
    }

    fn launch_instance(
        &mut self,
        instance_id: String,
        name: String,
        server: Option<String>,
        world: Option<String>,
    ) {
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        let quick_play = match (server, world) {
            (Some(server), _) => theseus::instance::QuickPlayType::Server(
                theseus::server_address::ServerAddress::Unresolved(server),
            ),
            (None, Some(world)) => {
                theseus::instance::QuickPlayType::Singleplayer(world)
            }
            (None, None) => theseus::instance::QuickPlayType::None,
        };
        self.message = format!("Starting {name}…");
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.launch_instance(&instance_id, quick_play))
                .map(|_| format!("{name} is launching"))
                .map_err(|error| error.to_string());
            let _ =
                proxy.send_event(UiMessage::Notice(result.unwrap_or_else(
                    |error| format!("Could not launch: {error}"),
                )));
            let _ = proxy.send_event(UiMessage::RefreshRunningProcesses);
            let _ = proxy.send_event(UiMessage::RefreshRecentWorlds);
        });
    }

    fn refresh_instances(&self) {
        if !self.ready {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        let pending = self.refresh_pending.clone();
        if pending.swap(true, std::sync::atomic::Ordering::AcqRel) {
            return;
        }
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.list_instances())
                .map(|items| {
                    let _ = proxy.send_event(UiMessage::InstalledModpackIds(
                        native_installed_modpack_ids(&items),
                    ));
                    items.into_iter().map(native_instance_entry).collect()
                })
                .map_err(|e| e.to_string());
            pending.store(false, std::sync::atomic::Ordering::Release);
            let _ = proxy.send_event(UiMessage::Instances(result));
        });
    }

    fn refresh_groups(&self) {
        if !self.ready {
            return;
        }
        let (Some(backend), Some(proxy)) =
            (self.backend.clone(), self.proxy.clone())
        else {
            return;
        };
        let runtime = self.runtime.clone();
        let pending = self.groups_refresh_pending.clone();
        if pending.swap(true, std::sync::atomic::Ordering::AcqRel) {
            return;
        }
        std::thread::spawn(move || {
            let result = runtime
                .block_on(backend.list_groups())
                .map_err(|error| error.to_string());
            pending.store(false, std::sync::atomic::Ordering::Release);
            let _ = proxy.send_event(UiMessage::Groups(result));
        });
    }
}

fn is_dark_theme(theme: theseus::data::Theme) -> bool {
    matches!(
        theme,
        theseus::data::Theme::Dark
            | theseus::data::Theme::Oled
            | theseus::data::Theme::Retro
    )
}

fn palette_for_theme(
    theme: theseus::data::Theme,
    system_theme: Option<winit::window::Theme>,
    preferred_dark_theme: theseus::data::Theme,
) -> u8 {
    match theme {
        theseus::data::Theme::Dark => 0,
        theseus::data::Theme::Light => 1,
        theseus::data::Theme::Oled => 2,
        theseus::data::Theme::Retro => 3,
        theseus::data::Theme::System => match system_theme {
            Some(winit::window::Theme::Light) => 1,
            Some(winit::window::Theme::Dark) | None => {
                match preferred_dark_theme {
                    theseus::data::Theme::Oled => 2,
                    theseus::data::Theme::Retro => 3,
                    _ => 0,
                }
            }
        },
    }
}

fn palette_for_native_color_theme(base: u8, color_theme: u8) -> u8 {
    let base = base.min(3);
    let color_theme = color_theme.min(8);
    if color_theme == 0 {
        base
    } else {
        16 + base * 16 + color_theme
    }
}

fn native_color_theme_label(color_theme: u8) -> &'static str {
    match color_theme {
        1 => "AMBER",
        2 => "GREEN",
        3 => "OCEAN",
        4 => "PURPLE",
        5 => "EMBER",
        6 => "ROSE",
        7 => "ARCTIC",
        8 => "TERMINAL",
        _ => "DEFAULT",
    }
}

fn theme_label(theme: theseus::data::Theme) -> &'static str {
    match theme {
        theseus::data::Theme::Dark => "DARK",
        theseus::data::Theme::Light => "LIGHT",
        theseus::data::Theme::Oled => "OLED",
        theseus::data::Theme::Retro => "RETRO",
        theseus::data::Theme::System => "SYSTEM",
    }
}

fn theme_from_user_preferences(
    preferences: &serde_json::Value,
) -> Option<theseus::data::Theme> {
    let appearance = preferences.get("appearance")?;
    if appearance.get("auto").and_then(serde_json::Value::as_bool) == Some(true)
    {
        return Some(theseus::data::Theme::System);
    }
    match appearance.get("theme")?.as_str()? {
        "system" => Some(theseus::data::Theme::System),
        "light" => Some(theseus::data::Theme::Light),
        "dark" => Some(theseus::data::Theme::Dark),
        "oled" => Some(theseus::data::Theme::Oled),
        "retro" => Some(theseus::data::Theme::Retro),
        _ => None,
    }
}

fn parse_launch_args(input: &str) -> Result<Option<Vec<String>>, &'static str> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(None);
    }
    shlex::split(input)
        .map(Some)
        .ok_or("JVM arguments contain an unmatched quote or escape.")
}

fn default_launch_hooks(
    pre_launch: &str,
    wrapper: &str,
    post_exit: &str,
) -> theseus::data::Hooks {
    theseus::data::Hooks {
        pre_launch: (!pre_launch.is_empty()).then(|| pre_launch.to_owned()),
        wrapper: (!wrapper.is_empty()).then(|| wrapper.to_owned()),
        post_exit: (!post_exit.is_empty()).then(|| post_exit.to_owned()),
    }
}

fn validate_instance_name(input: &str) -> Result<String, &'static str> {
    let name = input.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        return Err(
            "Enter a non-empty instance name without control characters.",
        );
    }
    if name.chars().count() > 64 {
        return Err("Instance names can contain at most 64 characters.");
    }
    Ok(name.to_owned())
}

fn delete_confirmation_matches(input: &str, instance_name: &str) -> bool {
    !instance_name.is_empty() && input == instance_name
}

fn safe_export_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches([' ', '.']);
    if cleaned.is_empty() {
        return "Modpack".into();
    }
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5",
        "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
        "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let base = cleaned.split('.').next().unwrap_or_default();
    if reserved
        .iter()
        .any(|reserved| base.eq_ignore_ascii_case(reserved))
    {
        format!("_{cleaned}")
    } else {
        cleaned.to_owned()
    }
}

fn parse_window_resolution(
    width: &str,
    height: &str,
) -> Result<Option<(u16, u16)>, &'static str> {
    match (width.trim().is_empty(), height.trim().is_empty()) {
        (true, true) => Ok(None),
        (false, false) => {
            let width = width.trim().parse::<u16>();
            let height = height.trim().parse::<u16>();
            match (width, height) {
                (Ok(width), Ok(height)) if width > 0 && height > 0 => {
                    Ok(Some((width, height)))
                }
                _ => Err(
                    "Resolution width and height must be positive values below 65536.",
                ),
            }
        }
        _ => Err(
            "Enter both resolution dimensions, or leave both blank for the launcher default.",
        ),
    }
}

fn parse_environment_json(
    input: &str,
) -> Result<Option<Vec<(String, String)>>, String> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    let variables: Vec<(String, String)> = serde_json::from_str(input)
        .map_err(|error| format!("Environment variables must be a JSON array of [key, value] pairs: {error}"))?;
    let mut keys = std::collections::HashSet::new();
    for (key, value) in &variables {
        if key.is_empty()
            || key.contains('=')
            || key.contains('\0')
            || value.contains('\0')
        {
            return Err("Environment variable names must be nonempty and cannot contain '=' or NUL characters; values cannot contain NUL characters.".into());
        }
        if !keys.insert(key.to_ascii_lowercase()) {
            return Err(format!(
                "Environment variable {key} is listed more than once."
            ));
        }
    }
    Ok(Some(variables))
}

fn parse_environment_variables(
    input: &str,
) -> Result<Vec<(String, String)>, String> {
    let mut variables = Vec::new();
    let mut keys = std::collections::HashSet::new();
    for entry in input.split_whitespace() {
        let Some((key, value)) = entry.split_once('=') else {
            return Err(format!(
                "Environment variables must use KEY=VALUE pairs. Invalid entry: {entry}"
            ));
        };
        if key.is_empty()
            || key.contains('\0')
            || value.contains('\0')
            || key.contains('=')
        {
            return Err(format!("Invalid environment variable name: {key}"));
        }
        if !keys.insert(key.to_ascii_lowercase()) {
            return Err(format!(
                "Environment variable {key} is listed more than once."
            ));
        }
        variables.push((key.to_owned(), value.to_owned()));
    }
    Ok(variables)
}

fn nonempty_setting(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn extract_minecraft_auth_code(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err(
            "Paste the authorization code from the Microsoft sign-in page."
                .into(),
        );
    }
    if input.len() > 4096 {
        return Err("The authorization code is too long.".into());
    }
    if let Ok(url) = url::Url::parse(input)
        && url.has_host()
    {
        if !is_trusted_microsoft_url(&url)
            || url.path() != "/oauth20_desktop.srf"
        {
            return Err(
                "Paste the code from the Microsoft callback page.".into()
            );
        }
        return url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .filter(|code| !code.is_empty() && code.len() <= 4096)
            .ok_or_else(|| {
                "The callback URL does not contain an authorization code."
                    .into()
            });
    }
    if input.chars().any(char::is_whitespace) {
        return Err(
            "The authorization code must not contain spaces or line breaks."
                .into(),
        );
    }
    Ok(input.to_owned())
}

fn is_trusted_microsoft_url(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("login.live.com")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
}

fn screenshot_grid_layout(width: usize) -> (usize, usize, usize, usize) {
    let content_width = width.saturating_sub(324);
    let gap = 12;
    let columns = ((content_width + gap) / 212).clamp(1, 4);
    let card_width =
        content_width.saturating_sub(gap * columns.saturating_sub(1)) / columns;
    let image_height = (card_width * 9 / 16).clamp(96, 180);
    let card_height = image_height + 100;
    (columns, card_width, image_height, card_height)
}

fn screenshot_copy_bounds(card_width: usize) -> UiBounds {
    UiBounds {
        x: card_width.saturating_sub(60),
        y: 8,
        width: 52,
        height: 24,
    }
}

fn screenshot_visible_rows(
    height: usize,
    stride: usize,
    remainder: usize,
) -> usize {
    height
        .saturating_sub(230)
        .saturating_add(remainder)
        .div_ceil(stride)
}

fn screenshot_max_scroll(
    height: usize,
    count: usize,
    columns: usize,
    stride: usize,
) -> f64 {
    count
        .div_ceil(columns)
        .saturating_mul(stride)
        .saturating_sub(if count == 0 { 0 } else { 12 })
        .saturating_sub(height.saturating_sub(230)) as f64
}

fn screenshot_row_position(
    row: usize,
    stride: usize,
    remainder: usize,
) -> (usize, usize) {
    let top = 230 + row * stride;
    (top.saturating_sub(remainder), remainder.saturating_sub(top))
}

fn screenshot_local_y(y: usize, scroll: usize, stride: usize) -> usize {
    y.saturating_sub(230).saturating_add(scroll % stride)
}

fn screenshot_prefetch_range(
    scroll: f64,
    stride: usize,
    columns: usize,
    visible_rows: usize,
    count: usize,
) -> std::ops::Range<usize> {
    let row = (scroll.max(0.0) as usize) / stride;
    let start = row.saturating_sub(2).saturating_mul(columns).min(count);
    let end = row
        .saturating_add(visible_rows)
        .saturating_add(4)
        .saturating_mul(columns)
        .min(count);
    start..end
}

fn screenshot_prefetch_ranges(
    current: f64,
    target: f64,
    stride: usize,
    columns: usize,
    visible_rows: usize,
    count: usize,
) -> [std::ops::Range<usize>; 2] {
    [target, current].map(|scroll| {
        screenshot_prefetch_range(scroll, stride, columns, visible_rows, count)
    })
}

fn decode_screenshot_thumbnail(
    path: std::path::PathBuf,
) -> Result<(u32, u32, Vec<u32>), String> {
    let mut reader = image::ImageReader::open(path)
        .map_err(|error| error.to_string())?
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    limits.max_alloc = Some(16 * 1024 * 1024);
    reader.limits(limits);
    let thumbnail = reader
        .decode()
        .map_err(|error| error.to_string())?
        .thumbnail(320, 180)
        .to_rgba8();
    let (width, height) = thumbnail.dimensions();
    let pixels = thumbnail
        .pixels()
        .map(|pixel| {
            let [red, green, blue, alpha] = pixel.0;
            let blend = |foreground: u8, background: u8| {
                (u16::from(foreground) * u16::from(alpha)
                    + u16::from(background) * (255 - u16::from(alpha)))
                    / 255
            };
            (u32::from(blend(red, 27)) << 16)
                | (u32::from(blend(green, 29)) << 8)
                | u32::from(blend(blue, 34))
        })
        .collect();
    Ok((width, height, pixels))
}

fn adjust_count(value: usize, delta: isize) -> usize {
    if delta < 0 {
        value.saturating_sub(delta.unsigned_abs()).max(1)
    } else {
        value.saturating_add(delta as usize)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let startup_args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let start_fresh_on_startup = startup_args
        .iter()
        .any(|arg| arg == std::ffi::OsStr::new("--start-fresh"));
    let pack_on_start = startup_args
        .into_iter()
        .map(std::path::PathBuf::from)
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mrpack"))
        })
        .next();
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(2)
            .enable_all()
            .build()?,
    );
    let event_loop = EventLoop::<UiMessage>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let event_loop_result = event_loop.run_app(&mut Launcher {
        runtime,
        window: None,
        surface: None,
        window_maximized: false,
        scroll_frame_time: std::time::Instant::now(),
        navigation_back: Vec::new(),
        navigation_forward: Vec::new(),
        navigation_last_route: None,
        navigation_restoring: false,
        backend: None,
        proxy: Some(proxy),
        ready: false,
        instances: Vec::new(),
        instance_cards: HashMap::new(),
        instance_metrics: HashMap::new(),
        instance_icons: HashMap::new(),
        instance_icons_pending: HashSet::new(),
        instance_icon_failures: HashSet::new(),
        groups: Vec::new(),
        selected_group: None,
        membership_mode: false,
        selected_library_instances: HashSet::new(),
        hidden_instance_ids: HashSet::new(),
        show_hidden_instances: false,
        hidden_instance_save_pending: HashMap::new(),
        hidden_instance_generation: 0,
        hidden_instance_load_generation: 0,
        library_selection_anchor: None,
        library_focused_instance: None,
        group_input: String::new(),
        create_group_open: false,
        create_group_from_selection: false,
        migration_plan: None,
        migration_prompt_open: false,
        migration_settings_entry: false,
        migration_busy: false,
        migration_saving_fresh_choice: false,
        selected_instance: None,
        rename_instance_open: false,
        rename_instance_input: String::new(),
        delete_confirmation_open: false,
        bulk_delete_confirmation_open: false,
        delete_confirmation_input: String::new(),
        instance_deletion_pending: false,
        instance_tab: 0,
        contents: Vec::new(),
        worlds: Vec::new(),
        server_statuses: HashMap::new(),
        server_motds: HashMap::new(),
        world_offset: 0,
        world_scroll_pixels: 0.0,
        world_scroll_target_pixels: 0.0,
        world_scroll_synced: 0,
        add_server_open: false,
        add_server_busy: false,
        server_edit_index: None,
        server_edit_source: None,
        server_remove_confirmation: None,
        world_editor_open: false,
        world_edit_path: String::new(),
        world_edit_name: String::new(),
        world_edit_reset_icon: false,
        world_delete_confirmation: None,
        server_name_input: String::new(),
        server_address_input: String::new(),
        server_pack_status: worlds::ServerPackStatus::Enabled,
        screenshots: Vec::new(),
        screenshot_thumbnails: HashMap::new(),
        screenshot_thumbnails_pending: HashSet::new(),
        screenshot_thumbnail_failures: HashSet::new(),
        screenshot_groups: Vec::new(),
        screenshot_group_mode: 0,
        screenshot_sort_mode: 0,
        screenshot_group_action_pending: false,
        screenshot_group_picker_open: false,
        screenshot_group_offset: 0,
        screenshot_group_scroll_pixels: 0.0,
        screenshot_group_scroll_target_pixels: 0.0,
        screenshot_search_query: String::new(),
        screenshot_group_rename_id: None,
        screenshot_group_name_input: String::new(),
        screenshot_group_delete_confirmation: None,
        screenshot_offset: 0,
        screenshot_scroll_pixels: 0.0,
        screenshot_scroll_target_pixels: 0.0,
        screenshot_scroll_synced: 0,
        selected_screenshot: None,
        selected_screenshot_ids: HashSet::new(),
        screenshot_viewer: false,
        screenshot_preview: None,
        screenshot_delete_confirmation: false,
        shared_instance_metadata: None,
        shared_instance_can_use: None,
        shared_instance_users: Vec::new(),
        shared_instance_invites: Vec::new(),
        shared_instance_invite_link: None,
        shared_instance_busy: false,
        shared_instance_preview: None,
        shared_instance_selected_files: HashSet::new(),
        shared_instance_preview_scroll_pixels: 0.0,
        shared_instance_preview_scroll_target_pixels: 0.0,
        shared_instance_progress: None,
        shared_instance_invite_open: false,
        shared_instance_user_query: String::new(),
        shared_instance_user_results: Vec::new(),
        shared_instance_selected_user_ids: HashSet::new(),
        shared_instance_pending_removal: None,
        shared_instance_unlink_confirmation: false,
        console_text: String::new(),
        console_scroll: 0,
        console_scroll_synced: 0,
        console_scroll_pixels: 0.0,
        console_scroll_target_pixels: 0.0,
        instance_logs: Vec::new(),
        selected_log: None,
        log_text: String::new(),
        log_search_query: String::new(),
        log_search_select_all: false,
        log_level_filter: 0,
        log_analysis: None,
        log_analysis_pending: false,
        log_share_pending: false,
        log_delete_confirmation: false,
        log_delete_pending: false,
        log_scroll: 0,
        log_scroll_synced: 0,
        log_scroll_pixels: 0.0,
        log_scroll_target_pixels: 0.0,
        process_state: None,
        running_processes: HashMap::new(),
        running_processes_pending: false,
        running_processes_refresh_dirty: false,
        file_path: String::new(),
        file_items: Vec::new(),
        file_preview: None,
        file_preview_scroll_pixels: 0.0,
        file_preview_scroll_target_pixels: 0.0,
        file_offset: 0,
        file_list_scroll_pixels: 0.0,
        file_list_scroll_target_pixels: 0.0,
        file_list_scroll_synced: 0,
        browse_query: String::new(),
        browse_query_cursor: 0,
        browse_query_select_all: false,
        browse_results: Vec::new(),
        browse_hidden_ids_dirty: false,
        browse_icons: HashMap::new(),
        browse_icons_pending: HashSet::new(),
        browse_icon_failures: HashSet::new(),
        news_articles: Vec::new(),
        news_error: None,
        news_loaded: false,
        sidebar_scroll_pixels: 0.0,
        sidebar_scroll_target_pixels: 0.0,
        onboarding_checklist: None,
        welcome_options_open: false,
        sidebar_friends: Vec::new(),
        recent_worlds: Vec::new(),
        jump_in_visible_items: 5,
        quick_instance_count_saving: false,
        native_color_theme: 0,
        native_color_theme_saving: false,
        preferred_dark_theme: theseus::data::Theme::Dark,
        theme_sync_across_devices: false,
        theme_sync_saving: false,
        theme_sync_generation: 0,
        jump_in_resize_drag: None,
        jump_in_overflow: None,
        jump_in_server_pings_pending: HashSet::new(),
        sidebar_friends_error: None,
        sidebar_friends_loaded: false,
        sidebar_friends_request_generation: 0,
        sidebar_friend_query: String::new(),
        friend_request_open: false,
        friend_incoming_open: false,
        friend_incoming_scroll_pixels: 0.0,
        friend_incoming_scroll_target_pixels: 0.0,
        friend_search_query: String::new(),
        friend_search_results: Vec::new(),
        friend_search_error: None,
        friend_search_loading: false,
        friend_request_pending: HashSet::new(),
        friend_remove_confirmation: None,
        friend_remove_error: None,
        friend_remove_pending: HashSet::new(),
        browse_selected: None,
        browse_details: None,
        browse_install_choice: None,
        browse_install_choice_generation: 0,
        browse_install_override: None,
        browse_installing_projects: HashMap::new(),
        browse_installing_modpacks: HashSet::new(),
        browse_install_notice: None,
        browse_install_scroll: 0.0,
        browse_install_scroll_target: 0.0,
        launcher_import_open: false,
        launcher_import_busy: false,
        launcher_import_source: None,
        launcher_import_offset: 0,
        launcher_import_scroll_pixels: 0.0,
        launcher_import_scroll_target_pixels: 0.0,
        launcher_import_focus: 0,
        global_skins: false,
        global_user_profile: false,
        user_profile_id: None,
        user_profile: None,
        user_profile_projects: Vec::new(),
        user_profile_filtered_project_indices: Vec::new(),
        user_profile_organizations: Vec::new(),
        user_profile_collections: Vec::new(),
        user_profile_type_filter: None,
        user_profile_show_collections: false,
        user_profile_loading: false,
        user_profile_error: None,
        user_profile_scroll_pixels: 0.0,
        user_profile_scroll_target_pixels: 0.0,
        minecraft_skins: Vec::new(),
        minecraft_skin_previews: HashMap::new(),
        minecraft_skin_previews_pending: HashSet::new(),
        minecraft_skin_preview_failures: HashSet::new(),
        minecraft_skin_scroll_pixels: 0.0,
        minecraft_skin_scroll_target_pixels: 0.0,
        minecraft_skin_variant:
            theseus::minecraft_skins::MinecraftSkinVariant::Classic,
        skin_delete_confirmation: None,
        minecraft_skin_rotation: 0.0,
        minecraft_skin_dragging: false,
        minecraft_skin_last_cursor_x: 0.0,
        global_browse: false,
        global_screenshots: false,
        global_hosting: false,
        hosting_servers: Vec::new(),
        hosting_servers_loading: false,
        hosting_servers_refreshing: false,
        hosting_servers_error: None,
        hosting_server_id: None,
        hosting_server_details: None,
        hosting_server_details_loading: false,
        hosting_server_details_error: None,
        hosting_tab: 0,
        hosting_backups: None,
        hosting_backups_loading: false,
        hosting_backups_error: None,
        hosting_backup_create_open: false,
        hosting_backup_create_name: String::new(),
        hosting_backup_search: String::new(),
        hosting_backup_show_deleted: false,
        hosting_backup_deleted_reason_filter: 0,
        hosting_backup_deleted_type_filter: 0,
        hosting_backup_deleted_recoverability_filter: 0,
        hosting_backup_deleted_sort: 0,
        hosting_backup_countdown_refresh: std::time::Instant::now(),
        hosting_backup_options_open: false,
        hosting_backup_auto_enabled: false,
        hosting_backup_manual_slots: 0,
        hosting_backup_interval_hours: 24,
        hosting_backup_options_saving: false,
        hosting_backup_rotation_confirmation: false,
        hosting_backup_rotation_count: 0,
        hosting_backup_selected: HashSet::new(),
        hosting_backup_creating: false,
        hosting_backup_prompt: None,
        hosting_backup_mutating: false,
        hosting_backup_downloading: false,
        hosting_backup_scroll_pixels: 0.0,
        hosting_backup_scroll_target_pixels: 0.0,
        hosting_access_users: Vec::new(),
        hosting_access_loading: false,
        hosting_access_loaded: false,
        hosting_access_error: None,
        hosting_access_remove_confirmation: None,
        hosting_access_scroll_pixels: 0.0,
        hosting_access_scroll_target_pixels: 0.0,
        hosting_access_invite_open: false,
        hosting_access_search_query: String::new(),
        hosting_access_search_results: Vec::new(),
        hosting_access_search_loading: false,
        hosting_access_search_complete: false,
        hosting_access_search_scroll_pixels: 0.0,
        hosting_access_search_scroll_target_pixels: 0.0,
        hosting_access_invite_error: None,
        hosting_access_role: 0,
        hosting_connection: None,
        hosting_socket_connecting: false,
        hosting_socket_connected: false,
        hosting_socket_error: None,
        hosting_power_state: None,
        hosting_live_stats: None,
        hosting_console_lines: VecDeque::new(),
        hosting_console_input: String::new(),
        hosting_console_scroll_pixels: 0.0,
        hosting_console_scroll_target_pixels: 0.0,
        hosting_console_follow_tail: true,
        hosting_power_pending: false,
        hosting_power_error: None,
        hosting_address_copied: false,
        hosting_scroll_pixels: 0.0,
        hosting_scroll_target_pixels: 0.0,
        library_search_query: String::new(),
        library_sort_mode: 3,
        library_group_mode: 0,
        library_collapsed_groups: HashSet::new(),
        library_sort_menu_open: false,
        library_group_menu_open: false,
        library_filter_open: false,
        library_loader_filters: HashSet::new(),
        library_instance_type_filters: HashSet::new(),
        library_install_state_filters: HashSet::new(),
        library_game_version_filters: HashSet::new(),
        library_game_version_filter_offset: 0,
        library_game_version_filter_scroll_pixels: 0.0,
        library_game_version_filter_scroll_target_pixels: 0.0,
        create_instance_open: false,
        browse_filter: 0,
        browse_sort: 0,
        browse_search_generation: 0,
        browse_request_generation: 0,
        browse_search_pending: false,
        installed_modpack_ids: HashSet::new(),

        browse_scroll_pixels: 0.0,
        browse_scroll_target_pixels: 0.0,
        browse_search_offset: 0,
        browse_page_size: 20,
        browse_view_menu_open: false,
        browse_sort_menu_open: false,
        browse_total_hits: 0,
        browse_error: None,
        browse_filter_options: None,
        browse_show_all_versions: false,
        browse_filter_options_error: None,
        browse_filter_options_loading: false,
        browse_filter_menu_open: false,
        browse_filter_group: 0,
        browse_filter_scroll_pixels: 0.0,
        browse_filter_scroll_target_pixels: 0.0,
        browse_selected_game_versions: HashSet::new(),
        browse_selected_loaders: HashSet::new(),
        browse_selected_categories: HashSet::new(),
        browse_selected_environments: HashSet::new(),
        browse_selected_licenses: HashSet::new(),
        browse_detail_selected_channels: HashSet::new(),
        browse_detail_selected_game_versions: HashSet::new(),
        browse_detail_selected_loaders: HashSet::new(),
        browse_detail_filter_menu_group: None,
        browse_detail_filter_option_offset: 0,
        browse_project_menu_open: false,
        browse_project_followed: false,
        browse_project_follow_pending: false,
        browse_project_follow_status_loaded: false,
        browse_project_follow_status_loading: false,
        browse_save_menu_open: false,
        browse_save_collections_loaded: false,
        browse_save_collections_loading: false,
        browse_save_collections: Vec::new(),
        browse_save_collection_pending: HashSet::new(),
        browse_save_scroll_pixels: 0.0,
        browse_collection_create_open: false,
        browse_collection_create_name: String::new(),
        browse_collection_creating: false,
        browse_detail_scroll_pixels: 0.0,
        browse_detail_scroll_target_pixels: 0.0,
        browse_gallery_index: 0,
        browse_gallery_viewer_open: false,
        browse_gallery_zoom: 1.0,
        browse_detail_tab: 0,
        browse_description_lines: None,
        settings_open: false,
        settings_section: 0,
        settings_jump_in_focused: false,
        settings_scroll_pixels: 0.0,
        settings_scroll_target_pixels: 0.0,
        global_screenshot_sync: None,
        default_window_width_input: String::new(),
        default_window_height_input: String::new(),
        window_defaults_saving: false,
        default_java_arguments_input: String::new(),
        default_environment_variables_input: String::new(),
        default_pre_launch_hook_input: String::new(),
        default_wrapper_hook_input: String::new(),
        default_post_exit_hook_input: String::new(),
        default_launch_settings_saving: false,
        java_open: false,
        settings: None,
        maximum_memory_mb: None,
        java_versions: Vec::new(),
        java_busy: false,
        instance_java_path: String::new(),
        instance_launch_args: String::new(),
        instance_memory_mb: String::new(),
        instance_max_memory_mb: 0,
        instance_fullscreen_mode: 0,
        instance_resolution_width: String::new(),
        instance_resolution_height: String::new(),
        instance_settings_loading: false,
        instance_settings_saving: false,
        instance_setup_page: 0,
        instance_environment_json: String::new(),
        instance_pre_launch_hook: String::new(),
        instance_wrapper_hook: String::new(),
        instance_post_exit_hook: String::new(),
        accounts_open: false,
        account_tab: 0,
        minecraft_accounts: Vec::new(),
        content_accounts: Vec::new(),
        modrinth_login_task: None,
        default_account: None,
        account_offset: 0,
        account_scroll_pixels: 0.0,
        account_scroll_target_pixels: 0.0,
        login_flow: None,
        login_code_input: String::new(),
        cursor: (0.0, 0.0),
        modifiers: winit::keyboard::ModifiersState::empty(),
        list_offset: 0,
        library_scroll_pixels: 0.0,
        library_scroll_target_pixels: 0.0,
        library_scroll_offset_synced: 0,
        content_scroll_pixels: 0.0,
        content_scroll_target_pixels: 0.0,
        content_filter: 0,
        filtered_content_cache: Vec::new(),
        name_input: String::new(),
        version_input: String::new(),
        loader_choice: 0,
        input_focus: 0,
        message: "Preparing native launcher…".into(),
        last_timed_message: String::new(),
        message_generation: 0,
        pack_on_start,
        start_fresh_on_startup,
        install_progress: None,
        active_job: None,
        job_control_pending: false,
        refresh_pending: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        groups_refresh_pending: Arc::new(std::sync::atomic::AtomicBool::new(
            false,
        )),
        console_refresh_pending: Arc::new(std::sync::atomic::AtomicBool::new(
            false,
        )),
        console_refresh_dirty: Arc::new(std::sync::atomic::AtomicBool::new(
            false,
        )),
        active_log_instance: Arc::new(std::sync::Mutex::new(None)),
    });
    eprintln!("Native launcher event loop returned: {event_loop_result:?}");
    event_loop_result?;
    Ok(())
}

#[cfg(test)]
mod library_selection_tests {
    use super::{
        NativeLibraryCardAction, library_filter_popup_layout,
        library_icon_prefetch_ranges, library_membership_toggle_visible,
        library_selection_button_bounds, library_visibility_matches,
        native_library_card_action, navigation_rail_button,
        settings_close_button, sidebar_footer_button, sidebar_instance_button,
        toggle_library_instance_selection, updated_library_selection_range,
    };
    use std::collections::HashSet;

    fn ids(values: &[&str]) -> HashSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn hidden_instances_are_opt_in_in_the_library_list() {
        let hidden = ids(&["hidden-id"]);
        assert!(!library_visibility_matches("hidden-id", &hidden, false));
        assert!(library_visibility_matches("hidden-id", &hidden, true));
        assert!(library_visibility_matches("visible-id", &hidden, false));
    }

    #[test]
    fn hidden_instance_filter_row_precedes_loader_options() {
        let layout = library_filter_popup_layout(
            super::UiBounds {
                x: 300,
                y: 400,
                width: 90,
                height: 30,
            },
            1100,
            720,
            3,
        );
        assert_eq!(layout.hidden_toggle_y + 24, layout.loader_rows_y);
        assert_eq!(layout.type_header_y, layout.loader_rows_y + 3 * 24 + 8);
        assert!(layout.clear_y < layout.y + layout.height);
    }

    #[test]
    fn library_icon_prefetch_covers_current_and_upward_target_without_gap() {
        let ranges = library_icon_prefetch_ranges(900, 0, 600, 200);
        assert_eq!(ranges, [(700, 1700), (0, 800)]);
        assert!(
            ranges
                .iter()
                .any(|(start, end)| { *start <= 0 && 100 < *end })
        );
        assert!(ranges.iter().all(|(start, end)| *end - *start <= 1000));
    }

    #[test]
    fn settings_close_target_matches_the_full_visible_button() {
        for width in [760, 1100] {
            let button = settings_close_button(width);
            assert_eq!(button.x, width - 142);
            assert_eq!(button.y, 82);
            assert_eq!((button.width, button.height), (112, 32));
            assert!(button.contains(button.x as f64, button.y as f64));
            assert!(button.contains(
                (button.x + button.width - 1) as f64,
                (button.y + button.height - 1) as f64,
            ));
            assert!(!button.contains(
                (button.x + button.width) as f64,
                (button.y + 1) as f64,
            ));
            assert!(!button.contains(
                (button.x + button.width - 1) as f64,
                (button.y + button.height) as f64,
            ));
        }
    }

    #[test]
    fn navigation_hit_targets_match_the_drawn_rail_and_shortcuts() {
        for y in [56, 108, 160, 212, 264] {
            let button = navigation_rail_button(y);
            assert_eq!((button.x, button.width, button.height), (8, 48, 48));
            assert!(button.contains(8.0, y as f64));
            assert!(button.contains(55.0, (y + 47) as f64));
            assert!(!button.contains(7.0, (y + 1) as f64));
            assert!(!button.contains(56.0, (y + 1) as f64));
            assert!(!button.contains(9.0, (y + 48) as f64));
        }

        let shortcut = sidebar_instance_button(2);
        assert!(shortcut.contains(12.0, shortcut.y as f64));
        assert!(shortcut.contains(51.0, (shortcut.y + 39) as f64));
        assert!(!shortcut.contains(11.0, (shortcut.y + 1) as f64));
        assert!(!shortcut.contains(52.0, (shortcut.y + 1) as f64));

        for height in [520, 720] {
            let settings = sidebar_footer_button(height, 1);
            assert!(settings.contains(8.0, settings.y as f64));
            assert!(settings.contains(55.0, (settings.y + 47) as f64));
            assert!(!settings.contains(56.0, (settings.y + 1) as f64));
        }
    }

    #[test]
    fn shift_selection_includes_the_anchor_and_forward_range() {
        let visible_order = vec!["a", "b", "c", "d"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();

        assert_eq!(
            updated_library_selection_range(
                &visible_order,
                Some("a"),
                "c",
                &ids(&["a"]),
            ),
            ids(&["a", "b", "c"]),
        );
    }

    #[test]
    fn shift_selection_supports_reverse_order_and_deselects_a_range() {
        let visible_order = vec!["a", "b", "c", "d"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();

        assert_eq!(
            updated_library_selection_range(
                &visible_order,
                Some("d"),
                "b",
                &ids(&["b", "c", "d"]),
            ),
            HashSet::new(),
        );
    }

    #[test]
    fn missing_anchor_or_target_falls_back_to_toggling_the_target() {
        let visible_order = vec!["a", "b", "c"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();

        assert_eq!(
            updated_library_selection_range(
                &visible_order,
                Some("missing"),
                "c",
                &ids(&["a"]),
            ),
            ids(&["a", "c"]),
        );
        assert_eq!(
            updated_library_selection_range(
                &visible_order,
                None,
                "a",
                &ids(&["a"]),
            ),
            HashSet::new(),
        );
    }

    #[test]
    fn enter_and_space_selection_toggle_only_the_focused_instance() {
        let mut selected = ids(&["a", "b"]);
        toggle_library_instance_selection(&mut selected, "b");
        assert_eq!(selected, ids(&["a"]));
        toggle_library_instance_selection(&mut selected, "c");
        assert_eq!(selected, ids(&["a", "c"]));
    }

    #[test]
    fn library_selection_button_stays_inside_compact_and_standard_cards() {
        for compact in [false, true] {
            for width in [128, 240, 400] {
                let card_x = 292;
                let card_y = 220;
                let bounds = library_selection_button_bounds(
                    card_x, card_y, width, compact,
                );
                assert!(bounds.x >= card_x);
                assert!(bounds.x + bounds.width <= card_x + width);
                assert!(bounds.y >= card_y);
                assert_eq!(bounds.width, if compact { 20 } else { 24 });
                assert_eq!(bounds.height, bounds.width);
                assert!(bounds.contains(
                    (bounds.x + bounds.width / 2) as f64,
                    (bounds.y + bounds.height / 2) as f64,
                ));
            }
        }
    }

    #[test]
    fn library_card_actions_follow_running_install_and_lock_state() {
        let action = |running,
                      installing,
                      installed,
                      locked,
                      launch_blocked,
                      selecting,
                      hovered| {
            native_library_card_action(
                running,
                installing,
                installed,
                locked,
                launch_blocked,
                selecting,
                hovered,
            )
        };
        assert_eq!(
            action(true, false, true, false, false, true, false),
            NativeLibraryCardAction::Stop,
        );
        assert_eq!(
            action(false, true, false, false, false, false, true),
            NativeLibraryCardAction::Installing,
        );
        assert_eq!(
            action(false, false, false, false, false, false, true),
            NativeLibraryCardAction::Repair,
        );
        assert_eq!(
            action(false, false, false, true, false, false, true),
            NativeLibraryCardAction::None,
        );
        assert_eq!(
            action(false, false, true, false, true, false, true),
            NativeLibraryCardAction::None,
        );
        assert_eq!(
            action(false, false, true, false, false, true, true),
            NativeLibraryCardAction::None,
        );
        assert_eq!(
            action(false, false, true, false, false, false, true),
            NativeLibraryCardAction::Play,
        );
    }

    #[test]
    fn running_card_shows_stop_instead_of_membership_toggle() {
        assert!(!library_membership_toggle_visible(
            true,
            NativeLibraryCardAction::Stop,
        ));
        assert!(library_membership_toggle_visible(
            true,
            NativeLibraryCardAction::Play,
        ));
        assert!(!library_membership_toggle_visible(
            false,
            NativeLibraryCardAction::Play,
        ));
    }
}

#[cfg(test)]
mod smooth_scroll_tests {
    use super::{
        Launcher, LibraryGridLayout, NativeLibraryGridRow,
        adjust_memory_setting, content_scroll_max, eased_scroll_position,
        home_library_toolbar_layout, library_toolbar_layout,
        message_dismissal_delay, profile_badge_labels, profile_project_matches,
        profile_project_types, profile_projects_max_scroll,
        visible_jump_in_count,
    };
    use std::time::Duration;

    #[test]
    fn screenshot_clipboard_png_retains_transparency() {
        let pixels =
            image::RgbaImage::from_raw(1, 1, vec![10, 20, 30, 45]).unwrap();
        let bytes = super::screenshot_clipboard_png(
            &image::DynamicImage::ImageRgba8(pixels),
        )
        .unwrap();
        let decoded = image::load_from_memory_with_format(
            &bytes,
            image::ImageFormat::Png,
        )
        .unwrap()
        .into_rgba8();
        assert_eq!(decoded.as_raw(), &[10, 20, 30, 45]);
    }

    #[test]
    fn screenshot_clipboard_bitmap_uses_compatible_header_and_preserves_colors()
    {
        let pixels = image::RgbaImage::from_raw(
            2,
            1,
            vec![10, 20, 30, 255, 40, 50, 60, 255],
        )
        .unwrap();
        let bytes = super::screenshot_clipboard_bitmap(
            image::DynamicImage::ImageRgba8(pixels),
        )
        .unwrap();
        assert_eq!(u32::from_le_bytes(bytes[14..18].try_into().unwrap()), 40);
        let decoded = image::load_from_memory_with_format(
            &bytes,
            image::ImageFormat::Bmp,
        )
        .unwrap()
        .into_rgb8();
        assert_eq!(decoded.as_raw(), &[10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn screenshot_copy_button_has_a_full_click_target() {
        let bounds = super::screenshot_copy_bounds(212);
        assert!(bounds.contains(152.0, 8.0));
        assert!(bounds.contains(203.0, 31.0));
        assert!(!bounds.contains(204.0, 31.0));
        assert!(!bounds.contains(152.0, 32.0));
    }

    #[test]
    fn screenshot_grid_counts_partial_rows_and_scrolls_to_last_card() {
        use super::{screenshot_max_scroll, screenshot_visible_rows};
        assert_eq!(screenshot_visible_rows(720, 292, 0), 2);
        assert_eq!(screenshot_visible_rows(720, 292, 200), 3);
        assert_eq!(screenshot_max_scroll(720, 9, 3, 292), 374.0);
        assert_eq!(screenshot_max_scroll(720, 3, 3, 292), 0.0);
        assert_eq!(screenshot_max_scroll(720, 0, 3, 292), 0.0);
    }

    #[test]
    fn screenshot_partial_row_geometry_matches_action_hit_testing() {
        use super::{screenshot_local_y, screenshot_row_position};
        let stride = 292;
        let remainder = 270;
        assert_eq!(screenshot_row_position(0, stride, remainder), (0, 40));
        assert_eq!(screenshot_row_position(1, stride, remainder), (252, 0));
        assert_eq!(screenshot_local_y(230, remainder, stride), 270);
        assert_eq!(screenshot_local_y(252, remainder, stride), 292);
    }

    #[test]
    fn screenshot_prefetch_covers_visible_and_destination_rows() {
        use super::{screenshot_prefetch_range, screenshot_prefetch_ranges};
        let [destination, visible] =
            screenshot_prefetch_ranges(225.0, 1225.0, 100, 3, 2, 90);
        assert_eq!(visible, 0..24);
        assert_eq!(destination, 30..54);
        assert_eq!(screenshot_prefetch_range(2900.0, 100, 3, 2, 90), 81..90);
        assert_eq!(screenshot_prefetch_range(9000.0, 100, 3, 2, 90), 90..90);
    }

    #[test]
    fn completed_loading_errors_are_dismissed() {
        assert_eq!(
            message_dismissal_delay("Loading failed: network unavailable"),
            Some(Duration::from_secs(12)),
        );
        assert_eq!(
            message_dismissal_delay("Could not load settings: offline"),
            Some(Duration::from_secs(12)),
        );
    }

    #[test]
    fn operation_messages_have_a_safety_timeout_if_completion_is_lost() {
        assert_eq!(
            message_dismissal_delay("Saving setting…"),
            Some(Duration::from_secs(15)),
        );
        assert_eq!(
            message_dismissal_delay("Setting saved."),
            Some(Duration::from_secs(5)),
        );
    }

    #[test]
    fn jump_in_count_is_limited_by_available_items_and_supported_maximum() {
        assert_eq!(visible_jump_in_count(5, 0, 2), 0);
        assert_eq!(visible_jump_in_count(1, 3, 2), 2);
        assert_eq!(visible_jump_in_count(3, 3, 2), 3);
        assert_eq!(visible_jump_in_count(1, 9, 1), 1);
        assert_eq!(visible_jump_in_count(24, 30, 2), 5);
    }

    #[test]
    fn home_library_toolbar_starts_after_recent_world_rows() {
        let base = library_toolbar_layout(1280);
        let one_row = home_library_toolbar_layout(1280, 1);
        let two_rows = home_library_toolbar_layout(1280, 2);
        assert_eq!(one_row.search.y, base.search.y + 128);
        assert_eq!(one_row.cards_top, base.cards_top + 128);
        assert_eq!(two_rows.search.y, one_row.search.y + 92);
        assert_eq!(two_rows.cards_top, one_row.cards_top + 92);

        let narrow = home_library_toolbar_layout(800, 1);
        assert_eq!(narrow.new_group.y, 188 + 128);
        assert_eq!(narrow.cards_top, 300 + 128);
        assert!(
            narrow
                .view_tabs()
                .iter()
                .all(|tab| tab.y + tab.height + 8 == narrow.search.y)
        );
        assert!(narrow.refresh(800).y > 188);
        assert!(
            narrow.membership(800).x + narrow.membership(800).width
                <= narrow.refresh(800).x
        );
    }

    #[test]
    fn narrow_library_toolbar_controls_stay_inside_the_content_column() {
        let layout = library_toolbar_layout(460);
        for bounds in [
            layout.search,
            layout.new_group,
            layout.new_instance,
            layout.import_pack,
            layout.sort,
            layout.filter,
            layout.group,
        ] {
            assert!(bounds.x >= 292);
            assert!(bounds.x + bounds.width <= 440);
        }
        assert_eq!(layout.search.y, 204);
        assert_eq!(layout.cards_top, 488);
        assert!(layout.view_tabs().iter().all(|tab| {
            tab.x + tab.width <= 440 && tab.y + tab.height <= layout.search.y
        }));
        let membership = layout.membership(460);
        assert_eq!(membership.y, layout.title_y() + 28);
        assert!(membership.x + membership.width <= 440);
    }

    #[test]
    fn content_scroll_stops_at_the_last_full_visible_row() {
        assert_eq!(content_scroll_max(8, 506), 2.0 * 74.0);
        assert_eq!(content_scroll_max(30, 506), 24.0 * 74.0);
        assert_eq!(content_scroll_max(30, 0), 29.0 * 74.0);
    }

    #[test]
    fn profile_project_scroll_is_bounded_by_the_viewport() {
        assert_eq!(profile_projects_max_scroll(4, 720), 0.0);
        assert_eq!(profile_projects_max_scroll(20, 720), 904.0);
        assert_eq!(profile_projects_max_scroll(20, 320), 1240.0);
    }

    #[test]
    fn profile_project_types_are_deduplicated_and_filterable() {
        let projects = vec![
            serde_json::json!({ "project_types": ["mod", "datapack"] }),
            serde_json::json!({ "project_type": "modpack" }),
            serde_json::json!({ "project_types": ["mod"] }),
        ];
        assert_eq!(
            profile_project_types(&projects),
            vec!["datapack", "mod", "modpack"]
        );
        assert!(profile_project_matches(&projects[0], "datapack"));
        assert!(!profile_project_matches(&projects[1], "mod"));
    }

    #[test]
    fn profile_badges_follow_profile_flags_dates_and_download_milestones() {
        let profile = serde_json::json!({"role":"moderator", "badges":1, "created":"2021-01-01T00:00:00Z"});
        let projects = vec![
            serde_json::json!({"downloads":1_200_000, "project_type":"modpack", "published":"2022-01-01T00:00:00Z"}),
        ];
        let badges = profile_badge_labels(Some(&profile), &projects);
        assert!(badges.contains(&"Modrinth Team"));
        assert!(badges.contains(&"Content Moderator"));
        assert!(badges.contains(&"Beta Tester"));
        assert!(badges.contains(&"Modrinth+ Member"));
        assert!(badges.contains(&"1m Downloads"));
        assert!(badges.contains(&"Early Modpack Adopter"));
        assert!(!badges.contains(&"10m Downloads"));
    }

    #[test]
    fn library_scroll_prefix_accounts_for_group_headers_and_card_rows() {
        let grid = LibraryGridLayout {
            columns: 2,
            card_width: 180,
            art_height: 120,
            card_height: 200,
            gap: 16,
            cards_top: 240,
            jump_in: false,
        };
        let rows = vec![
            NativeLibraryGridRow::Header("g1".into(), "Group 1".into(), 3),
            NativeLibraryGridRow::Cards(vec![0, 1]),
            NativeLibraryGridRow::Header("g2".into(), "Group 2".into(), 8),
        ];

        assert_eq!(Launcher::library_scroll_prefix(&rows, grid, 0), 0);
        assert_eq!(Launcher::library_scroll_prefix(&rows, grid, 1), 40);
        assert_eq!(Launcher::library_scroll_prefix(&rows, grid, 2), 256);
        assert_eq!(Launcher::library_scroll_prefix(&rows, grid, 3), 296);
    }

    #[test]
    fn easing_reaches_the_target_without_overshooting() {
        let mut position = 0.0;
        for _ in 0..100 {
            let next = eased_scroll_position(position, 520.0, 1.0 / 60.0);
            assert!(next >= position);
            assert!(next <= 520.0);
            position = next;
        }
        assert_eq!(position, 520.0);
    }

    #[test]
    fn default_memory_changes_by_sixty_four_mb_with_hardware_bounds() {
        assert_eq!(adjust_memory_setting(2048, 1, 8192), 2112);
        assert_eq!(adjust_memory_setting(512, -1, 8192), 512);
        assert_eq!(adjust_memory_setting(8192, 1, 8192), 8192);
        assert_eq!(adjust_memory_setting(4096, 1, 4096), 4096);
    }
}

#[cfg(test)]
mod auth_code_tests {
    use super::extract_minecraft_auth_code;

    #[test]
    fn accepts_an_authorization_code() {
        assert_eq!(
            extract_minecraft_auth_code("M.CODE_123-abc").unwrap(),
            "M.CODE_123-abc"
        );
    }

    #[test]
    fn extracts_code_from_microsoft_callback_url() {
        assert_eq!(
            extract_minecraft_auth_code("https://login.live.com/oauth20_desktop.srf?code=M.CODE%2B123&state=ignored").unwrap(),
            "M.CODE+123"
        );
    }

    #[test]
    fn rejects_untrusted_or_incomplete_callback_urls() {
        assert!(
            extract_minecraft_auth_code(
                "https://attacker.example/oauth20_desktop.srf?code=secret"
            )
            .is_err()
        );
        assert!(
            extract_minecraft_auth_code(
                "https://user@login.live.com/oauth20_desktop.srf?code=secret"
            )
            .is_err()
        );
        assert!(
            extract_minecraft_auth_code(
                "http://login.live.com/oauth20_desktop.srf?code=secret"
            )
            .is_err()
        );
        assert!(
            extract_minecraft_auth_code(
                "https://login.live.com/oauth20_desktop.srf?state=missing"
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod server_motd_tests {
    use super::{append_server_motd_text, gallery_zoom_after_scroll};

    #[test]
    fn flattens_nested_minecraft_chat_components() {
        let value = serde_json::json!({
            "text": "A server",
            "extra": [
                { "text": " with a ", "color": "green" },
                { "text": "welcome!" }
            ]
        });
        let mut output = String::new();
        append_server_motd_text(&value, &mut output);
        assert_eq!(output, "A server with a welcome!");
    }

    #[test]
    fn flattens_legacy_string_motds() {
        let value = serde_json::Value::String("Welcome to the server".into());
        let mut output = String::new();
        append_server_motd_text(&value, &mut output);
        assert_eq!(output, "Welcome to the server");
    }

    #[test]
    fn viewer_wheel_zoom_is_bounded_and_directional() {
        assert!(gallery_zoom_after_scroll(1.0, -58.0) > 1.0);
        assert_eq!(gallery_zoom_after_scroll(1.0, 58.0), 1.0);
        assert_eq!(gallery_zoom_after_scroll(5.0, -58.0), 5.0);
        assert!(gallery_zoom_after_scroll(2.0, 58.0) < 2.0);
    }
}

#[cfg(test)]
mod launch_args_tests {
    use super::render;
    use super::render::palette_color;
    use super::{
        SETTINGS_APPEARANCE_ROW_INDICES, SETTINGS_THEME_OPTIONS,
        default_launch_hooks, delete_confirmation_matches, is_dark_theme,
        native_color_theme_label, next_settings_theme,
        palette_for_native_color_theme, palette_for_theme,
        parse_environment_json, parse_environment_variables, parse_launch_args,
        parse_window_resolution, settings_row_position_at, settings_row_y,
        settings_theme_option_at, settings_theme_option_bounds,
        settings_theme_option_columns, settings_theme_selector_extra_height,
        theme_from_user_preferences, theme_label, uses_custom_window_controls,
        validate_instance_name,
    };

    #[test]
    fn parses_quoted_jvm_arguments_and_empty_defaults() {
        assert_eq!(parse_launch_args("  ").unwrap(), None);
        assert_eq!(
            parse_launch_args("-Xmx4G -Dlabel=\"North Star\"").unwrap(),
            Some(vec!["-Xmx4G".into(), "-Dlabel=North Star".into()])
        );
        assert!(parse_launch_args("-Dlabel=\"unterminated").is_err());
    }

    #[test]
    fn default_launch_hooks_preserve_commands_and_clear_empty_fields() {
        let hooks = default_launch_hooks(
            "echo before $INST_NAME",
            "",
            "echo after $INST_ID",
        );
        assert_eq!(hooks.pre_launch.as_deref(), Some("echo before $INST_NAME"));
        assert_eq!(hooks.wrapper, None);
        assert_eq!(hooks.post_exit.as_deref(), Some("echo after $INST_ID"));

        let cleared = default_launch_hooks("", "", "");
        assert!(cleared.pre_launch.is_none());
        assert!(cleared.wrapper.is_none());
        assert!(cleared.post_exit.is_none());
    }

    #[test]
    fn validates_resolution_pair_and_default_reset() {
        assert_eq!(parse_window_resolution("", " ").unwrap(), None);
        assert_eq!(
            parse_window_resolution("1920", "1080").unwrap(),
            Some((1920, 1080))
        );
        assert!(parse_window_resolution("1920", "").is_err());
        assert!(parse_window_resolution("0", "720").is_err());
        assert!(parse_window_resolution("65536", "720").is_err());
    }

    #[test]
    fn validates_environment_variable_json_pairs() {
        assert_eq!(parse_environment_json(" ").unwrap(), None);
        assert_eq!(
            parse_environment_json(r#"[["TOKEN","a;b"],["MODE","dev"]]"#)
                .unwrap(),
            Some(vec![
                ("TOKEN".into(), "a;b".into()),
                ("MODE".into(), "dev".into())
            ])
        );
        assert!(parse_environment_json("{bad json}").is_err());
        assert!(
            parse_environment_json(r#"[["PATH","one"],["path","two"]]"#)
                .is_err()
        );
        assert!(parse_environment_json(r#"[["BAD=KEY","x"]]"#).is_err());
    }

    #[test]
    fn parses_default_environment_variables_like_the_settings_ui() {
        assert_eq!(parse_environment_variables(" ").unwrap(), Vec::new());
        assert_eq!(
            parse_environment_variables("TOKEN=a=b MODE=dev").unwrap(),
            vec![
                ("TOKEN".into(), "a=b".into()),
                ("MODE".into(), "dev".into())
            ]
        );
        assert!(parse_environment_variables("missing-equals").is_err());
        assert!(parse_environment_variables("TOKEN=a token=b").is_err());
    }

    #[test]
    fn validates_instance_names() {
        assert_eq!(validate_instance_name("  My world  ").unwrap(), "My world");
        assert!(validate_instance_name(" ").is_err());
        assert!(validate_instance_name("bad\nname").is_err());
        assert!(validate_instance_name(&"x".repeat(65)).is_err());
    }

    #[test]
    fn deletion_requires_the_exact_instance_name() {
        assert!(delete_confirmation_matches("My instance", "My instance"));
        assert!(!delete_confirmation_matches("my instance", "My instance"));
        assert!(!delete_confirmation_matches("My instance ", "My instance"));
        assert!(!delete_confirmation_matches("anything", ""));
    }

    #[test]
    fn theme_preview_cards_select_each_supported_theme() {
        for row_width in [254, 580, 780] {
            let bounds = settings_theme_option_bounds(row_width, 146);
            for (theme, button) in
                SETTINGS_THEME_OPTIONS.into_iter().zip(bounds)
            {
                assert_eq!(
                    settings_theme_option_at(
                        row_width,
                        146,
                        (button.x + button.width / 2) as f64,
                        (button.y + button.height / 2) as f64,
                    )
                    .map(theme_label),
                    Some(theme).map(theme_label)
                );
            }
            let gap_x = (bounds[0].x + bounds[0].width + 1) as f64;
            assert!(
                settings_theme_option_at(row_width, 146, gap_x, 160.0)
                    .is_none()
            );
        }
    }

    #[test]
    fn settings_theme_preview_clicks_keep_the_drawn_coordinates() {
        let row_width = 580;
        let bounds = settings_theme_option_bounds(row_width, 146);
        for (theme, button) in SETTINGS_THEME_OPTIONS.into_iter().zip(bounds) {
            let pointer_x = render::pointer_x_to_logical(
                (button.x + button.width / 2) as f64,
                1024,
                true,
            );
            assert_eq!(
                settings_theme_option_at(
                    row_width,
                    146,
                    pointer_x,
                    (button.y + button.height / 2) as f64,
                )
                .map(theme_label),
                Some(theme).map(theme_label)
            );
        }
    }

    #[test]
    fn settings_theme_row_hit_targets_follow_full_scroll_offset() {
        assert_eq!(settings_row_y(0, 3, 580, 0.0), 618);
        assert_eq!(settings_row_y(0, 3, 580, 120.0), 498);
        assert_eq!(settings_row_y(1, 3, 580, 120.6), 175);
    }

    #[test]
    fn settings_appearance_rows_allow_both_theme_preview_rows() {
        assert_eq!(settings_row_position_at(0, 5, 580, 150.0, 0.0), Some(0));
        assert_eq!(settings_row_position_at(0, 5, 580, 210.0, 0.0), Some(0));
        assert_eq!(settings_row_position_at(0, 5, 580, 516.0, 0.0), Some(0));
        assert_eq!(settings_row_position_at(0, 5, 580, 520.0, 0.0), Some(1));
        assert_eq!(settings_row_position_at(0, 5, 580, 470.0, 50.0), Some(1));
        assert_eq!(settings_row_position_at(0, 5, 254, 300.0, 0.0), Some(0));
        assert_eq!(settings_row_position_at(0, 5, 254, 766.0, 0.0), Some(1));
        assert_eq!(settings_row_position_at(1, 6, 580, 176.0, 50.0), Some(1));
    }

    #[test]
    fn theme_preview_grid_matches_upstream_minimum_card_width() {
        assert_eq!(settings_theme_option_columns(254), 1);
        assert_eq!(settings_theme_option_columns(580), 2);
        assert_eq!(settings_theme_option_columns(780), 3);
        assert_eq!(settings_theme_selector_extra_height(580), 322);
    }

    #[test]
    fn keyboard_theme_selection_cycles_all_preview_options() {
        assert_eq!(
            theme_label(next_settings_theme(theseus::data::Theme::System, 1,)),
            "LIGHT"
        );
        assert_eq!(
            theme_label(next_settings_theme(theseus::data::Theme::System, -1,)),
            "RETRO"
        );
        assert_eq!(
            theme_label(next_settings_theme(theseus::data::Theme::Retro, 1,)),
            "SYSTEM"
        );
    }

    #[test]
    fn custom_window_controls_hide_when_native_decorations_are_enabled() {
        assert!(uses_custom_window_controls(false, false));
        assert!(!uses_custom_window_controls(false, true));
        assert!(!uses_custom_window_controls(true, false));
        assert!(!uses_custom_window_controls(true, true));
    }

    #[test]
    fn native_decorations_setting_is_only_shown_where_supported() {
        assert_eq!(
            SETTINGS_APPEARANCE_ROW_INDICES.contains(&34),
            !cfg!(target_os = "macos")
        );
    }

    #[test]
    fn synced_account_appearance_maps_to_native_themes() {
        let options = [
            (serde_json::json!({"appearance":{"auto":true}}), "SYSTEM"),
            (
                serde_json::json!({"appearance":{"auto":false,"theme":"light"}}),
                "LIGHT",
            ),
            (
                serde_json::json!({"appearance":{"auto":false,"theme":"dark"}}),
                "DARK",
            ),
            (
                serde_json::json!({"appearance":{"auto":false,"theme":"oled"}}),
                "OLED",
            ),
            (
                serde_json::json!({"appearance":{"auto":false,"theme":"retro"}}),
                "RETRO",
            ),
        ];
        for (preferences, expected) in options {
            assert_eq!(
                theme_from_user_preferences(&preferences).map(theme_label),
                Some(expected)
            );
        }
        assert!(theme_from_user_preferences(&serde_json::json!({})).is_none());
        assert!(theme_from_user_preferences(
            &serde_json::json!({"appearance":{"auto":false,"theme":"unknown"}})
        )
        .is_none());
    }

    #[test]
    fn system_theme_uses_the_window_theme_and_defaults_to_dark() {
        use theseus::data::Theme;
        use winit::window::Theme as WindowTheme;

        assert_eq!(
            palette_for_theme(
                Theme::System,
                Some(WindowTheme::Light),
                Theme::Retro,
            ),
            1
        );
        assert_eq!(
            palette_for_theme(
                Theme::System,
                Some(WindowTheme::Dark),
                Theme::Retro,
            ),
            3
        );
        assert_eq!(palette_for_theme(Theme::System, None, Theme::Oled), 2);
        assert_eq!(palette_for_theme(Theme::System, None, Theme::Light), 0);
        assert_eq!(
            palette_for_theme(
                Theme::Retro,
                Some(WindowTheme::Light),
                Theme::Dark,
            ),
            3
        );
        assert!(is_dark_theme(Theme::Dark));
        assert!(is_dark_theme(Theme::Oled));
        assert!(is_dark_theme(Theme::Retro));
        assert!(!is_dark_theme(Theme::System));
        assert!(!is_dark_theme(Theme::Light));
    }

    #[test]
    fn native_colorways_select_persistent_independent_palettes() {
        assert_eq!(palette_for_native_color_theme(0, 0), 0);
        assert_eq!(palette_for_native_color_theme(1, 0), 1);
        assert_eq!(palette_for_native_color_theme(2, 0), 2);
        for base in 0..=3 {
            for colorway in 1..=8 {
                assert_eq!(
                    palette_for_native_color_theme(base, colorway),
                    16 + base * 16 + colorway
                );
            }
        }
        assert_ne!(palette_for_native_color_theme(3, 1), 3);
        assert_eq!(native_color_theme_label(0), "DEFAULT");
        assert_eq!(native_color_theme_label(1), "AMBER");
        assert_eq!(native_color_theme_label(2), "GREEN");
        assert_eq!(native_color_theme_label(3), "OCEAN");
        assert_eq!(native_color_theme_label(4), "PURPLE");
        assert_eq!(native_color_theme_label(5), "EMBER");
        assert_eq!(native_color_theme_label(6), "ROSE");
        assert_eq!(native_color_theme_label(7), "ARCTIC");
        assert_eq!(native_color_theme_label(8), "TERMINAL");
    }

    #[test]
    fn each_theme_palette_maps_the_shared_surface_and_text_colors() {
        assert_eq!(palette_color(0x001b1d22, 0), 0x001b1d22);
        assert_eq!(palette_color(0x001b1d22, 1), 0x00f4f6f8);
        assert_eq!(palette_color(0x001b1d22, 2), 0x00000000);
        for palette in 3..=10 {
            assert_ne!(palette_color(0x001b1d22, palette), 0x001b1d22);
            assert_ne!(palette_color(0x00a8e66b, palette), 0x00a8e66b);
        }
        assert_eq!(palette_color(0x001b1d22, 3), 0x002e2519);
        assert_eq!(palette_color(0x00f1f3f5, 1), 0x001d252b);
        assert_eq!(palette_color(0x00bdc2ca, 2), 0x00c2c8cf);
        assert_eq!(palette_color(0x00f1f3f5, 3), 0x00f4e6c5);
        assert_eq!(palette_color(0x00123456, 1), 0x00123456);
    }

    #[test]
    fn colorways_preserve_the_selected_base_theme() {
        let dark_amber = palette_for_native_color_theme(0, 1);
        let light_amber = palette_for_native_color_theme(1, 1);
        let retro_amber = palette_for_native_color_theme(3, 1);
        assert_eq!(palette_color(0x001b1d22, dark_amber), 0x001b1d22);
        assert_eq!(palette_color(0x001b1d22, light_amber), 0x00f4f6f8);
        assert_eq!(palette_color(0x001b1d22, retro_amber), 0x002e2519);
        assert_eq!(
            palette_color(0x00a8e66b, dark_amber),
            palette_color(0x00a8e66b, 3)
        );
        assert_eq!(
            palette_color(0x00a8e66b, light_amber),
            palette_color(0x00a8e66b, 3)
        );
        assert_ne!(
            palette_color(0x001b1d22, retro_amber),
            palette_color(0x001b1d22, dark_amber)
        );
    }
}
