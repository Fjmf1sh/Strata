use super::*;

mod accounts_panel;
mod browse_panel;
mod content_panel;
mod friends_panel;
mod java_panel;
mod settings_panel;
mod sidebar_panel;
pub(super) use sidebar_panel::{
    project_sidebar_chip_action_at, project_sidebar_content_height,
    project_sidebar_link_at,
};
mod skin_panel;

fn draw_log_line_with_search_highlight(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    line: &str,
    query: &str,
    max_width: usize,
) {
    let query = query.trim().to_lowercase();
    let lower_line = line.to_lowercase();
    if query.is_empty() || lower_line.len() != line.len() {
        text_clipped(
            buffer, width, height, x, y, line, 0x00bdc2ca, 1, max_width,
        );
        return;
    }

    let mut cursor = 0;
    let mut draw_x = x;
    while let Some(relative_match) = lower_line[cursor..].find(&query) {
        let match_start = cursor + relative_match;
        let match_end = match_start + query.len();
        let prefix = &line[cursor..match_start];
        let prefix_width = render::measure_text_width(prefix, 1);
        if prefix_width > 0 {
            text_clipped(
                buffer,
                width,
                height,
                draw_x,
                y,
                prefix,
                0x00bdc2ca,
                1,
                max_width.saturating_sub(draw_x.saturating_sub(x)),
            );
        }
        draw_x = draw_x.saturating_add(prefix_width);
        let matched = &line[match_start..match_end];
        let match_width = render::measure_text_width(matched, 1);
        if draw_x.saturating_sub(x) >= max_width {
            break;
        }
        rounded_rect(
            buffer,
            width,
            height,
            draw_x,
            y.saturating_sub(1),
            match_width.min(max_width.saturating_sub(draw_x - x)),
            12,
            2,
            0x00ffd60a,
        );
        text_clipped(
            buffer,
            width,
            height,
            draw_x,
            y,
            matched,
            0x00000000,
            1,
            max_width.saturating_sub(draw_x - x),
        );
        draw_x = draw_x.saturating_add(match_width);
        cursor = match_end;
    }
    if cursor < line.len() && draw_x.saturating_sub(x) < max_width {
        text_clipped(
            buffer,
            width,
            height,
            draw_x,
            y,
            &line[cursor..],
            0x00bdc2ca,
            1,
            max_width.saturating_sub(draw_x - x),
        );
    }
}

fn draw_native_log_filter_pills(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    top: usize,
    present_levels: u8,
    active_filter: u8,
) {
    for (level, bounds) in
        instance_log_filter_bounds(width, top, present_levels)
    {
        let active = if level == 0 {
            active_filter == 0
        } else {
            active_filter & level != 0
        };
        rounded_rect(
            buffer,
            width,
            height,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            12,
            if active { 0x00303b2b } else { 0x0022262c },
        );
        let label = match level {
            0 => "ALL",
            LOG_LEVEL_ERROR => "ERROR",
            LOG_LEVEL_WARN => "WARN",
            LOG_LEVEL_INFO => "INFO",
            LOG_LEVEL_DEBUG => "DEBUG",
            LOG_LEVEL_TRACE => "TRACE",
            _ => continue,
        };
        text(
            buffer,
            width,
            height,
            bounds.x + 9,
            bounds.y + 10,
            label,
            if active { 0x00a8e66b } else { 0x00bdc2ca },
            1,
        );
    }
}

fn draw_library_card_action_icon(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    action: NativeLibraryCardAction,
    foreground: u32,
) {
    let center_x = x + 20;
    let center_y = y + 20;
    match action {
        NativeLibraryCardAction::Play => render::draw_navigation_icon(
            buffer, width, height, center_x, center_y, 7, foreground,
        ),
        NativeLibraryCardAction::Stop => rect(
            buffer,
            width,
            height,
            center_x - 6,
            center_y - 6,
            12,
            12,
            foreground,
        ),
        NativeLibraryCardAction::Repair => {
            render::draw_line(
                buffer,
                width,
                height,
                (center_x as isize, (center_y - 7) as isize),
                (center_x as isize, (center_y + 4) as isize),
                foreground,
            );
            render::draw_line(
                buffer,
                width,
                height,
                ((center_x - 5) as isize, (center_y - 1) as isize),
                (center_x as isize, (center_y + 4) as isize),
                foreground,
            );
            render::draw_line(
                buffer,
                width,
                height,
                ((center_x + 5) as isize, (center_y - 1) as isize),
                (center_x as isize, (center_y + 4) as isize),
                foreground,
            );
            render::draw_line(
                buffer,
                width,
                height,
                ((center_x - 8) as isize, (center_y + 8) as isize),
                ((center_x + 8) as isize, (center_y + 8) as isize),
                foreground,
            );
        }
        NativeLibraryCardAction::Installing | NativeLibraryCardAction::None => {
        }
    }
}

fn draw_library_selection_indicator(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    bounds: UiBounds,
    selected: bool,
) {
    let radius = bounds.width / 2;
    let outer = if selected { 0x00a8e66b } else { 0x00e4e9ee };
    rounded_rect(
        buffer,
        width,
        height,
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
        radius,
        outer,
    );
    if !selected {
        rounded_rect(
            buffer,
            width,
            height,
            bounds.x + 2,
            bounds.y + 2,
            bounds.width.saturating_sub(4),
            bounds.height.saturating_sub(4),
            radius.saturating_sub(2),
            0x0022262c,
        );
        return;
    }
    let left = bounds.x as isize;
    let top = bounds.y as isize;
    let check_color =
        render::contrasting_text_color(render::themed_color(0x00a8e66b));
    render::draw_line(
        buffer,
        width,
        height,
        (left + 5, top + (bounds.height / 2) as isize),
        (
            left + (bounds.width / 2) as isize - 1,
            top + bounds.height as isize - 6,
        ),
        check_color,
    );
    render::draw_line(
        buffer,
        width,
        height,
        (
            left + (bounds.width / 2) as isize - 1,
            top + bounds.height as isize - 6,
        ),
        (left + bounds.width as isize - 4, top + 5),
        check_color,
    );
}

fn draw_library_option_menu(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    bounds: UiBounds,
    options: &[&str],
    selected: usize,
    cursor: (f64, f64),
) {
    rounded_rect(
        buffer,
        width,
        height,
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
        12,
        0x00343a42,
    );
    rounded_rect(
        buffer,
        width,
        height,
        bounds.x + 1,
        bounds.y + 1,
        bounds.width.saturating_sub(2),
        bounds.height.saturating_sub(2),
        11,
        0x0022262c,
    );
    for (index, label) in options.iter().enumerate() {
        let row = library_dropdown_row(bounds, index);
        let active = row.contains(cursor.0, cursor.1);
        if index == selected || active {
            rounded_rect(
                buffer,
                width,
                height,
                row.x,
                row.y,
                row.width,
                row.height,
                7,
                if index == selected {
                    0x00303b2b
                } else {
                    0x00343a42
                },
            );
        }
        text_clipped(
            buffer,
            width,
            height,
            row.x + 10,
            row.y + 9,
            label,
            if index == selected {
                0x00a8e66b
            } else {
                0x00f1f3f5
            },
            1,
            row.width.saturating_sub(36),
        );
        if index == selected {
            text(
                buffer,
                width,
                height,
                row.x + row.width.saturating_sub(22),
                row.y + 8,
                "✓",
                0x00a8e66b,
                1,
            );
        }
    }
}

fn ensure_markdown_newline(text: &mut String) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
}

fn markdown_to_plain_text(markdown: &str) -> String {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    let mut text = String::new();
    let mut ordered_lists = Vec::<Option<u64>>::new();
    let mut in_code_block = false;
    let mut in_image = false;
    let mut image_index = 0usize;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Image { .. }) => {
                ensure_markdown_newline(&mut text);
                text.push_str(&format!("\u{1f}image:{image_index}\u{1f}\n"));
                image_index += 1;
                in_image = true;
            }
            Event::End(TagEnd::Image) => {
                ensure_markdown_newline(&mut text);
                in_image = false;
            }
            Event::Start(Tag::Paragraph) => ensure_markdown_newline(&mut text),
            Event::End(TagEnd::Paragraph) => {
                ensure_markdown_newline(&mut text);
                text.push('\n');
            }
            Event::Start(Tag::Heading { level, .. }) => {
                ensure_markdown_newline(&mut text);
                let depth = level as usize;
                text.push_str(&"#".repeat(depth));
                text.push(' ');
            }
            Event::End(TagEnd::Heading(_)) => {
                ensure_markdown_newline(&mut text);
                text.push('\n');
            }
            Event::Start(Tag::List(start)) => {
                ensure_markdown_newline(&mut text);
                ordered_lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                ensure_markdown_newline(&mut text);
                ordered_lists.pop();
            }
            Event::Start(Tag::Item) => {
                ensure_markdown_newline(&mut text);
                match ordered_lists.last_mut() {
                    Some(Some(index)) => {
                        text.push_str(&format!("{index}. "));
                        *index = index.saturating_add(1);
                    }
                    _ => text.push_str("• "),
                }
            }
            Event::End(TagEnd::Item) => ensure_markdown_newline(&mut text),
            Event::Start(Tag::BlockQuote(_)) => {
                ensure_markdown_newline(&mut text);
                text.push_str("│ ");
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                ensure_markdown_newline(&mut text)
            }
            Event::Start(Tag::CodeBlock(_)) => {
                ensure_markdown_newline(&mut text);
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                ensure_markdown_newline(&mut text);
                text.push('\n');
                in_code_block = false;
            }
            Event::Start(Tag::Table(_)) => ensure_markdown_newline(&mut text),
            Event::Start(Tag::TableHead | Tag::TableRow) => {
                ensure_markdown_newline(&mut text)
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                ensure_markdown_newline(&mut text)
            }
            Event::Start(Tag::TableCell) => {
                if !text.is_empty()
                    && !text.ends_with('\n')
                    && !text.ends_with(' ')
                {
                    text.push_str(" | ");
                }
            }
            Event::Text(_) if in_image => {}
            Event::Text(value) if in_code_block => {
                for line in value.lines() {
                    text.push_str("▌ ");
                    text.push_str(line);
                    text.push('\n');
                }
            }
            Event::Text(value) => text.push_str(&value),
            Event::Html(_) | Event::InlineHtml(_) => {
                ensure_markdown_newline(&mut text)
            }
            Event::Code(value)
            | Event::InlineMath(value)
            | Event::DisplayMath(value) => text.push_str(&value),
            Event::FootnoteReference(label) => {
                text.push_str(&format!("[^{label}]"))
            }
            Event::SoftBreak => text.push(' '),
            Event::HardBreak => text.push('\n'),
            Event::Rule => {
                ensure_markdown_newline(&mut text);
                text.push_str("────────────────────\n");
            }
            Event::TaskListMarker(checked) => {
                text.push_str(if checked { "[x] " } else { "[ ] " });
            }
            Event::End(_) | Event::Start(_) => {}
        }
    }
    text
}

fn redraw_screenshot_grid_header(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    library_width: usize,
    search: &str,
    group_mode: u8,
    sort_mode: u8,
    selected_count: usize,
    screenshot_count: usize,
    message: &str,
) {
    rect(
        buffer,
        w,
        h,
        248,
        56,
        library_width.saturating_sub(248),
        174,
        0x001b1d22,
    );
    text(buffer, w, h, 292, 84, "BACK TO LIBRARY", 0x00a8e66b, 1);
    rect(
        buffer,
        w,
        h,
        292,
        146,
        library_width.saturating_sub(562),
        28,
        0x001a1d21,
    );
    text(
        buffer,
        w,
        h,
        302,
        156,
        if search.is_empty() {
            "SEARCH SCREENSHOTS"
        } else {
            search
        },
        if search.is_empty() {
            0x007b838d
        } else {
            0x00f1f3f5
        },
        1,
    );
    if group_mode % 4 == 2 {
        rect(
            buffer,
            w,
            h,
            library_width.saturating_sub(252),
            146,
            110,
            28,
            0x00303b2b,
        );
        text(
            buffer,
            w,
            h,
            library_width.saturating_sub(244),
            156,
            "NEW GROUP",
            0x00a8e66b,
            1,
        );
    }
    rect(
        buffer,
        w,
        h,
        library_width.saturating_sub(142),
        146,
        112,
        28,
        0x0022262c,
    );
    text(
        buffer,
        w,
        h,
        library_width.saturating_sub(128),
        156,
        "REFRESH",
        0x00a8e66b,
        1,
    );
    let sort = match sort_mode % 4 {
        1 => "SORT: OLDEST",
        2 => "SORT: NAME A-Z",
        3 => "SORT: NAME Z-A",
        _ => "SORT: NEWEST",
    };
    let group = match group_mode % 4 {
        1 => "GROUP: INSTANCE",
        2 => "GROUP: CUSTOM",
        3 => "GROUP: NONE",
        _ => "GROUP: DATE",
    };
    rect(buffer, w, h, 292, 180, 140, 28, 0x0022262c);
    text(buffer, w, h, 300, 190, sort, 0x00a8e66b, 1);
    rect(buffer, w, h, 440, 180, 140, 28, 0x0022262c);
    text(buffer, w, h, 448, 190, group, 0x00a8e66b, 1);
    if group_mode % 4 == 2 {
        rect(buffer, w, h, 588, 180, 52, 28, 0x0022262c);
        text(buffer, w, h, 592, 190, "MOVE", 0x00a8e66b, 1);
    }
    if selected_count > 1 {
        rect(
            buffer,
            w,
            h,
            library_width.saturating_sub(116),
            180,
            52,
            28,
            0x0022262c,
        );
        text(
            buffer,
            w,
            h,
            library_width.saturating_sub(112),
            190,
            "ZIP",
            0x00a8e66b,
            1,
        );
        rect(
            buffer,
            w,
            h,
            library_width.saturating_sub(60),
            180,
            52,
            28,
            0x003b302b,
        );
        text(
            buffer,
            w,
            h,
            library_width.saturating_sub(56),
            190,
            &format!("DEL {selected_count}"),
            0x00e68a6b,
            1,
        );
    }
    text_clipped(
        buffer,
        w,
        h,
        292,
        216,
        message,
        0x007b838d,
        1,
        library_width.saturating_sub(464),
    );
    text_clipped(
        buffer,
        w,
        h,
        library_width.saturating_sub(160),
        216,
        &if selected_count > 1 {
            format!("{selected_count} SELECTED")
        } else {
            format!("{screenshot_count} SCREENSHOTS")
        },
        0x00bdc2ca,
        1,
        140,
    );
}

fn redraw_worlds_header(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    library_width: usize,
    message: &str,
) {
    rect(
        buffer,
        w,
        h,
        248,
        140,
        library_width.saturating_sub(248),
        50,
        0x001b1d22,
    );
    text_clipped(
        buffer,
        w,
        h,
        292,
        156,
        message,
        0x007b838d,
        1,
        library_width.saturating_sub(564),
    );
    for (x, label) in [
        (w.saturating_sub(260), "ADD SERVER"),
        (w.saturating_sub(142), "REFRESH"),
    ] {
        rect(buffer, w, h, x, 146, 112, 28, 0x0022262c);
        text(
            buffer,
            w,
            h,
            x + if label == "ADD SERVER" { 15 } else { 14 },
            156,
            label,
            0x00a8e66b,
            1,
        );
    }
    rect(buffer, w, h, 292, 182, w.saturating_sub(324), 1, 0x00363b42);
}

fn redraw_file_list_header(
    buffer: &mut [u32],
    w: usize,
    layout_width: usize,
    h: usize,
    file_path: &str,
    message: &str,
) {
    rect(
        buffer,
        w,
        h,
        248,
        146,
        layout_width.saturating_sub(248),
        56,
        0x001b1d22,
    );
    let path = if file_path.is_empty() {
        "/".to_owned()
    } else {
        format!("/{file_path}")
    };
    text_clipped(
        buffer,
        w,
        h,
        292,
        156,
        &path,
        0x00bdc2ca,
        1,
        if file_path.is_empty() {
            layout_width.saturating_sub(324)
        } else {
            layout_width.saturating_sub(408)
        },
    );
    if !file_path.is_empty() {
        rect(
            buffer,
            w,
            h,
            layout_width.saturating_sub(104),
            146,
            80,
            28,
            0x0022262c,
        );
        text(
            buffer,
            w,
            h,
            layout_width.saturating_sub(88),
            156,
            "UP",
            0x00a8e66b,
            1,
        );
    }
    text_clipped(
        buffer,
        w,
        h,
        292,
        178,
        message,
        0x007b838d,
        1,
        layout_width.saturating_sub(324),
    );
    rect(
        buffer,
        w,
        h,
        292,
        194,
        layout_width.saturating_sub(324),
        1,
        0x00363b42,
    );
}

pub(super) fn wrap_browse_body(body: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut lines = Vec::new();
    let plain_text = markdown_to_plain_text(body);
    for paragraph in plain_text.lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut line_chars = 0usize;
        for word in paragraph.split_whitespace() {
            let word_len = word.chars().count();
            if line_chars > 0 && line_chars + 1 + word_len > max_chars {
                lines.push(std::mem::take(&mut line));
                line_chars = 0;
            }
            if word_len > max_chars {
                for character in word.chars() {
                    if line_chars >= max_chars {
                        lines.push(std::mem::take(&mut line));
                        line_chars = 0;
                    }
                    line.push(character);
                    line_chars += 1;
                }
            } else {
                if line_chars > 0 {
                    line.push(' ');
                    line_chars += 1;
                }
                line.push_str(word);
                line_chars += word_len;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

pub(super) fn wrap_browse_body_with_links(
    body: &str,
    max_chars: usize,
    project_id: &str,
) -> (
    Vec<String>,
    Vec<BrowseDescriptionLink>,
    Vec<BrowseDescriptionStyle>,
    Vec<BrowseDescriptionImage>,
) {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    let mut markdown_links = Vec::<(String, String)>::new();
    let mut active_link: Option<(String, String)> = None;
    for event in Parser::new(body) {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                active_link = Some((String::new(), dest_url.to_string()));
            }
            Event::Text(value) | Event::Code(value) => {
                if let Some((label, _)) = active_link.as_mut() {
                    label.push_str(&value);
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some(link) = active_link.take() {
                    markdown_links.push(link);
                }
            }
            _ => {}
        }
    }

    let lines = wrap_browse_body(body, max_chars);
    let mut parsed_images = Vec::<(String, String)>::new();
    let mut active_image: Option<(String, String)> = None;
    for event in Parser::new(body) {
        match event {
            Event::Start(Tag::Image { dest_url, .. }) => {
                active_image = Some((dest_url.to_string(), String::new()));
            }
            Event::Text(value) if active_image.is_some() => {
                if let Some((_, alt)) = active_image.as_mut() {
                    alt.push_str(&value);
                }
            }
            Event::End(TagEnd::Image) => {
                if let Some(image) = active_image.take() {
                    parsed_images.push(image);
                }
            }
            _ => {}
        }
    }
    let images = parsed_images
        .into_iter()
        .enumerate()
        .filter_map(|(index, (url, alt))| {
            let marker = format!("\u{1f}image:{index}\u{1f}");
            let line_index = lines.iter().position(|line| line == &marker)?;
            Some(BrowseDescriptionImage {
                line_index,
                cache_key: format!("description-image:{project_id}:{index}"),
                url,
                alt,
            })
        })
        .collect();
    let mut links = Vec::new();
    let mut styles = Vec::new();
    let mut search_from_char = 0usize;
    for (label, url) in markdown_links {
        if label.is_empty()
            || !(url.starts_with("https://")
                || url.starts_with("http://")
                || url.starts_with("mailto:"))
        {
            continue;
        }
        let mut normalized = String::new();
        let mut positions = Vec::<Option<(usize, usize)>>::new();
        let mut previous_nonempty = false;
        for (line_index, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                if !normalized.ends_with(' ') {
                    normalized.push(' ');
                    positions.push(None);
                }
                previous_nonempty = false;
                continue;
            }
            if previous_nonempty {
                normalized.push(' ');
                positions.push(None);
            }
            let mut display = line.as_str();
            if let Some((hashes, body)) = line.split_once(' ')
                && hashes.starts_with('#')
                && hashes.chars().all(|character| character == '#')
            {
                display = body;
            } else if let Some(body) = line.strip_prefix("│ ") {
                display = body;
            } else if let Some(body) = line.strip_prefix("▌ ") {
                display = body;
            }
            for (char_index, character) in display.chars().enumerate() {
                normalized.push(character);
                positions.push(Some((line_index, char_index)));
            }
            previous_nonempty = true;
        }
        let search_byte = normalized
            .char_indices()
            .nth(search_from_char)
            .map_or(normalized.len(), |(byte, _)| byte);
        if let Some(relative_byte) = normalized[search_byte..].find(&label) {
            let start_byte = search_byte + relative_byte;
            let start_chars = normalized[..start_byte].chars().count();
            let end_chars = start_chars + label.chars().count();
            let mut active_run: Option<(usize, usize, usize)> = None;
            for position in positions
                .iter()
                .skip(start_chars)
                .take(end_chars.saturating_sub(start_chars))
            {
                let Some((line_index, char_index)) = position else {
                    if let Some((line_index, start_char, end_char)) =
                        active_run.take()
                    {
                        let (text_x, scale) =
                            browse_body_link_geometry(&lines[line_index]);
                        links.push(BrowseDescriptionLink {
                            line_index,
                            start_char,
                            end_char,
                            text_x,
                            scale,
                            url: url.clone(),
                        });
                    }
                    continue;
                };
                if let Some((run_line, _, run_end)) = active_run.as_mut()
                    && *run_line == *line_index
                    && *run_end == *char_index
                {
                    *run_end += 1;
                } else {
                    if let Some((line_index, start_char, end_char)) =
                        active_run.take()
                    {
                        let (text_x, scale) =
                            browse_body_link_geometry(&lines[line_index]);
                        links.push(BrowseDescriptionLink {
                            line_index,
                            start_char,
                            end_char,
                            text_x,
                            scale,
                            url: url.clone(),
                        });
                    }
                    active_run =
                        Some((*line_index, *char_index, *char_index + 1));
                }
            }
            if let Some((line_index, start_char, end_char)) = active_run {
                let (text_x, scale) =
                    browse_body_link_geometry(&lines[line_index]);
                links.push(BrowseDescriptionLink {
                    line_index,
                    start_char,
                    end_char,
                    text_x,
                    scale,
                    url: url.clone(),
                });
            }
            search_from_char = end_chars;
        }
    }
    let mut markdown_styles = Vec::<(String, bool, bool)>::new();
    let (mut bold_depth, mut italic_depth) = (0usize, 0usize);
    let mut previous_style = None;
    for event in Parser::new(body) {
        match event {
            Event::Start(Tag::Strong) => {
                bold_depth += 1;
                previous_style = None;
            }
            Event::End(TagEnd::Strong) => {
                bold_depth = bold_depth.saturating_sub(1);
                previous_style = None;
            }
            Event::Start(Tag::Emphasis) => {
                italic_depth += 1;
                previous_style = None;
            }
            Event::End(TagEnd::Emphasis) => {
                italic_depth = italic_depth.saturating_sub(1);
                previous_style = None;
            }
            Event::Text(value) if bold_depth > 0 || italic_depth > 0 => {
                let bold = bold_depth > 0;
                let italic = italic_depth > 0;
                if previous_style == Some((bold, italic))
                    && let Some((last, _, _)) = markdown_styles.last_mut()
                {
                    last.push_str(&value);
                } else {
                    markdown_styles.push((value.to_string(), bold, italic));
                }
                previous_style = Some((bold, italic));
            }
            Event::Text(_) | Event::Code(_) => {
                previous_style = None;
            }
            _ => previous_style = None,
        }
    }
    let mut search_char = 0usize;
    for (label, bold, italic) in markdown_styles {
        let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
        if label.is_empty() {
            continue;
        }
        if let Some((spans, end_char)) =
            browse_body_label_spans(&lines, &label, search_char)
        {
            for (line_index, start_char, end_char) in spans {
                styles.push(BrowseDescriptionStyle {
                    line_index,
                    start_char,
                    end_char,
                    bold,
                    italic,
                });
            }
            search_char = end_char;
        }
    }
    (lines, links, styles, images)
}

fn browse_body_link_display(line: &str) -> &str {
    if let Some(body) = line.strip_prefix("│ ") {
        body
    } else if let Some(body) = line.strip_prefix("▌ ") {
        body
    } else if let Some((hashes, body)) = line.split_once(' ')
        && hashes.starts_with('#')
        && hashes.chars().all(|character| character == '#')
    {
        body
    } else {
        line
    }
}

fn browse_body_label_spans(
    lines: &[String],
    label: &str,
    search_from_char: usize,
) -> Option<(Vec<(usize, usize, usize)>, usize)> {
    let mut normalized = String::new();
    let mut positions = Vec::<Option<(usize, usize)>>::new();
    let mut previous_nonempty = false;
    for (line_index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            if !normalized.ends_with(' ') {
                normalized.push(' ');
                positions.push(None);
            }
            previous_nonempty = false;
            continue;
        }
        if previous_nonempty {
            normalized.push(' ');
            positions.push(None);
        }
        for (char_index, character) in
            browse_body_link_display(line).chars().enumerate()
        {
            normalized.push(character);
            positions.push(Some((line_index, char_index)));
        }
        previous_nonempty = true;
    }
    let search_byte = normalized
        .char_indices()
        .nth(search_from_char)
        .map_or(normalized.len(), |(byte, _)| byte);
    let relative_byte = normalized[search_byte..].find(label)?;
    let start_byte = search_byte + relative_byte;
    let start_char = normalized[..start_byte].chars().count();
    let end_char = start_char + label.chars().count();
    let mut spans = Vec::new();
    let mut active_run: Option<(usize, usize, usize)> = None;
    for position in positions
        .iter()
        .skip(start_char)
        .take(end_char.saturating_sub(start_char))
    {
        let Some((line_index, char_index)) = position else {
            if let Some(run) = active_run.take() {
                spans.push(run);
            }
            continue;
        };
        if let Some((run_line, _, run_end)) = active_run.as_mut()
            && *run_line == *line_index
            && *run_end == *char_index
        {
            *run_end += 1;
        } else {
            if let Some(run) = active_run.take() {
                spans.push(run);
            }
            active_run = Some((*line_index, *char_index, *char_index + 1));
        }
    }
    if let Some(run) = active_run {
        spans.push(run);
    }
    Some((spans, end_char))
}

fn browse_body_link_geometry(line: &str) -> (usize, usize) {
    if line.starts_with("│ ") || line.starts_with("▌ ") {
        (304, 1)
    } else if let Some((hashes, _)) = line.split_once(' ')
        && hashes.starts_with('#')
        && hashes.chars().all(|character| character == '#')
    {
        (292, if hashes.len() <= 2 { 2 } else { 1 })
    } else {
        (292, 1)
    }
}

pub(super) fn browse_body_line_height(line: &str) -> usize {
    if line.starts_with("\u{1f}image:") {
        220
    } else if line.trim().is_empty() {
        10
    } else if line.starts_with("# ") {
        30
    } else if line.starts_with("## ") {
        26
    } else if line.starts_with("### ")
        || line.starts_with("#### ")
        || line.starts_with("##### ")
        || line.starts_with("###### ")
    {
        22
    } else {
        20
    }
}

pub(super) fn browse_body_height(lines: &[String]) -> usize {
    lines.iter().map(|line| browse_body_line_height(line)).sum()
}

pub(super) fn browse_body_content_offset(
    lines: &[String],
    line_index: usize,
) -> usize {
    lines
        .iter()
        .take(line_index)
        .map(|line| browse_body_line_height(line))
        .sum()
}

pub(super) fn browse_gallery_grid_metrics(
    layout_width: usize,
    image_count: usize,
) -> (usize, usize, usize, usize) {
    let content_width = layout_width.saturating_sub(324);
    let columns = ((content_width + 16) / 336).max(1);
    let card_width =
        content_width.saturating_sub(columns.saturating_sub(1) * 16) / columns;
    let card_height = card_width / 2 + 118;
    let rows = image_count.div_ceil(columns);
    let total_height = rows
        .saturating_mul(card_height)
        .saturating_add(rows.saturating_sub(1) * 16);
    (columns, card_width, card_height, total_height)
}

fn draw_browse_gallery_viewer(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    project: &BrowseProject,
    details: &BrowseProjectDetails,
    icons: &HashMap<String, (u32, u32, Vec<u32>)>,
    index: usize,
    zoom: f32,
) {
    rect(
        buffer,
        width,
        height,
        0,
        64,
        width,
        height.saturating_sub(64),
        0x000b0b0d,
    );
    let Some(image) = details.gallery.get(index) else {
        return;
    };
    if let Some(title) = image.title.as_deref() {
        text_clipped(
            buffer,
            width,
            height,
            32,
            82,
            title,
            0x00f1f3f5,
            1,
            width.saturating_sub(64),
        );
    }
    if let Some(description) = image.description.as_deref() {
        text_clipped(
            buffer,
            width,
            height,
            32,
            104,
            description,
            0x00bdc2ca,
            1,
            width.saturating_sub(64),
        );
    }
    let image_x = 32;
    let image_y = 140;
    let image_width = width.saturating_sub(64);
    let image_height = height.saturating_sub(232).max(64);
    let key = format!("gallery-full:{}:{index}", project.id);
    if let Some(image_data) = icons.get(&key) {
        crate::images::draw_contained_image_zoomed(
            buffer,
            width,
            height,
            image_data.0,
            image_data.1,
            &image_data.2,
            image_x,
            image_y,
            image_width,
            image_height,
            zoom,
        );
    } else {
        rounded_rect(
            buffer,
            width,
            height,
            image_x,
            image_y,
            image_width,
            image_height,
            12,
            0x0022262c,
        );
        text(
            buffer,
            width,
            height,
            image_x + 14,
            image_y + 16,
            "LOADING IMAGE…",
            0x007b838d,
            1,
        );
    }
    let toolbar_width = 444;
    let toolbar_x = width.saturating_sub(toolbar_width) / 2;
    let toolbar_y = height.saturating_sub(72);
    rounded_rect(
        buffer,
        width,
        height,
        toolbar_x,
        toolbar_y,
        toolbar_width,
        48,
        16,
        0x0022262c,
    );
    let index_label = format!("{} / {}", index + 1, details.gallery.len());
    let zoom_label = format!("{}%", (zoom * 100.0).round() as u32);
    for (offset, button_width, label, enabled) in [
        (12, 36, "<", details.gallery.len() > 1),
        (56, 56, index_label.as_str(), true),
        (120, 36, ">", details.gallery.len() > 1),
        (168, 36, "-", zoom > 1.001),
        (212, 64, zoom_label.as_str(), true),
        (284, 36, "+", zoom < 5.0),
        (340, 36, "Open", true),
        (384, 36, "X", true),
    ] {
        let x = toolbar_x + offset;
        rounded_rect(
            buffer,
            width,
            height,
            x,
            toolbar_y + 8,
            button_width,
            32,
            12,
            if enabled { 0x00343a42 } else { 0x00202429 },
        );
        text_clipped(
            buffer,
            width,
            height,
            x + 6,
            toolbar_y + 18,
            label,
            if enabled { 0x00bdc2ca } else { 0x007b838d },
            1,
            button_width.saturating_sub(12),
        );
    }
    rect(
        buffer,
        width,
        height,
        toolbar_x + 160,
        toolbar_y + 12,
        1,
        24,
        0x00484e57,
    );
    rect(
        buffer,
        width,
        height,
        toolbar_x + 332,
        toolbar_y + 12,
        1,
        24,
        0x00484e57,
    );
}

pub(super) fn browse_gallery_item_at(
    layout_width: usize,
    image_count: usize,
    scroll_pixels: f64,
    x: f64,
    y: f64,
) -> Option<usize> {
    let (columns, card_width, card_height, _) =
        browse_gallery_grid_metrics(layout_width, image_count);
    let stride_y = card_height.saturating_add(16);
    let stride_x = card_width.saturating_add(16);
    if x < 292.0 || y < 442.0 {
        return None;
    }
    let local_y = (y - 442.0 + scroll_pixels).max(0.0) as usize;
    let row = local_y / stride_y.max(1);
    if local_y % stride_y.max(1) >= card_height {
        return None;
    }
    let local_x = (x - 292.0) as usize;
    let column = local_x / stride_x.max(1);
    if column >= columns || local_x % stride_x.max(1) >= card_width {
        return None;
    }
    let index = row.saturating_mul(columns).saturating_add(column);
    (index < image_count).then_some(index)
}

