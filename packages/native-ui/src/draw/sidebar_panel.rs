use super::*;

fn format_news_date(value: &str) -> String {
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(value) {
        return date.format("%b %-d, %Y").to_string();
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return date.format("%b %-d, %Y").to_string();
    }
    value.to_owned()
}

fn draw_brand_gradient(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    left: usize,
    top: usize,
    panel_width: usize,
    panel_height: usize,
) {
    let base = themed_color(0x00141619);
    let top_tint = 0x00378949;
    let bottom_tint = 0x0044b68a;
    let panel_bottom = top.saturating_add(panel_height).min(height);
    let panel_right = left.saturating_add(panel_width).min(width);
    for y in top.min(height)..panel_bottom {
        let progress = if panel_height <= 1 {
            0.0
        } else {
            (y - top) as f32 / (panel_height - 1) as f32
        };
        let tint = mix_rgb(top_tint, bottom_tint, progress);
        let opacity = 0.10 + progress * 0.075;
        let row = y.saturating_mul(width);
        for x in left.min(width)..panel_right {
            buffer[row + x] = mix_rgb(base, tint, opacity);
        }
    }
}

fn mix_rgb(from: u32, to: u32, amount: f32) -> u32 {
    let mix_channel = |shift: u32| {
        let a = ((from >> shift) & 0xffu32) as f32;
        let b = ((to >> shift) & 0xffu32) as f32;
        (a + (b - a) * amount).round().clamp(0.0, 255.0) as u32
    };
    (mix_channel(16) << 16) | (mix_channel(8) << 8) | mix_channel(0)
}

fn draw_account_card(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    section_x: usize,
    section_width: usize,
    title_y: usize,
    card_y: usize,
    account: &native_backend::MinecraftAccount,
) {
    text(
        buffer,
        width,
        height,
        section_x,
        title_y,
        "PLAYING AS",
        0x007b838d,
        1,
    );
    rounded_rect(
        buffer,
        width,
        height,
        section_x,
        card_y,
        section_width,
        68,
        12,
        0x0022262c,
    );
    rounded_rect(
        buffer,
        width,
        height,
        section_x + 12,
        card_y + 12,
        44,
        44,
        22,
        0x00303b2b,
    );
    let initial = account
        .username
        .chars()
        .next()
        .map_or_else(String::new, |ch| ch.to_uppercase().to_string());
    text(
        buffer,
        width,
        height,
        section_x + 28,
        card_y + 27,
        &initial,
        0x00a8e66b,
        2,
    );
    text_clipped(
        buffer,
        width,
        height,
        section_x + 68,
        card_y + 17,
        &account.username,
        0x00f1f3f5,
        1,
        section_width.saturating_sub(92),
    );
    text(
        buffer,
        width,
        height,
        section_x + 68,
        card_y + 39,
        "Minecraft account",
        0x007b838d,
        1,
    );
}

enum ProjectSidebarBlock {
    Tags {
        title: String,
        tags: Vec<String>,
        action_group: Option<u8>,
    },
    Rows {
        title: String,
        rows: Vec<(String, String, Option<String>)>,
    },
}

