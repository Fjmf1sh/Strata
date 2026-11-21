use super::*;

pub(super) fn draw_content_panel(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    layout_width: usize,
    visible_contents: &[usize],
    contents: &[theseus::data::ContentItem],
    browse_icons: &HashMap<String, (u32, u32, Vec<u32>)>,
    content_scroll_pixels: f64,
    content_filter: u8,
    message: &str,
) {
    let content_count = format!(
        "{} OF {} CONTENT ITEMS",
        visible_contents.len(),
        contents.len()
    );
    text_clipped(
        buffer,
        w,
        h,
        292,
        150,
        &content_count,
        0x00bdc2ca,
        1,
        layout_width.saturating_sub(450),
    );
    for (index, (label, x, width)) in [
        ("ALL", 292, 48),
        ("MODS", 346, 58),
        ("RESOURCE", 410, 82),
        ("DATA", 498, 54),
        ("SHADERS", 558, 72),
    ]
    .iter()
    .enumerate()
    {
        rect(
            buffer,
            w,
            h,
            *x,
            164,
            *width,
            24,
            if content_filter == index as u8 {
                0x00303b2b
            } else {
                0x0022262c
            },
        );
        text(
            buffer,
            w,
            h,
            *x + 6,
            172,
            label,
            if content_filter == index as u8 {
                0x00a8e66b
            } else {
                0x00bdc2ca
            },
            1,
        );
    }
    text_clipped(
        buffer,
        w,
        h,
        292,
        194,
        message,
        0x00bdc2ca,
        1,
        layout_width.saturating_sub(324),
    );
    rect(
        buffer,
        w,
        h,
        292,
        206,
        layout_width.saturating_sub(324),
        1,
        0x00363b42,
    );
    rect_square(
        buffer,
        w,
        h,
        292,
        214,
        layout_width.saturating_sub(324),
        CONTENT_LIST_TOP - 214,
        0x0022262c,
    );
    text(buffer, w, h, 305, 230, "PROJECT", 0x007b838d, 1);
    text_clipped(
        buffer,
        w,
        h,
        layout_width.saturating_sub(236),
        230,
        "ACTIONS",
        0x007b838d,
        1,
        100,
    );
    rect(
        buffer,
        w,
        h,
        292,
        CONTENT_LIST_TOP - 1,
        layout_width.saturating_sub(324),
        1,
        0x00343a42,
    );
    let visible_rows =
        (h.saturating_sub(CONTENT_LIST_TOP) / CONTENT_ROW_HEIGHT).max(1);
    let content_offset =
        (content_scroll_pixels / CONTENT_ROW_HEIGHT as f64).floor() as usize;
    let scroll_remainder = content_scroll_pixels
        .rem_euclid(CONTENT_ROW_HEIGHT as f64)
        .round() as usize;
    for (index, content_index) in visible_contents
        .iter()
        .skip(content_offset)
        .take(visible_rows + usize::from(scroll_remainder > 0))
        .enumerate()
    {
        let item = &contents[*content_index];
        let y = (CONTENT_LIST_TOP + index * CONTENT_ROW_HEIGHT)
            .saturating_sub(scroll_remainder);
        rect_square(
            buffer,
            w,
            h,
            292,
            y,
            layout_width.saturating_sub(324),
            CONTENT_ROW_HEIGHT,
            if (content_offset + index) % 2 == 0 {
                0x0022262c
            } else {
                0x001b1d22
            },
        );
        rect(
            buffer,
            w,
            h,
            292,
            y,
            layout_width.saturating_sub(324),
            1,
            0x00343a42,
        );
        let display_name = item
            .project
            .as_ref()
            .map(|project| project.title.as_str())
            .or_else(|| {
                item.embedded_metadata
                    .as_ref()
                    .and_then(|meta| meta.name.as_deref())
            })
            .unwrap_or(&item.file_name);
        let owner_name = item.owner.as_ref().map(|owner| owner.name.as_str());
        let version_number = item
            .version
            .as_ref()
            .map(|version| version.version_number.as_str());
        let secondary_text = match (owner_name, version_number) {
            (Some(owner), Some(version)) => format!("{owner}  ·  {version}"),
            (Some(owner), None) => owner.to_owned(),
            (None, Some(version)) => format!("Version {version}"),
            (None, None) if item.project.is_none() => "Local file".to_owned(),
            (None, None) => format!("{:?}", item.project_type),
        };
        let action_left = if item.locked {
            layout_width.saturating_sub(120)
        } else if item.has_update {
            layout_width.saturating_sub(336)
        } else {
            layout_width.saturating_sub(236)
        };
        if let Some(project) = item.project.as_ref()
            && let Some((icon_width, icon_height, icon)) =
                browse_icons.get(&project.id)
        {
            draw_scaled_image(
                buffer,
                w,
                h,
                *icon_width,
                *icon_height,
                icon,
                305,
                y + 13,
                48,
                48,
            );
        } else {
            rect(buffer, w, h, 305, y + 13, 48, 48, 0x00343a42);
        }
        let row_text_width = action_left.saturating_sub(374);
        text_clipped(
            buffer,
            w,
            h,
            365,
            y + 15,
            display_name,
            0x00f1f3f5,
            1,
            row_text_width,
        );
        text_clipped(
            buffer,
            w,
            h,
            365,
            y + 39,
            &secondary_text,
            0x00bdc2ca,
            1,
            row_text_width,
        );
        if item.locked {
            text(
                buffer,
                w,
                h,
                layout_width.saturating_sub(120),
                y + 18,
                "MANAGED",
                0x007b838d,
                1,
            );
        } else {
            if item.has_update {
                rect(
                    buffer,
                    w,
                    h,
                    layout_width.saturating_sub(336),
                    y + 20,
                    88,
                    34,
                    0x00303b2b,
                );
                text(
                    buffer,
                    w,
                    h,
                    layout_width.saturating_sub(324),
                    y + 31,
                    "UPDATE",
                    0x00a8e66b,
                    1,
                );
            }
            rect(
                buffer,
                w,
                h,
                layout_width.saturating_sub(236),
                y + 20,
                88,
                34,
                0x003b302b,
            );
            text(
                buffer,
                w,
                h,
                layout_width.saturating_sub(224),
                y + 31,
                "REMOVE",
                0x00e68a6b,
                1,
            );
            let toggle_color =
                if item.enabled { 0x003b302b } else { 0x00303b2b };
            rect(
                buffer,
                w,
                h,
                layout_width.saturating_sub(136),
                y + 20,
                108,
                34,
                toggle_color,
            );
            text(
                buffer,
                w,
                h,
                layout_width.saturating_sub(122),
                y + 31,
                if item.enabled { "DISABLE" } else { "ENABLE" },
                0x00a8e66b,
                1,
            );
        }
    }
}