impl Launcher {
    pub(super) fn draw(&mut self) {
        if self.global_browse {
            self.instance_tab = 3;
        }
        render::set_clip_rect(None);
        render::set_page_horizontal_layout(None);
        render::set_page_vertical_layout(None, None);
        render::clear_text_commands();
        self.schedule_message_dismissal();
        self.sync_navigation_history();
        let Some(window) = self.window.clone() else {
            return;
        };
        let is_home_library = self.is_home_library();
        let visible_instances = if is_home_library {
            self.filtered_instance_indices()
        } else {
            Vec::new()
        };
        let initial_size = window.inner_size();
        let show_right_sidebar = self.should_show_right_sidebar();
        let log_delete_disabled = self.selected_log_delete_disabled();
        let instance_log_actions = self.selected_log.as_ref().and_then(|_| {
            instance_log_action_bounds(
                initial_size.width as usize,
                self.log_analysis.is_some(),
            )
        });
        let browse_filter_sidebar_visible =
            self.browse_filter_sidebar_visible();
        if browse_filter_sidebar_visible
            && self.ready
            && self.browse_filter_options_error.is_none()
        {
            self.load_browse_filter_options();
        }
        let sidebar_shortcuts =
            self.sidebar_instances(initial_size.height as usize);
        for (instance_id, _) in &sidebar_shortcuts {
            let icon_path = self
                .instance_cards
                .get(instance_id)
                .and_then(|card| card.icon_path.as_ref())
                .cloned();
            if let Some(icon_path) = icon_path {
                self.request_instance_icon(instance_id, &icon_path);
            }
        }
        let library_width = initial_size
            .width
            .saturating_sub(if show_right_sidebar { 300 } else { 0 })
            as usize;
        let initial_grid = self.library_grid_layout(library_width);
        let jump_in_items = if is_home_library && initial_grid.jump_in {
            self.jump_in_items()
        } else {
            Vec::new()
        };
        let jump_in_visible_count =
            self.jump_in_visible_count(library_width, jump_in_items.len());
        let jump_in_resize_handle = is_home_library
            .then(|| self.jump_in_resize_handle(library_width))
            .flatten();
        for item in jump_in_items.iter().take(jump_in_visible_count) {
            let NativeJumpInItem::World(recent_world) = item else {
                continue;
            };
            if let theseus::worlds::WorldDetails::Server { address, .. } =
                &recent_world.world.details
            {
                self.request_jump_in_server_status(address);
            }
            let Some(icon) = recent_world.world.icon.as_ref() else {
                continue;
            };
            let key = format!(
                "world:{}:{}",
                recent_world.instance_id,
                recent_world.world.world_id()
            );
            match icon {
                either::Either::Left(path) => {
                    let path = path.to_string_lossy().into_owned();
                    self.request_instance_icon(&key, &path);
                }
                either::Either::Right(url) => {
                    self.request_browse_icon(&key, url.as_str());
                }
            }
        }
        let library_grid_rows = if is_home_library {
            self.library_grid_rows(initial_grid.columns)
        } else {
            Vec::new()
        };
        let selected_icon_to_load =
            self.selected_instance
                .as_ref()
                .and_then(|(instance_id, _)| {
                    self.instance_cards
                        .get(instance_id)
                        .and_then(|card| card.icon_path.as_ref())
                        .map(|path| (instance_id.clone(), path.clone()))
                });
        if let Some((instance_id, icon_path)) = selected_icon_to_load {
            self.request_instance_icon(&instance_id, &icon_path);
        }
        if is_home_library
            && self.list_offset != self.library_scroll_offset_synced
        {
            let row_prefix = Self::library_scroll_prefix(
                &library_grid_rows,
                initial_grid,
                self.list_offset,
            );
            self.library_scroll_target_pixels =
                if self.home_page_scroll_active() {
                    initial_grid
                        .cards_top
                        .saturating_add(row_prefix)
                        .saturating_sub(64) as f64
                } else {
                    row_prefix as f64
                };
            self.library_scroll_offset_synced = self.list_offset;
        }
        let library_content_height: usize = library_grid_rows
            .iter()
            .map(|row| library_grid_row_height(row, initial_grid))
            .sum();
        let page_height = initial_size.height as usize;
        let library_scroll_max = if self.home_page_scroll_active() {
            initial_grid
                .cards_top
                .saturating_add(library_content_height)
                .saturating_sub(page_height.saturating_sub(24))
                as f64
        } else {
            let library_view_height = page_height
                .saturating_sub(initial_grid.cards_top.min(page_height));
            library_content_height.saturating_sub(library_view_height) as f64
        };
        if is_home_library {
            self.library_scroll_target_pixels = self
                .library_scroll_target_pixels
                .clamp(0.0, library_scroll_max);
            self.library_scroll_pixels =
                self.library_scroll_pixels.clamp(0.0, library_scroll_max);
        }
        let browse_results_visible = self.is_browse_results_page();
        let browse_results_layout =
            BrowseResultsLayout::new(library_width, page_height);
        if browse_results_visible {
            let browse_scroll_max =
                browse_results_layout.max_scroll(self.browse_results.len());
            self.browse_scroll_target_pixels = self
                .browse_scroll_target_pixels
                .clamp(0.0, browse_scroll_max);
            self.browse_scroll_pixels =
                self.browse_scroll_pixels.clamp(0.0, browse_scroll_max);
        }
        let library_game_version_options = if self.library_filter_open {
            self.library_game_version_options()
        } else {
            Vec::new()
        };
        let screenshot_order =
            if self.instance_tab == 8 && !self.screenshot_viewer {
                self.screenshot_order()
            } else {
                Vec::new()
            };
        let selected_screenshot_count =
            self.selected_screenshot_indices().len();
        let all_selected_are_members =
            self.selected_group.as_deref().is_some_and(|group_id| {
                self.selected_group_action_removes_membership(group_id)
            });
        if self.instance_tab == 8 && !self.screenshot_viewer {
            let size = window.inner_size();
            self.ensure_visible_screenshot_thumbnails(
                &screenshot_order,
                library_width,
                size.height as usize,
            );
        }
        if is_home_library {
            let lead =
                initial_grid.card_height.saturating_add(initial_grid.gap) * 2;
            let prefetch_ranges = library_icon_prefetch_ranges(
                self.library_scroll_pixels.max(0.0) as usize,
                self.library_scroll_target_pixels.max(0.0) as usize,
                page_height,
                lead,
            );
            let mut row_y = initial_grid.cards_top;
            let icons_to_load: Vec<(String, String)> = library_grid_rows
                .iter()
                .map_while(|row| {
                    let top = row_y;
                    row_y += library_grid_row_height(row, initial_grid);
                    Some((top, row_y, row))
                })
                .filter_map(|(top, bottom, row)| {
                    prefetch_ranges
                        .iter()
                        .any(|(start, end)| bottom > *start && top < *end)
                        .then_some(row)
                })
                .flat_map(|row| match row {
                    NativeLibraryGridRow::Header(_, _, _) => Vec::new(),
                    NativeLibraryGridRow::Cards(indices) => indices
                        .iter()
                        .filter_map(|index| {
                            let (instance_id, _, _) =
                                self.instances.get(*index)?;
                            let icon_path = self
                                .instance_cards
                                .get(instance_id)?
                                .icon_path
                                .as_ref()?;
                            Some((instance_id.clone(), icon_path.clone()))
                        })
                        .collect(),
                })
                .collect();
            for (instance_id, icon_path) in icons_to_load {
                self.request_instance_icon(&instance_id, &icon_path);
            }
        }
        if browse_results_visible {
            let result_count = self.browse_results.len();
            let current = browse_results_layout
                .icon_prefetch_rows(self.browse_scroll_pixels, result_count);
            let target = browse_results_layout.icon_prefetch_rows(
                self.browse_scroll_target_pixels,
                result_count,
            );
            let browse_icons_to_load: Vec<(String, String)> = self
                .browse_results
                .iter()
                .enumerate()
                .filter(|(index, _)| {
                    current.contains(index) || target.contains(index)
                })
                .filter_map(|(_, project)| {
                    Some((project.id.clone(), project.icon_url.clone()?))
                })
                .collect();
            for (project_id, icon_url) in browse_icons_to_load {
                self.request_browse_icon(&project_id, &icon_url);
            }
        }
        if self.global_user_profile {
            let avatar = self
                .user_profile
                .as_ref()
                .and_then(|profile| profile.get("avatar_url"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let user_id = self.user_profile_id.clone();
            if let (Some(user_id), Some(avatar)) = (user_id, avatar) {
                self.request_browse_icon(
                    &format!("user-profile:{user_id}"),
                    &avatar,
                );
            }
            let organization_icons = self
                .user_profile_organizations
                .iter()
                .take(5)
                .filter_map(|organization| {
                    Some((
                        organization
                            .get("id")
                            .or_else(|| organization.get("slug"))?
                            .as_str()?
                            .to_owned(),
                        organization.get("icon_url")?.as_str()?.to_owned(),
                    ))
                })
                .collect::<Vec<_>>();
            for (id, icon_url) in organization_icons {
                self.request_browse_icon(
                    &format!("user-organization:{id}"),
                    &icon_url,
                );
            }
        }
        let gallery_images_to_load = self
            .browse_selected
            .as_ref()
            .zip(self.browse_details.as_ref())
            .filter(|_| self.browse_detail_tab == 2)
            .map(|(project, details)| {
                if self.browse_gallery_viewer_open {
                    return details
                        .gallery
                        .get(self.browse_gallery_index)
                        .map(|image| {
                            (
                                format!(
                                    "gallery-full:{}:{}",
                                    project.id, self.browse_gallery_index
                                ),
                                image
                                    .raw_url
                                    .clone()
                                    .unwrap_or_else(|| image.url.clone()),
                            )
                        })
                        .into_iter()
                        .collect::<Vec<_>>();
                }
                let (columns, _, card_height, _) = browse_gallery_grid_metrics(
                    library_width,
                    details.gallery.len(),
                );
                let stride = card_height.saturating_add(16).max(1);
                let scroll_top = self.browse_detail_scroll_pixels;
                let scroll_bottom = scroll_top
                    + (initial_size.height as usize).saturating_sub(458) as f64;
                details
                    .gallery
                    .iter()
                    .enumerate()
                    .filter_map(|(index, image)| {
                        let row = index / columns;
                        let top = row.saturating_mul(stride) as f64;
                        let bottom = top + card_height as f64;
                        (bottom >= scroll_top && top <= scroll_bottom).then(
                            || {
                                (
                                    format!("gallery:{}:{}", project.id, index),
                                    image.url.clone(),
                                )
                            },
                        )
                    })
                    .collect::<Vec<_>>()
            });
        for (key, url) in gallery_images_to_load.into_iter().flatten() {
            self.request_browse_icon(&key, &url);
        }
        if show_right_sidebar && !browse_filter_sidebar_visible {
            let max_sidebar_scroll = sidebar_scroll_max(
                self.news_articles.len(),
                &self.sidebar_friends,
                initial_size.height as usize,
                self.onboarding_checklist.as_ref(),
                &self.sidebar_friend_query,
                sidebar_collapsed_groups(self.settings.as_ref()),
            ) as f64;
            self.sidebar_scroll_target_pixels = self
                .sidebar_scroll_target_pixels
                .clamp(0.0, max_sidebar_scroll);
            self.sidebar_scroll_pixels =
                self.sidebar_scroll_pixels.clamp(0.0, max_sidebar_scroll);
            let friend_avatars = sidebar_visible_friend_indices(
                &self.sidebar_friends,
                self.onboarding_checklist.as_ref(),
                &self.sidebar_friend_query,
                sidebar_collapsed_groups(self.settings.as_ref()),
                self.sidebar_scroll_pixels,
                initial_size.height as usize,
            )
            .into_iter()
            .filter_map(|index| {
                let friend = &self.sidebar_friends[index];
                friend.avatar_url.as_ref().map(|url| {
                    (format!("friend:{}", friend.user_id), url.clone())
                })
            })
            .collect::<Vec<_>>();
            for (key, url) in friend_avatars {
                self.request_browse_icon(&key, &url);
            }
            if self.friend_incoming_open {
                let height = initial_size.height as usize;
                let request_avatars = sidebar_visible_incoming_friend_indices(
                    &self.sidebar_friends,
                    self.friend_incoming_scroll_pixels,
                    height,
                )
                .into_iter()
                .filter_map(|index| {
                    let friend = &self.sidebar_friends[index];
                    friend.avatar_url.as_ref().map(|url| {
                        (format!("friend:{}", friend.user_id), url.clone())
                    })
                })
                .collect::<Vec<_>>();
                for (key, url) in request_avatars {
                    self.request_browse_icon(&key, &url);
                }
            }
            let news_top = sidebar_news_top(
                &self.sidebar_friends,
                self.onboarding_checklist.as_ref(),
                &self.sidebar_friend_query,
                sidebar_collapsed_groups(self.settings.as_ref()),
            );
            let first_visible_news = (self.sidebar_scroll_pixels
                - news_top.saturating_sub(48) as f64)
                .max(0.0)
                .div_euclid(SIDEBAR_NEWS_CARD_STRIDE as f64)
                .floor() as usize;
            let news_images: Vec<(String, String)> = self
                .news_articles
                .iter()
                .skip(first_visible_news)
                .take(4)
                .map(|article| {
                    (
                        format!("news:{}", article.link),
                        article.thumbnail.clone(),
                    )
                })
                .collect();
            for (id, url) in news_images {
                self.request_browse_icon(&id, &url);
            }
        }
        if let (Some(project), Some(details)) =
            (&self.browse_selected, &self.browse_details)
        {
            let max_chars =
                library_width.saturating_sub(324).div_euclid(7).max(24);
            let cached = self.browse_description_lines.as_ref().is_some_and(
                |(project_id, width, _, _, _, _)| {
                    project_id == &project.id && *width == max_chars
                },
            );
            if !cached {
                let (lines, links, styles, images) =
                    wrap_browse_body_with_links(
                        &details.body,
                        max_chars,
                        &project.id,
                    );
                self.browse_description_lines = Some((
                    project.id.clone(),
                    max_chars,
                    lines,
                    links,
                    styles,
                    images,
                ));
            }
        } else {
            self.browse_description_lines = None;
        }
        let description_images_to_load: Vec<(String, String)> = self
            .browse_description_lines
            .as_ref()
            .map(|(_, _, lines, _, _, images)| {
                let scroll_top = self.browse_detail_scroll_pixels;
                let viewport_bottom = scroll_top
                    + (initial_size.height as usize).saturating_sub(458) as f64;
                images
                    .iter()
                    .filter_map(|image| {
                        let top =
                            browse_body_content_offset(lines, image.line_index)
                                as f64;
                        let bottom = top
                            + browse_body_line_height(&lines[image.line_index])
                                as f64;
                        (bottom >= scroll_top && top <= viewport_bottom).then(
                            || (image.cache_key.clone(), image.url.clone()),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for (cache_key, url) in description_images_to_load {
            self.request_browse_icon(&cache_key, &url);
        }
        let browse_filter_menu_options =
            if self.browse_filter_menu_open || browse_filter_sidebar_visible {
                self.browse_filter_menu_options()
            } else {
                Vec::new()
            };
        let browse_filter_group_count = self.browse_filter_group_count();
        let browse_filter_uses_tags =
            self.browse_filter_group < BROWSE_LICENSE_GROUP;
        let browse_filter_waiting_for_tags =
            browse_filter_uses_tags && self.browse_filter_options_loading;
        if browse_filter_sidebar_visible {
            let layout = browse_filter_sidebar_layout(
                initial_size.width as usize,
                initial_size.height as usize,
                self.browse_filter_group,
                browse_filter_menu_options.len(),
                browse_filter_group_count,
            );
            let max = layout.max_scroll(browse_filter_menu_options.len());
            self.browse_filter_scroll_pixels =
                self.browse_filter_scroll_pixels.clamp(0.0, max);
            self.browse_filter_scroll_target_pixels =
                self.browse_filter_scroll_target_pixels.clamp(0.0, max);
        } else if self.browse_filter_menu_open {
            let panel = browse_filter_popup_bounds(
                library_width,
                initial_size.height as usize,
            );
            let (_, viewport) =
                browse_popup_filter_rows(panel, self.browse_filter_group);
            let max = browse_filter_menu_options
                .len()
                .saturating_mul(24)
                .saturating_sub(viewport.height) as f64;
            self.browse_filter_scroll_pixels =
                self.browse_filter_scroll_pixels.clamp(0.0, max);
            self.browse_filter_scroll_target_pixels =
                self.browse_filter_scroll_target_pixels.clamp(0.0, max);
        }
        let selected_loader = self.selected_loader();
        let palette = self.settings.as_ref().map_or(0, |settings| {
            palette_for_native_color_theme(
                palette_for_theme(
                    settings.theme,
                    window.theme(),
                    self.preferred_dark_theme,
                ),
                self.native_color_theme,
            )
        });
        render::set_active_palette(palette);
        let cursor = self.cursor;
        let toolbar = self.current_library_toolbar_layout(library_width);

        let page_scroll_active = self.home_page_scroll_active();
        let page_scroll_offset = if page_scroll_active {
            self.library_scroll_pixels.round() as usize
        } else {
            0
        };
        let wide_instance_header =
            self.uses_wide_instance_header(initial_size.width as usize);
        let instance_tabs = self.native_instance_tabs(wide_instance_header);
        let instance_more_rows = if self.instance_tab == 6 {
            self.instance_more_rows(wide_instance_header, library_width)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let instance_primary_action =
            self.selected_instance.as_ref().map(|(instance_id, _)| {
                let card = self.instance_cards.get(instance_id);
                native_library_card_action(
                    self.running_processes.contains_key(instance_id),
                    card.is_some_and(|card| card.install_state == "installing"),
                    card.is_none_or(|card| card.install_state == "installed"),
                    card.is_some_and(|card| card.content_locked),
                    card.is_some_and(|card| card.launch_blocked),
                    false,
                    true,
                )
            });
        let browse_selected_installed = self
            .browse_selected
            .as_ref()
            .is_some_and(|project| self.browse_project_is_installed(project));
        let browse_installing_for_instance = self
            .selected_instance
            .as_ref()
            .and_then(|(id, _)| self.browse_installing_projects.get(id));
        let hosting_can_manage_backups = self.hosting_can_manage_backups();
        let hosting_server_is_running = self.hosting_server_is_running();
        let hosting_backup_downloading = self.hosting_backup_downloading;
        let hosting_backup_mutating = self.hosting_backup_mutating;
        let hosting_backup_selected = &self.hosting_backup_selected;
        let hosting_backup_selected_count = hosting_backup_selected.len();
        let hosting_backup_selected_recoverable_count =
            self.hosting_backups.as_ref().map_or(0, |backups| {
                backups
                    .deleted_backups
                    .iter()
                    .filter(|backup| {
                        backup.can_undelete
                            && hosting_backup_selected.contains(&backup.id)
                    })
                    .count()
            });
        let Some(surface) = &mut self.surface else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };
        if surface.resize(width, height).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        let (w, h) = (size.width as usize, size.height as usize);
        let console_visible_lines = (h.saturating_sub(220) / 12).max(1);
        if self.console_scroll != self.console_scroll_synced {
            let offset = self.console_scroll.saturating_mul(12) as f64;
            self.console_scroll_pixels = offset;
            self.console_scroll_target_pixels = offset;
        }
        let console_max_scroll =
            self.console_text
                .lines()
                .count()
                .saturating_sub(console_visible_lines) as f64
                * 12.0;
        self.console_scroll_target_pixels = self
            .console_scroll_target_pixels
            .clamp(0.0, console_max_scroll);
        self.console_scroll_pixels =
            self.console_scroll_pixels.clamp(0.0, console_max_scroll);
        self.console_scroll =
            (self.console_scroll_pixels / 12.0).floor() as usize;
        self.console_scroll_synced = self.console_scroll;

        if self.log_scroll != self.log_scroll_synced {
            let stride = if self.selected_log.is_some() { 12 } else { 42 };
            let offset = self.log_scroll.saturating_mul(stride) as f64;
            self.log_scroll_pixels = offset;
            self.log_scroll_target_pixels = offset;
        }
        if self.selected_log.is_some() {
            let visible_lines = (h.saturating_sub(220) / 12).max(1);
            let max_scroll =
                self.log_text.lines().count().saturating_sub(visible_lines)
                    as f64
                    * 12.0;
            self.log_scroll_target_pixels =
                self.log_scroll_target_pixels.clamp(0.0, max_scroll);
            self.log_scroll_pixels =
                self.log_scroll_pixels.clamp(0.0, max_scroll);
            self.log_scroll = (self.log_scroll_pixels / 12.0).floor() as usize;
        } else {
            let visible_rows = h.saturating_sub(208) / 42;
            let max_scroll = self
                .instance_logs
                .len()
                .saturating_sub(visible_rows)
                .saturating_mul(42) as f64;
            self.log_scroll_target_pixels =
                self.log_scroll_target_pixels.clamp(0.0, max_scroll);
            self.log_scroll_pixels =
                self.log_scroll_pixels.clamp(0.0, max_scroll);
        }
        self.log_scroll_synced = self.log_scroll;
        buffer.fill(themed_color(0x001b1d22));
        rect(
            &mut buffer,
            w,
            h,
            0,
            NATIVE_TITLE_BAR_HEIGHT,
            64,
            h.saturating_sub(NATIVE_TITLE_BAR_HEIGHT),
            0x00202429,
        );
        rect(
            &mut buffer,
            w,
            h,
            0,
            0,
            w,
            NATIVE_TITLE_BAR_HEIGHT,
            0x00202429,
        );
        draw_northstar_mark(&mut buffer, w, h, 20, 16, 22);
        text(&mut buffer, w, h, 48, 20, "NORTHSTAR", 0x00f1f3f5, 2);
        let shell_title = if self.java_open {
            "Settings  /  Java".to_owned()
        } else if self.settings_open {
            format!(
                "Settings  /  {}",
                SETTINGS_SECTION_NAMES[self
                    .settings_section
                    .min(SETTINGS_SECTION_NAMES.len() - 1)]
            )
        } else if self.accounts_open {
            "Accounts".to_owned()
        } else if self.create_instance_open {
            "New instance".to_owned()
        } else if self.global_user_profile {
            format!(
                "User  /  {}",
                self.user_profile_id.as_deref().unwrap_or_default()
            )
        } else if self.global_skins {
            "Skin Selector".to_owned()
        } else if self.global_screenshots {
            "Screenshots".to_owned()
        } else if self.global_hosting {
            self.hosting_server_details
                .as_ref()
                .map(|details| details.server.name.clone())
                .or_else(|| {
                    self.hosting_server_id.as_deref().and_then(|server_id| {
                        self.hosting_servers
                            .iter()
                            .find(|server| server.server_id == server_id)
                            .map(|server| server.name.clone())
                    })
                })
                .unwrap_or_else(|| "Hosting".to_owned())
        } else if self.global_browse {
            self.browse_selected.as_ref().map_or_else(
                || "Discover".to_owned(),
                |project| format!("Discover  /  {}", project.title),
            )
        } else if let Some((_, name)) = &self.selected_instance {
            let section = match self.instance_tab {
                1 => "Console",
                2 => "Files",
                3 => "Discover",
                4 => "Content",
                5 => "Logs",
                6 => "Settings",
                7 => "Worlds & Servers",
                8 => "Screenshots",
                9 => "Shared",
                _ => "Overview",
            };
            format!("{name}  /  {section}")
        } else {
            "Home".to_owned()
        };
        for (x, backwards, enabled) in [
            (168, true, !self.navigation_back.is_empty()),
            (204, false, !self.navigation_forward.is_empty()),
        ] {
            let hovered = (x as f64..(x + 28) as f64).contains(&cursor.0)
                && (10.0..38.0).contains(&cursor.1);
            if hovered {
                rounded_rect(&mut buffer, w, h, x, 10, 28, 28, 6, 0x002e333a);
            }
            let border = if enabled { 0x00505964 } else { 0x00343a42 };
            rect(&mut buffer, w, h, x + 6, 10, 16, 1, border);
            rect(&mut buffer, w, h, x + 6, 37, 16, 1, border);
            rect(&mut buffer, w, h, x + 6, 10, 1, 28, border);
            rect(&mut buffer, w, h, x + 21, 10, 1, 28, border);
            let arrow = if enabled { 0x00cbd0d7 } else { 0x005b6067 };
            let middle = x + 14;
            let (top_x, center_x, bottom_x) = if backwards {
                (middle + 3, middle - 4, middle + 3)
            } else {
                (middle - 3, middle + 4, middle - 3)
            };
            for stroke in 0..2 {
                let stroke = stroke as isize;
                draw_line(
                    &mut buffer,
                    w,
                    h,
                    ((top_x as isize) + stroke, 18),
                    ((center_x as isize) + stroke, 24),
                    arrow,
                );
                draw_line(
                    &mut buffer,
                    w,
                    h,
                    ((center_x as isize) + stroke, 24),
                    ((bottom_x as isize) + stroke, 30),
                    arrow,
                );
            }
        }
        text_clipped(
            &mut buffer,
            w,
            h,
            244,
            20,
            &shell_title,
            0x00bdc2ca,
            1,
            w.saturating_sub(
                244 + if cfg!(target_os = "macos") { 16 } else { 160 },
            ),
        );
        if uses_custom_window_controls(
            cfg!(target_os = "macos"),
            self.settings
                .as_ref()
                .is_some_and(|settings| settings.native_decorations),
        ) {
            for (index, icon) in
                ["minimize", "maximize", "close"].into_iter().enumerate()
            {
                let x = w.saturating_sub(144) + index * 48;
                let hovered = (x as f64..(x + 48) as f64).contains(&cursor.0)
                    && cursor.1 < 48.0;
                if hovered {
                    rect(
                        &mut buffer,
                        w,
                        h,
                        x,
                        0,
                        48,
                        48,
                        if icon == "close" {
                            0x00c42b1d
                        } else {
                            0x002e333a
                        },
                    );
                }
                let color = if hovered { 0x00ffffff } else { 0x00bdc2ca };
                let center_x = x + 24;
                if icon == "minimize" {
                    rect(&mut buffer, w, h, center_x - 6, 23, 12, 1, color);
                } else if icon == "maximize" && self.window_maximized {
                    rect(&mut buffer, w, h, center_x - 4, 17, 10, 10, color);
                    rect(&mut buffer, w, h, center_x - 6, 19, 10, 10, color);
                    rect(&mut buffer, w, h, center_x - 5, 20, 8, 8, 0x00202429);
                } else if icon == "maximize" {
                    rect(&mut buffer, w, h, center_x - 6, 18, 12, 12, color);
                    rect(
                        &mut buffer,
                        w,
                        h,
                        center_x - 5,
                        19,
                        10,
                        10,
                        0x00202429,
                    );
                } else {
                    draw_line(
                        &mut buffer,
                        w,
                        h,
                        ((center_x - 5) as isize, 19),
                        ((center_x + 5) as isize, 29),
                        color,
                    );
                    draw_line(
                        &mut buffer,
                        w,
                        h,
                        ((center_x + 5) as isize, 19),
                        ((center_x - 5) as isize, 29),
                        color,
                    );
                }
            }
        }
        for (y, icon, selected, label) in [
            (56, 9, is_home_library, "Home"),
            (108, 1, self.global_browse, "Discover"),
            (160, 2, self.global_skins, "Skin Selector"),
            (212, 3, self.global_screenshots, "Screenshots"),
            (264, 8, self.global_hosting, "Hosting"),
        ] {
            if icon == 2
                && self.settings.as_ref().is_some_and(|settings| {
                    !settings.show_skin_selector_in_sidebar
                })
            {
                continue;
            }
            let bounds = navigation_rail_button(y);
            let hovered = bounds.contains(cursor.0, cursor.1);
            if selected || hovered {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    24,
                    if icon == 9 && selected {
                        0x001bd96a
                    } else if selected {
                        0x00303b2b
                    } else {
                        0x0022262c
                    },
                );
            }
            render::draw_navigation_icon(
                &mut buffer,
                w,
                h,
                32,
                y + 24,
                icon,
                if icon == 9 && selected {
                    0x0016181c
                } else if selected {
                    0x00a8e66b
                } else if hovered {
                    0x00f1f3f5
                } else {
                    0x00bdc2ca
                },
            );
            if hovered && !selected {
                let tooltip_width = label.chars().count() * 8 + 20;
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    68,
                    y + 8,
                    tooltip_width,
                    32,
                    8,
                    0x00343a42,
                );
                text(&mut buffer, w, h, 78, y + 17, label, 0x00f1f3f5, 1);
            }
        }
        rect(&mut buffer, w, h, 12, 268, 40, 1, 0x00363b42);
        for (index, (instance_id, name)) in sidebar_shortcuts.iter().enumerate()
        {
            let y = sidebar_instance_position(index);
            let bounds = sidebar_instance_button(index);
            let hovered = bounds.contains(cursor.0, cursor.1);
            let selected = self
                .selected_instance
                .as_ref()
                .is_some_and(|(selected_id, _)| selected_id == instance_id);
            if selected || hovered {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    10,
                    if selected { 0x00303b2b } else { 0x0022262c },
                );
            }
            if let Some(icon) = self.instance_icons.get(instance_id) {
                draw_scaled_instance_icon(
                    &mut buffer,
                    w,
                    h,
                    bounds.x + 4,
                    bounds.y + 4,
                    32,
                    icon,
                );
            } else {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x + 4,
                    bounds.y + 4,
                    32,
                    32,
                    8,
                    0x002e333a,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    bounds.x + 15,
                    bounds.y + 15,
                    &name.chars().next().unwrap_or('I').to_string(),
                    0x00f1f3f5,
                    1,
                );
            }
            if hovered {
                let tooltip_width = (name.chars().count() * 8 + 20).min(240);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    60,
                    y + 4,
                    tooltip_width,
                    32,
                    8,
                    0x00343a42,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    70,
                    y + 14,
                    name,
                    0x00f1f3f5,
                    1,
                    tooltip_width.saturating_sub(20),
                );
            }
        }
        let footer_positions = sidebar_footer_positions(h);
        for (index, (y, icon, selected, label)) in [
            (
                footer_positions[0],
                4,
                self.create_instance_open,
                "New instance",
            ),
            (
                footer_positions[1],
                5,
                self.settings_open || self.java_open,
                "Settings",
            ),
            (footer_positions[2], 6, self.accounts_open, "Accounts"),
        ]
        .into_iter()
        .enumerate()
        {
            let bounds = sidebar_footer_button(h, index);
            let hovered = bounds.contains(cursor.0, cursor.1);
            if selected || hovered {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    24,
                    if selected { 0x00303b2b } else { 0x0022262c },
                );
            }
            render::draw_navigation_icon(
                &mut buffer,
                w,
                h,
                32,
                y + 24,
                icon,
                if selected { 0x00a8e66b } else { 0x00bdc2ca },
            );
            if hovered && !selected {
                let tooltip_width = label.chars().count() * 8 + 20;
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    68,
                    y + 8,
                    tooltip_width,
                    32,
                    8,
                    0x00343a42,
                );
                text(&mut buffer, w, h, 78, y + 17, label, 0x00f1f3f5, 1);
            }
        }
        let screen_cursor = cursor;
        let cursor = (
            render::page_x_to_logical(cursor.0, library_width),
            if page_scroll_active {
                render::page_y_to_logical(cursor.1, page_scroll_offset)
            } else {
                cursor.1
            },
        );
        render::set_page_horizontal_layout(Some(library_width));
        render::set_page_vertical_layout(
            page_scroll_active.then_some(page_scroll_offset),
            Some((
                64,
                64,
                library_width.saturating_sub(64),
                h.saturating_sub(64),
            )),
        );
        let home_welcome = is_home_library
            && visible_instances.is_empty()
            && self.library_search_query.is_empty()
            && self
                .onboarding_checklist
                .as_ref()
                .is_some_and(|checklist| !checklist.has_created_instance);
        let show_library_filters = is_home_library && !home_welcome;
        if show_library_filters {
            let title = self
                .selected_group
                .as_ref()
                .and_then(|id| self.groups.iter().find(|group| &group.id == id))
                .map_or("Library", |group| group.name.as_str());
            let refresh = toolbar.refresh(library_width);
            let custom_group = self
                .selected_group
                .as_deref()
                .is_some_and(|id| id != "group:favorites");
            let membership = toolbar.membership(library_width);
            let title_right = if custom_group && membership.y == refresh.y {
                membership.x
            } else {
                refresh.x
            };
            text_clipped(
                &mut buffer,
                w,
                h,
                292,
                toolbar.title_y(),
                title,
                0x00f1f3f5,
                if toolbar.search.width < 216 { 1 } else { 2 },
                title_right.saturating_sub(302),
            );
            let [all, favorites] = toolbar.view_tabs();
            for (bounds, label, active) in [
                (all, "All instances", self.selected_group.is_none()),
                (
                    favorites,
                    "Favorites",
                    self.selected_group.as_deref() == Some("group:favorites"),
                ),
            ] {
                let hovered = bounds.contains(cursor.0, cursor.1);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    8,
                    if hovered {
                        0x00343a42
                    } else if active {
                        0x00303b2b
                    } else {
                        0x0022262c
                    },
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    bounds.x + 8,
                    bounds.y + 6,
                    label,
                    if active { 0x00a8e66b } else { 0x00bdc2ca },
                    1,
                    bounds.width.saturating_sub(16),
                );
            }
            for (bounds, label) in [
                (refresh, Some("Refresh")),
                (
                    membership,
                    custom_group.then_some(if self.membership_mode {
                        "Done"
                    } else {
                        "Manage members"
                    }),
                ),
            ] {
                if let Some(label) = label {
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        bounds.x,
                        bounds.y,
                        bounds.width,
                        bounds.height,
                        8,
                        if bounds.contains(cursor.0, cursor.1) {
                            0x00343a42
                        } else {
                            0x0022262c
                        },
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        bounds.x + 10,
                        bounds.y + 6,
                        label,
                        0x00bdc2ca,
                        1,
                        bounds.width.saturating_sub(20),
                    );
                }
            }
            if self.input_focus == 21 {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    toolbar.search.x,
                    toolbar.search.y,
                    toolbar.search.width,
                    toolbar.search.height,
                    8,
                    0x00678947,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    toolbar.search.x + 1,
                    toolbar.search.y + 1,
                    toolbar.search.width.saturating_sub(2),
                    toolbar.search.height.saturating_sub(2),
                    7,
                    0x0022262c,
                );
            } else {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    toolbar.search.x,
                    toolbar.search.y,
                    toolbar.search.width,
                    toolbar.search.height,
                    8,
                    0x0022262c,
                );
            }
            render::draw_search_icon(
                &mut buffer,
                w,
                h,
                toolbar.search.x + 20,
                toolbar.search.y + 18,
                0x007b838d,
            );
            let search_label = if self.library_search_query.is_empty() {
                "Search".to_owned()
            } else {
                self.library_search_query.clone()
            };
            text_clipped(
                &mut buffer,
                w,
                h,
                toolbar.search.x + 36,
                toolbar.search.y + 11,
                &search_label,
                if self.library_search_query.is_empty() {
                    0x007b838d
                } else {
                    0x00f1f3f5
                },
                1,
                toolbar.search.width.saturating_sub(76),
            );
            if self.input_focus == 21 && !self.library_search_query.is_empty() {
                let text_width =
                    self.library_search_query.chars().count().saturating_mul(8);
                let max_text_width = toolbar.search.width.saturating_sub(60);
                let caret_x =
                    toolbar.search.x + 36 + text_width.min(max_text_width);
                rect(
                    &mut buffer,
                    w,
                    h,
                    caret_x,
                    toolbar.search.y + 9,
                    1,
                    18,
                    0x00f1f3f5,
                );
            }
            if !self.library_search_query.is_empty() {
                text(
                    &mut buffer,
                    w,
                    h,
                    toolbar.search.x + toolbar.search.width.saturating_sub(26),
                    toolbar.search.y + 10,
                    "×",
                    0x00bdc2ca,
                    1,
                );
            }
            for (bounds, label, background, foreground) in [
                (toolbar.new_group, "New group", 0x0022262c, 0x00f1f3f5),
                (toolbar.new_instance, "New instance", 0x00a8e66b, 0x001b1d22),
                (toolbar.import_pack, "Import pack", 0x0022262c, 0x00f1f3f5),
            ] {
                let hovered = bounds.contains(cursor.0, cursor.1);
                let background = if hovered {
                    if label == "New instance" {
                        0x0042e686
                    } else {
                        0x00343a42
                    }
                } else {
                    background
                };
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    8,
                    background,
                );
                if label != "Import pack" {
                    render::draw_navigation_icon(
                        &mut buffer,
                        w,
                        h,
                        bounds.x + 17,
                        bounds.y + bounds.height / 2,
                        4,
                        if label == "New instance" {
                            0x001b1d22
                        } else {
                            0x00f1f3f5
                        },
                    );
                }
                let label_x =
                    bounds.x + if label == "Import pack" { 10 } else { 30 };
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    label_x,
                    bounds.y + 11,
                    label,
                    foreground,
                    1,
                    bounds
                        .width
                        .saturating_sub(label_x.saturating_sub(bounds.x))
                        .saturating_sub(10),
                );
            }
            rounded_rect(
                &mut buffer,
                w,
                h,
                toolbar.sort.x,
                toolbar.sort.y,
                toolbar.sort.width,
                toolbar.sort.height,
                8,
                if toolbar.sort.contains(cursor.0, cursor.1) {
                    0x00343a42
                } else {
                    0x0022262c
                },
            );
            let sort_label = match self.library_sort_mode / 2 {
                0 => "Name",
                1 => "Last played",
                2 => "Hours played",
                3 => "Date created",
                4 => "Date modified",
                5 => "Loader",
                _ => "Game version",
            };
            text(
                &mut buffer,
                w,
                h,
                toolbar.sort.x + 10,
                toolbar.sort.y + 8,
                "↕",
                0x00bdc2ca,
                1,
            );
            rect(
                &mut buffer,
                w,
                h,
                toolbar.sort.x + toolbar.sort.width - 36,
                toolbar.sort.y + 6,
                1,
                toolbar.sort.height - 12,
                0x00363b42,
            );
            let direction_bounds = UiBounds {
                x: toolbar.sort.x + toolbar.sort.width - 32,
                y: toolbar.sort.y,
                width: 32,
                height: toolbar.sort.height,
            };
            if direction_bounds.contains(cursor.0, cursor.1) {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    direction_bounds.x,
                    direction_bounds.y,
                    direction_bounds.width,
                    direction_bounds.height,
                    8,
                    0x00343a42,
                );
            }
            text(
                &mut buffer,
                w,
                h,
                toolbar.sort.x + toolbar.sort.width - 23,
                toolbar.sort.y + 8,
                if self.library_sort_mode % 2 == 0 {
                    "↑"
                } else {
                    "↓"
                },
                0x00bdc2ca,
                1,
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                toolbar.sort.x + 34,
                toolbar.sort.y + 8,
                sort_label,
                0x00bdc2ca,
                1,
                toolbar.sort.width.saturating_sub(70),
            );
            if show_library_filters {
                let filter_count = self.library_loader_filters.len()
                    + self.library_instance_type_filters.len()
                    + self.library_install_state_filters.len()
                    + self.library_game_version_filters.len()
                    + usize::from(self.show_hidden_instances);
                let filter_label = if filter_count == 0 {
                    "Filters".to_owned()
                } else {
                    format!("Filters ({filter_count})")
                };
                let filter_bounds = toolbar.filter;
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    filter_bounds.x,
                    filter_bounds.y,
                    filter_bounds.width,
                    filter_bounds.height,
                    8,
                    if filter_bounds.contains(cursor.0, cursor.1) {
                        0x00343a42
                    } else if self.library_filter_open || filter_count > 0 {
                        0x00303b2b
                    } else {
                        0x0022262c
                    },
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    filter_bounds.x + 10,
                    filter_bounds.y + 8,
                    &filter_label,
                    0x00f1f3f5,
                    1,
                    filter_bounds.width.saturating_sub(20),
                );
                let group_bounds = toolbar.group;
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    group_bounds.x,
                    group_bounds.y,
                    group_bounds.width,
                    group_bounds.height,
                    8,
                    if group_bounds.contains(cursor.0, cursor.1) {
                        0x00343a42
                    } else {
                        0x0022262c
                    },
                );
                for icon_y in 0..2 {
                    for icon_x in 0..2 {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            group_bounds.x + 11 + icon_x * 6,
                            group_bounds.y + 8 + icon_y * 6,
                            4,
                            4,
                            0x00bdc2ca,
                        );
                    }
                }
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    group_bounds.x + 34,
                    group_bounds.y + 8,
                    LIBRARY_GROUP_LABELS[self.library_group_mode as usize
                        % LIBRARY_GROUP_LABELS.len()],
                    0x00bdc2ca,
                    1,
                    group_bounds.width.saturating_sub(70),
                );
            }
            let status_message = self.message.to_ascii_lowercase();
            let empty_library_status_is_actionable = [
                "failed",
                "error",
                "could not",
                "unable",
                "loading",
                "starting",
                "importing",
                "installing",
                "saving",
                "deleting",
                "removing",
                "updating",
            ]
            .iter()
            .any(|keyword| status_message.contains(keyword));
            if !visible_instances.is_empty()
                || empty_library_status_is_actionable
            {
                if !self.message.is_empty() {
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        toolbar.cards_top.saturating_sub(24),
                        &self.message,
                        0x00bdc2ca,
                        1,
                        library_width.saturating_sub(316),
                    );
                }
            }
            if let Some(progress) = self.install_progress {
                let progress_width = library_width.saturating_sub(324).min(470);
                rect(
                    &mut buffer,
                    w,
                    h,
                    292,
                    toolbar.cards_top.saturating_sub(6),
                    progress_width,
                    3,
                    0x00363b42,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    292,
                    toolbar.cards_top.saturating_sub(6),
                    (progress_width as f32 * progress.clamp(0.0, 1.0)) as usize,
                    3,
                    0x00a8e66b,
                );
            }
        }
        if is_home_library && visible_instances.is_empty() {
            if home_welcome {
                let layout = welcome_screen_layout(library_width, h);
                if layout.show_mark {
                    let pattern_width = 416;
                    let pattern_height = if h <= 700 { 368 } else { 478 };
                    let pattern_left =
                        layout.center_x.saturating_sub(pattern_width / 2);
                    let pattern_top = layout.top.saturating_sub(208);
                    for dot_y in (0..pattern_height).step_by(9) {
                        let normalized_y =
                            dot_y as f64 / pattern_height as f64 * 2.0 - 1.0;
                        for dot_x in (0..pattern_width).step_by(9) {
                            let normalized_x =
                                dot_x as f64 / pattern_width as f64 * 2.0 - 1.0;
                            let radius = (normalized_x * normalized_x
                                + normalized_y * normalized_y)
                                .sqrt();
                            if radius >= 0.68 {
                                continue;
                            }
                            let opacity = if radius <= 0.1 {
                                50
                            } else {
                                (50.0 * (0.68 - radius) / 0.58) as u32
                            };
                            let color = render::blend_color(
                                0x001b1d22, 0x00f1f3f5, opacity,
                            );
                            rect(
                                &mut buffer,
                                w,
                                h,
                                pattern_left + dot_x,
                                pattern_top + dot_y,
                                1,
                                1,
                                color,
                            );
                        }
                    }
                    draw_northstar_mark(
                        &mut buffer,
                        w,
                        h,
                        layout.center_x.saturating_sub(50),
                        layout.top,
                        100,
                    );
                }
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.center_x.saturating_sub(90),
                    layout.title_y,
                    "Welcome to Northstar",
                    0x00f1f3f5,
                    12,
                );
                text_regular(
                    &mut buffer,
                    w,
                    h,
                    layout.center_x.saturating_sub(66),
                    layout.title_y + if layout.show_mark { 38 } else { 22 },
                    "Ready to start playing?",
                    0x00bdc2ca,
                    8,
                    240,
                );
                let button_x = layout
                    .center_x
                    .saturating_sub(WELCOME_SPLIT_BUTTON_WIDTH / 2);
                let split_button_hovered = (button_x as f64
                    ..(button_x + WELCOME_SPLIT_BUTTON_WIDTH) as f64)
                    .contains(&cursor.0)
                    && (layout.create_y as f64..layout.create_y as f64 + 40.0)
                        .contains(&cursor.1);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    button_x,
                    layout.create_y,
                    WELCOME_SPLIT_BUTTON_WIDTH,
                    40,
                    14,
                    if split_button_hovered {
                        0x00b5ee78
                    } else {
                        0x00a8e66b
                    },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    button_x + 51,
                    layout.create_y + 12,
                    "Create an instance",
                    0x00141619,
                    8,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    button_x + 34,
                    layout.create_y + 14,
                    2,
                    12,
                    0x00141619,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    button_x + 29,
                    layout.create_y + 19,
                    12,
                    2,
                    0x00141619,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    button_x + WELCOME_PRIMARY_BUTTON_WIDTH,
                    layout.create_y + 8,
                    1,
                    24,
                    0x0068ce87,
                );
                let menu_chevron_x =
                    button_x + WELCOME_PRIMARY_BUTTON_WIDTH + 14;
                draw_line(
                    &mut buffer,
                    w,
                    h,
                    (menu_chevron_x as isize, layout.create_y as isize + 18),
                    (
                        (menu_chevron_x + 4) as isize,
                        layout.create_y as isize + 22,
                    ),
                    0x00141619,
                );
                draw_line(
                    &mut buffer,
                    w,
                    h,
                    (
                        (menu_chevron_x + 4) as isize,
                        layout.create_y as isize + 22,
                    ),
                    (
                        (menu_chevron_x + 8) as isize,
                        layout.create_y as isize + 18,
                    ),
                    0x00141619,
                );
                let hint_x = layout.center_x.saturating_sub(92);
                text(
                    &mut buffer,
                    w,
                    h,
                    hint_x,
                    layout.create_y + 50,
                    "Press",
                    0x007b838d,
                    1,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    hint_x + 32,
                    layout.create_y + 47,
                    20,
                    20,
                    4,
                    0x0022262c,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    hint_x + 32,
                    layout.create_y + 47,
                    20,
                    1,
                    0x00363b42,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    hint_x + 32,
                    layout.create_y + 66,
                    20,
                    1,
                    0x00363b42,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    hint_x + 39,
                    layout.create_y + 51,
                    "N",
                    0x00bdc2ca,
                    1,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    hint_x + 58,
                    layout.create_y + 50,
                    "to quick create an instance",
                    0x007b838d,
                    1,
                    178,
                );
                if self.welcome_options_open {
                    let skip_hovered = (button_x as f64
                        ..(button_x + WELCOME_SPLIT_BUTTON_WIDTH) as f64)
                        .contains(&cursor.0)
                        && (layout.options_y as f64
                            ..layout.options_y as f64 + 36.0)
                            .contains(&cursor.1);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        button_x,
                        layout.options_y,
                        WELCOME_SPLIT_BUTTON_WIDTH,
                        36,
                        6,
                        if skip_hovered { 0x00343c45 } else { 0x002a3037 },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        button_x + 16,
                        layout.options_y + 10,
                        "Skip",
                        0x00f1f3f5,
                        1,
                    );
                }
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.center_x.saturating_sub(70),
                    layout.import_y.saturating_sub(30),
                    "Escaping another launcher?",
                    0x00bdc2ca,
                    1,
                );
                let import_x = layout
                    .center_x
                    .saturating_sub(WELCOME_IMPORT_BUTTON_WIDTH / 2);
                let import_hovered = (import_x as f64
                    ..(import_x + WELCOME_IMPORT_BUTTON_WIDTH) as f64)
                    .contains(&cursor.0)
                    && (layout.import_y as f64..layout.import_y as f64 + 40.0)
                        .contains(&cursor.1);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    import_x,
                    layout.import_y,
                    WELCOME_IMPORT_BUTTON_WIDTH,
                    40,
                    14,
                    if import_hovered {
                        0x00343a42
                    } else {
                        0x0022262c
                    },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    import_x + 26,
                    layout.import_y + 12,
                    "Import from launcher",
                    0x00f1f3f5,
                    1,
                );
            } else {
                text(
                    &mut buffer,
                    w,
                    h,
                    292,
                    toolbar.cards_top + 42,
                    if self.library_search_query.is_empty() {
                        "No instances yet"
                    } else {
                        "No instances match your search"
                    },
                    0x00f1f3f5,
                    2,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    292,
                    toolbar.cards_top + 78,
                    "Create an instance or import a modpack to get started.",
                    0x00bdc2ca,
                    1,
                );
            }
            if self.library_search_query.is_empty()
                && let Some(checklist) = self
                    .onboarding_checklist
                    .as_ref()
                    .filter(|checklist| checklist.show_checklist)
                && checklist.has_created_instance
            {
                let card_x = 292usize;
                let card_y = toolbar.cards_top + 112;
                let card_width = library_width.saturating_sub(324).min(620);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    card_x,
                    card_y,
                    card_width,
                    158,
                    12,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    card_x + 16,
                    card_y + 16,
                    "Getting started",
                    0x00f1f3f5,
                    2,
                );
                let steps = [
                    (
                        "Create your first instance",
                        checklist.has_created_instance,
                    ),
                    (
                        "Sign in to Minecraft",
                        checklist.has_logged_into_minecraft,
                    ),
                    ("Sign in to Modrinth", checklist.has_logged_into_modrinth),
                ];
                for (index, (label, complete)) in steps.iter().enumerate() {
                    let row_y = card_y + 48 + index * 34;
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        card_x + 16,
                        row_y,
                        18,
                        18,
                        9,
                        if *complete { 0x00a8e66b } else { 0x00363b42 },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        card_x + 44,
                        row_y + 2,
                        label,
                        if *complete { 0x007b838d } else { 0x00f1f3f5 },
                        1,
                    );
                }
            }
        }
        if initial_grid.jump_in {
            let items = &jump_in_items;
            let content_width = library_width.saturating_sub(324);
            let all_count = items.len().min(MAX_JUMP_IN_ITEMS);
            let visible_count = jump_in_visible_count;
            let columns =
                Self::jump_in_columns(library_width).min(visible_count);
            let tile_width = content_width
                .saturating_sub(12 * columns.saturating_sub(1))
                / columns.max(1);
            text(
                &mut buffer,
                w,
                h,
                292,
                HOME_JUMP_IN_TITLE_Y,
                "JUMP IN",
                0x00f1f3f5,
                1,
            );
            for (item_index, item) in
                items.iter().take(visible_count).enumerate()
            {
                let column = item_index % columns.max(1);
                let row = item_index / columns.max(1);
                let (
                    instance_id,
                    instance_name,
                    world_title,
                    world_icon_key,
                    server_address,
                    newly_added,
                    world_last_played,
                    world_mode,
                ) = match item {
                    NativeJumpInItem::Instance { index, newly_added } => {
                        let Some((id, name, _)) = self.instances.get(*index)
                        else {
                            continue;
                        };
                        (
                            id.clone(),
                            name.clone(),
                            None,
                            None,
                            None,
                            *newly_added,
                            None,
                            None,
                        )
                    }
                    NativeJumpInItem::World(recent_world) => {
                        let Some((id, name, _)) =
                            self.instances.iter().find(|instance| {
                                instance.0 == recent_world.instance_id
                            })
                        else {
                            continue;
                        };
                        let title = if recent_world.world.name.trim().is_empty()
                        {
                            recent_world.world.world_id().to_owned()
                        } else {
                            recent_world.world.name.clone()
                        };
                        (
                            id.clone(),
                            name.clone(),
                            Some(title),
                            Some(format!(
                                "world:{}:{}",
                                recent_world.instance_id,
                                recent_world.world.world_id()
                            )),
                            match &recent_world.world.details {
                                theseus::worlds::WorldDetails::Server {
                                    address,
                                    ..
                                } => Some(address.clone()),
                                theseus::worlds::WorldDetails::Singleplayer {
                                    ..
                                } => None,
                            },
                            false,
                            recent_world
                                .world
                                .last_played
                                .map(|played| played.timestamp()),
                            match &recent_world.world.details {
                                theseus::worlds::WorldDetails::Singleplayer {
                                    game_mode,
                                    hardcore,
                                    ..
                                } => Some(if *hardcore {
                                    "Hardcore".to_owned()
                                } else {
                                    format!("{game_mode:?}")
                                }),
                                theseus::worlds::WorldDetails::Server { .. } => {
                                    None
                                }
                            },
                        )
                    }
                };
                let x = 292 + column * (tile_width + 12);
                let y = HOME_JUMP_IN_TILES_Y + row * HOME_JUMP_IN_ROW_HEIGHT;
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    tile_width,
                    HOME_JUMP_IN_TILE_HEIGHT,
                    20,
                    0x002e333a,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    x + 1,
                    y + 1,
                    tile_width.saturating_sub(2),
                    HOME_JUMP_IN_TILE_HEIGHT - 2,
                    19,
                    if newly_added { 0x0022262c } else { 0x0027292e },
                );
                if newly_added {
                    for offset in (8..tile_width.saturating_sub(8)).step_by(10)
                    {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            x + offset,
                            y,
                            5.min(tile_width.saturating_sub(offset)),
                            1,
                            0x005d9162,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            x + offset,
                            y + HOME_JUMP_IN_TILE_HEIGHT - 1,
                            5.min(tile_width.saturating_sub(offset)),
                            1,
                            0x005d9162,
                        );
                    }
                    for offset in (8..HOME_JUMP_IN_TILE_HEIGHT - 8).step_by(10)
                    {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            x,
                            y + offset,
                            1,
                            5,
                            0x005d9162,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            x + tile_width - 1,
                            y + offset,
                            1,
                            5,
                            0x005d9162,
                        );
                    }
                }
                if let Some(icon) = world_icon_key
                    .as_ref()
                    .and_then(|key| self.browse_icons.get(key))
                {
                    draw_scaled_project_icon(
                        &mut buffer,
                        w,
                        h,
                        x + 12,
                        y + 16,
                        48,
                        icon,
                    );
                } else if let Some(icon) = world_icon_key
                    .as_ref()
                    .and_then(|key| self.instance_icons.get(key))
                {
                    draw_scaled_instance_icon(
                        &mut buffer,
                        w,
                        h,
                        x + 12,
                        y + 16,
                        48,
                        icon,
                    );
                } else if let Some(icon) = self.instance_icons.get(&instance_id)
                {
                    draw_scaled_instance_icon(
                        &mut buffer,
                        w,
                        h,
                        x + 12,
                        y + 16,
                        48,
                        icon,
                    );
                } else {
                    let initial = instance_name
                        .chars()
                        .next()
                        .map_or_else(String::new, |ch| {
                            ch.to_uppercase().to_string()
                        });
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 29,
                        y + 31,
                        &initial,
                        0x00f1f3f5,
                        2,
                    );
                }
                let title = world_title.as_deref().unwrap_or(&instance_name);
                let play_x = x + tile_width.saturating_sub(124);
                let info_x = x
                    + 56
                    + tile_width
                        .saturating_sub(48 + 120 + 24)
                        .saturating_mul(3)
                        / 7
                    + 8;
                let left_text_width = if world_title.is_some() {
                    info_x.saturating_sub(x + 76)
                } else {
                    play_x.saturating_sub(x + 76)
                };
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 68,
                    y + if world_title.is_some() { 14 } else { 20 },
                    title,
                    0x00f1f3f5,
                    1,
                    left_text_width.saturating_sub(if newly_added {
                        108
                    } else {
                        0
                    }),
                );
                if newly_added {
                    let name_width = render::measure_text_width(title, 1)
                        .min(left_text_width.saturating_sub(108));
                    let badge_x = (x + 68 + name_width + 8)
                        .min(play_x.saturating_sub(104));
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        badge_x,
                        y + 13,
                        96,
                        20,
                        8,
                        0x00303b2b,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        badge_x + 10,
                        y + 17,
                        1,
                        10,
                        0x00a8e66b,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        badge_x + 6,
                        y + 21,
                        9,
                        1,
                        0x00a8e66b,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        badge_x + 19,
                        y + 18,
                        "New instance",
                        0x00a8e66b,
                        1,
                        70,
                    );
                }
                if world_title.is_some() {
                    let last_played = world_last_played.map_or_else(
                        || "Not played yet".to_owned(),
                        relative_play_time,
                    );
                    let subtitle = format!("{instance_name} · {last_played}");
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        x + 68,
                        y + 44,
                        &subtitle,
                        0x007b838d,
                        1,
                        left_text_width,
                    );
                    let right_width = play_x.saturating_sub(info_x + 12);
                    if let Some(address) = server_address.as_ref() {
                        let status = self.server_statuses.get(address);
                        if let Some(motd) = self.server_motds.get(address) {
                            let lines = render::wrap_text_to_width(
                                motd,
                                1,
                                right_width,
                                2,
                            );
                            for (line, text_line) in
                                lines.lines().take(2).enumerate()
                            {
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    info_x,
                                    y + 16 + line * 18,
                                    text_line,
                                    0x00bdc2ca,
                                    1,
                                    right_width,
                                );
                            }
                            if let Some(status) = status.filter(|value| {
                                !value.is_empty()
                                    && value.as_str()
                                        != "Offline or unavailable"
                            }) {
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    info_x,
                                    y + 54,
                                    status,
                                    0x007b838d,
                                    1,
                                    right_width,
                                );
                            }
                        } else {
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                info_x,
                                y + 29,
                                status
                                    .map_or("Checking server…", String::as_str),
                                if status.is_some_and(|value| {
                                    value == "Offline or unavailable"
                                }) {
                                    0x00e68a6b
                                } else {
                                    0x007b838d
                                },
                                1,
                                right_width,
                            );
                        }
                    } else if let Some(mode) = world_mode.as_deref() {
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            info_x,
                            y + 29,
                            mode,
                            if mode == "Hardcore" {
                                0x00e68a6b
                            } else {
                                0x00bdc2ca
                            },
                            1,
                            right_width,
                        );
                    }
                } else {
                    let metadata = self
                        .instance_cards
                        .get(&instance_id)
                        .map(|card| {
                            format!("{} {}", card.loader, card.game_version)
                        })
                        .unwrap_or_else(|| "Minecraft".into());
                    let last_played = self
                        .instance_metrics
                        .get(&instance_id)
                        .and_then(|metrics| metrics.last_played)
                        .map_or_else(
                            || {
                                if newly_added {
                                    "Never played".to_owned()
                                } else {
                                    "Recently played".to_owned()
                                }
                            },
                            relative_play_time,
                        );
                    let subtitle = format!("{metadata} · {last_played}");
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        x + 68,
                        y + 44,
                        &subtitle,
                        0x007b838d,
                        1,
                        left_text_width.saturating_sub(if newly_added {
                            108
                        } else {
                            0
                        }),
                    );
                }
                let is_running =
                    self.running_processes.contains_key(&instance_id);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    play_x,
                    y + 24,
                    80,
                    32,
                    8,
                    if is_running { 0x004a2929 } else { 0x00a8e66b },
                );
                draw_library_card_action_icon(
                    &mut buffer,
                    w,
                    h,
                    play_x + 5,
                    y + 24,
                    if is_running {
                        NativeLibraryCardAction::Stop
                    } else {
                        NativeLibraryCardAction::Play
                    },
                    if is_running { 0x00ef8b8b } else { 0x001b1d22 },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    play_x + 34,
                    y + 35,
                    if is_running { "Stop" } else { "Play" },
                    if is_running { 0x00ef8b8b } else { 0x001b1d22 },
                    1,
                );
                let overflow_x = x + tile_width.saturating_sub(36);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    overflow_x,
                    y + 24,
                    32,
                    32,
                    8,
                    if self.jump_in_overflow == Some(item_index) {
                        0x0040474e
                    } else {
                        0x00343a42
                    },
                );
                for dot in 0..3 {
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        overflow_x + 14,
                        y + 31 + dot * 7,
                        4,
                        4,
                        2,
                        0x00bdc2ca,
                    );
                }
            }
            if let Some(menu_index) =
                self.jump_in_overflow.filter(|index| *index < visible_count)
            {
                let columns = columns.max(1);
                let column = menu_index % columns;
                let row = menu_index / columns;
                let tile_x = 292 + column * (tile_width + 12);
                let tile_y =
                    HOME_JUMP_IN_TILES_Y + row * HOME_JUMP_IN_ROW_HEIGHT;
                let menu_x = tile_x + tile_width.saturating_sub(176);
                let menu_y = tile_y + HOME_JUMP_IN_TILE_HEIGHT + 4;
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    menu_x,
                    menu_y,
                    172,
                    76,
                    10,
                    0x00343a42,
                );
                let is_world =
                    matches!(items[menu_index], NativeJumpInItem::World(_));
                for (option, label) in [
                    if is_world {
                        "View world"
                    } else {
                        "View instance"
                    },
                    "Open folder",
                ]
                .into_iter()
                .enumerate()
                {
                    let option_y = menu_y + 6 + option * 34;
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        menu_x + 6,
                        option_y,
                        160,
                        30,
                        6,
                        if (menu_x as f64..(menu_x + 172) as f64)
                            .contains(&cursor.0)
                            && (option_y as f64..(option_y + 30) as f64)
                                .contains(&cursor.1)
                        {
                            0x0040474e
                        } else {
                            0x00343a42
                        },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        menu_x + 18,
                        option_y + 9,
                        label,
                        0x00f1f3f5,
                        1,
                    );
                }
            }
            if all_count > Self::jump_in_columns(library_width)
                && let Some((handle_x, handle_y)) = jump_in_resize_handle
            {
                let hovered = (handle_x as f64..(handle_x + 48) as f64)
                    .contains(&cursor.0)
                    && (handle_y as f64..(handle_y + 16) as f64)
                        .contains(&cursor.1);
                let color = if hovered { 0x00a8e66b } else { 0x007b838d };
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    handle_x + 12,
                    handle_y + 4,
                    24,
                    2,
                    1,
                    color,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    handle_x + 12,
                    handle_y + 8,
                    24,
                    2,
                    1,
                    color,
                );
            }
        }
        if is_home_library {
            let grid = initial_grid;
            let mut cards_to_draw: Vec<(usize, usize, usize)> = Vec::new();
            let mut grid_y = grid.cards_top;
            let viewport_top = 64 + page_scroll_offset;
            let viewport_bottom = h.saturating_add(page_scroll_offset);
            for row in &library_grid_rows {
                let row_height = library_grid_row_height(row, grid);
                if grid_y >= viewport_bottom {
                    break;
                }
                if grid_y + row_height <= viewport_top {
                    grid_y += row_height;
                    continue;
                }
                match row {
                    NativeLibraryGridRow::Header(key, title, count) => {
                        text(
                            &mut buffer,
                            w,
                            h,
                            292,
                            grid_y + 12,
                            if self.library_collapsed_groups.contains(key) {
                                ">"
                            } else {
                                "v"
                            },
                            0x00a8e66b,
                            1,
                        );
                        let count_label = count.to_string();
                        let count_width = count_label
                            .chars()
                            .count()
                            .saturating_mul(8)
                            .saturating_add(12)
                            .max(24);
                        let title_x = 306usize;
                        let badge_x = (title_x
                            + title.chars().count().saturating_mul(8)
                            + 10)
                            .min(
                                library_width.saturating_sub(count_width + 16),
                            );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            title_x,
                            grid_y + 12,
                            title,
                            0x00f1f3f5,
                            1,
                            badge_x.saturating_sub(title_x + 8),
                        );
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            badge_x,
                            grid_y + 9,
                            count_width,
                            22,
                            11,
                            0x00343a42,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            badge_x
                                + count_width.saturating_sub(
                                    count_label.chars().count() * 8,
                                ) / 2,
                            grid_y + 15,
                            &count_label,
                            0x00bdc2ca,
                            1,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            grid_y + 36,
                            library_width.saturating_sub(324),
                            1,
                            0x00363b42,
                        );
                    }
                    NativeLibraryGridRow::Cards(indices) => {
                        cards_to_draw.extend(
                            indices.iter().enumerate().map(
                                |(column, index)| (*index, column, grid_y),
                            ),
                        );
                    }
                }
                grid_y += row_height;
            }
            for (instance_index, column, y) in cards_to_draw {
                let (instance_id, name, group_ids) =
                    &self.instances[instance_index];
                let x = 292 + column * (grid.card_width + grid.gap);
                let card_width = grid.card_width;
                let art_height = grid.art_height;
                let icon_size = art_height;
                let icon_x = x + card_width.saturating_sub(icon_size) / 2;
                let selected =
                    self.selected_instance.as_ref().is_some_and(
                        |(selected_id, _)| selected_id == instance_id,
                    ) || self.selected_library_instances.contains(instance_id);
                let focused = self.library_focused_instance.as_deref()
                    == Some(instance_id.as_str());
                let hovered = (x as f64..(x + card_width) as f64)
                    .contains(&cursor.0)
                    && (y as f64..(y + grid.card_height) as f64)
                        .contains(&cursor.1);
                let card_data = self.instance_cards.get(instance_id);
                let action = native_library_card_action(
                    self.running_processes.contains_key(instance_id),
                    card_data
                        .is_some_and(|card| card.install_state == "installing"),
                    card_data
                        .is_none_or(|card| card.install_state == "installed"),
                    card_data.is_some_and(|card| card.content_locked),
                    card_data.is_some_and(|card| card.launch_blocked),
                    !self.selected_library_instances.is_empty(),
                    hovered || focused,
                );
                let compact = self
                    .settings
                    .as_ref()
                    .is_some_and(|settings| settings.compact_instance_cards);
                let palette = [
                    (0x00343e50, 0x00455070),
                    (0x003b354a, 0x00524766),
                    (0x00304642, 0x00416056),
                    (0x00443a32, 0x005e4a3b),
                    (0x00333e46, 0x00465862),
                ];
                let color_index =
                    instance_id.bytes().fold(0usize, |hash, byte| {
                        hash.wrapping_mul(31).wrapping_add(byte as usize)
                    }) % palette.len();
                let (tile_color, _) = palette[color_index];
                if compact {
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        x,
                        y,
                        card_width,
                        grid.card_height,
                        12,
                        if selected {
                            0x007d7f82
                        } else if hovered || focused {
                            0x0040474f
                        } else {
                            0x00363b42
                        },
                    );
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        x + 1,
                        y + 1,
                        card_width.saturating_sub(2),
                        grid.card_height.saturating_sub(2),
                        11,
                        0x0022262c,
                    );
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        x + 10,
                        y + 12,
                        40,
                        40,
                        8,
                        tile_color,
                    );
                    let initial =
                        name.chars().next().map_or_else(String::new, |ch| {
                            ch.to_uppercase().to_string()
                        });
                    if let Some(icon) = self.instance_icons.get(instance_id) {
                        draw_scaled_instance_icon(
                            &mut buffer,
                            w,
                            h,
                            x + 10,
                            y + 12,
                            40,
                            icon,
                        );
                    } else {
                        text(
                            &mut buffer,
                            w,
                            h,
                            x + 24,
                            y + 22,
                            &initial,
                            0x00f1f3f5,
                            8,
                        );
                    }
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        x + 60,
                        y + 11,
                        name,
                        0x00f1f3f5,
                        8,
                        card_width.saturating_sub(78),
                    );
                    let subheading = card_data.map_or_else(
                        || "Minecraft".to_owned(),
                        |card| format!("{} {}", card.loader, card.game_version),
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        x + 60,
                        y + 34,
                        &subheading,
                        0x00bdc2ca,
                        1,
                        card_width.saturating_sub(78),
                    );
                    if library_membership_toggle_visible(
                        self.membership_mode,
                        action,
                    ) {
                        if let Some(group_id) = self.selected_group.as_ref() {
                            let is_member =
                                group_ids.iter().any(|id| id == group_id);
                            let button_x = x + card_width.saturating_sub(30);
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                button_x,
                                y + 8,
                                22,
                                22,
                                11,
                                if is_member { 0x003b302b } else { 0x00303b2b },
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                button_x + 7,
                                y + 11,
                                if is_member { "✓" } else { "+" },
                                0x00a8e66b,
                                1,
                            );
                        }
                    } else if action != NativeLibraryCardAction::None {
                        let (background, foreground) = match action {
                            NativeLibraryCardAction::Stop => {
                                (0x00c24655, 0x00ffffff)
                            }
                            NativeLibraryCardAction::Installing => {
                                (0x00343a42, 0x00bdc2ca)
                            }
                            _ => (0x00a8e66b, 0x001b1d22),
                        };
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            x + 10,
                            y + 12,
                            40,
                            40,
                            10,
                            background,
                        );
                        draw_library_card_action_icon(
                            &mut buffer,
                            w,
                            h,
                            x + 10,
                            y + 12,
                            action,
                            foreground,
                        );
                        if action == NativeLibraryCardAction::Installing {
                            text(
                                &mut buffer,
                                w,
                                h,
                                x + 17,
                                y + 23,
                                "…",
                                foreground,
                                1,
                            );
                        }
                    }
                    if !self.membership_mode && (hovered || focused || selected)
                    {
                        draw_library_selection_indicator(
                            &mut buffer,
                            w,
                            h,
                            library_selection_button_bounds(
                                x, y, card_width, true,
                            ),
                            selected,
                        );
                    }
                    if hovered {
                        render::brighten_rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            x,
                            y,
                            card_width,
                            grid.card_height,
                            12,
                            26,
                        );
                    }
                    continue;
                }
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    card_width,
                    grid.card_height,
                    20,
                    if selected {
                        0x007d7f82
                    } else if focused {
                        0x00a8e66b
                    } else {
                        0x002e333a
                    },
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    x + 1,
                    y + 1,
                    card_width.saturating_sub(2),
                    grid.card_height.saturating_sub(2),
                    19,
                    0x0022262c,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    icon_x,
                    y + 12,
                    icon_size,
                    icon_size,
                    16,
                    tile_color,
                );
                let initial =
                    name.chars().next().map_or_else(String::new, |ch| {
                        ch.to_uppercase().to_string()
                    });
                if let Some(icon) = self.instance_icons.get(instance_id) {
                    draw_scaled_instance_icon(
                        &mut buffer,
                        w,
                        h,
                        icon_x,
                        y + 12,
                        icon_size,
                        icon,
                    );
                } else {
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + card_width / 2 - 12,
                        y + 12 + art_height / 2 - 9,
                        &initial,
                        0x00f1f3f5,
                        3,
                    );
                }
                let card_text_width = card_width.saturating_sub(24);
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 12,
                    y + art_height + 24,
                    name,
                    0x00f1f3f5,
                    8,
                    card_text_width,
                );
                let subheading = card_data.map_or_else(
                    || "Minecraft".to_owned(),
                    |card| format!("{} {}", card.loader, card.game_version),
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 12,
                    y + art_height + 48,
                    &subheading,
                    0x00bdc2ca,
                    4,
                    card_text_width,
                );
                if library_membership_toggle_visible(
                    self.membership_mode,
                    action,
                ) {
                    if let Some(group_id) = self.selected_group.as_ref() {
                        let is_member =
                            group_ids.iter().any(|id| id == group_id);
                        let button_x = x + card_width.saturating_sub(36);
                        rect(
                            &mut buffer,
                            w,
                            h,
                            button_x,
                            y + art_height + 14,
                            28,
                            28,
                            if is_member { 0x003b302b } else { 0x00303b2b },
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            button_x + 9,
                            y + art_height + 23,
                            if is_member { "✓" } else { "+" },
                            0x00a8e66b,
                            0,
                        );
                    }
                } else if action != NativeLibraryCardAction::None {
                    let button_x = x + card_width.saturating_sub(62);
                    let button_y = y + art_height.saturating_sub(38);
                    let (background, foreground) = match action {
                        NativeLibraryCardAction::Stop => {
                            (0x00c24655, 0x00ffffff)
                        }
                        NativeLibraryCardAction::Installing => {
                            (0x00343a42, 0x00bdc2ca)
                        }
                        _ => (0x00a8e66b, 0x001b1d22),
                    };
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        button_x,
                        button_y,
                        40,
                        40,
                        10,
                        background,
                    );
                    draw_library_card_action_icon(
                        &mut buffer,
                        w,
                        h,
                        button_x,
                        button_y,
                        action,
                        foreground,
                    );
                    if action == NativeLibraryCardAction::Installing {
                        text(
                            &mut buffer,
                            w,
                            h,
                            button_x + 7,
                            button_y + 11,
                            "…",
                            foreground,
                            1,
                        );
                    }
                }
                if !self.membership_mode && (hovered || focused || selected) {
                    draw_library_selection_indicator(
                        &mut buffer,
                        w,
                        h,
                        library_selection_button_bounds(
                            x, y, card_width, false,
                        ),
                        selected,
                    );
                }
                if hovered {
                    render::brighten_rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        x,
                        y,
                        card_width,
                        grid.card_height,
                        20,
                        26,
                    );
                }
            }
            if self
                .selected_group
                .as_deref()
                .is_some_and(|id| id != "group:favorites")
            {
                let group_y = h.saturating_sub(88);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    292,
                    group_y,
                    172,
                    28,
                    7,
                    0x0022262c,
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    302,
                    group_y + 9,
                    if self.group_input.is_empty() {
                        "GROUP NAME"
                    } else {
                        &self.group_input
                    },
                    if self.group_input.is_empty() {
                        0x007b838d
                    } else {
                        0x00f1f3f5
                    },
                    1,
                    154,
                );
                for (x, width, label) in [
                    (292, 34, "UP"),
                    (329, 40, "DOWN"),
                    (372, 52, "RENAME"),
                    (427, 44, "DELETE"),
                ] {
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        x,
                        group_y - 34,
                        width,
                        26,
                        6,
                        if label == "DELETE" {
                            0x003b302b
                        } else {
                            0x00303b2b
                        },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 3,
                        group_y - 25,
                        label,
                        if label == "DELETE" {
                            0x00e68a6b
                        } else {
                            0x00a8e66b
                        },
                        1,
                    );
                }
            }
        }
        if is_home_library
            && !self.selected_library_instances.is_empty()
            && self.selected_instance.is_none()
            && !self.global_browse
            && !self.global_hosting
            && !self.global_skins
            && !self.global_screenshots
            && !self.settings_open
            && !self.accounts_open
            && !self.java_open
        {
            let bar_x = 292;
            let bar_y = h.saturating_sub(62);
            let bar_width = library_width.saturating_sub(324);
            rect(&mut buffer, w, h, bar_x, bar_y, bar_width, 48, 0x002e333a);
            text(
                &mut buffer,
                w,
                h,
                bar_x + 12,
                bar_y + 16,
                &format!("{} SELECTED", self.selected_library_instances.len()),
                0x00f1f3f5,
                1,
            );
            let clear_x = bar_x + bar_width.saturating_sub(72);
            rect(&mut buffer, w, h, clear_x, bar_y + 8, 64, 32, 0x0022262c);
            text(
                &mut buffer,
                w,
                h,
                clear_x + 12,
                bar_y + 18,
                "CLEAR",
                0x00bdc2ca,
                1,
            );
            if self.selected_group.is_some() {
                let add_x = clear_x.saturating_sub(108);
                rect(
                    &mut buffer,
                    w,
                    h,
                    add_x,
                    bar_y + 8,
                    100,
                    32,
                    if all_selected_are_members {
                        0x003b302b
                    } else {
                        0x00303b2b
                    },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    add_x + if all_selected_are_members { 2 } else { 10 },
                    bar_y + 18,
                    if all_selected_are_members {
                        "REMOVE GROUP"
                    } else {
                        "ADD TO GROUP"
                    },
                    if all_selected_are_members {
                        0x00ff8b82
                    } else {
                        0x00a8e66b
                    },
                    1,
                );
                let delete_x = add_x.saturating_sub(80);
                rect(
                    &mut buffer,
                    w,
                    h,
                    delete_x,
                    bar_y + 8,
                    72,
                    32,
                    0x003b302b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    delete_x + 12,
                    bar_y + 18,
                    "Delete",
                    0x00ff8b82,
                    1,
                );
            } else {
                let delete_x = clear_x.saturating_sub(80);
                rect(
                    &mut buffer,
                    w,
                    h,
                    delete_x,
                    bar_y + 8,
                    72,
                    32,
                    0x003b302b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    delete_x + 12,
                    bar_y + 18,
                    "DELETE",
                    0x00ff8b82,
                    1,
                );
            }
            let new_group_x =
                clear_x.saturating_sub(if self.selected_group.is_some() {
                    272
                } else {
                    164
                });
            rect(
                &mut buffer,
                w,
                h,
                new_group_x,
                bar_y + 8,
                76,
                32,
                0x002b3038,
            );
            text(
                &mut buffer,
                w,
                h,
                new_group_x + 8,
                bar_y + 18,
                "NEW GROUP",
                0x00d3d7dd,
                1,
            );
        }
        if self.global_hosting {
            rect(
                &mut buffer,
                w,
                h,
                64,
                48,
                library_width.saturating_sub(64),
                h.saturating_sub(48),
                0x001b1d22,
            );
            let selected_hosting_server = self.hosting_server_id.as_deref();
            let hosting_server =
                selected_hosting_server.and_then(|server_id| {
                    self.hosting_servers
                        .iter()
                        .find(|server| server.server_id == server_id)
                });
            let hosting_details = self.hosting_server_details.as_ref();
            let hosting_title = hosting_details
                .map(|details| details.server.name.as_str())
                .or_else(|| hosting_server.map(|server| server.name.as_str()))
                .unwrap_or("HOSTING");
            text(&mut buffer, w, h, 104, 78, hosting_title, 0x00f1f3f5, 2);
            text_clipped(
                &mut buffer,
                w,
                h,
                104,
                108,
                if selected_hosting_server.is_some() {
                    "SERVER OVERVIEW"
                } else {
                    "Your Modrinth servers"
                },
                0x00a8adb5,
                1,
                library_width.saturating_sub(144),
            );
            if selected_hosting_server.is_none() {
                let refresh = hosting_servers_refresh_button(library_width);
                let hovered = refresh.contains(cursor.0, cursor.1);
                let loading = self.hosting_servers_loading
                    || self.hosting_servers_refreshing;
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    refresh.x,
                    refresh.y,
                    refresh.width,
                    refresh.height,
                    8,
                    if loading {
                        0x00343a42
                    } else if hovered {
                        0x00b9f279
                    } else {
                        0x00a8e66b
                    },
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    refresh.x + 10,
                    refresh.y + 11,
                    if loading { "LOADING…" } else { "REFRESH" },
                    0x001b1d22,
                    1,
                    refresh.width.saturating_sub(20),
                );
            }
            if let Some(server_id) = selected_hosting_server {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    96,
                    130,
                    176,
                    32,
                    8,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    108,
                    141,
                    "←  ALL SERVERS",
                    0x00a8e66b,
                    1,
                );
                if let Some(details) = hosting_details {
                    let state = self
                        .hosting_power_state
                        .as_deref()
                        .unwrap_or(&details.server.status);
                    let running = state == "running";
                    let can_start = matches!(
                        state,
                        "stopped" | "idle" | "crashed" | "available"
                    );
                    let label = if self.hosting_power_pending {
                        "WORKING…"
                    } else if running {
                        "STOP SERVER"
                    } else if can_start {
                        "START SERVER"
                    } else {
                        "SERVER BUSY"
                    };
                    let enabled =
                        !self.hosting_power_pending && (running || can_start);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        288,
                        130,
                        136,
                        32,
                        8,
                        if enabled { 0x003b7d35 } else { 0x0022262c },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        300,
                        141,
                        label,
                        if enabled { 0x00f1f3f5 } else { 0x007f8791 },
                        1,
                    );
                }
                for (tab, x, width, label) in [
                    (0, 440, 96, "OVERVIEW"),
                    (1, 544, 96, "BACKUPS"),
                    (2, 648, 88, "ACCESS"),
                ] {
                    let hovered = (x as f64..(x + width) as f64)
                        .contains(&cursor.0)
                        && (130.0..162.0).contains(&cursor.1);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        x,
                        130,
                        width,
                        32,
                        8,
                        if self.hosting_tab == tab {
                            0x00303b2b
                        } else if hovered {
                            0x00343a42
                        } else {
                            0x0022262c
                        },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 12,
                        141,
                        label,
                        if self.hosting_tab == tab {
                            0x00a8e66b
                        } else {
                            0x00bdc2ca
                        },
                        1,
                    );
                }
                if self.hosting_server_details_loading {
                    text(
                        &mut buffer,
                        w,
                        h,
                        104,
                        196,
                        "Loading server details…",
                        0x00bdc2ca,
                        1,
                    );
                } else if let Some(error) =
                    self.hosting_server_details_error.as_deref()
                {
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        104,
                        196,
                        error,
                        0x00ff8b82,
                        1,
                        library_width.saturating_sub(144),
                    );
                } else if self.hosting_tab == 1 {
                    let panel_width =
                        library_width.saturating_sub(144).min(940);
                    let panel_height = h.saturating_sub(246);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        96,
                        182,
                        panel_width,
                        panel_height,
                        12,
                        0x0022262c,
                    );
                    let deleted_count = self
                        .hosting_backups
                        .as_ref()
                        .map_or(0, |backups| backups.deleted_backups.len());
                    if deleted_count > 0 {
                        for (x, label, selected) in [
                            (120, "HISTORY", !self.hosting_backup_show_deleted),
                            (184, "DELETED", self.hosting_backup_show_deleted),
                        ] {
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                194,
                                60,
                                32,
                                7,
                                if selected { 0x00343a42 } else { 0x0022262c },
                            );
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                x + 5,
                                205,
                                label,
                                if selected { 0x00f1f3f5 } else { 0x007f8791 },
                                1,
                                52,
                            );
                        }
                    } else {
                        text(
                            &mut buffer,
                            w,
                            h,
                            120,
                            202,
                            "BACKUP HISTORY",
                            0x00f1f3f5,
                            2,
                        );
                    }
                    let search_query =
                        self.hosting_backup_search.trim().to_lowercase();
                    let matching_indices = self
                        .hosting_backups
                        .as_ref()
                        .map_or_else(Vec::new, |backups| {
                            if self.hosting_backup_show_deleted {
                                let mut indices = backups
                                    .deleted_backups
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(index, backup)| {
                                        hosting_deleted_backup_matches(
                                            backup,
                                            &search_query,
                                            self.hosting_backup_deleted_reason_filter,
                                            self.hosting_backup_deleted_type_filter,
                                            self.hosting_backup_deleted_recoverability_filter,
                                        )
                                        .then_some(index)
                                    })
                                    .collect::<Vec<_>>();
                                indices.sort_by(|left, right| {
                                    hosting_deleted_backup_sort_order(
                                        &backups.deleted_backups[*left],
                                        &backups.deleted_backups[*right],
                                        self.hosting_backup_deleted_sort,
                                    )
                                });
                                indices
                            } else {
                                backups
                                    .backups
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(index, backup)| {
                                        (backup.status == "done"
                                            && hosting_backup_search_matches(
                                                &backup.name,
                                                &search_query,
                                            ))
                                        .then_some(index)
                                    })
                                    .collect::<Vec<_>>()
                            }
                        });
                    let total_backup_count =
                        self.hosting_backups.as_ref().map_or(0, |backups| {
                            if self.hosting_backup_show_deleted {
                                backups.deleted_backups.len()
                            } else {
                                backups
                                    .backups
                                    .iter()
                                    .filter(|backup| backup.status == "done")
                                    .count()
                            }
                        });
                    let backup_count = matching_indices.len();
                    let manual_count = self
                        .hosting_backups
                        .as_ref()
                        .map_or(0, |backups| backups.manual_count);
                    let manual_limit = self
                        .hosting_backups
                        .as_ref()
                        .map_or(0, |backups| backups.options.split_manual);
                    let auto_count = self
                        .hosting_backups
                        .as_ref()
                        .map_or(0, |backups| backups.auto_count);
                    let auto_limit = self
                        .hosting_backups
                        .as_ref()
                        .map_or(0, |backups| backups.options.split_automatic);
                    let auto_interval = self
                        .hosting_backups
                        .as_ref()
                        .map_or(0, |backups| backups.options.interval);
                    let next_auto_backup =
                        self.hosting_backups.as_ref().and_then(|backups| {
                            backups.next_auto_backup_at.as_deref()
                        });
                    let subtitle = if hosting_backup_selected_count > 0 {
                        format!(
                            "{hosting_backup_selected_count} backups selected"
                        )
                    } else if self.hosting_backup_show_deleted {
                        format!("{total_backup_count} deleted backups")
                    } else if auto_limit > 0 {
                        let countdown = hosting_backup_countdown_label(
                            next_auto_backup,
                            chrono::Utc::now(),
                            auto_count >= auto_limit,
                        );
                        format!(
                            "Auto {auto_count}/{auto_limit} · every {} · {countdown}",
                            hosting_backup_interval_label(auto_interval),
                        )
                    } else {
                        format!(
                            "Auto off · manual slots {manual_count}/{manual_limit}"
                        )
                    };
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        120,
                        224,
                        &subtitle,
                        0x00bdc2ca,
                        1,
                        panel_width.saturating_sub(208),
                    );
                    let create_x = 96 + panel_width.saturating_sub(156);
                    if hosting_backup_selected_count > 0 {
                        let delete_x = create_x.saturating_sub(160);
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            delete_x,
                            194,
                            148,
                            32,
                            8,
                            if self.hosting_backup_show_deleted {
                                if hosting_backup_selected_recoverable_count > 0
                                {
                                    0x00303b2b
                                } else {
                                    0x002e333a
                                }
                            } else if hosting_can_manage_backups {
                                0x003d2929
                            } else {
                                0x002e333a
                            },
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            delete_x + 10,
                            205,
                            if self.hosting_backup_show_deleted {
                                "RECOVER SELECTED"
                            } else {
                                "DELETE SELECTED"
                            },
                            if self.hosting_backup_show_deleted {
                                if hosting_backup_selected_recoverable_count > 0
                                {
                                    0x00a8e66b
                                } else {
                                    0x007f8791
                                }
                            } else if hosting_can_manage_backups {
                                0x00ffaaa3
                            } else {
                                0x007f8791
                            },
                            1,
                            128,
                        );
                    } else if self.hosting_backup_show_deleted
                        && !self.hosting_backup_options_open
                    {
                        let label = match self.hosting_backup_deleted_sort {
                            1 => "SORT: OLDEST",
                            2 => "SORT: NAME A-Z",
                            3 => "SORT: NAME Z-A",
                            4 => "SORT: CAUSE",
                            5 => "SORT: DEADLINE",
                            _ => "SORT: NEWEST",
                        };
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            create_x,
                            194,
                            132,
                            32,
                            8,
                            0x00343a42,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            create_x + 8,
                            205,
                            label,
                            0x00d3d7dd,
                            1,
                            116,
                        );
                    } else if !self.hosting_backup_show_deleted
                        && !self.hosting_backup_options_open
                        && !self.hosting_backup_create_open
                        && panel_width >= 520
                    {
                        let settings_x = create_x.saturating_sub(112);
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            settings_x,
                            194,
                            104,
                            32,
                            8,
                            0x00343a42,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            settings_x + 10,
                            205,
                            "AUTO SETTINGS",
                            0x00d3d7dd,
                            1,
                            88,
                        );
                    }
                    if !self.hosting_backup_options_open
                        && !self.hosting_backup_show_deleted
                    {
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            create_x,
                            194,
                            132,
                            32,
                            8,
                            if !hosting_can_manage_backups
                                || manual_count >= manual_limit
                            {
                                0x002e333a
                            } else {
                                0x00303b2b
                            },
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            create_x + 10,
                            205,
                            if self.hosting_backup_creating {
                                "CREATING…"
                            } else {
                                "CREATE BACKUP"
                            },
                            if !hosting_can_manage_backups
                                || manual_count >= manual_limit
                            {
                                0x007f8791
                            } else {
                                0x00a8e66b
                            },
                            1,
                            112,
                        );
                    }
                    let list_top = if self.hosting_backup_options_open {
                        436
                    } else if self.hosting_backup_show_deleted {
                        328
                    } else {
                        296
                    };
                    if self.hosting_backup_create_open {
                        let field_width =
                            panel_width.saturating_sub(252).max(120);
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            120,
                            248,
                            field_width,
                            32,
                            8,
                            0x00141619,
                        );
                        text_input_clipped(
                            &mut buffer,
                            w,
                            h,
                            132,
                            257,
                            if self.hosting_backup_create_name.is_empty() {
                                "Backup name (optional)"
                            } else {
                                &self.hosting_backup_create_name
                            },
                            if self.input_focus == 38 {
                                0x00f1f3f5
                            } else {
                                0x007f8791
                            },
                            1,
                            field_width.saturating_sub(24),
                        );
                        let submit_x = 128 + field_width;
                        for (x, label, color) in [
                            (
                                submit_x,
                                if self.hosting_backup_creating {
                                    "QUEUE"
                                } else {
                                    "CREATE"
                                },
                                0x00303b2b,
                            ),
                            (submit_x + 92, "CANCEL", 0x00343a42),
                        ] {
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                248,
                                84,
                                32,
                                8,
                                color,
                            );
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                x + 8,
                                259,
                                label,
                                0x00f1f3f5,
                                1,
                                68,
                            );
                        }
                    } else if self.hosting_backup_options_open {
                        let slot_limit = self
                            .hosting_backups
                            .as_ref()
                            .map_or(0, |backups| backups.options.limit.min(15));
                        let toggle_label = if self.hosting_backup_auto_enabled {
                            "AUTOMATIC BACKUPS  ·  ON"
                        } else {
                            "AUTOMATIC BACKUPS  ·  OFF"
                        };
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            120,
                            248,
                            220,
                            32,
                            8,
                            if self.hosting_backup_auto_enabled {
                                0x00303b2b
                            } else {
                                0x00343a42
                            },
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            132,
                            259,
                            toggle_label,
                            0x00f1f3f5,
                            1,
                            196,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            120,
                            282,
                            "Automatically back up your server at regular intervals.",
                            0x00bdc2ca,
                            1,
                            panel_width.saturating_sub(144),
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            120,
                            294,
                            "BACKUP INTERVAL",
                            0x00bdc2ca,
                            1,
                        );
                        let choices = hosting_backup_interval_choices(
                            self.hosting_backup_interval_hours,
                        );
                        let interval_width = 56usize;
                        let interval_gap = 8usize;
                        for (index, interval) in choices.iter().enumerate() {
                            let x =
                                120 + index * (interval_width + interval_gap);
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                306,
                                interval_width,
                                32,
                                7,
                                if *interval
                                    == self.hosting_backup_interval_hours
                                {
                                    0x00343a42
                                } else {
                                    if self.hosting_backup_auto_enabled {
                                        0x0022262c
                                    } else {
                                        0x001c2025
                                    }
                                },
                            );
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                x + 5,
                                317,
                                &hosting_backup_interval_label(*interval),
                                if self.hosting_backup_auto_enabled {
                                    0x00f1f3f5
                                } else {
                                    0x007f8791
                                },
                                1,
                                interval_width - 10,
                            );
                        }
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            120,
                            352,
                            &format!(
                                "MANUAL SLOTS  ·  {} / {slot_limit}",
                                self.hosting_backup_manual_slots
                            ),
                            0x00bdc2ca,
                            1,
                            220,
                        );
                        for (x, label) in [(344usize, "−"), (390, "+")] {
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                342,
                                36,
                                32,
                                7,
                                if self.hosting_backup_auto_enabled {
                                    0x00343a42
                                } else {
                                    0x002e333a
                                },
                            );
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                x + 12,
                                351,
                                label,
                                0x00f1f3f5,
                                2,
                                16,
                            );
                        }
                        for (x, width, label, color) in [
                            (120usize, 104usize, "SAVE", 0x00303b2b),
                            (232, 104, "CANCEL", 0x00343a42),
                        ] {
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                390,
                                width,
                                32,
                                8,
                                if self.hosting_backup_options_saving {
                                    0x002e333a
                                } else {
                                    color
                                },
                            );
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                x + 10,
                                401,
                                if label == "SAVE"
                                    && self.hosting_backup_options_saving
                                {
                                    "SAVING…"
                                } else {
                                    label
                                },
                                0x00f1f3f5,
                                1,
                                width - 20,
                            );
                        }
                    } else {
                        let field_width = panel_width.saturating_sub(48);
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            120,
                            248,
                            field_width,
                            32,
                            8,
                            0x00141619,
                        );
                        text_input_clipped(
                            &mut buffer,
                            w,
                            h,
                            132,
                            257,
                            if self.hosting_backup_search.is_empty() {
                                if self.hosting_backup_show_deleted {
                                    "Search deleted backups by name"
                                } else {
                                    "Search backups by name"
                                }
                            } else {
                                &self.hosting_backup_search
                            },
                            if self.input_focus == 39 {
                                0x00f1f3f5
                            } else {
                                0x007f8791
                            },
                            1,
                            field_width.saturating_sub(24),
                        );
                        if self.hosting_backup_show_deleted {
                            let filters = [
                                (
                                    112,
                                    80,
                                    match self.hosting_backup_deleted_reason_filter {
                                        1 => "CAUSE:SUSP",
                                        2 => "CAUSE:DEL",
                                        _ => "CAUSE:ANY",
                                    },
                                ),
                                (
                                    198,
                                    64,
                                    match self.hosting_backup_deleted_type_filter {
                                        1 => "TYPE:AUTO",
                                        2 => "TYPE:MAN",
                                        _ => "TYPE:ANY",
                                    },
                                ),
                                (
                                    268,
                                    112,
                                    match self.hosting_backup_deleted_recoverability_filter {
                                        1 => "RECOV:YES",
                                        2 => "RECOV:NO",
                                        _ => "RECOV:ANY",
                                    },
                                ),
                            ];
                            for (x, width, label) in filters {
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    x,
                                    286,
                                    width,
                                    32,
                                    7,
                                    0x00343a42,
                                );
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    x + 6,
                                    297,
                                    label,
                                    0x00d3d7dd,
                                    1,
                                    width.saturating_sub(12),
                                );
                            }
                        }
                    }
                    if self.hosting_backups_loading {
                        text(
                            &mut buffer,
                            w,
                            h,
                            120,
                            list_top + 14,
                            "Loading backups…",
                            0x00bdc2ca,
                            1,
                        );
                    } else if let Some(error) =
                        self.hosting_backups_error.as_deref()
                    {
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            120,
                            list_top + 14,
                            error,
                            0x00ff8b82,
                            1,
                            panel_width.saturating_sub(48),
                        );
                    } else if backup_count == 0 {
                        text(
                            &mut buffer,
                            w,
                            h,
                            120,
                            list_top + 14,
                            if total_backup_count == 0
                                && !self.hosting_backup_show_deleted
                            {
                                "No backups yet. Create one to protect this world."
                            } else if total_backup_count == 0 {
                                "No deleted backups are available."
                            } else {
                                "No backups match your search."
                            },
                            0x00bdc2ca,
                            1,
                        );
                    } else if let Some(backups) = &self.hosting_backups {
                        let viewport_height = h.saturating_sub(list_top + 54);
                        let max_scroll = backup_count
                            .saturating_mul(62)
                            .saturating_sub(viewport_height)
                            as f64;
                        self.hosting_backup_scroll_target_pixels = self
                            .hosting_backup_scroll_target_pixels
                            .clamp(0.0, max_scroll);
                        self.hosting_backup_scroll_pixels = self
                            .hosting_backup_scroll_pixels
                            .clamp(0.0, max_scroll);
                        render::set_clip_rect(Some((
                            112,
                            list_top,
                            panel_width.saturating_sub(32),
                            viewport_height,
                        )));
                        let offset = self.hosting_backup_scroll_pixels;
                        let first = (offset / 62.0).floor() as usize;
                        if self.hosting_backup_show_deleted {
                            for (display_index, backup_index) in
                                matching_indices
                                    .iter()
                                    .copied()
                                    .enumerate()
                                    .skip(first)
                                    .take(viewport_height / 62 + 2)
                            {
                                let Some(backup) =
                                    backups.deleted_backups.get(backup_index)
                                else {
                                    continue;
                                };
                                let y = (list_top as f64
                                    + display_index as f64 * 62.0
                                    - offset)
                                    .max(0.0)
                                    .round()
                                    as usize;
                                let selected = hosting_backup_selected
                                    .contains(&backup.id);
                                let row_right =
                                    112 + panel_width.saturating_sub(32);
                                let [
                                    selection_bounds,
                                    copy_bounds,
                                    recover_bounds,
                                ] = hosting_deleted_backup_row_controls(
                                    row_right, y,
                                );
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    112,
                                    y,
                                    panel_width.saturating_sub(32),
                                    54,
                                    8,
                                    0x001b1d22,
                                );
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    selection_bounds.x,
                                    selection_bounds.y,
                                    selection_bounds.width,
                                    selection_bounds.height,
                                    4,
                                    if selected {
                                        0x005a9142
                                    } else {
                                        0x00343a42
                                    },
                                );
                                if selected {
                                    text_clipped(
                                        &mut buffer,
                                        w,
                                        h,
                                        123,
                                        y + 21,
                                        "✓",
                                        0x00f1f3f5,
                                        1,
                                        12,
                                    );
                                }
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    174,
                                    y + 8,
                                    &backup.name,
                                    0x00f1f3f5,
                                    1,
                                    panel_width.saturating_sub(236),
                                );
                                let details = format!(
                                    "{}  ·  {}  ·  {}  ·  {}",
                                    hosting_backup_creator_name(
                                        backup.created_by.as_ref(),
                                        backup.automated,
                                    )
                                    .unwrap_or_else(|| "Unknown".into()),
                                    if backup.deleted_suspension {
                                        "SUSPENDED"
                                    } else {
                                        "DELETED"
                                    },
                                    backup.created_at,
                                    if backup.can_undelete {
                                        format!(
                                            "recover by {}",
                                            backup.undelete_deadline
                                        )
                                    } else {
                                        "unrecoverable".into()
                                    }
                                );
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    174,
                                    y + 28,
                                    &details,
                                    0x007f8791,
                                    1,
                                    panel_width.saturating_sub(236),
                                );
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    copy_bounds.x,
                                    copy_bounds.y,
                                    copy_bounds.width,
                                    copy_bounds.height,
                                    6,
                                    0x00343a42,
                                );
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    copy_bounds.x + 6,
                                    copy_bounds.y + 6,
                                    "ID",
                                    0x00d3d7dd,
                                    1,
                                    copy_bounds.width - 12,
                                );
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    recover_bounds.x,
                                    recover_bounds.y,
                                    recover_bounds.width,
                                    recover_bounds.height,
                                    7,
                                    if backup.can_undelete
                                        && !hosting_backup_mutating
                                    {
                                        0x00303b2b
                                    } else {
                                        0x002e333a
                                    },
                                );
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    recover_bounds.x + 8,
                                    recover_bounds.y + 11,
                                    "RECOVER",
                                    if backup.can_undelete
                                        && !hosting_backup_mutating
                                    {
                                        0x00a8e66b
                                    } else {
                                        0x007f8791
                                    },
                                    1,
                                    84,
                                );
                            }
                        } else {
                            for (display_index, backup_index) in
                                matching_indices
                                    .iter()
                                    .copied()
                                    .enumerate()
                                    .skip(first)
                                    .take(viewport_height / 62 + 2)
                            {
                                let Some(backup) =
                                    backups.backups.get(backup_index)
                                else {
                                    continue;
                                };
                                let can_manage = hosting_can_manage_backups;
                                let can_restore = can_manage
                                    && !hosting_server_is_running
                                    && backup.status == "done";
                                let y = (list_top as f64
                                    + display_index as f64 * 62.0
                                    - offset)
                                    .max(0.0)
                                    .round()
                                    as usize;
                                let creator = backup
                                    .history
                                    .iter()
                                    .find(|entry| {
                                        entry.operation_type == "create"
                                    })
                                    .and_then(|entry| entry.user_info.as_ref());
                                let creator = hosting_backup_creator_name(
                                    creator,
                                    backup.automated,
                                )
                                .unwrap_or_else(|| "Unknown".into());
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    112,
                                    y,
                                    panel_width.saturating_sub(32),
                                    54,
                                    8,
                                    0x001b1d22,
                                );
                                let is_selected = hosting_backup_selected
                                    .contains(&backup.id);
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    120,
                                    y + 17,
                                    20,
                                    20,
                                    5,
                                    if is_selected {
                                        0x007fcf56
                                    } else {
                                        0x00343a42
                                    },
                                );
                                if is_selected {
                                    text_clipped(
                                        &mut buffer,
                                        w,
                                        h,
                                        123,
                                        y + 20,
                                        "✓",
                                        0x00141619,
                                        1,
                                        14,
                                    );
                                }
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    152,
                                    y + 8,
                                    &backup.name,
                                    0x00f1f3f5,
                                    1,
                                    panel_width.saturating_sub(364),
                                );
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    152,
                                    y + 28,
                                    &format!(
                                        "{}  ·  {}  ·  {}",
                                        creator,
                                        backup.status.to_uppercase(),
                                        backup.created_at
                                    ),
                                    0x007f8791,
                                    1,
                                    panel_width.saturating_sub(364),
                                );
                                let row_right =
                                    112 + panel_width.saturating_sub(32);
                                let action_bounds =
                                    hosting_backup_action_bounds(
                                        row_right as f64,
                                    );
                                let actions = [
                                    (
                                        if hosting_backup_downloading {
                                            "WAIT"
                                        } else {
                                            "DOWNLOAD"
                                        },
                                        backup.status == "done"
                                            && !hosting_backup_downloading,
                                    ),
                                    ("RESTORE", can_restore),
                                    (
                                        "RENAME",
                                        can_manage && backup.status == "done",
                                    ),
                                    (
                                        "DELETE",
                                        can_manage && backup.status == "done",
                                    ),
                                ];
                                for ((left, right), (label, enabled)) in
                                    action_bounds.into_iter().zip(actions)
                                {
                                    let x = left.round() as usize;
                                    let width = (right - left).round() as usize;
                                    rounded_rect(
                                        &mut buffer,
                                        w,
                                        h,
                                        x,
                                        y + 11,
                                        width,
                                        32,
                                        7,
                                        if !enabled {
                                            0x002e333a
                                        } else if label == "DELETE" {
                                            0x003d2929
                                        } else {
                                            0x00343a42
                                        },
                                    );
                                    text_clipped(
                                        &mut buffer,
                                        w,
                                        h,
                                        x + 6,
                                        y + 22,
                                        label,
                                        if enabled {
                                            0x00d3d7dd
                                        } else {
                                            0x007f8791
                                        },
                                        1,
                                        width.saturating_sub(12),
                                    );
                                }
                            }
                        }
                        render::set_clip_rect(None);
                    }
                } else if self.hosting_tab == 2 {
                    let panel_width =
                        library_width.saturating_sub(144).min(940);
                    let max_scroll = self
                        .hosting_access_users
                        .len()
                        .saturating_mul(62)
                        .saturating_sub(h.saturating_sub(278))
                        as f64;
                    self.hosting_access_scroll_target_pixels = self
                        .hosting_access_scroll_target_pixels
                        .clamp(0.0, max_scroll);
                    self.hosting_access_scroll_pixels = self
                        .hosting_access_scroll_pixels
                        .clamp(0.0, max_scroll);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        96,
                        182,
                        panel_width,
                        h.saturating_sub(246),
                        12,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        120,
                        202,
                        "PEOPLE WITH ACCESS",
                        0x00f1f3f5,
                        2,
                    );
                    let invite_x = 96 + panel_width.saturating_sub(156);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        invite_x,
                        194,
                        132,
                        32,
                        8,
                        if self.hosting_access_invite_open {
                            0x00343a42
                        } else {
                            0x00303b2b
                        },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        invite_x + 12,
                        205,
                        if self.hosting_access_invite_open {
                            "CLOSE"
                        } else {
                            "INVITE USER"
                        },
                        if self.hosting_access_loading {
                            0x007f8791
                        } else {
                            0x00f1f3f5
                        },
                        1,
                    );
                    if self.hosting_access_invite_open {
                        let search_width =
                            panel_width.saturating_sub(200).clamp(120, 360);
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            120,
                            248,
                            search_width,
                            32,
                            8,
                            0x00141619,
                        );
                        text_input_clipped(
                            &mut buffer,
                            w,
                            h,
                            132,
                            257,
                            if self.hosting_access_search_query.is_empty() {
                                "Search for a username"
                            } else {
                                &self.hosting_access_search_query
                            },
                            if self.input_focus == 34 {
                                0x00f1f3f5
                            } else {
                                0x007f8791
                            },
                            1,
                            search_width.saturating_sub(24),
                        );
                        let search_button_x = 132 + search_width;
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            search_button_x,
                            248,
                            88,
                            32,
                            8,
                            0x002e333a,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            search_button_x + 14,
                            259,
                            if self.hosting_access_search_loading {
                                "SEARCHING"
                            } else {
                                "SEARCH"
                            },
                            0x00f1f3f5,
                            1,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            120,
                            300,
                            "ROLE",
                            0x007f8791,
                            1,
                        );
                        for (role, x, label) in
                            [(0, 176, "VIEWER"), (1, 268, "EDITOR")]
                        {
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                292,
                                84,
                                32,
                                8,
                                if self.hosting_access_role == role {
                                    0x00303b2b
                                } else {
                                    0x0022262c
                                },
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                x + 12,
                                303,
                                label,
                                if self.hosting_access_role == role {
                                    0x00a8e66b
                                } else {
                                    0x00bdc2ca
                                },
                                1,
                            );
                        }
                        if let Some(error) =
                            self.hosting_access_invite_error.as_deref()
                        {
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                120,
                                332,
                                error,
                                0x00ff8b82,
                                1,
                                panel_width.saturating_sub(48),
                            );
                        } else if !self.hosting_access_search_query.is_empty()
                            && self.hosting_access_search_complete
                            && !self.hosting_access_search_loading
                            && self.hosting_access_search_results.is_empty()
                        {
                            text(
                                &mut buffer,
                                w,
                                h,
                                120,
                                332,
                                "No matching users found.",
                                0x00a8adb5,
                                1,
                            );
                        }
                        let existing_ids = self
                            .hosting_access_users
                            .iter()
                            .map(|member| member.user.id.to_lowercase())
                            .collect::<HashSet<_>>();
                        let candidates = self
                            .hosting_access_search_results
                            .iter()
                            .filter(|user| {
                                !existing_ids.contains(&user.id.to_lowercase())
                            })
                            .collect::<Vec<_>>();
                        let viewport_height = h.saturating_sub(436);
                        let max_search_scroll = candidates
                            .len()
                            .saturating_mul(54)
                            .saturating_sub(viewport_height)
                            as f64;
                        self.hosting_access_search_scroll_target_pixels = self
                            .hosting_access_search_scroll_target_pixels
                            .clamp(0.0, max_search_scroll);
                        self.hosting_access_search_scroll_pixels = self
                            .hosting_access_search_scroll_pixels
                            .clamp(0.0, max_search_scroll);
                        let search_row_offset =
                            (self.hosting_access_search_scroll_pixels / 54.0)
                                .floor() as usize;
                        let search_scroll_remainder =
                            (self.hosting_access_search_scroll_pixels % 54.0)
                                .round() as usize;
                        let visible_search_rows =
                            viewport_height.saturating_add(53) / 54;
                        if !candidates.is_empty() {
                            render::set_clip_rect(Some((
                                112,
                                360,
                                panel_width.saturating_sub(32),
                                viewport_height,
                            )));
                            for (index, user) in candidates
                                .iter()
                                .skip(search_row_offset)
                                .take(visible_search_rows)
                                .enumerate()
                            {
                                let y = (360 + index * 54)
                                    .saturating_sub(search_scroll_remainder);
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    112,
                                    y,
                                    panel_width.saturating_sub(32),
                                    48,
                                    8,
                                    0x001b1d22,
                                );
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    128,
                                    y + 8,
                                    &user.username,
                                    0x00f1f3f5,
                                    1,
                                    panel_width.saturating_sub(216),
                                );
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    128,
                                    y + 26,
                                    &user.id,
                                    0x007f8791,
                                    1,
                                    panel_width.saturating_sub(216),
                                );
                                let add_x =
                                    96 + panel_width.saturating_sub(112);
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    add_x,
                                    y + 8,
                                    88,
                                    32,
                                    8,
                                    0x00303b2b,
                                );
                                text(
                                    &mut buffer,
                                    w,
                                    h,
                                    add_x + 14,
                                    y + 19,
                                    if self.hosting_access_loading {
                                        "ADDING…"
                                    } else {
                                        "INVITE"
                                    },
                                    0x00a8e66b,
                                    1,
                                );
                            }
                            render::set_clip_rect(None);
                        } else if self.hosting_access_search_complete
                            && self.hosting_access_invite_error.is_none()
                        {
                            text(
                                &mut buffer,
                                w,
                                h,
                                120,
                                366,
                                "Everyone matching already has access.",
                                0x007f8791,
                                1,
                            );
                        } else if self.hosting_access_search_results.is_empty()
                            && self.hosting_access_invite_error.is_none()
                            && !self.hosting_access_search_loading
                            && !self.hosting_access_search_complete
                        {
                            text(
                                &mut buffer,
                                w,
                                h,
                                120,
                                366,
                                if self.hosting_access_search_query.is_empty() {
                                    "Search by username to invite someone."
                                } else {
                                    "Press Enter or Search to find this user."
                                },
                                0x007f8791,
                                1,
                            );
                        }
                    } else if self.hosting_access_loading {
                        text(
                            &mut buffer,
                            w,
                            h,
                            120,
                            248,
                            "Loading access…",
                            0x00bdc2ca,
                            1,
                        );
                    } else if let Some(error) =
                        self.hosting_access_error.as_deref()
                    {
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            120,
                            248,
                            error,
                            0x00ff8b82,
                            1,
                            panel_width.saturating_sub(48),
                        );
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            120,
                            280,
                            88,
                            32,
                            8,
                            0x00343a42,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            137,
                            290,
                            "RETRY",
                            0x00f1f3f5,
                            1,
                        );
                    } else if self.hosting_access_users.is_empty() {
                        text(
                            &mut buffer,
                            w,
                            h,
                            120,
                            248,
                            "No other users have access to this server.",
                            0x00a8adb5,
                            1,
                        );
                    } else {
                        let columns = hosting_access_row_layout(panel_width, 0);
                        text(
                            &mut buffer,
                            w,
                            h,
                            120,
                            244,
                            "USER",
                            0x007f8791,
                            1,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            columns.role.x,
                            244,
                            "ROLE",
                            0x007f8791,
                            1,
                        );
                        if columns.joined.width > 0 {
                            text(
                                &mut buffer,
                                w,
                                h,
                                columns.joined.x,
                                244,
                                "STATUS",
                                0x007f8791,
                                1,
                            );
                        }
                        text(
                            &mut buffer,
                            w,
                            h,
                            columns.remove.x,
                            244,
                            "ACTIONS",
                            0x007f8791,
                            1,
                        );
                        let owner_id = hosting_details
                            .map(|details| details.server.owner_id.as_str())
                            .or_else(|| {
                                hosting_server
                                    .map(|server| server.owner_id.as_str())
                            });
                        let viewport_top = 264usize;
                        let viewport_height = h.saturating_sub(326);
                        render::set_clip_rect(Some((
                            112,
                            viewport_top,
                            panel_width.saturating_sub(32),
                            viewport_height,
                        )));
                        let offset = self.hosting_access_scroll_pixels;
                        let first = (offset / 62.0).floor() as usize;
                        let count = viewport_height / 62 + 2;
                        for (index, entry) in self
                            .hosting_access_users
                            .iter()
                            .enumerate()
                            .skip(first)
                            .take(count)
                        {
                            let row_y = 264.0 + index as f64 * 62.0 - offset;
                            let y = row_y.max(0.0).round() as usize;
                            if y + 54 <= h.saturating_sub(62) {
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    112,
                                    y,
                                    panel_width.saturating_sub(32),
                                    54,
                                    8,
                                    0x001b1d22,
                                );
                            }
                            let row = hosting_access_row_layout(panel_width, y);
                            let pending_invite = entry.added_on.is_none();
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                row.user.x,
                                row.user.y,
                                &entry.user.username,
                                0x00f1f3f5,
                                1,
                                if pending_invite {
                                    row.user.width.saturating_sub(32)
                                } else {
                                    row.user.width
                                },
                            );
                            let member_role = entry.role();
                            let is_owner = owner_id.is_some_and(|id| {
                                id.eq_ignore_ascii_case(&entry.user.id)
                            }) || matches!(
                                member_role,
                                native_backend::HostingServerAccessRole::Owner
                            );
                            let role = match member_role {
                                native_backend::HostingServerAccessRole::Owner => "OWNER",
                                native_backend::HostingServerAccessRole::Viewer => "VIEWER",
                                native_backend::HostingServerAccessRole::Editor => "EDITOR",
                            };
                            let confirming = self
                                .hosting_access_remove_confirmation
                                .as_deref()
                                == Some(entry.user.id.as_str());
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                row.role.x,
                                row.role.y,
                                row.role.width,
                                row.role.height,
                                7,
                                if is_owner { 0x00303b2b } else { 0x00343a42 },
                            );
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                row.role.x + 8,
                                row.role.y + 11,
                                role,
                                if is_owner { 0x00a8e66b } else { 0x00bdc2ca },
                                1,
                                row.role.width.saturating_sub(16),
                            );
                            if row.joined.width > 0 {
                                text(
                                    &mut buffer,
                                    w,
                                    h,
                                    row.joined.x,
                                    row.joined.y + 11,
                                    if entry.added_on.is_some() {
                                        "ACTIVE"
                                    } else {
                                        "INVITED"
                                    },
                                    if entry.added_on.is_some() {
                                        0x00a8e66b
                                    } else {
                                        0x00bdc2ca
                                    },
                                    1,
                                );
                            }
                            if pending_invite && !is_owner {
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    row.cancel_invite.x,
                                    row.cancel_invite.y,
                                    row.cancel_invite.width,
                                    row.cancel_invite.height,
                                    7,
                                    if confirming {
                                        0x006a3030
                                    } else {
                                        0x00343a42
                                    },
                                );
                                text(
                                    &mut buffer,
                                    w,
                                    h,
                                    row.cancel_invite.x + 8,
                                    row.cancel_invite.y + 11,
                                    if confirming { "✓" } else { "×" },
                                    if confirming {
                                        0x00ffaaaa
                                    } else {
                                        0x00d3d7dd
                                    },
                                    1,
                                );
                            }
                            if !is_owner {
                                rounded_rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    row.remove.x,
                                    row.remove.y,
                                    row.remove.width,
                                    row.remove.height,
                                    7,
                                    if confirming {
                                        0x006a3030
                                    } else {
                                        0x00343a42
                                    },
                                );
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    row.remove.x + 5,
                                    row.remove.y + 11,
                                    if confirming {
                                        "CONFIRM"
                                    } else if entry.added_on.is_none() {
                                        "RESEND"
                                    } else {
                                        "REMOVE"
                                    },
                                    if confirming {
                                        0x00ffaaaa
                                    } else {
                                        0x00d3d7dd
                                    },
                                    1,
                                    62,
                                );
                            }
                        }
                        render::set_clip_rect(None);
                    }
                } else if let Some(details) = hosting_details {
                    let server = &details.server;
                    let panel_width =
                        library_width.saturating_sub(144).min(940);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        96,
                        182,
                        panel_width,
                        242,
                        12,
                        0x0022262c,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        120,
                        202,
                        &server.name,
                        0x00f1f3f5,
                        2,
                        panel_width.saturating_sub(48),
                    );
                    if let Some((
                        cpu,
                        ram_used,
                        ram_total,
                        storage_used,
                        storage_total,
                    )) = self.hosting_live_stats
                    {
                        let mib = |bytes: u64| bytes as f64 / 1_048_576.0;
                        let metrics = format!(
                            "CPU {cpu:.1}%  ·  RAM {:.0}/{:.0} MiB  ·  Storage {:.0}/{:.0} MiB",
                            mib(ram_used),
                            mib(ram_total),
                            mib(storage_used),
                            mib(storage_total)
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            120,
                            370,
                            &metrics,
                            0x00a8adb5,
                            1,
                            panel_width.saturating_sub(48),
                        );
                    }
                    let version =
                        server.mc_version.as_deref().unwrap_or("Minecraft");
                    let loader = server.loader.as_deref().unwrap_or("Vanilla");
                    let platform =
                        format!("{}  ·  {loader} {}", server.game, version);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        120,
                        238,
                        &platform,
                        0x00bdc2ca,
                        1,
                        panel_width.saturating_sub(48),
                    );
                    let address =
                        format!("{}:{}", server.net.domain, server.net.port);
                    text(&mut buffer, w, h, 120, 282, "ADDRESS", 0x007f8791, 1);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        120,
                        302,
                        &address,
                        0x00f1f3f5,
                        1,
                        panel_width.saturating_sub(144),
                    );
                    let copy_x = 96 + panel_width.saturating_sub(88);
                    let copy_hovered = (copy_x as f64..(copy_x + 72) as f64)
                        .contains(&cursor.0)
                        && (296.0..326.0).contains(&cursor.1);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        copy_x,
                        296,
                        72,
                        30,
                        7,
                        if copy_hovered { 0x00343a42 } else { 0x002e333a },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        copy_x + 15,
                        305,
                        if self.hosting_address_copied {
                            "COPIED"
                        } else {
                            "COPY"
                        },
                        if self.hosting_address_copied {
                            0x00a8e66b
                        } else {
                            0x00d3d7dd
                        },
                        1,
                    );
                    let status = format!("STATUS  ·  {}", server.status);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        120,
                        350,
                        &status,
                        0x00a8e66b,
                        1,
                        panel_width.saturating_sub(48),
                    );
                    let format_bytes = |bytes: u64| {
                        let mut value = bytes as f64;
                        let units = ["B", "KiB", "MiB", "GiB", "TiB"];
                        let mut unit = 0;
                        while value >= 1024.0 && unit < units.len() - 1 {
                            value /= 1024.0;
                            unit += 1;
                        }
                        format!("{value:.1} {}", units[unit])
                    };
                    let backup = format!(
                        "Backups  ·  {} / {}",
                        format_bytes(details.used_backup_quota),
                        format_bytes(details.backup_quota)
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        120,
                        390,
                        &backup,
                        0x00a8adb5,
                        1,
                        panel_width.saturating_sub(48),
                    );
                    let notice = details.notices.first();
                    let datacenter = details.datacenter.as_deref();
                    let metadata_column_width =
                        panel_width.saturating_sub(72) / 2;
                    if let Some(datacenter) = datacenter {
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            120,
                            410,
                            datacenter,
                            0x007f8791,
                            1,
                            if notice.is_some() {
                                metadata_column_width
                            } else {
                                panel_width.saturating_sub(48)
                            },
                        );
                    }
                    if let Some(notice) = notice {
                        let notice_text =
                            format!("{}  ·  {}", notice.title, notice.message);
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            if datacenter.is_some() {
                                144 + metadata_column_width
                            } else {
                                120
                            },
                            410,
                            &notice_text,
                            0x00a8adb5,
                            1,
                            if datacenter.is_some() {
                                metadata_column_width
                            } else {
                                panel_width.saturating_sub(48)
                            },
                        );
                    }
                    let console_top = 444usize;
                    let console_bottom = h.saturating_sub(66);
                    if console_bottom > console_top + 56 {
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            96,
                            console_top,
                            panel_width,
                            console_bottom - console_top,
                            12,
                            0x00141619,
                        );
                        let socket_status = if self.hosting_socket_connected {
                            "CONNECTED"
                        } else if self.hosting_socket_connecting {
                            "CONNECTING…"
                        } else {
                            "DISCONNECTED"
                        };
                        let console_error = self
                            .hosting_socket_error
                            .as_deref()
                            .or(self.hosting_power_error.as_deref());
                        text(
                            &mut buffer,
                            w,
                            h,
                            116,
                            console_top + 14,
                            "CONSOLE",
                            0x00f1f3f5,
                            1,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            230,
                            console_top + 14,
                            console_error.unwrap_or(socket_status),
                            if console_error.is_some() {
                                0x00ff8b82
                            } else {
                                0x007f8791
                            },
                            1,
                            panel_width.saturating_sub(260),
                        );
                        let console_bounds = hosting_console_log_bounds(
                            w,
                            h,
                            show_right_sidebar,
                        );
                        let viewport_height = console_bounds.height;
                        let max_scroll = hosting_console_max_scroll(
                            self.hosting_console_lines.len(),
                            viewport_height,
                        );
                        if self.hosting_console_follow_tail {
                            self.hosting_console_scroll_pixels = max_scroll;
                            self.hosting_console_scroll_target_pixels =
                                max_scroll;
                        } else {
                            self.hosting_console_scroll_pixels = self
                                .hosting_console_scroll_pixels
                                .clamp(0.0, max_scroll);
                            self.hosting_console_scroll_target_pixels = self
                                .hosting_console_scroll_target_pixels
                                .clamp(0.0, max_scroll);
                        }
                        let scroll_offset = self.hosting_console_scroll_pixels;
                        let visible_lines = hosting_console_visible_range(
                            self.hosting_console_lines.len(),
                            viewport_height,
                            scroll_offset,
                        );
                        render::set_clip_rect(Some((
                            console_bounds.x,
                            console_bounds.y,
                            console_bounds.width,
                            console_bounds.height,
                        )));
                        for index in visible_lines {
                            let Some(line) =
                                self.hosting_console_lines.get(index)
                            else {
                                continue;
                            };
                            let line_y = (console_bounds.y as f64
                                + index as f64
                                    * HOSTING_CONSOLE_ROW_HEIGHT as f64
                                - scroll_offset)
                                .round()
                                .max(0.0)
                                as usize;
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                console_bounds.x + 4,
                                line_y + 2,
                                line,
                                0x00bdc2ca,
                                1,
                                console_bounds.width.saturating_sub(16),
                            );
                        }
                        render::set_clip_rect(None);
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            112,
                            console_bottom - 36,
                            panel_width.saturating_sub(32),
                            26,
                            6,
                            0x0022262c,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            122,
                            console_bottom - 28,
                            if self.hosting_console_input.is_empty() {
                                "Type a server command…"
                            } else {
                                &self.hosting_console_input
                            },
                            0x00d3d7dd,
                            1,
                            panel_width.saturating_sub(56),
                        );
                    }
                    let _ = server_id;
                } else {
                    text(
                        &mut buffer,
                        w,
                        h,
                        104,
                        196,
                        "Server details are not available.",
                        0x00bdc2ca,
                        1,
                    );
                }
            } else if self.hosting_servers_loading {
                text(
                    &mut buffer,
                    w,
                    h,
                    104,
                    160,
                    "Loading servers…",
                    0x00bdc2ca,
                    1,
                );
            } else if let Some(error) = self.hosting_servers_error.as_deref() {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    104,
                    160,
                    error,
                    0x00ff8b82,
                    1,
                    library_width.saturating_sub(144),
                );
            } else if self.hosting_servers.is_empty() {
                text(
                    &mut buffer,
                    w,
                    h,
                    104,
                    160,
                    "No Hosting servers found for this account.",
                    0x00bdc2ca,
                    1,
                );
            } else {
                let row_width = library_width.saturating_sub(144).min(900);
                render::set_clip_rect(Some((
                    96,
                    148,
                    row_width,
                    h.saturating_sub(190),
                )));
                for (index, server) in self.hosting_servers.iter().enumerate() {
                    let row_y = 148.0 + index as f64 * 82.0
                        - self.hosting_scroll_pixels;
                    let row_bottom = row_y + 66.0;
                    let visible_top = row_y.max(148.0);
                    let visible_bottom =
                        row_bottom.min(h.saturating_sub(42) as f64);
                    if visible_top >= visible_bottom {
                        continue;
                    }
                    let y = row_y.max(0.0).round() as usize;
                    let visible_y = visible_top.round() as usize;
                    let visible_height = (visible_bottom.round() as usize)
                        .saturating_sub(visible_y);
                    if row_y >= 148.0
                        && row_bottom <= h.saturating_sub(42) as f64
                    {
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            96,
                            y,
                            row_width,
                            66,
                            10,
                            0x0022262c,
                        );
                    } else {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            96,
                            visible_y,
                            row_width,
                            visible_height,
                            0x0022262c,
                        );
                    }
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        116,
                        y + 12,
                        &server.name,
                        0x00f1f3f5,
                        1,
                        row_width.saturating_sub(300),
                    );
                    let version =
                        server.mc_version.as_deref().unwrap_or("Minecraft");
                    let loader = server.loader.as_deref().unwrap_or("Vanilla");
                    let details =
                        format!("{version}  ·  {loader}  ·  {}", server.status);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        116,
                        y + 36,
                        &details,
                        0x00a8adb5,
                        1,
                        row_width.saturating_sub(300),
                    );
                    let address =
                        format!("{}:{}", server.net.domain, server.net.port);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        116 + row_width.saturating_sub(250),
                        y + 25,
                        &address,
                        0x00bdc2ca,
                        1,
                        232,
                    );
                }
                render::set_clip_rect(None);
            }
        } else if self.global_skins {
            skin_panel::draw_skin_panel(
                &mut buffer,
                w,
                library_width,
                h,
                &self.minecraft_skins,
                &self.minecraft_skin_previews,
                self.minecraft_skin_scroll_pixels,
                self.minecraft_skin_variant,
                self.minecraft_skin_rotation,
                self.default_account
                    .and_then(|id| {
                        self.minecraft_accounts
                            .iter()
                            .find(|account| account.id == id)
                    })
                    .map(|account| account.username.as_str()),
                self.settings
                    .as_ref()
                    .is_some_and(|settings| settings.hide_nametag_skins_page),
            );
        } else if self.global_user_profile {
            rect(
                &mut buffer,
                w,
                h,
                248,
                56,
                library_width.saturating_sub(248),
                h.saturating_sub(56),
                0x001b1d22,
            );
            rect(&mut buffer, w, h, 292, 80, 122, 28, 0x0022262c);
            text(&mut buffer, w, h, 300, 90, "BACK TO LIBRARY", 0x00a8e66b, 1);
            let profile = self.user_profile.as_ref();
            let organization_column_visible = library_width >= 980
                && !self.user_profile_organizations.is_empty();
            let profile_text_width = if organization_column_visible {
                library_width.saturating_sub(648)
            } else {
                library_width.saturating_sub(400)
            };
            let profile_status_width = if organization_column_visible {
                library_width.saturating_sub(556)
            } else {
                library_width.saturating_sub(324)
            };
            let display_name = profile
                .and_then(|value| {
                    value.get("name").and_then(|value| value.as_str())
                })
                .or_else(|| {
                    profile.and_then(|value| {
                        value.get("username").and_then(|value| value.as_str())
                    })
                })
                .unwrap_or("User profile");
            rect(&mut buffer, w, h, 292, 140, 72, 72, 0x0022262c);
            if let Some(user_id) = self.user_profile_id.as_ref()
                && let Some((avatar_width, avatar_height, avatar)) =
                    self.browse_icons.get(&format!("user-profile:{user_id}"))
            {
                draw_scaled_image(
                    &mut buffer,
                    w,
                    h,
                    *avatar_width,
                    *avatar_height,
                    avatar,
                    292,
                    140,
                    72,
                    72,
                );
            }
            text_clipped(
                &mut buffer,
                w,
                h,
                384,
                142,
                display_name,
                0x00f1f3f5,
                2,
                profile_text_width.min(360).max(96),
            );
            if let Some(username) = profile.and_then(|value| {
                value.get("username").and_then(|value| value.as_str())
            }) {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    384,
                    180,
                    &format!("@{username}"),
                    0x00999fa8,
                    1,
                    profile_text_width,
                );
            }
            if organization_column_visible {
                let org_x = library_width.saturating_sub(252);
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    org_x,
                    142,
                    "ORGANIZATIONS",
                    0x00f1f3f5,
                    1,
                    160,
                );
                for (index, organization) in
                    self.user_profile_organizations.iter().take(5).enumerate()
                {
                    let y = 176 + index * 46;
                    rect(&mut buffer, w, h, org_x, y, 228, 40, 0x0022262c);
                    if let Some(id) = organization
                        .get("id")
                        .or_else(|| organization.get("slug"))
                        .and_then(serde_json::Value::as_str)
                        && let Some((icon_width, icon_height, icon)) = self
                            .browse_icons
                            .get(&format!("user-organization:{id}"))
                    {
                        draw_scaled_image(
                            &mut buffer,
                            w,
                            h,
                            *icon_width,
                            *icon_height,
                            icon,
                            org_x + 4,
                            y + 4,
                            32,
                            32,
                        );
                    }
                    let name = organization
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Organization");
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        org_x + 44,
                        y + 15,
                        name,
                        0x00c4c8ce,
                        1,
                        172,
                    );
                }
            }
            if self.user_profile_loading {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    292,
                    224,
                    "Loading profile…",
                    0x00b0b6bf,
                    1,
                    profile_status_width,
                );
            } else if let Some(error) = &self.user_profile_error {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    292,
                    224,
                    &format!("Could not load profile: {error}"),
                    0x00ff7777,
                    1,
                    profile_status_width,
                );
            } else {
                if let Some(bio) = profile.and_then(|value| {
                    value.get("bio").and_then(|value| value.as_str())
                }) {
                    let wrapped_bio = render::wrap_text_to_width(
                        bio,
                        1,
                        profile_text_width,
                        4,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        384,
                        214,
                        &wrapped_bio,
                        0x00c4c8ce,
                        1,
                        profile_text_width,
                    );
                }
                let downloads = self
                    .user_profile_projects
                    .iter()
                    .filter_map(|project| project.get("downloads"))
                    .filter_map(serde_json::Value::as_u64)
                    .sum::<u64>();
                let joined = profile
                    .and_then(|value| value.get("created"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    384,
                    290,
                    &format!(
                        "{} projects  ·  {downloads} downloads  ·  Joined {joined}",
                        self.user_profile_projects.len()
                    ),
                    0x00999fa8,
                    1,
                    profile_text_width,
                );
                let mut badge_x = 292usize;
                for badge in
                    profile_badge_labels(profile, &self.user_profile_projects)
                {
                    let badge_width =
                        (badge.len() * 7 + 18).min(profile_text_width);
                    if badge_x + badge_width > library_width.saturating_sub(24)
                    {
                        break;
                    }
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        badge_x,
                        266,
                        badge_width,
                        24,
                        6,
                        0x00333c32,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        badge_x + 8,
                        274,
                        badge,
                        0x00a8e66b,
                        1,
                        badge_width.saturating_sub(16),
                    );
                    badge_x += badge_width + 6;
                }
                let profile_content_width = if library_width >= 980 {
                    library_width.saturating_sub(570)
                } else {
                    library_width.saturating_sub(336)
                };
                let mut tabs = vec![None];
                tabs.extend(
                    profile_project_types(&self.user_profile_projects)
                        .into_iter()
                        .map(Some),
                );
                if !self.user_profile_collections.is_empty() {
                    tabs.push(Some("collection".into()));
                }
                for (index, tab) in tabs.iter().enumerate() {
                    let x = 292 + index * 96;
                    if x + 88 > library_width.saturating_sub(24) {
                        break;
                    }
                    let collection_tab = tab.as_deref() == Some("collection");
                    let selected = if collection_tab {
                        self.user_profile_show_collections
                    } else {
                        !self.user_profile_show_collections
                            && tab == &self.user_profile_type_filter
                    };
                    rect(
                        &mut buffer,
                        w,
                        h,
                        x,
                        306,
                        88,
                        28,
                        if selected { 0x003b633a } else { 0x0022262c },
                    );
                    let label = if collection_tab {
                        "Collections"
                    } else {
                        tab.as_deref().unwrap_or("All")
                    };
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 8,
                        316,
                        &label.to_ascii_uppercase(),
                        if selected { 0x00a8e66b } else { 0x00c4c8ce },
                        1,
                    );
                }
                let filtered_projects =
                    &self.user_profile_filtered_project_indices;
                if self.user_profile_show_collections
                    && self.user_profile_collections.is_empty()
                {
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        360,
                        "No collections",
                        0x00999fa8,
                        1,
                    );
                } else if !self.user_profile_show_collections
                    && filtered_projects.is_empty()
                {
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        360,
                        if self.user_profile_type_filter.is_some() {
                            "No projects in this category"
                        } else {
                            "No public projects"
                        },
                        0x00999fa8,
                        1,
                    );
                }
                let profile_viewport_height =
                    h.saturating_sub(40).saturating_sub(352);
                let profile_first_row = (self.user_profile_scroll_pixels / 62.0)
                    .floor()
                    .max(0.0) as usize;
                let profile_last_row = ((self.user_profile_scroll_pixels
                    + profile_viewport_height as f64)
                    / 62.0)
                    .ceil() as usize
                    + 1;
                let visible_profile_projects = filtered_projects
                    .get(
                        profile_first_row
                            ..profile_last_row.min(filtered_projects.len()),
                    )
                    .unwrap_or_default();
                for (visible_index, project_index) in
                    visible_profile_projects.iter().enumerate()
                {
                    let index = profile_first_row + visible_index;
                    let Some(project) =
                        self.user_profile_projects.get(*project_index)
                    else {
                        continue;
                    };
                    let row_y = 352.0 + index as f64 * 62.0
                        - self.user_profile_scroll_pixels;
                    if row_y < 352.0
                        || row_y + 54.0 > h.saturating_sub(40) as f64
                    {
                        continue;
                    }
                    let y = row_y as usize;
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        y,
                        profile_content_width,
                        54,
                        0x0022262c,
                    );
                    let title = project
                        .get("title")
                        .and_then(|value| value.as_str())
                        .unwrap_or("Untitled project");
                    let description = project
                        .get("description")
                        .and_then(|value| value.as_str())
                        .unwrap_or("");
                    text(&mut buffer, w, h, 306, y + 10, title, 0x00f1f3f5, 1);
                    text(
                        &mut buffer,
                        w,
                        h,
                        306,
                        y + 30,
                        description,
                        0x00999fa8,
                        1,
                    );
                }
                if self.user_profile_show_collections {
                    for (index, collection) in
                        self.user_profile_collections.iter().enumerate()
                    {
                        let row_y = 352.0 + index as f64 * 62.0
                            - self.user_profile_scroll_pixels;
                        if row_y < 352.0
                            || row_y + 54.0 > h.saturating_sub(40) as f64
                        {
                            continue;
                        }
                        let y = row_y as usize;
                        rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            y,
                            profile_content_width,
                            54,
                            0x0022262c,
                        );
                        let title = collection
                            .get("name")
                            .and_then(|value| value.as_str())
                            .unwrap_or("Collection");
                        let description = collection
                            .get("description")
                            .and_then(|value| value.as_str())
                            .unwrap_or("");
                        let project_count = collection
                            .get("projects")
                            .and_then(serde_json::Value::as_array)
                            .map_or(0, Vec::len);
                        text(
                            &mut buffer,
                            w,
                            h,
                            306,
                            y + 10,
                            title,
                            0x00f1f3f5,
                            1,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            306,
                            y + 30,
                            &format!("{project_count} projects  {description}"),
                            0x00999fa8,
                            1,
                        );
                    }
                }
            }
        } else if let Some((instance_id, name)) = self.selected_instance.clone()
        {
            rect(
                &mut buffer,
                w,
                h,
                248,
                56,
                library_width.saturating_sub(248),
                h.saturating_sub(56),
                0x001b1d22,
            );
            if !self.global_browse
                && !self.global_screenshots
                && self.instance_tab != 8
            {
                let back = instance_back_bounds(wide_instance_header);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    back.x,
                    back.y,
                    back.width,
                    back.height,
                    8,
                    if back.contains(cursor.0, cursor.1) {
                        0x00343a42
                    } else {
                        0x0022262c
                    },
                );
                let label = if wide_instance_header {
                    "‹"
                } else {
                    "Library"
                };
                text(
                    &mut buffer,
                    w,
                    h,
                    back.x + 10,
                    back.y + 8,
                    label,
                    0x00a8e66b,
                    1,
                );
            }
            if self.instance_tab != 8
                && !self.global_browse
                && instance_id != GLOBAL_DISCOVER_ID
            {
                let action = instance_primary_action
                    .unwrap_or(NativeLibraryCardAction::None);
                let (label, color, text_color) = match action {
                    NativeLibraryCardAction::Stop => {
                        ("Stop", 0x004a2929, 0x00ef8b8b)
                    }
                    NativeLibraryCardAction::Repair => {
                        ("Repair", 0x00303b2b, 0x00a8e66b)
                    }
                    NativeLibraryCardAction::Play => {
                        ("Play", 0x00a8e66b, 0x001b1d22)
                    }
                    NativeLibraryCardAction::Installing => {
                        ("Installing", 0x0022262c, 0x007b838d)
                    }
                    NativeLibraryCardAction::None => {
                        ("Locked", 0x0022262c, 0x007b838d)
                    }
                };
                let button = instance_primary_bounds(library_width);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    button.x,
                    button.y,
                    button.width,
                    button.height,
                    8,
                    color,
                );
                let text_width = render::measure_text_width(label, 1);
                let text_x =
                    button.x + button.width.saturating_sub(text_width) / 2;
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    text_x,
                    button.y + 7,
                    label,
                    text_color,
                    1,
                    button.width,
                );
            }
            if self.instance_tab != 8
                && !self.global_browse
                && instance_id != GLOBAL_DISCOVER_ID
            {
                let metadata =
                    self.instance_cards.get(&instance_id).map_or_else(
                        || "Minecraft".to_owned(),
                        |card| format!("{} {}", card.loader, card.game_version),
                    );
                let metrics = self.instance_metrics.get(&instance_id);
                let mut metadata_items = vec![metadata];
                if self
                    .settings
                    .as_ref()
                    .is_none_or(|settings| settings.show_play_time)
                    && let Some(metrics) =
                        metrics.filter(|item| item.playtime > 0)
                {
                    let hours = metrics.playtime / 3600;
                    let minutes = (metrics.playtime % 3600) / 60;
                    metadata_items.push(if hours > 0 {
                        format!("{hours}h {minutes}m played")
                    } else {
                        format!("{minutes}m played")
                    });
                }
                metadata_items.push(
                    metrics.and_then(|item| item.last_played).map_or_else(
                        || "Never played".to_owned(),
                        relative_play_time,
                    ),
                );
                let (metadata_x, metadata_y, metadata_width) =
                    if wide_instance_header {
                        (360, 98, 140)
                    } else {
                        (292, 65, library_width.saturating_sub(324))
                    };
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    metadata_x,
                    metadata_y,
                    &metadata_items.join("  ·  "),
                    0x00bdc2ca,
                    1,
                    metadata_width,
                );
            }
            if self.instance_tab == 8 {
                if self.screenshot_viewer {
                    rect(&mut buffer, w, h, 292, 146, 112, 28, 0x0022262c);
                    text(
                        &mut buffer,
                        w,
                        h,
                        300,
                        156,
                        "BACK TO LIST",
                        0x00a8e66b,
                        1,
                    );
                    if let Some(index) = self.selected_screenshot {
                        if let Some(screenshot) = self.screenshots.get(index) {
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                414,
                                156,
                                &screenshot.file_name,
                                0x00bdc2ca,
                                1,
                                library_width.saturating_sub(426),
                            );
                        }
                    }
                    for (x, width, label, color) in [
                        (
                            library_width.saturating_sub(356),
                            104,
                            "OPEN FILE",
                            0x0022262c,
                        ),
                        (
                            library_width.saturating_sub(246),
                            96,
                            "EXPORT ZIP",
                            0x0022262c,
                        ),
                        (
                            library_width.saturating_sub(144),
                            112,
                            "DELETE",
                            0x003b302b,
                        ),
                    ] {
                        rect(&mut buffer, w, h, x, 146, width, 28, color);
                        text(
                            &mut buffer,
                            w,
                            h,
                            x + 6,
                            156,
                            label,
                            if label == "DELETE" {
                                0x00e68a6b
                            } else {
                                0x00a8e66b
                            },
                            1,
                        );
                    }
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        182,
                        &self.message,
                        0x007b838d,
                        1,
                        library_width.saturating_sub(324),
                    );
                    let (preview_x, preview_y) = (292, 202);
                    let (preview_width, preview_height) = (
                        library_width.saturating_sub(324),
                        h.saturating_sub(226),
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        preview_x,
                        preview_y,
                        preview_width,
                        preview_height,
                        0x00141619,
                    );
                    if let Some((image_width, image_height, pixels)) =
                        self.screenshot_preview.as_ref()
                    {
                        draw_scaled_image(
                            &mut buffer,
                            w,
                            h,
                            *image_width,
                            *image_height,
                            pixels,
                            preview_x + 8,
                            preview_y + 8,
                            preview_width.saturating_sub(16),
                            preview_height.saturating_sub(16),
                        );
                    } else {
                        text(
                            &mut buffer,
                            w,
                            h,
                            preview_x + 16,
                            preview_y + 18,
                            "LOADING SCREENSHOT PREVIEW",
                            0x007b838d,
                            1,
                        );
                    }
                    if self.screenshot_delete_confirmation {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            300,
                            h.saturating_sub(110),
                            library_width.saturating_sub(352),
                            64,
                            0x003b302b,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            312,
                            h.saturating_sub(96),
                            &format!(
                                "DELETE {} SELECTED SCREENSHOT{}?",
                                selected_screenshot_count,
                                if selected_screenshot_count == 1 {
                                    ""
                                } else {
                                    "S"
                                }
                            ),
                            0x00f1f3f5,
                            1,
                        );
                        for (x, label) in [
                            (library_width.saturating_sub(202), "DELETE"),
                            (library_width.saturating_sub(122), "CANCEL"),
                        ] {
                            rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                h.saturating_sub(102),
                                72,
                                32,
                                0x0022262c,
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                x + 5,
                                h.saturating_sub(90),
                                label,
                                if label == "DELETE" {
                                    0x00e68a6b
                                } else {
                                    0x00bdc2ca
                                },
                                1,
                            );
                        }
                    }
                } else {
                    let group_label = match self.screenshot_group_mode % 4 {
                        1 => "GROUP: INSTANCE",
                        2 => "GROUP: CUSTOM",
                        3 => "GROUP: NONE",
                        _ => "GROUP: DATE",
                    };
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        146,
                        library_width.saturating_sub(562),
                        28,
                        0x001a1d21,
                    );
                    text_input_clipped(
                        &mut buffer,
                        w,
                        h,
                        302,
                        156,
                        if self.screenshot_search_query.is_empty() {
                            "SEARCH SCREENSHOTS"
                        } else {
                            &self.screenshot_search_query
                        },
                        if self.screenshot_search_query.is_empty() {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        },
                        1,
                        library_width.saturating_sub(572),
                    );
                    if self.screenshot_group_mode % 4 == 2 {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            library_width.saturating_sub(252),
                            146,
                            110,
                            28,
                            0x00303b2b,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            library_width.saturating_sub(244),
                            156,
                            "NEW GROUP",
                            0x00a8e66b,
                            1,
                        );
                    }
                    rect(
                        &mut buffer,
                        w,
                        h,
                        library_width.saturating_sub(142),
                        146,
                        112,
                        28,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        library_width.saturating_sub(128),
                        156,
                        "REFRESH",
                        0x00a8e66b,
                        1,
                    );
                    let sort_label = match self.screenshot_sort_mode % 4 {
                        1 => "SORT: OLDEST",
                        2 => "SORT: NAME A-Z",
                        3 => "SORT: NAME Z-A",
                        _ => "SORT: NEWEST",
                    };
                    rect(&mut buffer, w, h, 292, 180, 140, 28, 0x0022262c);
                    text(
                        &mut buffer,
                        w,
                        h,
                        300,
                        190,
                        sort_label,
                        0x00a8e66b,
                        1,
                    );
                    rect(&mut buffer, w, h, 440, 180, 140, 28, 0x0022262c);
                    text(
                        &mut buffer,
                        w,
                        h,
                        448,
                        190,
                        group_label,
                        0x00a8e66b,
                        1,
                    );
                    if self.screenshot_group_mode % 4 == 2 {
                        rect(&mut buffer, w, h, 588, 180, 52, 28, 0x0022262c);
                        text(
                            &mut buffer,
                            w,
                            h,
                            592,
                            190,
                            "MOVE",
                            0x00a8e66b,
                            1,
                        );
                    }
                    if self.selected_screenshot_ids.len() > 1 {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            library_width.saturating_sub(116),
                            180,
                            52,
                            28,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            library_width.saturating_sub(112),
                            190,
                            "ZIP",
                            0x00a8e66b,
                            1,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            library_width.saturating_sub(60),
                            180,
                            52,
                            28,
                            0x003b302b,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            library_width.saturating_sub(56),
                            190,
                            &format!(
                                "DEL {}",
                                self.selected_screenshot_ids.len()
                            ),
                            0x00e68a6b,
                            1,
                        );
                    }
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        216,
                        &self.message,
                        0x007b838d,
                        1,
                        library_width.saturating_sub(464),
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        library_width.saturating_sub(160),
                        216,
                        &if self.selected_screenshot_ids.len() > 1 {
                            format!(
                                "{} SELECTED",
                                self.selected_screenshot_ids.len()
                            )
                        } else {
                            format!("{} SCREENSHOTS", screenshot_order.len())
                        },
                        0x00bdc2ca,
                        1,
                        140,
                    );
                    if screenshot_order.is_empty() {
                        text(
                            &mut buffer,
                            w,
                            h,
                            292,
                            278,
                            if self.screenshot_search_query.is_empty() {
                                "NO SCREENSHOTS FOUND"
                            } else {
                                "NO SCREENSHOTS MATCH THAT SEARCH"
                            },
                            0x007b838d,
                            1,
                        );
                    }
                    let (columns, card_width, image_height, card_height) =
                        screenshot_grid_layout(library_width);
                    let stride = card_height.saturating_add(12);
                    if self.screenshot_offset != self.screenshot_scroll_synced {
                        self.screenshot_scroll_pixels =
                            (self.screenshot_offset / columns * stride) as f64;
                        self.screenshot_scroll_target_pixels =
                            self.screenshot_scroll_pixels;
                        self.screenshot_scroll_synced = self.screenshot_offset;
                    }
                    let max_scroll = screenshot_max_scroll(
                        h,
                        screenshot_order.len(),
                        columns,
                        stride,
                    );
                    self.screenshot_scroll_target_pixels =
                        self.screenshot_scroll_target_pixels.min(max_scroll);
                    self.screenshot_scroll_pixels =
                        self.screenshot_scroll_pixels.min(max_scroll);
                    let scroll_rows = (self.screenshot_scroll_pixels
                        / stride as f64)
                        .floor() as usize;
                    let scroll_remainder =
                        (self.screenshot_scroll_pixels as usize) % stride;
                    self.screenshot_offset = scroll_rows * columns;
                    self.screenshot_scroll_synced = self.screenshot_offset;
                    render::set_clip_rect(Some((
                        292,
                        230,
                        library_width.saturating_sub(292),
                        h.saturating_sub(230),
                    )));
                    for (index, screenshot_index) in screenshot_order
                        .iter()
                        .skip(scroll_rows * columns)
                        .take(
                            screenshot_visible_rows(
                                h,
                                stride,
                                scroll_remainder,
                            ) * columns,
                        )
                        .enumerate()
                    {
                        let Some(screenshot) =
                            self.screenshots.get(*screenshot_index)
                        else {
                            continue;
                        };
                        let column = index % columns;
                        let row = index / columns;
                        let x = 292 + column * (card_width + 12);
                        let (y, hidden) = screenshot_row_position(
                            row,
                            stride,
                            scroll_remainder,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            x,
                            y,
                            card_width,
                            card_height.saturating_sub(hidden),
                            if self
                                .selected_screenshot_ids
                                .contains(&screenshot.id)
                                || self.selected_screenshot
                                    == Some(*screenshot_index)
                            {
                                0x002c3430
                            } else {
                                0x0022262c
                            },
                        );
                        let image_x = x + 8;
                        let image_y = y + 8;
                        let image_width = card_width.saturating_sub(16);
                        let image_view_height = image_height.saturating_sub(8);
                        if hidden == 0 {
                            rect(
                                &mut buffer,
                                w,
                                h,
                                image_x,
                                image_y,
                                image_width,
                                image_view_height,
                                0x00141619,
                            );
                            if let Some((thumb_width, thumb_height, pixels)) =
                                self.screenshot_thumbnails.get(&screenshot.id)
                            {
                                draw_scaled_image(
                                    &mut buffer,
                                    w,
                                    h,
                                    *thumb_width,
                                    *thumb_height,
                                    pixels,
                                    image_x,
                                    image_y,
                                    image_width,
                                    image_view_height,
                                );
                            } else {
                                text(
                                    &mut buffer,
                                    w,
                                    h,
                                    image_x + 8,
                                    image_y + image_view_height / 2,
                                    if self
                                        .screenshot_thumbnail_failures
                                        .contains(&screenshot.id)
                                    {
                                        "PREVIEW UNAVAILABLE"
                                    } else {
                                        "LOADING PREVIEW"
                                    },
                                    0x007b838d,
                                    1,
                                );
                            }
                        }
                        if hidden == 0 {
                            let copy = screenshot_copy_bounds(card_width);
                            rounded_rect(
                                &mut buffer,
                                w,
                                h,
                                x + copy.x,
                                y + copy.y,
                                copy.width,
                                copy.height,
                                5,
                                0x00343a42,
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                x + copy.x + 7,
                                y + copy.y + 7,
                                "COPY",
                                0x00f1f3f5,
                                1,
                            );
                        }
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            x + 8,
                            (y + image_height + 7).saturating_sub(hidden),
                            &screenshot.file_name,
                            0x00f1f3f5,
                            1,
                            card_width.saturating_sub(20),
                        );
                        let sublabel = match self.screenshot_group_mode % 4 {
                            0 => screenshot
                                .created_at
                                .format("%Y-%m-%d")
                                .to_string(),
                            2 => screenshot
                                .group_id
                                .as_ref()
                                .and_then(|id| {
                                    self.screenshot_groups
                                        .iter()
                                        .find(|group| &group.id == id)
                                })
                                .map(|group| group.name.clone())
                                .unwrap_or_else(|| "UNGROUPED".into()),
                            3 => screenshot.instance_name.clone(),
                            _ => screenshot.instance_name.clone(),
                        };
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            x + 8,
                            (y + image_height + 24).saturating_sub(hidden),
                            &sublabel,
                            0x007b838d,
                            1,
                            card_width.saturating_sub(20),
                        );
                        for (button_x, button_width, label, color) in [
                            (x + 8, 48, "VIEW", 0x00303b2b),
                            (x + 60, 52, "EXPORT", 0x0022262c),
                            (x + 116, 56, "DELETE", 0x003b302b),
                        ] {
                            rect(
                                &mut buffer,
                                w,
                                h,
                                button_x,
                                (y + card_height - 30).saturating_sub(hidden),
                                button_width,
                                24,
                                color,
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                button_x + 5,
                                (y + card_height - 22).saturating_sub(hidden),
                                label,
                                if label == "DELETE" {
                                    0x00e68a6b
                                } else {
                                    0x00a8e66b
                                },
                                1,
                            );
                        }
                    }
                    render::set_clip_rect(None);
                    if self.screenshot_delete_confirmation {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            300,
                            h.saturating_sub(110),
                            w.saturating_sub(352),
                            64,
                            0x003b302b,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            312,
                            h.saturating_sub(96),
                            "DELETE THIS SCREENSHOT?",
                            0x00f1f3f5,
                            1,
                        );
                        for (x, label) in [
                            (w.saturating_sub(202), "DELETE"),
                            (w.saturating_sub(122), "CANCEL"),
                        ] {
                            rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                h.saturating_sub(102),
                                72,
                                32,
                                0x0022262c,
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                x + 5,
                                h.saturating_sub(90),
                                label,
                                if label == "DELETE" {
                                    0x00e68a6b
                                } else {
                                    0x00bdc2ca
                                },
                                1,
                            );
                        }
                    }
                    if self.screenshot_group_picker_open {
                        let panel_width = 420.min(w.saturating_sub(32));
                        let can_manage_group = self
                            .selected_screenshot_ids
                            .len()
                            <= 1
                            && self
                                .selected_screenshot
                                .and_then(|index| self.screenshots.get(index))
                                .and_then(|screenshot| {
                                    screenshot.group_id.as_ref()
                                })
                                .and_then(|id| {
                                    self.screenshot_groups
                                        .iter()
                                        .find(|group| &group.id == id)
                                })
                                .is_some();
                        let group_count = self
                            .screenshot_groups
                            .len()
                            .saturating_sub(self.screenshot_group_offset)
                            .min(8);
                        let panel_height = 58
                            + (group_count + 1) * 34
                            + if can_manage_group { 78 } else { 42 };
                        let panel_x = w.saturating_sub(panel_width) / 2;
                        let panel_y = h.saturating_sub(panel_height) / 2;
                        rect(
                            &mut buffer,
                            w,
                            h,
                            0,
                            56,
                            w,
                            h.saturating_sub(56),
                            0x00101215,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            panel_x,
                            panel_y,
                            panel_width,
                            panel_height,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 14,
                            panel_y + 14,
                            "MOVE SCREENSHOT TO GROUP",
                            0x00f1f3f5,
                            1,
                        );
                        render::set_clip_rect(Some((
                            panel_x + 12,
                            panel_y + 42,
                            panel_width.saturating_sub(24),
                            9 * 34,
                        )));
                        let scroll_remainder =
                            (self.screenshot_group_scroll_pixels % 34.0)
                                as usize;
                        for index in 0..=group_count {
                            let y =
                                panel_y + 42 + index * 34 - scroll_remainder;
                            let label = if index < group_count {
                                self.screenshot_groups
                                    [self.screenshot_group_offset + index]
                                    .name
                                    .as_str()
                            } else {
                                "UNGROUPED"
                            };
                            rect(
                                &mut buffer,
                                w,
                                h,
                                panel_x + 12,
                                y,
                                panel_width.saturating_sub(24),
                                28,
                                0x002e333a,
                            );
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                panel_x + 20,
                                y + 9,
                                label,
                                0x00a8e66b,
                                1,
                                panel_width.saturating_sub(48),
                            );
                        }
                        render::set_clip_rect(None);
                        let cancel_y = panel_y + 46 + (group_count + 1) * 34;
                        rect(
                            &mut buffer,
                            w,
                            h,
                            panel_x + panel_width.saturating_sub(92),
                            cancel_y,
                            80,
                            28,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            panel_x + panel_width.saturating_sub(84),
                            cancel_y + 9,
                            "CANCEL",
                            0x00bdc2ca,
                            1,
                        );
                        if can_manage_group {
                            for (x, label, color) in [
                                (panel_x + 12, "RENAME GROUP", 0x00303b2b),
                                (panel_x + 148, "DELETE GROUP", 0x003b302b),
                            ] {
                                rect(
                                    &mut buffer,
                                    w,
                                    h,
                                    x,
                                    cancel_y + 34,
                                    128,
                                    28,
                                    color,
                                );
                                text(
                                    &mut buffer,
                                    w,
                                    h,
                                    x + 7,
                                    cancel_y + 43,
                                    label,
                                    if label == "DELETE GROUP" {
                                        0x00e68a6b
                                    } else {
                                        0x00a8e66b
                                    },
                                    1,
                                );
                            }
                        }
                    }
                    if self.screenshot_group_rename_id.is_some() {
                        let panel_width = 400.min(w.saturating_sub(32));
                        let panel_height = 154;
                        let panel_x = w.saturating_sub(panel_width) / 2;
                        let panel_y = h.saturating_sub(panel_height) / 2;
                        rect(
                            &mut buffer,
                            w,
                            h,
                            0,
                            56,
                            w,
                            h.saturating_sub(56),
                            0x00101215,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            panel_x,
                            panel_y,
                            panel_width,
                            panel_height,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 16,
                            panel_y + 16,
                            "RENAME SCREENSHOT GROUP",
                            0x00f1f3f5,
                            1,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 16,
                            panel_y + 48,
                            panel_width.saturating_sub(32),
                            34,
                            0x001a1d21,
                        );
                        text_input_clipped(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 24,
                            panel_y + 60,
                            &self.screenshot_group_name_input,
                            0x00f1f3f5,
                            1,
                            panel_width.saturating_sub(48),
                        );
                        for (x, label, color) in [
                            (panel_x + 16, "SAVE", 0x00303b2b),
                            (panel_x + 104, "CANCEL", 0x0022262c),
                        ] {
                            rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                panel_y + 100,
                                80,
                                30,
                                color,
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                x + 8,
                                panel_y + 110,
                                label,
                                0x00a8e66b,
                                1,
                            );
                        }
                    }
                    if self.screenshot_group_delete_confirmation.is_some() {
                        let panel_width = 380.min(w.saturating_sub(32));
                        let panel_height = 122;
                        let panel_x = w.saturating_sub(panel_width) / 2;
                        let panel_y = h.saturating_sub(panel_height) / 2;
                        rect(
                            &mut buffer,
                            w,
                            h,
                            0,
                            56,
                            w,
                            h.saturating_sub(56),
                            0x00101215,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            panel_x,
                            panel_y,
                            panel_width,
                            panel_height,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 16,
                            panel_y + 20,
                            "DELETE THIS SCREENSHOT GROUP?",
                            0x00f1f3f5,
                            1,
                        );
                        for (x, label, color) in [
                            (panel_x + 16, "DELETE", 0x003b302b),
                            (panel_x + 104, "CANCEL", 0x0022262c),
                        ] {
                            rect(
                                &mut buffer,
                                w,
                                h,
                                x,
                                panel_y + 66,
                                80,
                                30,
                                color,
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                x + 8,
                                panel_y + 76,
                                label,
                                if label == "DELETE" {
                                    0x00e68a6b
                                } else {
                                    0x00a8e66b
                                },
                                1,
                            );
                        }
                    }
                }
            } else if self.instance_tab == 0 {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(392),
                    146,
                    112,
                    28,
                    8,
                    0x003b302b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(376),
                    155,
                    "Delete",
                    0x00e68a6b,
                    1,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(270),
                    146,
                    112,
                    28,
                    8,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(258),
                    155,
                    "Duplicate",
                    0x00a8e66b,
                    1,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(142),
                    146,
                    112,
                    28,
                    8,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(130),
                    155,
                    "Export",
                    0x00a8e66b,
                    1,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(270),
                    82,
                    112,
                    32,
                    8,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(254),
                    94,
                    "Browse",
                    0x00a8e66b,
                    1,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(142),
                    82,
                    112,
                    32,
                    8,
                    0x00303b2b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(130),
                    94,
                    "Add file",
                    0x00a8e66b,
                    1,
                );
            }
            if self.rename_instance_open {
                if wide_instance_header {
                    rect(&mut buffer, w, h, 360, 78, 140, 28, 0x0022262c);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        368,
                        88,
                        &self.rename_instance_input,
                        0x00f1f3f5,
                        1,
                        124,
                    );
                    for (x, width, label, color) in [
                        (506, 44, "SAVE", 0x00303b2b),
                        (554, 52, "CANCEL", 0x0022262c),
                    ] {
                        rect(&mut buffer, w, h, x, 78, width, 28, color);
                        text(
                            &mut buffer,
                            w,
                            h,
                            x + 3,
                            88,
                            label,
                            0x00a8e66b,
                            1,
                        );
                    }
                } else {
                    rect(&mut buffer, w, h, 292, 112, 122, 28, 0x0022262c);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        298,
                        122,
                        &self.rename_instance_input,
                        0x00f1f3f5,
                        1,
                        110,
                    );
                    for (x, width, label, color) in [
                        (418, 38, "SAVE", 0x00303b2b),
                        (460, 40, "CANCEL", 0x0022262c),
                    ] {
                        rect(&mut buffer, w, h, x, 112, width, 28, color);
                        text(
                            &mut buffer,
                            w,
                            h,
                            x + 4,
                            122,
                            label,
                            0x00a8e66b,
                            1,
                        );
                    }
                }
            } else if !self.global_browse
                && !self.global_hosting
                && !self.global_skins
                && !self.global_screenshots
                && self.instance_tab != 8
            {
                if wide_instance_header {
                    if let Some(icon) = self.instance_icons.get(&instance_id) {
                        draw_scaled_instance_icon(
                            &mut buffer,
                            w,
                            h,
                            292,
                            72,
                            56,
                            icon,
                        );
                    } else {
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            72,
                            56,
                            56,
                            12,
                            0x00343a42,
                        );
                        let initial = name
                            .chars()
                            .next()
                            .map_or_else(String::new, |ch| {
                                ch.to_uppercase().to_string()
                            });
                        text(
                            &mut buffer,
                            w,
                            h,
                            312,
                            89,
                            &initial,
                            0x00f1f3f5,
                            2,
                        );
                    }
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        360,
                        78,
                        &name,
                        0x00f1f3f5,
                        2,
                        140,
                    );
                    rect(&mut buffer, w, h, 506, 78, 56, 28, 0x0022262c);
                    text(&mut buffer, w, h, 510, 88, "Rename", 0x00a8e66b, 1);
                } else {
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        119,
                        &name,
                        0x00f1f3f5,
                        2,
                        112,
                    );
                    rect(&mut buffer, w, h, 414, 112, 84, 28, 0x0022262c);
                    text(&mut buffer, w, h, 422, 122, "Rename", 0x00a8e66b, 1);
                }
            } else if self.global_screenshots || self.instance_tab == 8 {
                text(&mut buffer, w, h, 292, 119, "Screenshots", 0x00f1f3f5, 2);
            } else {
                text(&mut buffer, w, h, 292, 119, "Discover", 0x00f1f3f5, 2);
            }
            for (x, width, tab, index) in instance_tabs {
                if self.global_browse
                    || self.global_screenshots
                    || self.instance_tab == 8
                {
                    continue;
                }
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    112,
                    width,
                    28,
                    8,
                    if self.instance_tab == index {
                        0x00303b2b
                    } else {
                        0x0022262c
                    },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 4,
                    122,
                    tab,
                    if self.instance_tab == index {
                        0x00a8e66b
                    } else {
                        0x00bdc2ca
                    },
                    1,
                );
            }
            if self.instance_tab == 0 {
                content_panel::draw_content_panel(
                    &mut buffer,
                    w,
                    h,
                    library_width,
                    &self.filtered_content_cache,
                    &self.contents,
                    &self.browse_icons,
                    self.content_scroll_pixels,
                    self.content_filter,
                    &self.message,
                );
            } else if self.instance_tab == 1 {
                let query = self.log_search_query.trim().to_lowercase();
                let present_levels =
                    native_log_present_levels(&self.console_text);
                let console_filter_top =
                    instance_console_search_bounds(w).y + 38;
                let console_body_top = instance_log_filter_bottom(
                    w,
                    console_filter_top,
                    present_levels,
                ) + 8;
                let log_lines = self
                    .console_text
                    .lines()
                    .filter(|line| {
                        native_log_line_matches_filter(
                            line,
                            self.log_level_filter,
                        ) && (query.is_empty()
                            || line.to_lowercase().contains(&query))
                    })
                    .collect::<Vec<_>>();
                let max_lines =
                    (h.saturating_sub(console_body_top + 40) / 12).max(1);
                let max_scroll = log_lines
                    .len()
                    .saturating_sub(max_lines)
                    .saturating_mul(12) as f64;
                let scroll_pixels = self.console_scroll_pixels.min(max_scroll);
                let scroll = (scroll_pixels / 12.0).floor() as usize;
                let scroll_remainder =
                    scroll_pixels.rem_euclid(12.0).round() as usize;
                let end = log_lines.len().saturating_sub(scroll);
                let start = end.saturating_sub(max_lines);
                let process_running = self
                    .process_state
                    .as_ref()
                    .is_some_and(|process| process.instance_id == instance_id);
                text(
                    &mut buffer,
                    w,
                    h,
                    292,
                    162,
                    if process_running {
                        "RUNNING  •  LIVE CONSOLE"
                    } else {
                        "NOT RUNNING  •  LIVE CONSOLE"
                    },
                    0x00bdc2ca,
                    1,
                );
                if !self.console_text.is_empty() {
                    let share_bounds = instance_console_share_bounds(w);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        share_bounds.x,
                        share_bounds.y,
                        share_bounds.width,
                        share_bounds.height,
                        6,
                        if self.log_share_pending {
                            0x0022262c
                        } else {
                            0x00303b2b
                        },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        share_bounds.x + 9,
                        share_bounds.y + 10,
                        if self.log_share_pending {
                            "SHARING"
                        } else {
                            "SHARE"
                        },
                        0x00a8e66b,
                        1,
                    );
                    let clear_bounds = instance_console_clear_bounds(w);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        clear_bounds.x,
                        clear_bounds.y,
                        clear_bounds.width,
                        clear_bounds.height,
                        6,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        clear_bounds.x + 10,
                        clear_bounds.y + 10,
                        "CLEAR",
                        0x00bdc2ca,
                        1,
                    );
                }
                let share_status = if self.log_share_pending
                    || self.message.starts_with("Share link copied")
                    || self.message.starts_with("Log shared, but")
                    || self.message.starts_with("Could not share log:")
                {
                    self.message.as_str()
                } else {
                    "Share uploads the live console text to mclo.gs."
                };
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    292,
                    181,
                    share_status,
                    0x007b838d,
                    1,
                    library_width.saturating_sub(324),
                );
                if process_running {
                    rect(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(142),
                        154,
                        112,
                        28,
                        0x003b302b,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(127),
                        164,
                        "STOP GAME",
                        0x00e68a6b,
                        1,
                    );
                }
                let search_bounds = instance_console_search_bounds(w);
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    search_bounds.x,
                    search_bounds.y,
                    search_bounds.width,
                    search_bounds.height,
                    6,
                    if self.input_focus == 40 {
                        0x002b3038
                    } else {
                        0x0022262c
                    },
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    search_bounds.x + 10,
                    search_bounds.y + 10,
                    if self.log_search_query.is_empty() {
                        "SEARCH LOGS"
                    } else {
                        &self.log_search_query
                    },
                    if self.log_search_query.is_empty() {
                        0x007b838d
                    } else {
                        0x00f1f3f5
                    },
                    1,
                    search_bounds.width.saturating_sub(116),
                );
                let match_label = if self.log_search_query.trim().is_empty() {
                    format!("{} LINES", log_lines.len())
                } else {
                    format!("{} MATCHES", log_lines.len())
                };
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    search_bounds.x + search_bounds.width.saturating_sub(100),
                    search_bounds.y + 10,
                    &match_label,
                    0x007b838d,
                    1,
                    92,
                );
                draw_native_log_filter_pills(
                    &mut buffer,
                    w,
                    h,
                    console_filter_top,
                    present_levels,
                    self.log_level_filter,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    292,
                    console_body_top,
                    w.saturating_sub(324),
                    h.saturating_sub(console_body_top + 24),
                    0x00141619,
                );
                let log_line_width = library_width.saturating_sub(344);
                for (line_index, line) in
                    log_lines[start..end].iter().enumerate()
                {
                    draw_log_line_with_search_highlight(
                        &mut buffer,
                        w,
                        h,
                        302,
                        (console_body_top + 16 + line_index * 12)
                            .saturating_sub(scroll_remainder),
                        line,
                        &self.log_search_query,
                        log_line_width,
                    );
                }
            } else if self.instance_tab == 3 {
                let instance_filter_summary = (!self.global_browse)
                    .then(|| {
                        self.selected_instance
                            .as_ref()
                            .and_then(|(id, _)| self.instance_cards.get(id))
                            .and_then(|card| {
                                browse_instance_filter_summary(
                                    match self.browse_filter {
                                        1 => "resourcepack",
                                        2 => "datapack",
                                        3 => "shader",
                                        4 => "modpack",
                                        _ => "mod",
                                    },
                                    &card.game_version,
                                    &card.loader,
                                )
                            })
                    })
                    .flatten();
                browse_panel::draw_browse_panel(
                    &mut buffer,
                    w,
                    h,
                    library_width,
                    self.global_browse,
                    instance_filter_summary.as_deref(),
                    self.browse_filter,
                    &self.browse_query,
                    self.browse_query_cursor,
                    self.browse_query_select_all,
                    self.input_focus == 5,
                    browse_selected_installed,
                    if self.global_browse {
                        &[]
                    } else {
                        &self.contents
                    },
                    browse_installing_for_instance,
                    &self.browse_installing_modpacks,
                    self.browse_sort,
                    &self.browse_icons,
                    &self.browse_icon_failures,
                    &self.message,
                    self.browse_error.as_deref(),
                    self.browse_search_pending,
                    self.settings.as_ref().is_some_and(|settings| {
                        settings.hide_installed_modpacks
                    }),
                    self.browse_selected.as_ref(),
                    self.browse_details.as_ref(),
                    self.browse_gallery_index,
                    self.browse_gallery_viewer_open,
                    self.browse_detail_scroll_pixels,
                    self.browse_detail_tab,
                    self.browse_description_lines
                        .as_ref()
                        .map_or(&[], |(_, _, lines, _, _, _)| lines.as_slice()),
                    self.browse_description_lines
                        .as_ref()
                        .map_or(&[], |(_, _, _, links, _, _)| links.as_slice()),
                    self.browse_description_lines
                        .as_ref()
                        .map_or(&[], |(_, _, _, _, styles, _)| {
                            styles.as_slice()
                        }),
                    self.browse_description_lines
                        .as_ref()
                        .map_or(&[], |(_, _, _, _, _, images)| {
                            images.as_slice()
                        }),
                    &self.browse_results,
                    self.browse_scroll_pixels,
                    self.browse_search_offset,
                    self.browse_page_size,
                    self.browse_total_hits,
                    browse_filter_sidebar_visible,
                    self.browse_filter_menu_open
                        && !browse_filter_sidebar_visible,
                    browse_filter_waiting_for_tags,
                    self.browse_filter_group,
                    browse_filter_group_count,
                    self.browse_show_all_versions,
                    &browse_filter_menu_options,
                    &self.browse_selected_game_versions,
                    &self.browse_selected_loaders,
                    &self.browse_selected_categories,
                    &self.browse_selected_environments,
                    &self.browse_selected_licenses,
                    &self.browse_detail_selected_channels,
                    &self.browse_detail_selected_game_versions,
                    &self.browse_detail_selected_loaders,
                    self.browse_detail_filter_menu_group,
                    self.browse_detail_filter_option_offset,
                    self.browse_project_menu_open,
                    self.browse_project_followed,
                    self.browse_project_follow_pending,
                    self.browse_project_follow_status_loading,
                    self.browse_save_menu_open,
                    &self.browse_save_collections,
                    self.browse_save_collections_loading,
                    &self.browse_save_collection_pending,
                    self.browse_save_scroll_pixels,
                    self.browse_filter_scroll_pixels,
                    self.browse_view_menu_open,
                    self.browse_sort_menu_open,
                );
            } else if self.instance_tab == 4 {
                for (x, label, page) in
                    [(560, "JAVA + VIDEO", 0), (674, "ENV + HOOKS", 1)]
                {
                    rect(
                        &mut buffer,
                        w,
                        h,
                        x,
                        146,
                        108,
                        26,
                        if self.instance_setup_page == page {
                            0x00303b2b
                        } else {
                            0x0022262c
                        },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 7,
                        155,
                        label,
                        if self.instance_setup_page == page {
                            0x00a8e66b
                        } else {
                            0x00bdc2ca
                        },
                        1,
                    );
                }
                if self.instance_setup_page == 0 {
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        153,
                        "JAVA EXECUTABLE",
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        164,
                        w.saturating_sub(470),
                        34,
                        0x0022262c,
                    );
                    let java_value: String =
                        if self.instance_java_path.is_empty() {
                            "Use the launcher default".into()
                        } else {
                            self.instance_java_path.clone()
                        };
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        302,
                        177,
                        &java_value,
                        if self.instance_java_path.is_empty() {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        },
                        1,
                        w.saturating_sub(490),
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(142),
                        164,
                        112,
                        34,
                        0x00303b2b,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(128),
                        177,
                        "CHOOSE",
                        0x00a8e66b,
                        1,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        213,
                        "EXTRA JAVA ARGUMENTS",
                        0x00bdc2ca,
                        1,
                    );
                    rect(&mut buffer, w, h, 292, 224, 440, 34, 0x0022262c);
                    text(
                        &mut buffer,
                        w,
                        h,
                        302,
                        237,
                        if self.instance_launch_args.is_empty() {
                            "Use default arguments"
                        } else {
                            &self.instance_launch_args
                        },
                        if self.instance_launch_args.is_empty() {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        },
                        1,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        273,
                        &format!(
                            "MAXIMUM MEMORY (MB, 512–{})",
                            self.instance_max_memory_mb
                        ),
                        0x00bdc2ca,
                        1,
                    );
                    rect(&mut buffer, w, h, 292, 284, 180, 34, 0x0022262c);
                    text(
                        &mut buffer,
                        w,
                        h,
                        302,
                        297,
                        if self.instance_memory_mb.is_empty() {
                            "Use launcher default"
                        } else {
                            &self.instance_memory_mb
                        },
                        if self.instance_memory_mb.is_empty() {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        },
                        1,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        333,
                        "FORCE FULLSCREEN",
                        0x00bdc2ca,
                        1,
                    );
                    rect(&mut buffer, w, h, 292, 344, 200, 34, 0x00303b2b);
                    text(
                        &mut buffer,
                        w,
                        h,
                        304,
                        357,
                        match self.instance_fullscreen_mode {
                            1 => "ON",
                            2 => "OFF",
                            _ => "USE LAUNCHER DEFAULT",
                        },
                        0x00a8e66b,
                        1,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        390,
                        "GAME WINDOW RESOLUTION",
                        0x00bdc2ca,
                        1,
                    );
                    rect(&mut buffer, w, h, 292, 401, 120, 34, 0x0022262c);
                    text(
                        &mut buffer,
                        w,
                        h,
                        302,
                        414,
                        if self.instance_resolution_width.is_empty() {
                            "DEFAULT"
                        } else {
                            &self.instance_resolution_width
                        },
                        if self.instance_resolution_width.is_empty() {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        },
                        1,
                    );
                    text(&mut buffer, w, h, 423, 412, "×", 0x00bdc2ca, 1);
                    rect(&mut buffer, w, h, 446, 401, 120, 34, 0x0022262c);
                    text(
                        &mut buffer,
                        w,
                        h,
                        456,
                        414,
                        if self.instance_resolution_height.is_empty() {
                            "DEFAULT"
                        } else {
                            &self.instance_resolution_height
                        },
                        if self.instance_resolution_height.is_empty() {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        },
                        1,
                    );
                    rect(&mut buffer, w, h, 292, 455, 166, 34, 0x00303b2b);
                    text(
                        &mut buffer,
                        w,
                        h,
                        306,
                        468,
                        if self.instance_settings_saving {
                            "SAVING…"
                        } else {
                            "SAVE SETTINGS"
                        },
                        0x00a8e66b,
                        1,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        505,
                        &self.message,
                        0x007b838d,
                        1,
                        library_width.saturating_sub(324),
                    );
                } else {
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        185,
                        "ENVIRONMENT VARIABLES AS JSON PAIRS",
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        196,
                        w.saturating_sub(324),
                        34,
                        0x0022262c,
                    );
                    let env_text = &self.instance_environment_json;
                    if self.input_focus == 12 {
                        text_input_clipped(
                            &mut buffer,
                            w,
                            h,
                            302,
                            209,
                            env_text,
                            if env_text.is_empty() {
                                0x007b838d
                            } else {
                                0x00f1f3f5
                            },
                            1,
                            w.saturating_sub(340),
                        );
                    } else {
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            302,
                            209,
                            if env_text.is_empty() {
                                "[ [\"KEY\", \"value\"] ] or leave blank for defaults"
                            } else {
                                env_text
                            },
                            if env_text.is_empty() {
                                0x007b838d
                            } else {
                                0x00f1f3f5
                            },
                            1,
                            w.saturating_sub(340),
                        );
                    }
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        238,
                        "PRE-LAUNCH COMMAND",
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        249,
                        w.saturating_sub(324),
                        34,
                        0x0022262c,
                    );
                    if self.input_focus == 13 {
                        text_input_clipped(
                            &mut buffer,
                            w,
                            h,
                            302,
                            262,
                            &self.instance_pre_launch_hook,
                            0x00f1f3f5,
                            1,
                            w.saturating_sub(340),
                        );
                    } else {
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            302,
                            262,
                            if self.instance_pre_launch_hook.is_empty() {
                                ""
                            } else {
                                &self.instance_pre_launch_hook
                            },
                            0x00f1f3f5,
                            1,
                            w.saturating_sub(340),
                        );
                    }
                    if self.input_focus == 14 {
                        text_input_clipped(
                            &mut buffer,
                            w,
                            h,
                            302,
                            318,
                            &self.instance_wrapper_hook,
                            0x00f1f3f5,
                            1,
                            w.saturating_sub(340),
                        );
                    } else {
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            302,
                            318,
                            &self.instance_wrapper_hook,
                            0x00f1f3f5,
                            1,
                            w.saturating_sub(340),
                        );
                    }
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        294,
                        "JAVA WRAPPER COMMAND",
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        305,
                        w.saturating_sub(324),
                        34,
                        0x0022262c,
                    );
                    if self.input_focus == 15 {
                        text_input_clipped(
                            &mut buffer,
                            w,
                            h,
                            302,
                            374,
                            &self.instance_post_exit_hook,
                            0x00f1f3f5,
                            1,
                            w.saturating_sub(340),
                        );
                    } else {
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            302,
                            374,
                            &self.instance_post_exit_hook,
                            0x00f1f3f5,
                            1,
                            w.saturating_sub(340),
                        );
                    }
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        350,
                        "POST-EXIT COMMAND",
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        361,
                        w.saturating_sub(324),
                        34,
                        0x0022262c,
                    );
                    rect(&mut buffer, w, h, 292, 412, 166, 34, 0x00303b2b);
                    text(
                        &mut buffer,
                        w,
                        h,
                        306,
                        425,
                        if self.instance_settings_saving {
                            "SAVING…"
                        } else {
                            "SAVE SETTINGS"
                        },
                        0x00a8e66b,
                        1,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        463,
                        &self.message,
                        0x007b838d,
                        1,
                        library_width.saturating_sub(324),
                    );
                }
            } else if self.instance_tab == 5 {
                if let Some((filename, is_crash)) = self.selected_log.as_ref() {
                    rect(&mut buffer, w, h, 292, 146, 112, 28, 0x0022262c);
                    text(
                        &mut buffer,
                        w,
                        h,
                        302,
                        156,
                        "BACK TO LOGS",
                        0x00a8e66b,
                        1,
                    );
                    let title = format!(
                        "{} · {}",
                        if *is_crash {
                            "CRASH REPORT"
                        } else {
                            "GAME LOG"
                        },
                        filename
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        416,
                        156,
                        &title,
                        0x00bdc2ca,
                        1,
                        library_width.saturating_sub(450),
                    );
                    if let Some(
                        [
                            copy_bounds,
                            share_bounds,
                            delete_bounds,
                            analyse_bounds,
                        ],
                    ) = instance_log_actions
                    {
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            copy_bounds.x,
                            copy_bounds.y,
                            copy_bounds.width,
                            copy_bounds.height,
                            6,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            copy_bounds.x + 10,
                            copy_bounds.y + 10,
                            "COPY",
                            0x00bdc2ca,
                            1,
                        );
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            share_bounds.x,
                            share_bounds.y,
                            share_bounds.width,
                            share_bounds.height,
                            6,
                            if self.log_share_pending {
                                0x0022262c
                            } else {
                                0x00303b2b
                            },
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            share_bounds.x + 10,
                            share_bounds.y + 10,
                            if self.log_share_pending {
                                "SHARING"
                            } else {
                                "SHARE"
                            },
                            0x00a8e66b,
                            1,
                        );
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            delete_bounds.x,
                            delete_bounds.y,
                            delete_bounds.width,
                            delete_bounds.height,
                            6,
                            if self.log_delete_pending {
                                0x0022262c
                            } else if log_delete_disabled {
                                0x0022262c
                            } else {
                                0x003b302b
                            },
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            delete_bounds.x + 10,
                            delete_bounds.y + 10,
                            if self.log_delete_pending {
                                "DELETING"
                            } else {
                                "DELETE"
                            },
                            if log_delete_disabled {
                                0x007b838d
                            } else {
                                0x00e68a6b
                            },
                            1,
                        );
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            analyse_bounds.x,
                            analyse_bounds.y,
                            analyse_bounds.width,
                            analyse_bounds.height,
                            6,
                            if self.log_analysis_pending {
                                0x0022262c
                            } else {
                                0x00303b2b
                            },
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            analyse_bounds.x + 10,
                            analyse_bounds.y + 10,
                            if self.log_analysis_pending {
                                "ANALYZING"
                            } else {
                                "ANALYZE"
                            },
                            0x00a8e66b,
                            1,
                        );
                    }
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        176,
                        &self.message,
                        0x007b838d,
                        1,
                    );
                    let search_bounds = instance_log_search_bounds(
                        w,
                        self.log_analysis.is_some(),
                    );
                    let log_filter_top =
                        search_bounds.y + search_bounds.height + 8;
                    let present_levels =
                        native_log_present_levels(&self.log_text);
                    let log_body_top = instance_log_filter_bottom(
                        w,
                        log_filter_top,
                        present_levels,
                    ) + 8;
                    if let Some(analysis) = self.log_analysis.as_ref() {
                        rounded_rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            196,
                            w.saturating_sub(324),
                            96,
                            8,
                            0x001b1d22,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            302,
                            204,
                            "CRASH INSIGHTS",
                            0x00e6a66b,
                            1,
                        );
                        let insight_lines = log_insight_lines(analysis);
                        if insight_lines.is_empty() {
                            text_clipped(
                                &mut buffer,
                                w,
                                h,
                                302,
                                224,
                                "No known crash problems were detected.",
                                0x00bdc2ca,
                                1,
                                library_width.saturating_sub(344),
                            );
                        } else {
                            for (index, line) in
                                insight_lines.iter().take(4).enumerate()
                            {
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    302,
                                    224 + index * 16,
                                    line,
                                    if line.starts_with("Fix:") {
                                        0x00a8e66b
                                    } else {
                                        0x00bdc2ca
                                    },
                                    1,
                                    library_width.saturating_sub(344),
                                );
                            }
                        }
                    }
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        search_bounds.x,
                        search_bounds.y,
                        search_bounds.width,
                        search_bounds.height,
                        6,
                        if self.input_focus == 40 {
                            0x002b3038
                        } else {
                            0x0022262c
                        },
                    );
                    text_input_clipped(
                        &mut buffer,
                        w,
                        h,
                        search_bounds.x + 10,
                        search_bounds.y + 10,
                        if self.log_search_query.is_empty() {
                            "SEARCH LOGS"
                        } else {
                            &self.log_search_query
                        },
                        if self.log_search_query.is_empty() {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        },
                        1,
                        search_bounds.width.saturating_sub(116),
                    );
                    draw_native_log_filter_pills(
                        &mut buffer,
                        w,
                        h,
                        log_filter_top,
                        present_levels,
                        self.log_level_filter,
                    );
                    let matching_log_line_count = native_log_match_count(
                        &self.log_text,
                        &self.log_search_query,
                        self.log_level_filter,
                    );
                    let match_label = if self.log_search_query.trim().is_empty()
                    {
                        format!("{} LINES", matching_log_line_count)
                    } else {
                        format!("{} MATCHES", matching_log_line_count)
                    };
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        search_bounds.x
                            + search_bounds.width.saturating_sub(100),
                        search_bounds.y + 10,
                        &match_label,
                        0x007b838d,
                        1,
                        92,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        log_body_top,
                        w.saturating_sub(324),
                        h.saturating_sub(log_body_top + 24),
                        0x00141619,
                    );
                    let query = self.log_search_query.trim().to_lowercase();
                    let lines = self
                        .log_text
                        .lines()
                        .filter(|line| {
                            native_log_line_matches_filter(
                                line,
                                self.log_level_filter,
                            ) && (query.is_empty()
                                || line.to_lowercase().contains(&query))
                        })
                        .collect::<Vec<_>>();
                    let max_lines =
                        (h.saturating_sub(log_body_top + 38) / 12).max(1);
                    let max_scroll = lines
                        .len()
                        .saturating_sub(max_lines)
                        .saturating_mul(12)
                        as f64;
                    let scroll_pixels = self.log_scroll_pixels.min(max_scroll);
                    let scroll = (scroll_pixels / 12.0).floor() as usize;
                    let scroll_remainder =
                        scroll_pixels.rem_euclid(12.0).round() as usize;
                    let end = lines.len().saturating_sub(scroll);
                    let start = end.saturating_sub(max_lines);
                    let log_line_width = library_width.saturating_sub(344);
                    for (index, line) in lines[start..end].iter().enumerate() {
                        draw_log_line_with_search_highlight(
                            &mut buffer,
                            w,
                            h,
                            302,
                            (log_body_top + 16 + index * 12)
                                .saturating_sub(scroll_remainder),
                            line,
                            &self.log_search_query,
                            log_line_width,
                        );
                    }
                } else {
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        156,
                        &self.message,
                        0x007b838d,
                        1,
                        library_width.saturating_sub(456),
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(142),
                        146,
                        112,
                        28,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(128),
                        156,
                        "REFRESH",
                        0x00a8e66b,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        182,
                        w.saturating_sub(324),
                        1,
                        0x00363b42,
                    );
                    let visible_rows = (h.saturating_sub(208) / 42).min(14);
                    let max_scroll =
                        self.instance_logs
                            .len()
                            .saturating_sub(visible_rows)
                            .saturating_mul(42) as f64;
                    let scroll_pixels = self.log_scroll_pixels.min(max_scroll);
                    let row_offset = (scroll_pixels / 42.0).floor() as usize;
                    let scroll_remainder =
                        scroll_pixels.rem_euclid(42.0).round() as usize;
                    for (index, (filename, is_crash, _age)) in self
                        .instance_logs
                        .iter()
                        .skip(row_offset)
                        .take(visible_rows + usize::from(scroll_remainder > 0))
                        .enumerate()
                    {
                        let y =
                            (190 + index * 42).saturating_sub(scroll_remainder);
                        rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            y,
                            w.saturating_sub(324),
                            36,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            304,
                            y + 6,
                            if *is_crash {
                                "CRASH REPORT"
                            } else {
                                "GAME LOG"
                            },
                            if *is_crash { 0x00e68a6b } else { 0x00a8e66b },
                            1,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            304,
                            y + 21,
                            filename,
                            0x00bdc2ca,
                            1,
                            library_width.saturating_sub(350),
                        );
                    }
                    if scroll_pixels > 0.0 {
                        flush_native_text(&mut buffer, w, h);
                        clear_text_commands();
                        rect(
                            &mut buffer,
                            w,
                            h,
                            248,
                            146,
                            w.saturating_sub(248),
                            44,
                            0x001b1d22,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            292,
                            156,
                            &self.message,
                            0x007b838d,
                            1,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            w.saturating_sub(142),
                            146,
                            112,
                            28,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            w.saturating_sub(128),
                            156,
                            "REFRESH",
                            0x00a8e66b,
                            1,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            182,
                            w.saturating_sub(324),
                            1,
                            0x00363b42,
                        );
                        flush_native_text(&mut buffer, w, h);
                        clear_text_commands();
                    }
                }
            } else if self.instance_tab == 6 {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    292,
                    156,
                    &self.message,
                    0x00bdc2ca,
                    1,
                    library_width.saturating_sub(324),
                );
                for (bounds, label, description, _) in instance_more_rows {
                    let hovered = bounds.contains(cursor.0, cursor.1);
                    rounded_rect(
                        &mut buffer,
                        w,
                        h,
                        bounds.x,
                        bounds.y,
                        bounds.width,
                        bounds.height,
                        10,
                        if hovered { 0x00343a42 } else { 0x0022262c },
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        bounds.x + 14,
                        bounds.y + 7,
                        label,
                        0x00f1f3f5,
                        1,
                        bounds.width.saturating_sub(68),
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        bounds.x + 14,
                        bounds.y + 26,
                        description,
                        0x00bdc2ca,
                        1,
                        bounds.width.saturating_sub(68),
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        bounds.x + bounds.width.saturating_sub(28),
                        bounds.y + 17,
                        "›",
                        0x00a8e66b,
                        2,
                    );
                }
            } else if self.instance_tab == 7 {
                let visible_rows = ((h.saturating_sub(190) + 53) / 54).max(1);
                if self.world_offset != self.world_scroll_synced {
                    self.world_scroll_pixels = (self.world_offset * 54) as f64;
                    self.world_scroll_target_pixels = self.world_scroll_pixels;
                    self.world_scroll_synced = self.world_offset;
                }
                let max_scroll = self
                    .worlds
                    .len()
                    .saturating_mul(54)
                    .saturating_sub(h.saturating_sub(190))
                    as f64;
                self.world_scroll_pixels =
                    self.world_scroll_pixels.min(max_scroll);
                self.world_scroll_target_pixels =
                    self.world_scroll_target_pixels.min(max_scroll);
                let row_offset =
                    (self.world_scroll_pixels / 54.0).floor() as usize;
                let scroll_remainder = (self.world_scroll_pixels as usize) % 54;
                self.world_offset = row_offset;
                self.world_scroll_synced = row_offset;
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    292,
                    156,
                    &self.message,
                    0x007b838d,
                    1,
                    library_width.saturating_sub(564),
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(260),
                    146,
                    112,
                    28,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(245),
                    156,
                    "ADD SERVER",
                    0x00a8e66b,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(142),
                    146,
                    112,
                    28,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    w.saturating_sub(128),
                    156,
                    "REFRESH",
                    0x00a8e66b,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    292,
                    182,
                    w.saturating_sub(324),
                    1,
                    0x00363b42,
                );
                for (index, world) in self
                    .worlds
                    .iter()
                    .skip(row_offset)
                    .take(visible_rows)
                    .enumerate()
                {
                    let y = 190 + index * 54 - scroll_remainder;
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        y,
                        w.saturating_sub(324),
                        48,
                        0x0022262c,
                    );
                    let editable_server = matches!(
                        &world.details,
                        worlds::WorldDetails::Server {
                            source:
                                theseus::instance::ServerSource::LocalDesynced,
                            ..
                        }
                    );
                    let editable_world = matches!(
                        &world.details,
                        worlds::WorldDetails::Singleplayer {
                            locked: false,
                            ..
                        }
                    );
                    let action_left = if editable_server || editable_world {
                        w.saturating_sub(330)
                    } else {
                        w.saturating_sub(260)
                    };
                    let row_text_width = action_left.saturating_sub(316);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        304,
                        y + 6,
                        &world.name,
                        0x00f1f3f5,
                        1,
                        row_text_width,
                    );
                    let details = match &world.details {
                        worlds::WorldDetails::Singleplayer {
                            game_mode,
                            hardcore,
                            locked,
                            ..
                        } => format!(
                            "WORLD · {game_mode:?}{}{}",
                            if *hardcore { " · HARDCORE" } else { "" },
                            if *locked { " · RUNNING" } else { "" }
                        ),
                        worlds::WorldDetails::Server { address, .. } => {
                            format!("SERVER · {address}")
                        }
                    };
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        304,
                        y + 25,
                        &details,
                        0x007b838d,
                        1,
                        row_text_width,
                    );
                    if let worlds::WorldDetails::Server { address, .. } =
                        &world.details
                    {
                        if let Some(status) = self.server_statuses.get(address)
                        {
                            let status_width = action_left.saturating_sub(610);
                            if status_width >= 32 {
                                text_clipped(
                                    &mut buffer,
                                    w,
                                    h,
                                    600,
                                    y + 25,
                                    status,
                                    0x00a8e66b,
                                    1,
                                    status_width,
                                );
                            }
                        }
                    }
                    if editable_server || editable_world {
                        rect(
                            &mut buffer,
                            w,
                            h,
                            w.saturating_sub(330),
                            y + 9,
                            58,
                            30,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            w.saturating_sub(320),
                            y + 20,
                            "EDIT",
                            0x00f1f3f5,
                            1,
                        );
                    }
                    let favorite = matches!(
                        &world.display_status,
                        worlds::DisplayStatus::Favorite
                    );
                    let hidden = matches!(
                        &world.display_status,
                        worlds::DisplayStatus::Hidden
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(260),
                        y + 9,
                        66,
                        30,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(254),
                        y + 20,
                        if hidden { "UNHIDE" } else { "HIDE" },
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(188),
                        y + 9,
                        82,
                        30,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(182),
                        y + 20,
                        if favorite { "UNFAVORITE" } else { "FAVORITE" },
                        0x00a8e66b,
                        1,
                    );
                    let is_locked = matches!(
                        &world.details,
                        worlds::WorldDetails::Singleplayer { locked: true, .. }
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(100),
                        y + 9,
                        76,
                        30,
                        if is_locked { 0x00202429 } else { 0x00303b2b },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(88),
                        y + 20,
                        if is_locked { "RUNNING" } else { "PLAY" },
                        if is_locked { 0x007b838d } else { 0x00a8e66b },
                        1,
                    );
                }
            } else if self.instance_tab == 9 {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    292,
                    156,
                    &self.message,
                    0x00bdc2ca,
                    1,
                    library_width.saturating_sub(324),
                );
                let attachment = self
                    .shared_instance_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.shared_instance.as_ref());
                let can_manage = self.shared_instance_can_use == Some(true);
                let is_owner = attachment.is_some_and(|attachment| {
                    attachment.role == theseus::data::SharedInstanceRole::Owner
                });
                let mut actions = Vec::new();
                if can_manage && is_owner {
                    actions.push((292, 96, "INVITE"));
                }
                if can_manage && (attachment.is_none() || is_owner) {
                    actions.push((w.saturating_sub(362), 100, "REVIEW"));
                }
                if can_manage && is_owner {
                    actions.push((w.saturating_sub(252), 100, "CREATE LINK"));
                }
                actions.push((w.saturating_sub(142), 112, "REFRESH"));
                for (x, width, label) in actions {
                    rect(&mut buffer, w, h, x, 146, width, 28, 0x0022262c);
                    text(&mut buffer, w, h, x + 6, 156, label, 0x00a8e66b, 1);
                }
                if let Some(attachment) = self
                    .shared_instance_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.shared_instance.as_ref())
                {
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        192,
                        &format!(
                            "SHARED INSTANCE · {} · VERSION {} OF {}",
                            attachment.role.as_str().to_uppercase(),
                            attachment.applied_version.unwrap_or_default(),
                            attachment.latest_version.unwrap_or_default()
                        ),
                        0x00a8e66b,
                        1,
                    );
                }
                if let Some(preview) = &self.shared_instance_preview {
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        224,
                        &format!("CONTENT CHANGES · {}", preview.diffs.len()),
                        0x00f1f3f5,
                        1,
                    );
                    for (index, diff) in
                        preview.diffs.iter().take(3).enumerate()
                    {
                        let summary = diff
                            .project_name
                            .as_deref()
                            .or(diff.file_name.as_deref())
                            .unwrap_or("Content change");
                        text(
                            &mut buffer,
                            w,
                            h,
                            304,
                            246 + index * 18,
                            &format!("{:?} · {}", diff.type_, summary),
                            0x00bdc2ca,
                            1,
                        );
                    }
                    let files_top = 310;
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        files_top,
                        &format!(
                            "FILES · {} SELECTED · {} AVAILABLE",
                            self.shared_instance_selected_files.len(),
                            preview
                                .files
                                .iter()
                                .filter(|file| !file.blocked)
                                .count()
                        ),
                        0x00f1f3f5,
                        1,
                    );
                    let visible =
                        (h.saturating_sub(files_top + 142) / 28).max(1);
                    let max_scroll = preview
                        .files
                        .len()
                        .saturating_sub(visible)
                        .saturating_mul(28)
                        as f64;
                    self.shared_instance_preview_scroll_target_pixels = self
                        .shared_instance_preview_scroll_target_pixels
                        .min(max_scroll);
                    self.shared_instance_preview_scroll_pixels = self
                        .shared_instance_preview_scroll_pixels
                        .min(max_scroll);
                    let row_offset =
                        (self.shared_instance_preview_scroll_pixels / 28.0)
                            .floor() as usize;
                    let scroll_remainder =
                        self.shared_instance_preview_scroll_pixels
                            .rem_euclid(28.0)
                            .round() as usize;
                    let list_bottom = (files_top + 22 + visible * 28)
                        .min(h.saturating_sub(100));
                    render::set_clip_rect(Some((
                        292,
                        files_top + 22,
                        w.saturating_sub(324),
                        list_bottom.saturating_sub(files_top + 22),
                    )));
                    let total_size = preview
                        .files
                        .iter()
                        .filter(|file| {
                            !file.blocked
                                && self
                                    .shared_instance_selected_files
                                    .contains(&file.path)
                        })
                        .fold(0u64, |total, file| {
                            total.saturating_add(file.size)
                        });
                    for (index, file) in preview
                        .files
                        .iter()
                        .skip(row_offset)
                        .take(visible + 1)
                        .enumerate()
                    {
                        let y = files_top + 22 + index * 28 - scroll_remainder;
                        let selected = self
                            .shared_instance_selected_files
                            .contains(&file.path);
                        rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            y,
                            w.saturating_sub(324),
                            24,
                            0x0022262c,
                        );
                        let marker = if file.blocked {
                            "BLOCKED"
                        } else if selected {
                            "[x]"
                        } else {
                            "[ ]"
                        };
                        text(
                            &mut buffer,
                            w,
                            h,
                            300,
                            y + 8,
                            marker,
                            if file.blocked { 0x00e68a6b } else { 0x00a8e66b },
                            1,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            354,
                            y + 8,
                            &file.path,
                            0x00bdc2ca,
                            1,
                            library_width.saturating_sub(440),
                        );
                    }
                    render::set_clip_rect(None);
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        h.saturating_sub(84),
                        &format!(
                            "SELECTED FILE DATA: {:.1} MiB / 2048 MiB",
                            total_size as f64 / 1_048_576.0
                        ),
                        0x007b838d,
                        1,
                        library_width.saturating_sub(324),
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(196),
                        h.saturating_sub(70),
                        172,
                        32,
                        0x00303b2b,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(188),
                        h.saturating_sub(59),
                        &format!(
                            "PUBLISH {} FILES",
                            self.shared_instance_selected_files.len()
                        ),
                        0x00a8e66b,
                        1,
                    );
                } else {
                    text(&mut buffer, w, h, 292, 224, "MEMBERS", 0x00f1f3f5, 1);
                    let attachment = self
                        .shared_instance_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.shared_instance.as_ref());
                    let visible_members = self
                        .shared_instance_users
                        .iter()
                        .filter(|member| {
                            !attachment.is_some_and(|attachment| {
                                attachment.linked_user_id.as_deref()
                                    == Some(member.id.as_str())
                                    || attachment.manager_id.as_deref()
                                        == Some(member.id.as_str())
                            })
                        })
                        .take(8)
                        .collect::<Vec<_>>();
                    for (index, member) in visible_members.iter().enumerate() {
                        let y = 244 + index * 40;
                        rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            y,
                            w.saturating_sub(324),
                            34,
                            0x0022262c,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            304,
                            y + 12,
                            &member.id,
                            0x00bdc2ca,
                            1,
                            library_width.saturating_sub(416),
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            w.saturating_sub(204),
                            y + 12,
                            &format!("{:?}", member.join_type).to_uppercase(),
                            0x007b838d,
                            1,
                        );
                        if member.join_type
                            != theseus::instance::SharedInstanceJoinType::Owner
                        {
                            rect(
                                &mut buffer,
                                w,
                                h,
                                w.saturating_sub(100),
                                y + 3,
                                76,
                                28,
                                0x003b302b,
                            );
                            text(
                                &mut buffer,
                                w,
                                h,
                                w.saturating_sub(96),
                                y + 12,
                                "REVOKE",
                                0x00e68a6b,
                                1,
                            );
                        }
                    }
                    let invite_top = 252 + visible_members.len() * 40;
                    text(
                        &mut buffer,
                        w,
                        h,
                        292,
                        invite_top,
                        "ACTIVE INVITE LINKS",
                        0x00f1f3f5,
                        1,
                    );
                    for (index, invite) in self
                        .shared_instance_invites
                        .iter()
                        .take(h.saturating_sub(invite_top + 100) / 42)
                        .enumerate()
                    {
                        let y = invite_top + 20 + index * 40;
                        rect(
                            &mut buffer,
                            w,
                            h,
                            292,
                            y,
                            w.saturating_sub(324),
                            34,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            304,
                            y + 12,
                            &format!(
                                "{} / {} uses · expires {}",
                                invite.uses,
                                invite.max_uses,
                                invite.expiration.format("%Y-%m-%d")
                            ),
                            0x00bdc2ca,
                            1,
                        );
                        rect(
                            &mut buffer,
                            w,
                            h,
                            w.saturating_sub(94),
                            y + 3,
                            64,
                            28,
                            0x003b302b,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            w.saturating_sub(89),
                            y + 12,
                            "REVOKE",
                            0x00e68a6b,
                            1,
                        );
                    }
                }
                if let Some(link) = &self.shared_instance_invite_link {
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        292,
                        h.saturating_sub(54),
                        link,
                        0x007b838d,
                        1,
                        library_width.saturating_sub(324),
                    );
                }
                if self.shared_instance_invite_open {
                    let panel_width = 500.min(w.saturating_sub(32));
                    let panel_height = 420.min(h.saturating_sub(32));
                    let panel_x = (w.saturating_sub(panel_width)) / 2;
                    let panel_y = (h.saturating_sub(panel_height)) / 2;
                    rect(&mut buffer, w, h, 0, 0, w, h, 0x00101418);
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x,
                        panel_y,
                        panel_width,
                        panel_height,
                        0x001c2026,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + 20,
                        "INVITE PLAYERS",
                        0x00f1f3f5,
                        2,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + 42,
                        panel_width.saturating_sub(162),
                        32,
                        0x0022262c,
                    );
                    text_input_clipped(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 24,
                        panel_y + 53,
                        if self.shared_instance_user_query.is_empty() {
                            "SEARCH USERNAME"
                        } else {
                            &self.shared_instance_user_query
                        },
                        if self.shared_instance_user_query.is_empty() {
                            0x007b838d
                        } else {
                            0x00f1f3f5
                        },
                        1,
                        panel_width.saturating_sub(178),
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 134,
                        panel_y + 42,
                        118,
                        32,
                        0x00303b2b,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 124,
                        panel_y + 53,
                        "SEARCH",
                        0x00a8e66b,
                        1,
                    );
                    for (index, user) in self
                        .shared_instance_user_results
                        .iter()
                        .take(6)
                        .enumerate()
                    {
                        let y = panel_y + 84 + index * 42;
                        let selected = self
                            .shared_instance_selected_user_ids
                            .contains(&user.id);
                        rect(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 16,
                            y,
                            panel_width.saturating_sub(32),
                            36,
                            0x0022262c,
                        );
                        text(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 24,
                            y + 13,
                            if selected { "[x]" } else { "[ ]" },
                            0x00a8e66b,
                            1,
                        );
                        text_clipped(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 54,
                            y + 13,
                            &user.username,
                            0x00f1f3f5,
                            1,
                            panel_width.saturating_sub(214),
                        );
                    }
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + panel_height - 36,
                        &format!(
                            "{} SELECTED",
                            self.shared_instance_selected_user_ids.len()
                        ),
                        0x007b838d,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 192,
                        panel_y + panel_height - 48,
                        88,
                        32,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 180,
                        panel_y + panel_height - 37,
                        "CANCEL",
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 96,
                        panel_y + panel_height - 48,
                        80,
                        32,
                        0x00303b2b,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 88,
                        panel_y + panel_height - 37,
                        "INVITE",
                        0x00a8e66b,
                        1,
                    );
                }
                if let Some(user_id) = &self.shared_instance_pending_removal {
                    let panel_width = 480.min(w.saturating_sub(32));
                    let panel_height = 230.min(h.saturating_sub(32));
                    let panel_x = (w.saturating_sub(panel_width)) / 2;
                    let panel_y = (h.saturating_sub(panel_height)) / 2;
                    let recipient_count = self
                        .shared_instance_users
                        .iter()
                        .filter(|user| user.join_type != theseus::instance::SharedInstanceJoinType::Owner)
                        .count();
                    rect(&mut buffer, w, h, 0, 0, w, h, 0x00101418);
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x,
                        panel_y,
                        panel_width,
                        panel_height,
                        0x001c2026,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + 24,
                        "REVOKE PLAYER ACCESS?",
                        0x00f1f3f5,
                        2,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + 68,
                        user_id,
                        0x00bdc2ca,
                        1,
                        panel_width.saturating_sub(32),
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + 96,
                        "Their installed copy will remain on their device.",
                        0x007b838d,
                        1,
                    );
                    if recipient_count <= 1 {
                        text(
                            &mut buffer,
                            w,
                            h,
                            panel_x + 16,
                            panel_y + 120,
                            "Removing the last recipient may unlink sharing.",
                            0x00e68a6b,
                            1,
                        );
                    }
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 204,
                        panel_y + panel_height - 52,
                        88,
                        32,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 192,
                        panel_y + panel_height - 41,
                        "CANCEL",
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 108,
                        panel_y + panel_height - 52,
                        92,
                        32,
                        0x003b302b,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 100,
                        panel_y + panel_height - 41,
                        "REVOKE",
                        0x00e68a6b,
                        1,
                    );
                }
                if self.shared_instance_unlink_confirmation {
                    let panel_width = 480.min(w.saturating_sub(32));
                    let panel_height = 220.min(h.saturating_sub(32));
                    let panel_x = (w.saturating_sub(panel_width)) / 2;
                    let panel_y = (h.saturating_sub(panel_height)) / 2;
                    let pack_name = self
                        .shared_instance_metadata
                        .as_ref()
                        .and_then(|metadata| match &metadata.link {
                            theseus::data::InstanceLink::ImportedModpack {
                                name,
                                filename,
                                ..
                            } => name.as_deref().or(filename.as_deref()),
                            _ => None,
                        })
                        .unwrap_or("Imported modpack");
                    rect(&mut buffer, w, h, 0, 0, w, h, 0x00101418);
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x,
                        panel_y,
                        panel_width,
                        panel_height,
                        0x001c2026,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + 24,
                        "SHARING REQUIRES UNLINKING",
                        0x00f1f3f5,
                        2,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + 70,
                        pack_name,
                        0x00bdc2ca,
                        1,
                        panel_width.saturating_sub(32),
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + 16,
                        panel_y + 96,
                        "Remove the imported pack association to continue.",
                        0x007b838d,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 204,
                        panel_y + panel_height - 52,
                        88,
                        32,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 192,
                        panel_y + panel_height - 41,
                        "CANCEL",
                        0x00bdc2ca,
                        1,
                    );
                    rect(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 108,
                        panel_y + panel_height - 52,
                        92,
                        32,
                        0x00303b2b,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        panel_x + panel_width - 104,
                        panel_y + panel_height - 41,
                        "UNLINK",
                        0x00a8e66b,
                        1,
                    );
                }
            } else if let Some((path, preview)) = self.file_preview.as_ref() {
                rect(&mut buffer, w, h, 292, 146, 112, 28, 0x0022262c);
                text(
                    &mut buffer,
                    w,
                    h,
                    302,
                    156,
                    "BACK TO FILES",
                    0x00a8e66b,
                    1,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    416,
                    156,
                    path,
                    0x00bdc2ca,
                    1,
                    library_width.saturating_sub(440),
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    292,
                    182,
                    w.saturating_sub(324),
                    h.saturating_sub(206),
                    0x00141619,
                );
                let lines: Vec<&str> = preview.lines().collect();
                let max_lines = (h.saturating_sub(220) / 12).max(1);
                let scroll =
                    self.file_offset.min(lines.len().saturating_sub(max_lines));
                let end = lines.len().saturating_sub(scroll);
                let start = end.saturating_sub(max_lines + 1);
                let preview_width = library_width.saturating_sub(346);
                let scroll_remainder =
                    self.file_preview_scroll_pixels.rem_euclid(12.0) as usize;
                render::set_clip_rect(Some((
                    292,
                    182,
                    library_width.saturating_sub(324),
                    h.saturating_sub(206),
                )));
                for (index, line) in lines[start..end].iter().enumerate() {
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        302,
                        190 + index * 12 + scroll_remainder,
                        line,
                        0x00bdc2ca,
                        1,
                        preview_width,
                    );
                }
                render::set_clip_rect(None);
            } else {
                let visible_path: String = if self.file_path.is_empty() {
                    "/".into()
                } else {
                    format!("/{}", self.file_path)
                };
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    292,
                    156,
                    &visible_path,
                    0x00bdc2ca,
                    1,
                    if self.file_path.is_empty() {
                        library_width.saturating_sub(324)
                    } else {
                        library_width.saturating_sub(408)
                    },
                );
                if !self.file_path.is_empty() {
                    rect(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(104),
                        146,
                        80,
                        28,
                        0x0022262c,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        w.saturating_sub(88),
                        156,
                        "UP",
                        0x00a8e66b,
                        1,
                    );
                }
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    292,
                    178,
                    &self.message,
                    0x007b838d,
                    1,
                    library_width.saturating_sub(324),
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    292,
                    194,
                    w.saturating_sub(324),
                    1,
                    0x00363b42,
                );
                let visible_rows = (h.saturating_sub(208) / 42).min(14).max(1);
                if self.file_offset != self.file_list_scroll_synced {
                    self.file_list_scroll_pixels =
                        (self.file_offset * 42) as f64;
                    self.file_list_scroll_target_pixels =
                        self.file_list_scroll_pixels;
                    self.file_list_scroll_synced = self.file_offset;
                }
                let max_scroll = self
                    .file_items
                    .len()
                    .saturating_sub(visible_rows)
                    .saturating_mul(42) as f64;
                self.file_list_scroll_pixels =
                    self.file_list_scroll_pixels.min(max_scroll);
                self.file_list_scroll_target_pixels =
                    self.file_list_scroll_target_pixels.min(max_scroll);
                let row_offset =
                    (self.file_list_scroll_pixels / 42.0).floor() as usize;
                let scroll_remainder =
                    (self.file_list_scroll_pixels as usize) % 42;
                self.file_offset = row_offset;
                self.file_list_scroll_synced = row_offset;
                for (index, (name, kind, _, size, read_only)) in self
                    .file_items
                    .iter()
                    .skip(row_offset)
                    .take(visible_rows + usize::from(scroll_remainder > 0))
                    .enumerate()
                {
                    let y = 202 + index * 42 - scroll_remainder;
                    rect(
                        &mut buffer,
                        w,
                        h,
                        292,
                        y,
                        w.saturating_sub(324),
                        36,
                        0x0022262c,
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        304,
                        y + 6,
                        name,
                        0x00f1f3f5,
                        1,
                        library_width.saturating_sub(348),
                    );
                    let detail = if kind == "directory" {
                        "DIRECTORY".to_string()
                    } else if kind == "symlink" {
                        "SYMLINK BLOCKED".to_string()
                    } else {
                        format!(
                            "{}{}",
                            size.map(|n| format!("{} BYTES", n))
                                .unwrap_or_else(|| "FILE".into()),
                            if *read_only { "  READ ONLY" } else { "" }
                        )
                    };
                    text(
                        &mut buffer,
                        w,
                        h,
                        304,
                        y + 21,
                        &detail,
                        0x007b838d,
                        1,
                    );
                }
            }
            if self
                .selected_instance
                .as_ref()
                .is_some_and(|selected| selected.0 != instance_id)
            {
                self.selected_instance = None;
            }
        }
        if self.settings_open {
            settings_panel::draw_settings_panel(
                &mut buffer,
                w,
                library_width,
                h,
                self.settings.as_ref(),
                self.maximum_memory_mb,
                self.global_screenshot_sync,
                self.jump_in_visible_items,
                self.native_color_theme,
                self.theme_sync_across_devices,
                self.theme_sync_saving,
                palette_for_theme(
                    theseus::data::Theme::System,
                    window.theme(),
                    self.preferred_dark_theme,
                ),
                self.settings_jump_in_focused,
                &self.default_window_width_input,
                &self.default_window_height_input,
                &self.default_java_arguments_input,
                &self.default_environment_variables_input,
                &self.default_pre_launch_hook_input,
                &self.default_wrapper_hook_input,
                &self.default_post_exit_hook_input,
                self.default_launch_settings_saving,
                self.input_focus,
                self.settings_section,
                &self.message,
                self.settings_scroll_pixels,
            );
        }
        if self.java_open {
            java_panel::draw_java_panel(
                &mut buffer,
                w,
                h,
                &self.java_versions,
                self.java_busy,
                &self.message,
            );
        }
        if self.accounts_open {
            let account_count = if self.account_tab == 1 {
                self.content_accounts.len()
            } else {
                self.minecraft_accounts.len()
            };
            let visible_accounts = (h.saturating_sub(258) / 58).min(12);
            let max_account_scroll = account_count
                .saturating_sub(visible_accounts)
                .saturating_mul(58) as f64;
            self.account_scroll_target_pixels = self
                .account_scroll_target_pixels
                .clamp(0.0, max_account_scroll);
            self.account_scroll_pixels =
                self.account_scroll_pixels.clamp(0.0, max_account_scroll);
            self.account_offset =
                (self.account_scroll_pixels / 58.0).floor() as usize;
            accounts_panel::draw_accounts_panel(
                &mut buffer,
                w,
                h,
                &self.message,
                self.account_tab,
                self.modrinth_login_task.is_some(),
                &self.content_accounts,
                self.login_flow.is_some(),
                &self.login_code_input,
                &self.minecraft_accounts,
                self.account_offset,
                self.account_scroll_pixels,
                self.default_account,
            );
        }
        render::set_page_horizontal_layout(None);
        render::set_page_vertical_layout(None, None);
        let cursor = screen_cursor;
        if self.browse_gallery_viewer_open
            && let (Some(project), Some(details)) =
                (self.browse_selected.as_ref(), self.browse_details.as_ref())
        {
            draw_browse_gallery_viewer(
                &mut buffer,
                w,
                h,
                project,
                details,
                &self.browse_icons,
                self.browse_gallery_index,
                self.browse_gallery_zoom,
            );
        }
        if browse_filter_sidebar_visible {
            browse_panel::draw_filter_sidebar(
                &mut buffer,
                w,
                h,
                screen_cursor,
                self.browse_filter_group,
                browse_filter_group_count,
                self.browse_show_all_versions,
                &browse_filter_menu_options,
                self.browse_filter_scroll_pixels,
                [
                    &self.browse_selected_game_versions,
                    &self.browse_selected_loaders,
                    &self.browse_selected_categories,
                    &self.browse_selected_licenses,
                    &self.browse_selected_environments,
                ],
                browse_filter_waiting_for_tags || !self.ready,
                self.browse_filter_options_error
                    .as_deref()
                    .filter(|_| browse_filter_uses_tags),
            );
        } else if show_right_sidebar && !self.browse_gallery_viewer_open {
            sidebar_panel::draw_right_sidebar(
                &mut buffer,
                w,
                h,
                cursor,
                &self.minecraft_accounts,
                self.default_account,
                self.onboarding_checklist.as_ref(),
                &self.sidebar_friend_query,
                sidebar_collapsed_groups(self.settings.as_ref()),
                &self.sidebar_friends,
                self.sidebar_friends_error.as_deref(),
                self.sidebar_friends_loaded,
                &self.news_articles,
                &self.browse_icons,
                self.news_error.as_deref(),
                self.news_loaded,
                self.sidebar_scroll_pixels,
                self.browse_selected.as_ref(),
                self.browse_details.as_ref(),
            );
        }
        if self.delete_confirmation_open || self.bulk_delete_confirmation_open {
            let dialog_width = 460usize;
            let dialog_height = 220usize;
            let x = w.saturating_sub(dialog_width) / 2;
            let y = h.saturating_sub(dialog_height) / 2;
            let deletion_title = if self.bulk_delete_confirmation_open {
                format!(
                    "REMOVE {} INSTANCES?",
                    self.selected_library_instances.len()
                )
            } else {
                "REMOVE INSTANCE?".to_owned()
            };
            let confirmation_prompt = if self.bulk_delete_confirmation_open {
                format!(
                    "TYPE DELETE {} TO CONFIRM.",
                    self.selected_library_instances.len()
                )
            } else {
                "TYPE THE INSTANCE NAME TO CONFIRM.".to_owned()
            };
            rect(&mut buffer, w, h, 0, 0, w, h, 0x00101215);
            rect(
                &mut buffer,
                w,
                h,
                x,
                y,
                dialog_width,
                dialog_height,
                0x0022262c,
            );
            text(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 20,
                &deletion_title,
                0x00f1f3f5,
                2,
            );
            text(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 56,
                "INSTANCE FILES WILL BE PERMANENTLY DELETED.",
                0x00e68a6b,
                1,
            );
            text(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 76,
                &confirmation_prompt,
                0x00bdc2ca,
                1,
            );
            if !self.bulk_delete_confirmation_open {
                if let Some((_, name)) = self.selected_instance.as_ref() {
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 20,
                        y + 92,
                        &name
                            .chars()
                            .skip(name.chars().count().saturating_sub(68))
                            .collect::<String>(),
                        0x00f1f3f5,
                        1,
                    );
                }
            }
            rect(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 102,
                dialog_width - 40,
                34,
                0x00141619,
            );
            let input_char_count =
                self.delete_confirmation_input.chars().count();
            text(
                &mut buffer,
                w,
                h,
                x + 30,
                y + 115,
                &self
                    .delete_confirmation_input
                    .chars()
                    .skip(input_char_count.saturating_sub(60))
                    .collect::<String>(),
                0x00f1f3f5,
                1,
            );
            rect(&mut buffer, w, h, x + 260, y + 164, 80, 32, 0x002e333a);
            text(
                &mut buffer,
                w,
                h,
                x + 274,
                y + 176,
                if self.instance_deletion_pending {
                    "WAIT…"
                } else {
                    "CANCEL"
                },
                0x00bdc2ca,
                1,
            );
            let can_delete = if self.bulk_delete_confirmation_open {
                !self.selected_library_instances.is_empty()
                    && self
                        .delete_confirmation_input
                        .trim()
                        .eq_ignore_ascii_case(&format!(
                            "DELETE {}",
                            self.selected_library_instances.len()
                        ))
            } else {
                self.selected_instance.as_ref().is_some_and(|(_, name)| {
                    delete_confirmation_matches(
                        &self.delete_confirmation_input,
                        name,
                    )
                })
            } && !self.instance_deletion_pending;
            rect(
                &mut buffer,
                w,
                h,
                x + 350,
                y + 164,
                90,
                32,
                if can_delete { 0x003b302b } else { 0x002e333a },
            );
            text(
                &mut buffer,
                w,
                h,
                x + 366,
                y + 176,
                if self.instance_deletion_pending {
                    "WAIT…"
                } else {
                    "DELETE"
                },
                if can_delete { 0x00e68a6b } else { 0x007b838d },
                1,
            );
        }
        if let Some(choice) = &self.browse_install_choice {
            let project = &choice.project;
            flush_native_text(&mut buffer, w, h);
            clear_text_commands();
            for pixel in buffer.iter_mut() {
                *pixel = blend_color(*pixel, 0, 176);
            }
            let layout =
                InstallInstancePickerLayout::new(w, h, self.instances.len());
            let panel = layout.panel;
            let close = layout.close;
            let list = layout.list;
            rounded_rect(
                &mut buffer,
                w,
                h,
                panel.x,
                panel.y,
                panel.width,
                panel.height,
                16,
                0x0022262c,
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 17,
                "Choose an instance",
                0x00f1f3f5,
                2,
                panel.width.saturating_sub(80),
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 48,
                &project.title,
                0x00bdc2ca,
                1,
                panel.width.saturating_sub(40),
            );
            rounded_rect(
                &mut buffer,
                w,
                h,
                close.x,
                close.y,
                close.width,
                close.height,
                8,
                if close.contains(cursor.0, cursor.1) {
                    0x00343a42
                } else {
                    0x001b1d22
                },
            );
            text(
                &mut buffer,
                w,
                h,
                close.x + 11,
                close.y + 7,
                "×",
                0x00f1f3f5,
                2,
            );
            let max_scroll = layout.max_scroll(self.instances.len());
            let scroll =
                self.browse_install_scroll.clamp(0.0, max_scroll) as usize;
            let first = scroll / INSTALL_INSTANCE_ROW_HEIGHT;
            let remainder = scroll % INSTALL_INSTANCE_ROW_HEIGHT;
            let count =
                (list.height + remainder).div_ceil(INSTALL_INSTANCE_ROW_HEIGHT);
            let hovered = layout.row_at(cursor.0, cursor.1, scroll as f64);
            render::set_clip_rect(Some((
                list.x,
                list.y,
                list.width,
                list.height,
            )));
            if self.instances.is_empty() {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    list.x + 12,
                    list.y + 8,
                    "No instances available",
                    0x00bdc2ca,
                    1,
                    list.width.saturating_sub(24),
                );
            }
            for (index, (id, name, _)) in
                self.instances.iter().skip(first).take(count).enumerate()
            {
                let row_y =
                    list.y + index * INSTALL_INSTANCE_ROW_HEIGHT - remainder;
                let card = self.instance_cards.get(id);
                let already_installed =
                    matches!(choice.presence(id), Some(Ok(true)));
                let installing = self
                    .browse_installing_projects
                    .get(id)
                    .is_some_and(|projects| projects.contains(&project.id));
                let enabled = can_pick_install_instance(card)
                    && !installing
                    && choice.versions.is_some()
                    && choice.presence(id).is_some()
                    && (!already_installed || choice.version_id.is_some());
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    list.x,
                    row_y,
                    list.width.saturating_sub(8),
                    INSTALL_INSTANCE_ROW_HEIGHT - 6,
                    8,
                    if enabled && hovered == Some(first + index) {
                        0x00343a42
                    } else {
                        0x001b1d22
                    },
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    list.x + 12,
                    row_y + 5,
                    name,
                    if enabled { 0x00f1f3f5 } else { 0x0096a2b0 },
                    1,
                    list.width.saturating_sub(38),
                );
                if let Some(card) = card {
                    let availability = if installing {
                        " · Installing this project"
                    } else if already_installed {
                        " · Already installed"
                    } else if matches!(choice.presence(id), Some(Err(_))) {
                        " · Installed status unavailable"
                    } else if choice.presence(id).is_none() {
                        " · Checking installed status…"
                    } else if card.content_locked {
                        " · Content locked"
                    } else if card.install_state != "installed" {
                        " · Installation incomplete"
                    } else {
                        choice.compatibility_label(card)
                    };
                    let metadata = format!(
                        "{} {}{availability}",
                        card.loader, card.game_version
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        list.x + 12,
                        row_y + 25,
                        &metadata,
                        0x00bdc2ca,
                        1,
                        list.width.saturating_sub(38),
                    );
                }
            }
            if max_scroll > 0.0 {
                let thumb_height = (list.height * list.height
                    / (list.height + max_scroll as usize))
                    .clamp(1, list.height.max(1));
                let thumb_y = list.y
                    + ((list.height.saturating_sub(thumb_height) as f64)
                        * scroll as f64
                        / max_scroll) as usize;
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    list.x + list.width.saturating_sub(4),
                    thumb_y,
                    4,
                    thumb_height,
                    2,
                    0x00535d6b,
                );
            }
            render::set_clip_rect(None);
        }
        if self.launcher_import_open {
            flush_native_text(&mut buffer, w, h);
            render::clear_text_commands();
            for pixel in buffer.iter_mut() {
                *pixel = blend_color(*pixel, 0, 176);
            }
            let dialog_width = 520usize.min(w.saturating_sub(24));
            let x = w.saturating_sub(dialog_width) / 2;
            let visible = 7usize;
            let rows = self
                .launcher_import_source
                .as_ref()
                .map_or(import_launcher_options().len() + 1, |source| {
                    source.folders.len()
                });
            let dialog_height = 112 + rows.min(visible) * 44;
            let y = h.saturating_sub(dialog_height) / 2;
            rect(
                &mut buffer,
                w,
                h,
                x,
                y,
                dialog_width,
                dialog_height,
                0x0022262c,
            );
            text(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 18,
                if self.launcher_import_source.is_some() {
                    "IMPORT FROM LAUNCHER"
                } else {
                    "IMPORT CONTENT"
                },
                0x00f1f3f5,
                2,
            );
            if let Some(source) = &self.launcher_import_source {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 44,
                    &source.path.display().to_string(),
                    0x00bdc2ca,
                    1,
                    dialog_width.saturating_sub(40),
                );
                render::set_clip_rect(Some((
                    x + 16,
                    y + 68,
                    dialog_width.saturating_sub(32),
                    visible * 44,
                )));
                let scroll_remainder =
                    (self.launcher_import_scroll_pixels % 44.0) as usize;
                for (index, folder) in source
                    .folders
                    .iter()
                    .skip(self.launcher_import_offset)
                    .take(visible)
                    .enumerate()
                {
                    let row_y = y + 68 + index * 44 - scroll_remainder;
                    rect(
                        &mut buffer,
                        w,
                        h,
                        x + 16,
                        row_y,
                        dialog_width.saturating_sub(32),
                        38,
                        if self.launcher_import_offset + index
                            == self.launcher_import_focus
                        {
                            0x003b414a
                        } else {
                            0x001b1d22
                        },
                    );
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        x + 26,
                        row_y + 12,
                        folder,
                        0x00f1f3f5,
                        1,
                        dialog_width.saturating_sub(48),
                    );
                }
                render::set_clip_rect(None);
                if source.folders.is_empty() {
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 20,
                        y + 76,
                        "NO IMPORTABLE INSTANCES FOUND.",
                        0x00bdc2ca,
                        1,
                    );
                }
            } else {
                for (index, label) in std::iter::once("Modrinth .mrpack file")
                    .chain(
                        import_launcher_options().iter().map(|(name, _)| *name),
                    )
                    .enumerate()
                {
                    let row_y = y + 56 + index * 44;
                    rect(
                        &mut buffer,
                        w,
                        h,
                        x + 16,
                        row_y,
                        dialog_width.saturating_sub(32),
                        38,
                        if index == self.launcher_import_focus {
                            0x003b414a
                        } else {
                            0x001b1d22
                        },
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 26,
                        row_y + 12,
                        label,
                        0x00f1f3f5,
                        1,
                    );
                }
            }
        }
        if self.create_group_open
            || self.create_instance_open
            || self.migration_prompt_open
            || self.add_server_open
            || self.server_remove_confirmation.is_some()
            || self.world_editor_open
            || self.world_delete_confirmation.is_some()
            || self.log_delete_confirmation
        {
            flush_native_text(&mut buffer, w, h);
            render::clear_text_commands();
            for pixel in buffer.iter_mut() {
                *pixel = blend_color(*pixel, 0, 176);
            }
            if self.log_delete_confirmation {
                let dialog_width = 460usize.min(w.saturating_sub(24));
                let dialog_height = 190usize.min(h.saturating_sub(24));
                let x = w.saturating_sub(dialog_width) / 2;
                let y = h.saturating_sub(dialog_height) / 2;
                rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    dialog_width,
                    dialog_height,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 20,
                    "DELETE LOG FILE?",
                    0x00f1f3f5,
                    2,
                );
                if let Some((filename, _)) = self.selected_log.as_ref() {
                    text_clipped(
                        &mut buffer,
                        w,
                        h,
                        x + 20,
                        y + 58,
                        filename,
                        0x00bdc2ca,
                        1,
                        dialog_width.saturating_sub(40),
                    );
                }
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 86,
                    "Deleting this file cannot be undone.",
                    0x00e6a56b,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 204,
                    y + dialog_height - 48,
                    84,
                    32,
                    0x003b302b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 186,
                    y + dialog_height - 37,
                    "DELETE",
                    0x00e68a6b,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 108,
                    y + dialog_height - 48,
                    88,
                    32,
                    0x00343a42,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 94,
                    y + dialog_height - 37,
                    "CANCEL",
                    0x00f1f3f5,
                    1,
                );
            } else if let Some((_, name)) = &self.world_delete_confirmation {
                let dialog_width = 460usize.min(w.saturating_sub(24));
                let dialog_height = 190usize.min(h.saturating_sub(24));
                let x = w.saturating_sub(dialog_width) / 2;
                let y = h.saturating_sub(dialog_height) / 2;
                rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    dialog_width,
                    dialog_height,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 20,
                    "DELETE WORLD?",
                    0x00f1f3f5,
                    2,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 62,
                    name,
                    0x00bdc2ca,
                    1,
                    dialog_width.saturating_sub(40),
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 88,
                    "This permanently deletes the save from this instance.",
                    0x00e6a56b,
                    1,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 108,
                    "This cannot be undone.",
                    0x00e6a56b,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 204,
                    y + dialog_height - 48,
                    84,
                    32,
                    0x003b302b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 186,
                    y + dialog_height - 37,
                    "DELETE",
                    0x00e68a6b,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 108,
                    y + dialog_height - 48,
                    88,
                    32,
                    0x00343a42,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 94,
                    y + dialog_height - 37,
                    "CANCEL",
                    0x00f1f3f5,
                    1,
                );
            } else if self.world_editor_open {
                let dialog_width = 500usize.min(w.saturating_sub(24));
                let dialog_height = 270usize.min(h.saturating_sub(24));
                let x = w.saturating_sub(dialog_width) / 2;
                let y = h.saturating_sub(dialog_height) / 2;
                rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    dialog_width,
                    dialog_height,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 22,
                    "Edit world",
                    0x00f1f3f5,
                    2,
                );
                text(&mut buffer, w, h, x + 24, y + 58, "Name", 0x00bdc2ca, 1);
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 76,
                    dialog_width - 48,
                    38,
                    0x001b1d22,
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 88,
                    if self.world_edit_name.is_empty() {
                        "Minecraft World"
                    } else {
                        &self.world_edit_name
                    },
                    if self.world_edit_name.is_empty() {
                        0x007b838d
                    } else {
                        0x00f1f3f5
                    },
                    1,
                    dialog_width.saturating_sub(72),
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 132,
                    dialog_width - 48,
                    36,
                    if self.world_edit_reset_icon {
                        0x00303b2b
                    } else {
                        0x001b1d22
                    },
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 143,
                    if self.world_edit_reset_icon {
                        "✓  Reset world icon"
                    } else {
                        "Reset world icon"
                    },
                    if self.world_edit_reset_icon {
                        0x00a8e66b
                    } else {
                        0x00bdc2ca
                    },
                    1,
                    dialog_width.saturating_sub(72),
                );
                rect(&mut buffer, w, h, x + 24, y + 212, 84, 34, 0x003b302b);
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 38,
                    y + 223,
                    "DELETE",
                    0x00e68a6b,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 204,
                    y + 212,
                    84,
                    34,
                    0x002e333a,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 187,
                    y + 223,
                    "Cancel",
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 112,
                    y + 212,
                    88,
                    34,
                    if self.add_server_busy {
                        0x0040474e
                    } else {
                        0x00a8e66b
                    },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 100,
                    y + 223,
                    if self.add_server_busy {
                        "Saving…"
                    } else {
                        "Save"
                    },
                    if self.add_server_busy {
                        0x00aeb4bc
                    } else {
                        0x001b1d22
                    },
                    1,
                );
            } else if let Some((_, name, is_synced)) =
                &self.server_remove_confirmation
            {
                let dialog_width = 460usize.min(w.saturating_sub(24));
                let dialog_height = 190usize.min(h.saturating_sub(24));
                let x = w.saturating_sub(dialog_width) / 2;
                let y = h.saturating_sub(dialog_height) / 2;
                rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    dialog_width,
                    dialog_height,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 20,
                    "REMOVE SERVER?",
                    0x00f1f3f5,
                    2,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 62,
                    name,
                    0x00bdc2ca,
                    1,
                    dialog_width.saturating_sub(40),
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 20,
                    y + 90,
                    if *is_synced {
                        "This may remove the server from synced instances."
                    } else {
                        "This removes the server entry from this instance."
                    },
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 204,
                    y + dialog_height - 48,
                    84,
                    32,
                    0x003b302b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 186,
                    y + dialog_height - 37,
                    "REMOVE",
                    0x00e68a6b,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 108,
                    y + dialog_height - 48,
                    88,
                    32,
                    0x00343a42,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 94,
                    y + dialog_height - 37,
                    "CANCEL",
                    0x00f1f3f5,
                    1,
                );
            } else if self.migration_prompt_open {
                let dialog_width = 520usize.min(w.saturating_sub(24));
                let dialog_height = 258usize.min(h.saturating_sub(24));
                let x = w.saturating_sub(dialog_width) / 2;
                let y = h.saturating_sub(dialog_height) / 2;
                let enough_space =
                    self.migration_plan.as_ref().is_some_and(|plan| {
                        plan.available_bytes
                            >= plan
                                .copy_bytes
                                .saturating_add(MIGRATION_SPACE_RESERVE)
                    });
                rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    dialog_width,
                    dialog_height,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 22,
                    "Existing Modrinth library",
                    0x00f1f3f5,
                    2,
                );
                if let Some(plan) = &self.migration_plan {
                    let gib = plan.copy_bytes as f64 / 1_073_741_824.0;
                    let free_gib =
                        plan.available_bytes as f64 / 1_073_741_824.0;
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 24,
                        y + 64,
                        &format!(
                            "Found {} instances ({gib:.1} GiB to copy).",
                            plan.instance_count
                        ),
                        0x00bdc2ca,
                        1,
                    );
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 24,
                        y + 88,
                        &format!(
                            "Available space: {free_gib:.1} GiB. The original files stay in place."
                        ),
                        0x00bdc2ca,
                        1,
                    );
                }
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 122,
                    if self.migration_busy {
                        if self.migration_saving_fresh_choice {
                            "Saving your fresh-start choice…"
                        } else {
                            "Copying and verifying the library…"
                        }
                    } else if self.message.starts_with("Library copy failed:") {
                        &self.message
                    } else if !enough_space {
                        "Copying the library needs more free space."
                    } else {
                        "Copy this library into Northstar?"
                    },
                    if enough_space { 0x00f1f3f5 } else { 0x00e6a56b },
                    1,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 146,
                    if self.migration_settings_entry {
                        "The source library remains untouched during the copy."
                    } else {
                        "Start fresh creates a separate library; the original stays untouched."
                    },
                    0x00bdc2ca,
                    1,
                );
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 230,
                    y + 190,
                    104,
                    36,
                    8,
                    0x00343a42,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 214,
                    y + 202,
                    if self.migration_settings_entry {
                        "Cancel"
                    } else {
                        "Start fresh"
                    },
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 118,
                    y + 190,
                    110,
                    36,
                    if enough_space { 0x00a8e66b } else { 0x0040474e },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 105,
                    y + 202,
                    if enough_space {
                        "Copy library"
                    } else {
                        "Need space"
                    },
                    if enough_space { 0x001b1d22 } else { 0x00aeb4bc },
                    1,
                );
            } else if self.create_group_open {
                let dialog_width = 460usize.min(w.saturating_sub(24));
                let dialog_height = 224usize.min(h.saturating_sub(24));
                let x = w.saturating_sub(dialog_width) / 2;
                let y = h.saturating_sub(dialog_height) / 2;
                rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    dialog_width,
                    dialog_height,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 24,
                    "New group",
                    0x00f1f3f5,
                    2,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 62,
                    "Give this group a name.",
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 96,
                    dialog_width.saturating_sub(48),
                    40,
                    0x001b1d22,
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 109,
                    if self.group_input.is_empty() {
                        "Group name"
                    } else {
                        &self.group_input
                    },
                    if self.group_input.is_empty() {
                        0x007b838d
                    } else {
                        0x00f1f3f5
                    },
                    1,
                    dialog_width.saturating_sub(72),
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 204,
                    y + 164,
                    84,
                    34,
                    0x002e333a,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 187,
                    y + 175,
                    "Cancel",
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 112,
                    y + 164,
                    88,
                    34,
                    0x00a8e66b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 100,
                    y + 175,
                    "Create",
                    0x001b1d22,
                    1,
                );
            } else if self.add_server_open {
                let dialog_width = 500usize.min(w.saturating_sub(24));
                let dialog_height = 330usize.min(h.saturating_sub(24));
                let x = w.saturating_sub(dialog_width) / 2;
                let y = h.saturating_sub(dialog_height) / 2;
                rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    dialog_width,
                    dialog_height,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 22,
                    if self.server_edit_index.is_some() {
                        "Edit server"
                    } else {
                        "Add a server"
                    },
                    0x00f1f3f5,
                    2,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 58,
                    "Name (optional)",
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 76,
                    dialog_width - 48,
                    38,
                    0x001b1d22,
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 88,
                    if self.server_name_input.is_empty() {
                        "Minecraft Server"
                    } else {
                        &self.server_name_input
                    },
                    if self.server_name_input.is_empty() {
                        0x007b838d
                    } else {
                        0x00f1f3f5
                    },
                    1,
                    dialog_width.saturating_sub(72),
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 124,
                    "Server address",
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 142,
                    dialog_width - 48,
                    38,
                    0x001b1d22,
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 154,
                    if self.server_address_input.is_empty() {
                        "example.org"
                    } else {
                        &self.server_address_input
                    },
                    if self.server_address_input.is_empty() {
                        0x007b838d
                    } else {
                        0x00f1f3f5
                    },
                    1,
                    dialog_width.saturating_sub(72),
                );
                let pack_label = match self.server_pack_status {
                    worlds::ServerPackStatus::Enabled => {
                        "Resource packs: Enabled"
                    }
                    worlds::ServerPackStatus::Prompt => "Resource packs: Ask",
                    worlds::ServerPackStatus::Disabled => {
                        "Resource packs: Disabled"
                    }
                };
                rect(&mut buffer, w, h, x + 24, y + 196, 210, 36, 0x001b1d22);
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 207,
                    pack_label,
                    0x00f1f3f5,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 270,
                    84,
                    34,
                    if self.server_edit_index.is_some()
                        && self.server_edit_source
                            == Some(
                                theseus::instance::ServerSource::LocalDesynced,
                            )
                    {
                        0x003b302b
                    } else {
                        0x0022262c
                    },
                );
                if self.server_edit_index.is_some()
                    && self.server_edit_source
                        == Some(theseus::instance::ServerSource::LocalDesynced)
                {
                    text(
                        &mut buffer,
                        w,
                        h,
                        x + 38,
                        y + 281,
                        "REMOVE",
                        0x00e68a6b,
                        1,
                    );
                }
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 204,
                    y + 270,
                    84,
                    34,
                    0x002e333a,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 187,
                    y + 281,
                    "Cancel",
                    0x00bdc2ca,
                    1,
                );
                let can_add = !self.server_address_input.trim().is_empty()
                    && !self.add_server_busy;
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 112,
                    y + 270,
                    88,
                    34,
                    if can_add { 0x00a8e66b } else { 0x0040474e },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 100,
                    y + 281,
                    if self.add_server_busy {
                        if self.server_edit_index.is_some() {
                            "Saving…"
                        } else {
                            "Adding…"
                        }
                    } else if self.server_edit_index.is_some() {
                        "Save"
                    } else {
                        "Add"
                    },
                    if can_add { 0x001b1d22 } else { 0x00aeb4bc },
                    1,
                );
            } else {
                let dialog_width = 520usize.min(w.saturating_sub(24));
                let dialog_height = 320usize.min(h.saturating_sub(24));
                let x = w.saturating_sub(dialog_width) / 2;
                let y = h.saturating_sub(dialog_height) / 2;
                rect(
                    &mut buffer,
                    w,
                    h,
                    x,
                    y,
                    dialog_width,
                    dialog_height,
                    0x0022262c,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 22,
                    "New instance",
                    0x00f1f3f5,
                    2,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 58,
                    "Choose a name, Minecraft version and mod loader.",
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 92,
                    dialog_width - 48,
                    38,
                    0x001b1d22,
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 104,
                    if self.name_input.is_empty() {
                        "Instance name"
                    } else {
                        &self.name_input
                    },
                    if self.name_input.is_empty() {
                        0x007b838d
                    } else {
                        0x00f1f3f5
                    },
                    1,
                    dialog_width.saturating_sub(72),
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + 24,
                    y + 140,
                    dialog_width - 48,
                    38,
                    0x001b1d22,
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 152,
                    if self.version_input.is_empty() {
                        "Minecraft version (latest)"
                    } else {
                        &self.version_input
                    },
                    if self.version_input.is_empty() {
                        0x007b838d
                    } else {
                        0x00f1f3f5
                    },
                    1,
                    dialog_width.saturating_sub(72),
                );
                let loader_label =
                    format!("Mod loader: {}", selected_loader.as_str());
                rect(&mut buffer, w, h, x + 24, y + 188, 220, 38, 0x001b1d22);
                text(
                    &mut buffer,
                    w,
                    h,
                    x + 36,
                    y + 200,
                    &loader_label,
                    0x00f1f3f5,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 204,
                    y + 260,
                    84,
                    34,
                    0x002e333a,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 187,
                    y + 271,
                    "Cancel",
                    0x00bdc2ca,
                    1,
                );
                rect(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 112,
                    y + 260,
                    88,
                    34,
                    0x00a8e66b,
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    x + dialog_width - 100,
                    y + 271,
                    "Create",
                    0x001b1d22,
                    1,
                );
            }
        }
        if self.library_filter_open && show_library_filters {
            let mut loaders: Vec<String> = self
                .instance_cards
                .values()
                .map(|card| card.loader.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            loaders.sort_by_key(|loader| loader.to_lowercase());
            let layout = library_filter_popup_layout(
                toolbar.filter.on_screen(library_width, page_scroll_offset),
                library_width,
                h,
                loaders.len(),
            );
            rect(
                &mut buffer,
                w,
                h,
                layout.x,
                layout.y,
                layout.width,
                layout.height,
                0x0022262c,
            );
            text(
                &mut buffer,
                w,
                h,
                layout.x + 12,
                layout.y + 12,
                "MOD LOADER",
                0x00a8e66b,
                1,
            );
            let hidden_selected = self.show_hidden_instances;
            rect(
                &mut buffer,
                w,
                h,
                layout.x + 12,
                layout.hidden_toggle_y + 4,
                16,
                16,
                if hidden_selected {
                    0x00a8e66b
                } else {
                    0x00363b42
                },
            );
            if hidden_selected {
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 15,
                    layout.hidden_toggle_y + 7,
                    "✓",
                    0x001b1d22,
                    1,
                );
            }
            text(
                &mut buffer,
                w,
                h,
                layout.x + 36,
                layout.hidden_toggle_y + 7,
                "Show hidden instances",
                0x00f1f3f5,
                1,
            );
            for (index, loader) in loaders.iter().enumerate() {
                let row_y = layout.loader_rows_y + index * 24;
                let selected = self
                    .library_loader_filters
                    .contains(&loader.to_lowercase());
                rect(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 12,
                    row_y,
                    16,
                    16,
                    if selected { 0x00a8e66b } else { 0x00363b42 },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 36,
                    row_y + 3,
                    loader,
                    0x00f1f3f5,
                    1,
                );
                if selected {
                    text(
                        &mut buffer,
                        w,
                        h,
                        layout.x + 15,
                        row_y + 3,
                        "✓",
                        0x001b1d22,
                        1,
                    );
                }
            }
            text(
                &mut buffer,
                w,
                h,
                layout.x + 12,
                layout.type_header_y,
                "INSTANCE TYPE",
                0x00a8e66b,
                1,
            );
            for (index, instance_type) in
                ["Custom", "Modpack", "Server"].iter().enumerate()
            {
                let row_y = layout.type_rows_y + index * 24;
                let selected = self
                    .library_instance_type_filters
                    .contains(&instance_type.to_lowercase());
                rect(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 12,
                    row_y,
                    16,
                    16,
                    if selected { 0x00a8e66b } else { 0x00363b42 },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 36,
                    row_y + 3,
                    instance_type,
                    0x00f1f3f5,
                    1,
                );
                if selected {
                    text(
                        &mut buffer,
                        w,
                        h,
                        layout.x + 15,
                        row_y + 3,
                        "✓",
                        0x001b1d22,
                        1,
                    );
                }
            }
            let game_versions = &library_game_version_options;
            let visible_game_versions = game_versions
                .iter()
                .skip(self.library_game_version_filter_offset)
                .take(3);
            text(
                &mut buffer,
                w,
                h,
                layout.x + 12,
                layout.game_version_header_y,
                "GAME VERSION",
                0x00a8e66b,
                1,
            );
            render::set_clip_rect(Some((
                layout.x + 8,
                layout.game_version_rows_y,
                layout.width.saturating_sub(16),
                3 * 24,
            )));
            let scroll_remainder = (self
                .library_game_version_filter_scroll_pixels
                % 24.0) as usize;
            for (index, game_version) in visible_game_versions.enumerate() {
                let row_y =
                    layout.game_version_rows_y + index * 24 - scroll_remainder;
                let selected =
                    self.library_game_version_filters.contains(game_version);
                rect(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 12,
                    row_y,
                    16,
                    16,
                    if selected { 0x00a8e66b } else { 0x00363b42 },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 36,
                    row_y + 3,
                    game_version,
                    0x00f1f3f5,
                    1,
                );
                if selected {
                    text(
                        &mut buffer,
                        w,
                        h,
                        layout.x + 15,
                        row_y + 3,
                        "✓",
                        0x001b1d22,
                        1,
                    );
                }
            }
            render::set_clip_rect(None);
            if self.library_game_version_filter_offset > 0 {
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.x + layout.width - 24,
                    layout.game_version_header_y,
                    "↑",
                    0x00bdc2ca,
                    1,
                );
            }
            if self.library_game_version_filter_offset + 3 < game_versions.len()
            {
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.x + layout.width - 24,
                    layout.game_version_rows_y + 2 * 24,
                    "↓",
                    0x00bdc2ca,
                    1,
                );
            }
            text(
                &mut buffer,
                w,
                h,
                layout.x + 12,
                layout.state_header_y,
                "INSTALL STATE",
                0x00a8e66b,
                1,
            );
            for (index, (label, key)) in [
                ("Installed", "installed"),
                ("Installing", "installing"),
                ("Not installed", "not_installed"),
            ]
            .iter()
            .enumerate()
            {
                let row_y = layout.state_rows_y + index * 24;
                let selected =
                    self.library_install_state_filters.contains(*key);
                rect(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 12,
                    row_y,
                    16,
                    16,
                    if selected { 0x00a8e66b } else { 0x00363b42 },
                );
                text(
                    &mut buffer,
                    w,
                    h,
                    layout.x + 36,
                    row_y + 3,
                    label,
                    0x00f1f3f5,
                    1,
                );
                if selected {
                    text(
                        &mut buffer,
                        w,
                        h,
                        layout.x + 15,
                        row_y + 3,
                        "✓",
                        0x001b1d22,
                        1,
                    );
                }
            }
            rect(
                &mut buffer,
                w,
                h,
                layout.x + 12,
                layout.clear_y,
                layout.width - 24,
                26,
                0x00303b2b,
            );
            text(
                &mut buffer,
                w,
                h,
                layout.x + 22,
                layout.clear_y + 8,
                "Clear filters",
                0x00a8e66b,
                1,
            );
        }
        if show_library_filters && self.library_sort_menu_open {
            let bounds = library_dropdown_layout(
                toolbar.sort.on_screen(library_width, page_scroll_offset),
                library_width,
                h,
                LIBRARY_SORT_LABELS.len(),
                208,
            );
            draw_library_option_menu(
                &mut buffer,
                w,
                h,
                bounds,
                &LIBRARY_SORT_LABELS,
                (self.library_sort_mode / 2) as usize,
                cursor,
            );
        }
        if show_library_filters && self.library_group_menu_open {
            let bounds = library_dropdown_layout(
                toolbar.group.on_screen(library_width, page_scroll_offset),
                library_width,
                h,
                LIBRARY_GROUP_LABELS.len(),
                208,
            );
            draw_library_option_menu(
                &mut buffer,
                w,
                h,
                bounds,
                &LIBRARY_GROUP_LABELS,
                self.library_group_mode as usize % LIBRARY_GROUP_LABELS.len(),
                cursor,
            );
        }
        if let Some(skin) = &self.skin_delete_confirmation {
            flush_native_text(&mut buffer, w, h);
            clear_text_commands();
            for pixel in buffer.iter_mut() {
                *pixel = blend_color(*pixel, 0, 176);
            }
            let dialog_width = 460usize.min(w.saturating_sub(24));
            let dialog_height = 190usize.min(h.saturating_sub(24));
            let x = w.saturating_sub(dialog_width) / 2;
            let y = h.saturating_sub(dialog_height) / 2;
            rect(
                &mut buffer,
                w,
                h,
                x,
                y,
                dialog_width,
                dialog_height,
                0x0022262c,
            );
            text(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 20,
                "DELETE SAVED SKIN?",
                0x00f1f3f5,
                2,
            );
            let name = skin.name.as_deref().unwrap_or("Minecraft skin");
            text_clipped(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 62,
                name,
                0x00bdc2ca,
                1,
                dialog_width.saturating_sub(40),
            );
            text(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 86,
                "This removes the saved copy from Northstar.",
                0x00bdc2ca,
                1,
            );
            text(
                &mut buffer,
                w,
                h,
                x + 20,
                y + 106,
                "Your current Minecraft profile skin stays equipped.",
                0x00bdc2ca,
                1,
            );
            rect(
                &mut buffer,
                w,
                h,
                x + dialog_width - 204,
                y + dialog_height - 48,
                84,
                32,
                0x003b302b,
            );
            text(
                &mut buffer,
                w,
                h,
                x + dialog_width - 186,
                y + dialog_height - 37,
                "DELETE",
                0x00e68a6b,
                1,
            );
            rect(
                &mut buffer,
                w,
                h,
                x + dialog_width - 108,
                y + dialog_height - 48,
                88,
                32,
                0x00343a42,
            );
            text(
                &mut buffer,
                w,
                h,
                x + dialog_width - 94,
                y + dialog_height - 37,
                "CANCEL",
                0x00f1f3f5,
                1,
            );
        }
        if let Some(friend) = &self.friend_remove_confirmation {
            friends_panel::draw_friend_remove_confirmation(
                &mut buffer,
                w,
                h,
                friend,
                self.friend_remove_error.as_deref(),
                self.friend_remove_pending.contains(&friend.user_id),
            );
        } else if self.friend_incoming_open {
            friends_panel::draw_incoming_friend_requests(
                &mut buffer,
                w,
                h,
                &self.sidebar_friends,
                &self.browse_icons,
                &self.friend_request_pending,
                &self.friend_remove_pending,
                self.friend_remove_error
                    .as_deref()
                    .or(self.friend_search_error.as_deref()),
                self.friend_incoming_scroll_pixels,
            );
        } else if self.friend_request_open {
            friends_panel::draw_friend_request_modal(
                &mut buffer,
                w,
                h,
                &self.friend_search_query,
                &self.friend_search_results,
                self.friend_search_error.as_deref(),
                self.friend_search_loading,
                &self.friend_request_pending,
            );
        }
        flush_native_text(&mut buffer, w, h);
        if self.selected_instance.is_some()
            && self.instance_tab == 2
            && self.file_preview.is_none()
            && self.file_list_scroll_pixels > 0.0
        {
            clear_text_commands();
            redraw_file_list_header(
                &mut buffer,
                w,
                library_width,
                h,
                &self.file_path,
                &self.message,
            );
            flush_native_text(&mut buffer, w, h);
        }
        if self.selected_instance.is_some()
            && self.instance_tab == 7
            && self.world_scroll_pixels > 0.0
            && !self.world_editor_open
            && !self.add_server_open
            && self.world_delete_confirmation.is_none()
            && self.server_remove_confirmation.is_none()
        {
            clear_text_commands();
            redraw_worlds_header(
                &mut buffer,
                w,
                h,
                library_width,
                &self.message,
            );
            flush_native_text(&mut buffer, w, h);
        }
        if self.selected_instance.is_some()
            && self.instance_tab == 8
            && !self.screenshot_viewer
            && !self.screenshot_group_picker_open
            && self.screenshot_scroll_pixels > 0.0
        {
            clear_text_commands();
            redraw_screenshot_grid_header(
                &mut buffer,
                w,
                h,
                library_width,
                &self.screenshot_search_query,
                self.screenshot_group_mode,
                self.screenshot_sort_mode,
                self.selected_screenshot_ids.len(),
                screenshot_order.len(),
                &self.message,
            );
            flush_native_text(&mut buffer, w, h);
        }
        if self.global_skins && self.minecraft_skin_scroll_pixels > 0.0 {
            rect(
                &mut buffer,
                w,
                h,
                248,
                h.saturating_sub(56),
                w.saturating_sub(248),
                56,
                0x001b1d22,
            );
            clear_text_commands();
            skin_panel::redraw_skin_header(
                &mut buffer,
                w,
                library_width,
                h,
                self.minecraft_skin_variant,
            );
            flush_native_text(&mut buffer, w, h);
        }
        if self.selected_instance.is_some()
            && self.instance_tab == 0
            && self.content_scroll_pixels > 0.0
        {
            clear_text_commands();
            content_panel::redraw_content_header(
                &mut buffer,
                w,
                h,
                library_width,
                &self.filtered_content_cache,
                &self.contents,
                self.content_filter,
                &self.message,
            );
            flush_native_text(&mut buffer, w, h);
        }
        if self.hosting_backup_rotation_confirmation {
            clear_text_commands();
            for pixel in buffer.iter_mut() {
                *pixel = blend_color(*pixel, 0, 176);
            }
            let (panel, cancel, confirm) = hosting_backup_rotation_layout(w, h);
            rounded_rect(
                &mut buffer,
                w,
                h,
                panel.x,
                panel.y,
                panel.width,
                panel.height,
                16,
                0x0022262c,
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 20,
                "Rotate existing backups?",
                0x00f1f3f5,
                2,
                panel.width.saturating_sub(40),
            );
            let message = format!(
                "Changing these settings will remove {} existing backup(s). Continue?",
                self.hosting_backup_rotation_count
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 62,
                &message,
                0x00ffaaa3,
                1,
                panel.width.saturating_sub(40),
            );
            for (bounds, label, color) in [
                (cancel, "Cancel", 0x00343a42),
                (confirm, "Continue", 0x003b302b),
            ] {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    8,
                    color,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    bounds.x + 10,
                    bounds.y + 10,
                    label,
                    0x00f1f3f5,
                    1,
                    bounds.width.saturating_sub(20),
                );
            }
            flush_native_text(&mut buffer, w, h);
        } else if let Some(prompt) = &self.hosting_backup_prompt {
            clear_text_commands();
            for pixel in buffer.iter_mut() {
                *pixel = blend_color(*pixel, 0, 176);
            }
            let (panel, field, cancel, confirm) =
                hosting_backup_prompt_layout(w, h);
            rounded_rect(
                &mut buffer,
                w,
                h,
                panel.x,
                panel.y,
                panel.width,
                panel.height,
                16,
                0x0022262c,
            );
            let (title, description, confirm_label) = match prompt.action {
                HostingBackupPromptAction::Restore => (
                    "Restore backup",
                    "This replaces the current world and server files. A safety backup is queued first.",
                    "Restore",
                ),
                HostingBackupPromptAction::Rename => (
                    "Rename backup",
                    "Choose a name up to 64 characters.",
                    "Save name",
                ),
                HostingBackupPromptAction::Delete => (
                    if prompt.backup.id.is_empty() {
                        "Delete selected backups"
                    } else {
                        "Delete backup"
                    },
                    if prompt.backup.id.is_empty() {
                        "These selected backups will be permanently deleted and cannot be recovered."
                    } else {
                        "This backup will be permanently deleted and cannot be recovered."
                    },
                    if prompt.backup.id.is_empty() {
                        "Delete backups"
                    } else {
                        "Delete backup"
                    },
                ),
            };
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 20,
                title,
                0x00f1f3f5,
                2,
                panel.width.saturating_sub(40),
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 58,
                description,
                if prompt.action == HostingBackupPromptAction::Rename {
                    0x00bdc2ca
                } else {
                    0x00ffaaa3
                },
                1,
                panel.width.saturating_sub(40),
            );
            if prompt.action == HostingBackupPromptAction::Rename {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    field.x,
                    field.y,
                    field.width,
                    field.height,
                    8,
                    0x00141619,
                );
                text_input_clipped(
                    &mut buffer,
                    w,
                    h,
                    field.x + 12,
                    field.y + 10,
                    &self.hosting_backup_create_name,
                    if self.input_focus == 39 {
                        0x00f1f3f5
                    } else {
                        0x007f8791
                    },
                    1,
                    field.width.saturating_sub(24),
                );
            } else {
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    panel.x + 20,
                    panel.y + 112,
                    &prompt.backup.name,
                    0x00f1f3f5,
                    1,
                    panel.width.saturating_sub(40),
                );
            }
            for (bounds, label, color) in [
                (cancel, "Cancel", 0x00343a42),
                (
                    confirm,
                    if self.hosting_backup_mutating {
                        "Working…"
                    } else {
                        confirm_label
                    },
                    if prompt.action == HostingBackupPromptAction::Delete
                        || prompt.action == HostingBackupPromptAction::Restore
                    {
                        0x003d2929
                    } else {
                        0x00303b2b
                    },
                ),
            ] {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    8,
                    color,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    bounds.x + 10,
                    bounds.y + 12,
                    label,
                    0x00f1f3f5,
                    1,
                    bounds.width.saturating_sub(20),
                );
            }
            flush_native_text(&mut buffer, w, h);
        } else if self.browse_collection_create_open {
            clear_text_commands();
            for pixel in buffer.iter_mut() {
                *pixel = blend_color(*pixel, 0, 176);
            }
            let panel_width = 460usize.min(w.saturating_sub(24));
            let panel_height = 210usize.min(h.saturating_sub(24));
            let left = w.saturating_sub(panel_width) / 2;
            let top = h.saturating_sub(panel_height) / 2;
            rounded_rect(
                &mut buffer,
                w,
                h,
                left,
                top,
                panel_width,
                panel_height,
                16,
                0x0022262c,
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                left + 20,
                top + 20,
                "Create collection",
                0x00f1f3f5,
                2,
                panel_width.saturating_sub(40),
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                left + 20,
                top + 52,
                "Choose a name between 3 and 64 characters.",
                0x00bdc2ca,
                1,
                panel_width.saturating_sub(40),
            );
            rounded_rect(
                &mut buffer,
                w,
                h,
                left + 20,
                top + 76,
                panel_width.saturating_sub(40),
                40,
                8,
                0x00343a42,
            );
            let name = if self.browse_collection_create_name.is_empty() {
                "Collection name"
            } else {
                &self.browse_collection_create_name
            };
            text_clipped(
                &mut buffer,
                w,
                h,
                left + 32,
                top + 90,
                name,
                0x00f1f3f5,
                1,
                panel_width.saturating_sub(64),
            );
            let cancel_x = left + panel_width - 220;
            let create_x = left + panel_width - 108;
            for (button_x, button_w, label, color) in [
                (cancel_x, 104usize, "Cancel", 0x00343a42),
                (
                    create_x,
                    88usize,
                    if self.browse_collection_creating {
                        "Creating…"
                    } else {
                        "Create"
                    },
                    0x00364c3a,
                ),
            ] {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    button_x,
                    top + 158,
                    button_w,
                    40,
                    8,
                    color,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    button_x + 10,
                    top + 171,
                    label,
                    0x00f1f3f5,
                    1,
                    button_w.saturating_sub(20),
                );
            }
            flush_native_text(&mut buffer, w, h);
        } else if let Some(choice) = &self.browse_install_override {
            clear_text_commands();
            for pixel in buffer.iter_mut() {
                *pixel = blend_color(*pixel, 0, 176);
            }
            let (panel, [cancel, install]) =
                browse_install_override_layout(w, h);
            rounded_rect(
                &mut buffer,
                w,
                h,
                panel.x,
                panel.y,
                panel.width,
                panel.height,
                16,
                0x0022262c,
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 20,
                "Version compatibility",
                0x00f1f3f5,
                2,
                panel.width.saturating_sub(40),
            );
            let line = format!(
                "{} {} does not list support for {}.",
                choice.project.title, choice.version_label, choice.target_label
            );
            let line = render::wrap_text_to_width(
                &line,
                1,
                panel.width.saturating_sub(40),
                2,
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 58,
                &line,
                0x00bdc2ca,
                1,
                panel.width.saturating_sub(40),
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                panel.x + 20,
                panel.y + 114,
                "Dependencies will not be installed automatically.",
                0x00bdc2ca,
                1,
                panel.width.saturating_sub(40),
            );
            for (bounds, label, color) in [
                (cancel, "Cancel", 0x00343a42),
                (install, "Install anyway", 0x003b302b),
            ] {
                rounded_rect(
                    &mut buffer,
                    w,
                    h,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    8,
                    color,
                );
                text_clipped(
                    &mut buffer,
                    w,
                    h,
                    bounds.x + 10,
                    bounds.y + 10,
                    label,
                    if bounds.contains(cursor.0, cursor.1) {
                        0x00f1f3f5
                    } else {
                        0x00bdc2ca
                    },
                    1,
                    bounds.width.saturating_sub(20),
                );
            }
            flush_native_text(&mut buffer, w, h);
        } else if let Some(notice) = &self.browse_install_notice {
            clear_text_commands();
            let width = w.saturating_sub(104).min(540);
            let x = w.saturating_sub(width + 20);
            let lines = render::wrap_text_to_width(
                &notice.text,
                1,
                width.saturating_sub(30),
                3,
            );
            let height = 28 + lines.lines().count().max(1) * 18;
            let y = h.saturating_sub(height + 60);
            let border = if notice.failed {
                0x00e68a6b
            } else {
                0x00a8e66b
            };
            rounded_rect(
                &mut buffer,
                w,
                h,
                x,
                y,
                width,
                height,
                10,
                0x0022262c,
            );
            rect(
                &mut buffer,
                w,
                h,
                x + 8,
                y + 8,
                2,
                height.saturating_sub(16),
                border,
            );
            text_clipped(
                &mut buffer,
                w,
                h,
                x + 18,
                y + 12,
                &lines,
                0x00f1f3f5,
                1,
                width.saturating_sub(30),
            );
            flush_native_text(&mut buffer, w, h);
        }
        let _ = buffer.present();
    }
}