fn project_sidebar_blocks(
    project: &BrowseProject,
    details: &BrowseProjectDetails,
) -> Vec<ProjectSidebarBlock> {
    let mut blocks = vec![ProjectSidebarBlock::Tags {
        title: "Minecraft Java".into(),
        tags: details.game_versions.clone(),
        action_group: None,
    }];
    if project.project_type != "resourcepack" {
        blocks.push(ProjectSidebarBlock::Tags {
            title: "Platforms".into(),
            tags: if details.loaders.is_empty() {
                vec!["No mod loader".into()]
            } else {
                details.loaders.clone()
            },
            action_group: Some(1),
        });
    }
    if project.project_type == "mod" || project.project_type == "modpack" {
        let supported = |side: &str| !matches!(side, "unknown" | "unsupported");
        let mut environments = Vec::new();
        if supported(&details.client_side) {
            environments.push("Client-side".into());
        }
        if supported(&details.server_side) {
            environments.push("Server-side".into());
        }
        if !environments.is_empty() {
            blocks.push(ProjectSidebarBlock::Tags {
                title: "Environments".into(),
                tags: environments,
                action_group: None,
            });
        }
    }
    let mut all_tags = details.categories.clone();
    all_tags.extend(details.additional_categories.iter().cloned());
    if !all_tags.is_empty() {
        blocks.push(ProjectSidebarBlock::Tags {
            title: "Categories".into(),
            tags: all_tags,
            action_group: Some(2),
        });
    }
    let mut detail_rows = vec![
        ("Author".into(), project.author.clone(), None),
        ("Published".into(), details.published.clone(), None),
        ("Updated".into(), details.updated.clone(), None),
    ];
    detail_rows.push((
        "License".into(),
        details.license_name.clone(),
        details.license_url.clone(),
    ));
    blocks.push(ProjectSidebarBlock::Rows {
        title: "Details".into(),
        rows: detail_rows,
    });
    if !details.links.is_empty() {
        blocks.push(ProjectSidebarBlock::Rows {
            title: "Links".into(),
            rows: details
                .links
                .iter()
                .map(|(label, url)| {
                    (label.clone(), url.clone(), Some(url.clone()))
                })
                .collect(),
        });
    }
    blocks
}

fn project_sidebar_chip_layout(
    tags: &[String],
    left: usize,
    width: usize,
    top: usize,
) -> (Vec<(String, usize, usize, usize)>, usize) {
    let right = left.saturating_add(width);
    let mut x = left;
    let mut y = top;
    let mut chips = Vec::new();
    for tag in tags {
        let chip_width =
            tag.chars().count().saturating_mul(8).saturating_add(18);
        if x > left && x.saturating_add(chip_width) > right {
            x = left;
            y = y.saturating_add(30);
        }
        let chip_width = chip_width.min(width);
        chips.push((tag.clone(), x, y, chip_width));
        x = x.saturating_add(chip_width).saturating_add(6);
    }
    let end = if chips.is_empty() { top } else { y + 24 };
    (chips, end)
}

fn project_sidebar_layout_end(
    project: &BrowseProject,
    details: &BrowseProjectDetails,
    left: usize,
    width: usize,
) -> usize {
    let mut y = 108usize;
    for block in project_sidebar_blocks(project, details) {
        match block {
            ProjectSidebarBlock::Tags { tags, .. } => {
                let (_, bottom) =
                    project_sidebar_chip_layout(&tags, left, width, y + 26);
                y = bottom.max(y + 26).saturating_add(18);
            }
            ProjectSidebarBlock::Rows { rows, .. } => {
                y = y.saturating_add(30 + rows.len().saturating_mul(32) + 18);
            }
        }
    }
    y
}

pub(crate) fn project_sidebar_chip_action_at(
    project: &BrowseProject,
    details: &BrowseProjectDetails,
    width: usize,
    content_x: f64,
    content_y: f64,
) -> Option<(u8, String)> {
    let left = width.saturating_sub(300).saturating_add(16);
    let section_width = width.min(300).saturating_sub(32);
    let mut y = 108usize;
    for block in project_sidebar_blocks(project, details) {
        match block {
            ProjectSidebarBlock::Tags {
                tags, action_group, ..
            } => {
                let (chips, bottom) = project_sidebar_chip_layout(
                    &tags,
                    left,
                    section_width,
                    y + 26,
                );
                if let Some(group) = action_group {
                    if let Some((label, _, _, _)) =
                        chips.into_iter().find(|(_, x, chip_y, chip_width)| {
                            content_x >= *x as f64
                                && content_x < (*x + *chip_width) as f64
                                && content_y >= *chip_y as f64
                                && content_y < (*chip_y + 24) as f64
                        })
                    {
                        if label != "No mod loader" {
                            return Some((group, label));
                        }
                    }
                }
                y = bottom.max(y + 26).saturating_add(18);
            }
            ProjectSidebarBlock::Rows { rows, .. } => {
                y = y.saturating_add(30 + rows.len().saturating_mul(32) + 18);
            }
        }
    }
    None
}