pub(super) fn redraw_content_header(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    layout_width: usize,
    visible_contents: &[usize],
    contents: &[theseus::data::ContentItem],
    content_filter: u8,
    message: &str,
) {
    rect(
        buffer,
        w,
        h,
        248,
        146,
        layout_width.saturating_sub(248),
        68,
        0x001b1d22,
    );
    let content_count = format!(
        "{} OF {} CONTENT ITEMS",
        visible_contents.len(),
        contents.len()
    );
    text_clipped(
        buffer,
        w,
        h,
        292,
        150,
        &content_count,
        0x00bdc2ca,
        1,
        layout_width.saturating_sub(450),
    );
    rect(
        buffer,
        w,
        h,
        layout_width.saturating_sub(142),
        146,
        112,
        18,
        0x00303b2b,
    );
    text(
        buffer,
        w,
        h,
        layout_width.saturating_sub(128),
        151,
        "EXPORT PACK",
        0x00a8e66b,
        1,
    );
    for (index, (label, x, width)) in [
        ("ALL", 292, 48),
        ("MODS", 346, 58),
        ("RESOURCE", 410, 82),
        ("DATA", 498, 54),
        ("SHADERS", 558, 72),
    ]
    .iter()
    .enumerate()
    {
        rect(
            buffer,
            w,
            h,
            *x,
            164,
            *width,
            24,
            if content_filter == index as u8 {
                0x00303b2b
            } else {
                0x0022262c
            },
        );
        text(
            buffer,
            w,
            h,
            *x + 6,
            172,
            label,
            if content_filter == index as u8 {
                0x00a8e66b
            } else {
                0x00bdc2ca
            },
            1,
        );
    }
    text_clipped(
        buffer,
        w,
        h,
        292,
        194,
        message,
        0x00bdc2ca,
        1,
        layout_width.saturating_sub(324),
    );
    rect(
        buffer,
        w,
        h,
        292,
        206,
        layout_width.saturating_sub(324),
        1,
        0x00363b42,
    );
}