#[cfg(test)]
mod browse_markdown_tests {
    use super::*;

    #[test]
    fn embedded_images_get_scrollable_image_blocks() {
        let body = "Before the image.\n\n![A test diagram](https://cdn.example.test/diagram.png)\n\nAfter the image.";
        let (lines, _, _, images) =
            wrap_browse_body_with_links(body, 32, "project-test");

        assert_eq!(images.len(), 1);
        let image = &images[0];
        assert_eq!(image.url, "https://cdn.example.test/diagram.png");
        assert_eq!(image.alt, "A test diagram");
        assert_eq!(image.cache_key, "description-image:project-test:0");
        assert!(lines[image.line_index].starts_with('\u{1f}'));
        assert_eq!(browse_body_line_height(&lines[image.line_index]), 220);
        assert!(browse_body_height(&lines) >= 220);
    }

    #[test]
    fn wrapped_markdown_links_and_emphasis_map_each_line() {
        let body = "**bold text that continues across several wrapped lines** and [a linked label that also wraps over several lines](https://example.test/page)";
        let (lines, links, styles, _) =
            wrap_browse_body_with_links(body, 12, "project-test");

        assert!(lines.len() > 3);
        assert!(styles.len() >= 3);
        assert!(links.len() >= 3);
        assert!(links.iter().all(|link| {
            link.url == "https://example.test/page"
                && link.start_char < link.end_char
        }));
    }

    #[test]
    fn gallery_grid_hit_targets_follow_columns_and_scroll_rows() {
        let (columns, card_width, card_height, total_height) =
            browse_gallery_grid_metrics(1100, 3);
        assert_eq!(columns, 2);
        assert!(card_width >= 320);
        assert!(total_height > card_height);
        assert_eq!(browse_gallery_item_at(1100, 3, 0.0, 292.0, 442.0), Some(0),);
        assert_eq!(
            browse_gallery_item_at(
                1100,
                3,
                0.0,
                (292 + card_width + 16) as f64,
                442.0,
            ),
            Some(1),
        );
        assert_eq!(
            browse_gallery_item_at(
                1100,
                3,
                (card_height + 16) as f64,
                292.0,
                442.0
            ),
            Some(2),
        );
    }
}