pub(crate) fn project_sidebar_content_height(
    project: &BrowseProject,
    details: &BrowseProjectDetails,
    width: usize,
) -> usize {
    project_sidebar_layout_end(project, details, 16, width.saturating_sub(32))
        .saturating_add(24)
}

pub(crate) fn project_sidebar_link_at(
    project: &BrowseProject,
    details: &BrowseProjectDetails,
    width: usize,
    content_y: f64,
) -> Option<String> {
    let left = width.saturating_sub(300).saturating_add(16);
    let section_width = width.min(300).saturating_sub(32);
    let mut y = 108usize;
    for block in project_sidebar_blocks(project, details) {
        match block {
            ProjectSidebarBlock::Tags { tags, .. } => {
                let (_, bottom) = project_sidebar_chip_layout(
                    &tags,
                    left,
                    section_width,
                    y + 26,
                );
                y = bottom.max(y + 26).saturating_add(18);
            }
            ProjectSidebarBlock::Rows { rows, .. } => {
                let row_start = y + 30;
                for (index, (_, _, link)) in rows.iter().enumerate() {
                    let row_y = row_start + index * 32;
                    if link.is_some()
                        && (row_y as f64..(row_y + 30) as f64)
                            .contains(&content_y)
                    {
                        return link.clone();
                    }
                }
                y = row_start
                    .saturating_add(rows.len().saturating_mul(32) + 18);
            }
        }
    }
    None
}

fn draw_project_right_sidebar(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    project: &BrowseProject,
    details: &BrowseProjectDetails,
    scroll_pixels: f64,
) {
    let sidebar_width = 300.min(width);
    let left = width.saturating_sub(sidebar_width);
    let section_x = left + 16;
    let section_width = sidebar_width.saturating_sub(32);
    let content_height =
        project_sidebar_content_height(project, details, width);
    let max_scroll = content_height.saturating_sub(height.saturating_sub(48));
    let scroll_offset = (scroll_pixels as usize).min(max_scroll);
    crate::render::set_page_vertical_layout(
        Some(scroll_offset),
        Some((left, 48, sidebar_width, height.saturating_sub(48))),
    );
    text(
        buffer,
        width,
        height,
        section_x,
        74,
        "COMPATIBILITY",
        0x00f1f3f5,
        2,
    );
    let mut y = 108usize;
    for block in project_sidebar_blocks(project, details) {
        match block {
            ProjectSidebarBlock::Tags { title, tags, .. } => {
                text(
                    buffer, width, height, section_x, y, &title, 0x00bdc2ca, 1,
                );
                let (chips, bottom) = project_sidebar_chip_layout(
                    &tags,
                    section_x,
                    section_width,
                    y + 26,
                );
                for (label, x, chip_y, chip_width) in chips {
                    rounded_rect(
                        buffer, width, height, x, chip_y, chip_width, 24, 12,
                        0x0022262c,
                    );
                    text_clipped(
                        buffer,
                        width,
                        height,
                        x + 9,
                        chip_y + 7,
                        &label,
                        0x00bdc2ca,
                        1,
                        chip_width.saturating_sub(18),
                    );
                }
                y = bottom.max(y + 26).saturating_add(18);
            }
            ProjectSidebarBlock::Rows { title, rows } => {
                text(
                    buffer,
                    width,
                    height,
                    section_x,
                    y,
                    &title.to_uppercase(),
                    0x00f1f3f5,
                    1,
                );
                let row_start = y + 30;
                for (index, (label, value, link)) in rows.iter().enumerate() {
                    let row_y = row_start + index * 32;
                    text_clipped(
                        buffer, width, height, section_x, row_y, label,
                        0x007b838d, 1, 84,
                    );
                    text_clipped(
                        buffer,
                        width,
                        height,
                        section_x + 88,
                        row_y,
                        value,
                        if link.is_some() {
                            0x00a8e66b
                        } else {
                            0x00bdc2ca
                        },
                        1,
                        section_width.saturating_sub(88),
                    );
                }
                y = row_start
                    .saturating_add(rows.len().saturating_mul(32) + 18);
            }
        }
    }
    crate::render::set_page_vertical_layout(None, None);
}

pub(super) fn draw_right_sidebar(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cursor: (f64, f64),
    accounts: &[native_backend::MinecraftAccount],
    default_account: Option<uuid::Uuid>,
    onboarding_checklist: Option<&theseus::data::OnboardingChecklist>,
    friend_query: &str,
    collapsed_groups: [bool; 4],
    friends: &[native_backend::NativeFriend],
    friends_error: Option<&str>,
    friends_loaded: bool,
    articles: &[native_backend::NewsArticle],
    news_icons: &HashMap<String, (u32, u32, Vec<u32>)>,
    news_error: Option<&str>,
    news_loaded: bool,
    scroll_pixels: f64,
    browse_selected: Option<&BrowseProject>,
    browse_details: Option<&BrowseProjectDetails>,
) {
    if let (Some(project), Some(details)) = (browse_selected, browse_details) {
        draw_brand_gradient(
            buffer,
            width,
            height,
            width.saturating_sub(300),
            48,
            300.min(width),
            height.saturating_sub(48),
        );
        rect(
            buffer,
            width,
            height,
            width.saturating_sub(300),
            48,
            1,
            height.saturating_sub(48),
            0x00363b42,
        );
        draw_project_right_sidebar(
            buffer,
            width,
            height,
            project,
            details,
            scroll_pixels,
        );
        return;
    }
    let articles = &articles[..articles.len().min(4)];
    let sidebar_width = 300.min(width);
    let left = width.saturating_sub(sidebar_width);
    let friend_sections = sidebar_friend_sections(
        friends,
        onboarding_checklist,
        friend_query,
        collapsed_groups,
    );
    let news_top = sidebar_news_top(
        friends,
        onboarding_checklist,
        friend_query,
        collapsed_groups,
    );
    draw_brand_gradient(
        buffer,
        width,
        height,
        left,
        48,
        sidebar_width,
        height.saturating_sub(48),
    );
    rect(
        buffer,
        width,
        height,
        left,
        48,
        1,
        height.saturating_sub(48),
        0x00363b42,
    );

    let max_scroll = sidebar_scroll_max(
        articles.len(),
        friends,
        height,
        onboarding_checklist,
        friend_query,
        collapsed_groups,
    );
    let scroll_offset = (scroll_pixels as usize).min(max_scroll);
    crate::render::set_page_vertical_layout(
        Some(scroll_offset),
        Some((left, 48, sidebar_width, height.saturating_sub(48))),
    );
    let cursor = (
        cursor.0,
        if cursor.1 >= 48.0 {
            cursor.1 + scroll_offset as f64
        } else {
            cursor.1
        },
    );

    let section_x = left + 16;
    let section_width = sidebar_width.saturating_sub(32);
    let show_checklist =
        onboarding_checklist.is_some_and(|checklist| checklist.show_checklist);
    let account = accounts
        .iter()
        .find(|account| Some(account.id) == default_account)
        .or_else(|| accounts.iter().find(|account| account.active));
    if show_checklist {
        text(
            buffer,
            width,
            height,
            section_x,
            74,
            "Getting started",
            0x00f1f3f5,
            2,
        );
        rounded_rect(
            buffer,
            width,
            height,
            section_x,
            98,
            section_width,
            118,
            12,
            0x0022262c,
        );
        let Some(checklist) = onboarding_checklist else {
            unreachable!("checklist visibility requires checklist data");
        };
        for (index, (label, complete)) in [
            ("Create first instance", checklist.has_created_instance),
            ("Sign in to Minecraft", checklist.has_logged_into_minecraft),
            ("Sign in to Modrinth", checklist.has_logged_into_modrinth),
        ]
        .into_iter()
        .enumerate()
        {
            let row_y = 104 + index * 36;
            rounded_rect(
                buffer,
                width,
                height,
                section_x + 8,
                row_y,
                section_width.saturating_sub(16),
                32,
                8,
                if complete { 0x0021262b } else { 0x002a3037 },
            );
            rounded_rect(
                buffer,
                width,
                height,
                section_x + 18,
                row_y + 8,
                16,
                16,
                8,
                if complete { 0x00a8e66b } else { 0x003b414a },
            );
            if complete {
                draw_line(
                    buffer,
                    width,
                    height,
                    ((section_x + 22) as isize, (row_y + 16) as isize),
                    ((section_x + 25) as isize, (row_y + 19) as isize),
                    0x00141619,
                );
                draw_line(
                    buffer,
                    width,
                    height,
                    ((section_x + 25) as isize, (row_y + 19) as isize),
                    ((section_x + 31) as isize, (row_y + 12) as isize),
                    0x00141619,
                );
            } else {
                rounded_rect(
                    buffer,
                    width,
                    height,
                    section_x + 22,
                    row_y + 12,
                    8,
                    8,
                    4,
                    0x00bdc2ca,
                );
            }
            text_clipped(
                buffer,
                width,
                height,
                section_x + 42,
                row_y + 11,
                label,
                if complete { 0x007b838d } else { 0x00f1f3f5 },
                1,
                section_width.saturating_sub(58),
            );
        }
    }
    if let Some(account) = account {
        let (title_y, card_y) = if show_checklist {
            (244, 270)
        } else {
            (78, 104)
        };
        draw_account_card(
            buffer,
            width,
            height,
            section_x,
            section_width,
            title_y,
            card_y,
            account,
        );
    } else if !show_checklist {
        text(
            buffer,
            width,
            height,
            section_x,
            78,
            "GET STARTED",
            0x007b838d,
            1,
        );
        rounded_rect(
            buffer,
            width,
            height,
            section_x,
            104,
            section_width,
            108,
            12,
            0x0022262c,
        );
        text(
            buffer,
            width,
            height,
            section_x + 14,
            120,
            "Connect your Minecraft account",
            0x00f1f3f5,
            1,
        );
        text(
            buffer,
            width,
            height,
            section_x + 14,
            142,
            "Sign in to play and manage skins.",
            0x00bdc2ca,
            1,
        );
        rounded_rect(
            buffer,
            width,
            height,
            section_x + 14,
            164,
            section_width.saturating_sub(28),
            32,
            8,
            0x00a8e66b,
        );
        text(
            buffer,
            width,
            height,
            section_x + 36,
            174,
            "LOG IN TO MINECRAFT",
            0x00141619,
            1,
        );
    }

    let friends_header_y = sidebar_friend_header_y(onboarding_checklist);
    let friends_divider_y = friends_header_y.saturating_sub(16);
    let incoming_request_count = sidebar_incoming_friend_indices(friends).len();
    rect(
        buffer,
        width,
        height,
        left,
        friends_divider_y,
        sidebar_width,
        1,
        0x00363b42,
    );
    if friends.is_empty() {
        text(
            buffer,
            width,
            height,
            section_x,
            friends_header_y,
            "Friends",
            0x00f1f3f5,
            2,
        );
        rounded_rect(
            buffer,
            width,
            height,
            left + sidebar_width.saturating_sub(44),
            friends_header_y.saturating_sub(6),
            28,
            24,
            6,
            0x0022262c,
        );
        text(
            buffer,
            width,
            height,
            left + sidebar_width.saturating_sub(35),
            friends_header_y + 1,
            "+",
            0x00a8e66b,
            1,
        );
    } else {
        let header_y = friends_header_y.saturating_sub(6);
        rounded_rect(
            buffer, width, height, section_x, header_y, 30, 30, 8, 0x0022262c,
        );
        text(
            buffer,
            width,
            height,
            section_x + 10,
            header_y + 9,
            "+",
            0x00d5d9df,
            1,
        );
        let search_x = section_x + 38;
        let inbox_width = if incoming_request_count > 0 { 34 } else { 0 };
        let search_width =
            section_width.saturating_sub(38).saturating_sub(inbox_width);
        rounded_rect(
            buffer,
            width,
            height,
            search_x,
            header_y,
            search_width,
            30,
            8,
            0x0022262c,
        );
        rounded_rect(
            buffer,
            width,
            height,
            search_x + 10,
            header_y + 9,
            9,
            9,
            5,
            0x007b838d,
        );
        rect(
            buffer,
            width,
            height,
            search_x + 18,
            header_y + 18,
            5,
            2,
            0x007b838d,
        );
        text_clipped(
            buffer,
            width,
            height,
            search_x + 28,
            header_y + 10,
            if friend_query.is_empty() {
                "Search friends..."
            } else {
                friend_query
            },
            if friend_query.is_empty() {
                0x007b838d
            } else {
                0x00f1f3f5
            },
            1,
            search_width.saturating_sub(38),
        );
        if incoming_request_count > 0 {
            let inbox_x = left + sidebar_width.saturating_sub(44);
            rounded_rect(
                buffer, width, height, inbox_x, header_y, 28, 30, 8, 0x0022262c,
            );
            rect(
                buffer,
                width,
                height,
                inbox_x + 7,
                header_y + 11,
                14,
                10,
                0x00bdc2ca,
            );
            draw_line(
                buffer,
                width,
                height,
                ((inbox_x + 7) as isize, (header_y + 11) as isize),
                ((inbox_x + 14) as isize, (header_y + 16) as isize),
                0x0022262c,
            );
            draw_line(
                buffer,
                width,
                height,
                ((inbox_x + 14) as isize, (header_y + 16) as isize),
                ((inbox_x + 21) as isize, (header_y + 11) as isize),
                0x0022262c,
            );
            rounded_rect(
                buffer,
                width,
                height,
                inbox_x + 17,
                header_y - 4,
                17,
                14,
                7,
                0x00e65b5b,
            );
            let badge = if incoming_request_count > 9 {
                "9+".to_owned()
            } else {
                incoming_request_count.to_string()
            };
            text(
                buffer,
                width,
                height,
                inbox_x + if incoming_request_count > 9 { 20 } else { 22 },
                header_y - 1,
                &badge,
                0x00ffffff,
                1,
            );
        }
    }
    if !friends_loaded {
        text(
            buffer,
            width,
            height,
            section_x,
            friends_header_y + 26,
            "Loading friends…",
            0x00bdc2ca,
            1,
        );
    } else if friends_error.is_some() {
        text(
            buffer,
            width,
            height,
            section_x,
            friends_header_y + 26,
            "Friends could not be loaded.",
            0x00bdc2ca,
            1,
        );
    } else if friends.is_empty() {
        text(
            buffer,
            width,
            height,
            section_x,
            friends_header_y + 26,
            "Add friends to see what they're playing!",
            0x00bdc2ca,
            1,
        );
    } else if friend_sections.is_empty() {
        text_clipped(
            buffer,
            width,
            height,
            section_x + 12,
            friends_header_y + 30,
            &format!("No friends matching '{}'.", friend_query.trim()),
            0x00bdc2ca,
            1,
            section_width.saturating_sub(24),
        );
    } else {
        for section in &friend_sections {
            text(
                buffer,
                width,
                height,
                section_x,
                section.header_y,
                &format!(
                    "{} - {}",
                    section.group.label(),
                    section.friend_indices.len()
                ),
                0x00bdc2ca,
                1,
            );
            let chevron_x = left + sidebar_width.saturating_sub(23);
            let chevron_y = section.header_y + 6;
            let (start_y, end_y) = if section.collapsed {
                (chevron_y, chevron_y + 6)
            } else {
                (chevron_y + 3, chevron_y - 3)
            };
            draw_line(
                buffer,
                width,
                height,
                ((chevron_x - 4) as isize, start_y as isize),
                (chevron_x as isize, end_y as isize),
                0x007b838d,
            );
            draw_line(
                buffer,
                width,
                height,
                (chevron_x as isize, end_y as isize),
                ((chevron_x + 4) as isize, start_y as isize),
                0x007b838d,
            );
            if section.collapsed {
                continue;
            }
            for (row_index, friend_index) in
                section.friend_indices.iter().enumerate()
            {
                let friend = &friends[*friend_index];
                let y = section.header_y + 24 + row_index * 38;
                let hovered = (section_x as f64
                    ..(section_x + section_width) as f64)
                    .contains(&cursor.0)
                    && (y as f64..(y + 34) as f64).contains(&cursor.1);
                rounded_rect(
                    buffer,
                    width,
                    height,
                    section_x,
                    y,
                    section_width,
                    34,
                    8,
                    if hovered { 0x0022262c } else { 0x001b1d22 },
                );
                let avatar_key = format!("friend:{}", friend.user_id);
                if let Some((image_width, image_height, pixels)) =
                    news_icons.get(&avatar_key)
                {
                    draw_scaled_image(
                        buffer,
                        width,
                        height,
                        *image_width,
                        *image_height,
                        pixels,
                        section_x + 6,
                        y + 5,
                        24,
                        24,
                    );
                    if friend.online && friend.accepted {
                        rounded_rect(
                            buffer,
                            width,
                            height,
                            section_x + 24,
                            y + 22,
                            7,
                            7,
                            4,
                            0x001bd96a,
                        );
                    }
                } else {
                    rounded_rect(
                        buffer,
                        width,
                        height,
                        section_x + 6,
                        y + 5,
                        24,
                        24,
                        12,
                        if friend.online && friend.accepted {
                            0x00303b2b
                        } else {
                            0x002e333a
                        },
                    );
                    let initial = friend
                        .username
                        .chars()
                        .next()
                        .map_or_else(String::new, |ch| {
                            ch.to_uppercase().to_string()
                        });
                    text(
                        buffer,
                        width,
                        height,
                        section_x + 14,
                        y + 12,
                        &initial,
                        if friend.online && friend.accepted {
                            0x00a8e66b
                        } else {
                            0x00bdc2ca
                        },
                        1,
                    );
                }
                text_clipped(
                    buffer,
                    width,
                    height,
                    section_x + 38,
                    y + 5,
                    &friend.username,
                    0x00f1f3f5,
                    1,
                    section_width.saturating_sub(if hovered { 76 } else { 54 }),
                );
                let status = if friend.incoming_request {
                    "INCOMING REQUEST · CLICK TO ACCEPT"
                } else if !friend.accepted {
                    "REQUEST SENT"
                } else if let Some(instance) = friend.playing_on.as_deref() {
                    instance
                } else if friend.online {
                    "ONLINE"
                } else {
                    "OFFLINE"
                };
                text_clipped(
                    buffer,
                    width,
                    height,
                    section_x + 38,
                    y + 19,
                    status,
                    0x007b838d,
                    1,
                    section_width.saturating_sub(54),
                );
                if hovered {
                    rounded_rect(
                        buffer,
                        width,
                        height,
                        section_x + section_width.saturating_sub(28),
                        y + 5,
                        22,
                        24,
                        6,
                        0x003b302b,
                    );
                    text(
                        buffer,
                        width,
                        height,
                        section_x + section_width.saturating_sub(21),
                        y + 11,
                        "×",
                        0x00e68a6b,
                        1,
                    );
                }
            }
        }
    }
    rect(
        buffer,
        width,
        height,
        left,
        news_top - 46,
        sidebar_width,
        1,
        0x00363b42,
    );
    text(
        buffer,
        width,
        height,
        section_x,
        news_top - 28,
        "NEWS",
        0x00f1f3f5,
        2,
    );
    text(
        buffer,
        width,
        height,
        left + sidebar_width.saturating_sub(78),
        news_top - 26,
        "News",
        0x007b838d,
        1,
    );

    if articles.is_empty() {
        text(
            buffer,
            width,
            height,
            section_x,
            news_top + 8,
            if !news_loaded {
                "Loading news…"
            } else if news_error.is_some() {
                "News could not be loaded."
            } else {
                "No news articles are available."
            },
            0x00bdc2ca,
            1,
        );
        if news_loaded && news_error.is_some() {
            rounded_rect(
                buffer,
                width,
                height,
                section_x,
                news_top + 38,
                84,
                30,
                6,
                0x002a3037,
            );
            text(
                buffer,
                width,
                height,
                section_x + 18,
                news_top + 48,
                "RETRY",
                0x00d5d9df,
                1,
            );
        }
        crate::render::set_page_vertical_layout(None, None);
        return;
    }

    let card_top = news_top;
    let card_height = SIDEBAR_NEWS_CARD_HEIGHT;
    let stride = SIDEBAR_NEWS_CARD_STRIDE;
    let first_visible = (scroll_offset.saturating_sub(card_top - 48) / stride)
        .min(articles.len().saturating_sub(1));
    let visible_cards = height.saturating_sub(card_top - 48).div_ceil(stride);
    for (visible_index, article) in articles
        .iter()
        .skip(first_visible)
        .take(visible_cards + 1)
        .enumerate()
    {
        let y = card_top + (first_visible + visible_index) * stride;
        let hovered = (section_x as f64..(section_x + section_width) as f64)
            .contains(&cursor.0)
            && (y as f64..(y + card_height) as f64).contains(&cursor.1);
        rounded_rect(
            buffer,
            width,
            height,
            section_x,
            y,
            section_width,
            card_height,
            12,
            if hovered { 0x0022262c } else { 0x001b1d22 },
        );
        let image_x = section_x + 8;
        let image_y = y + 8;
        let image_width = section_width.saturating_sub(16);
        let image_height = 142;
        rounded_rect(
            buffer,
            width,
            height,
            image_x,
            image_y,
            image_width,
            image_height,
            8,
            0x002e333a,
        );
        if let Some((image_width_px, image_height_px, pixels)) =
            news_icons.get(&format!("news:{}", article.link))
        {
            draw_scaled_image(
                buffer,
                width,
                height,
                *image_width_px,
                *image_height_px,
                pixels,
                image_x + 1,
                image_y + 1,
                image_width.saturating_sub(2),
                image_height.saturating_sub(2),
            );
        }
        let title = crate::render::wrap_text_to_width(
            &article.title,
            8,
            section_width.saturating_sub(20),
            2,
        );
        text_clipped(
            buffer,
            width,
            height,
            section_x + 10,
            y + 158,
            &title,
            0x00f1f3f5,
            8,
            section_width.saturating_sub(20),
        );
        let summary = crate::render::wrap_text_to_width(
            &article.summary,
            1,
            section_width.saturating_sub(20),
            2,
        );
        text_clipped(
            buffer,
            width,
            height,
            section_x + 10,
            y + 198,
            &summary,
            0x00bdc2ca,
            1,
            section_width.saturating_sub(20),
        );
        text_clipped(
            buffer,
            width,
            height,
            section_x + 10,
            y + 254,
            &format_news_date(&article.date),
            0x007b838d,
            1,
            section_width.saturating_sub(20),
        );
    }
    let footer_y = card_top + articles.len().min(4) * stride + 4;
    rounded_rect(
        buffer,
        width,
        height,
        section_x,
        footer_y,
        section_width,
        32,
        8,
        0x00303b2b,
    );
    text(
        buffer,
        width,
        height,
        section_x + section_width.saturating_sub(102) / 2,
        footer_y + 10,
        "VIEW ALL NEWS",
        0x00a8e66b,
        1,
    );
    crate::render::set_page_vertical_layout(None, None);
}
