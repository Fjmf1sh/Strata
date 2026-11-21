use super::*;

pub(super) fn draw_friend_request_modal(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    query: &str,
    results: &[theseus::users::SearchUser],
    error: Option<&str>,
    loading: bool,
    pending: &HashSet<String>,
) {
    let panel_width = 560usize.min(width.saturating_sub(32));
    let panel_height = 470usize.min(height.saturating_sub(32));
    let x = width.saturating_sub(panel_width) / 2;
    let y = height.saturating_sub(panel_height) / 2;
    rect(buffer, width, height, 0, 0, width, height, 0x00101215);
    rounded_rect(
        buffer,
        width,
        height,
        x,
        y,
        panel_width,
        panel_height,
        16,
        0x001b1d22,
    );
    text(
        buffer,
        width,
        height,
        x + 24,
        y + 24,
        "ADD FRIEND",
        0x00f1f3f5,
        2,
    );
    text(
        buffer,
        width,
        height,
        x + 24,
        y + 52,
        "Find a Modrinth user and send a friend request.",
        0x00bdc2ca,
        1,
    );
    text(
        buffer,
        width,
        height,
        x + panel_width.saturating_sub(60),
        y + 24,
        "CLOSE",
        0x007b838d,
        1,
    );
    rect(
        buffer,
        width,
        height,
        x + 24,
        y + 74,
        panel_width.saturating_sub(48),
        42,
        0x0022262c,
    );
    text(
        buffer,
        width,
        height,
        x + 36,
        y + 89,
        if query.is_empty() { "USERNAME" } else { query },
        if query.is_empty() {
            0x007b838d
        } else {
            0x00f1f3f5
        },
        1,
    );
    let search_x = x + panel_width.saturating_sub(136);
    rounded_rect(
        buffer,
        width,
        height,
        search_x,
        y + 124,
        112,
        36,
        8,
        0x00303b2b,
    );
    text(
        buffer,
        width,
        height,
        search_x + 25,
        y + 136,
        "SEARCH",
        0x00a8e66b,
        1,
    );
    if loading {
        text(
            buffer,
            width,
            height,
            x + 24,
            y + 178,
            "Searching Modrinth users…",
            0x00bdc2ca,
            1,
        );
    } else if let Some(error) = error {
        text_clipped(
            buffer,
            width,
            height,
            x + 24,
            y + 178,
            error,
            0x00e68a6b,
            1,
            panel_width.saturating_sub(48),
        );
    } else if results.is_empty() {
        text(
            buffer,
            width,
            height,
            x + 24,
            y + 178,
            "Search for a username to see matching accounts.",
            0x007b838d,
            1,
        );
    } else {
        for (index, user) in results.iter().take(5).enumerate() {
            let row_y = y + 170 + index * 54;
            rounded_rect(
                buffer,
                width,
                height,
                x + 24,
                row_y,
                panel_width.saturating_sub(48),
                48,
                8,
                0x0022262c,
            );
            let initial =
                user.username.chars().next().map_or_else(String::new, |ch| {
                    ch.to_uppercase().to_string()
                });
            rounded_rect(
                buffer,
                width,
                height,
                x + 32,
                row_y + 8,
                32,
                32,
                16,
                0x00303b2b,
            );
            text(
                buffer,
                width,
                height,
                x + 43,
                row_y + 19,
                &initial,
                0x00a8e66b,
                1,
            );
            text_clipped(
                buffer,
                width,
                height,
                x + 76,
                row_y + 11,
                &user.username,
                0x00f1f3f5,
                1,
                panel_width.saturating_sub(220),
            );
            let is_pending = pending.contains(&user.id);
            rounded_rect(
                buffer,
                width,
                height,
                x + panel_width.saturating_sub(136),
                row_y + 8,
                104,
                32,
                8,
                if is_pending { 0x002e333a } else { 0x00303b2b },
            );
            text(
                buffer,
                width,
                height,
                x + panel_width.saturating_sub(if is_pending {
                    115
                } else {
                    108
                }),
                row_y + 18,
                if is_pending { "SENDING" } else { "ADD FRIEND" },
                if is_pending { 0x007b838d } else { 0x00a8e66b },
                1,
            );
        }
    }
    text(
        buffer,
        width,
        height,
        x + panel_width.saturating_sub(84),
        y + panel_height.saturating_sub(32),
        "ESC TO CLOSE",
        0x007b838d,
        1,
    );
}

pub(super) fn draw_incoming_friend_requests(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    friends: &[native_backend::NativeFriend],
    avatars: &HashMap<String, (u32, u32, Vec<u32>)>,
    accepting: &HashSet<String>,
    removing: &HashSet<String>,
    error: Option<&str>,
    scroll_pixels: f64,
) {
    let panel_width = 560usize.min(width.saturating_sub(32));
    let panel_height = 500usize.min(height.saturating_sub(32));
    let x = width.saturating_sub(panel_width) / 2;
    let y = height.saturating_sub(panel_height) / 2;
    let incoming = sidebar_incoming_friend_indices(friends);
    rect(buffer, width, height, 0, 0, width, height, 0x00101215);
    rounded_rect(
        buffer,
        width,
        height,
        x,
        y,
        panel_width,
        panel_height,
        16,
        0x001b1d22,
    );
    text(
        buffer,
        width,
        height,
        x + 24,
        y + 24,
        "FRIEND REQUESTS",
        0x00f1f3f5,
        2,
    );
    text(
        buffer,
        width,
        height,
        x + panel_width.saturating_sub(60),
        y + 24,
        "CLOSE",
        0x007b838d,
        1,
    );
    let list_top = y + 82;
    let viewport_height = panel_height.saturating_sub(134);
    let scroll = (scroll_pixels as usize)
        .min(friend_incoming_modal_max_scroll(incoming.len(), height) as usize);
    crate::render::set_clip_rect(Some((
        x + 16,
        list_top,
        panel_width.saturating_sub(32),
        viewport_height,
    )));
    if incoming.is_empty() {
        text(
            buffer,
            width,
            height,
            x + 24,
            list_top + 20,
            "You have no pending friend requests.",
            0x00bdc2ca,
            1,
        );
    } else {
        for (row_index, friend_index) in incoming.iter().enumerate() {
            let friend = &friends[*friend_index];
            let row_y = (list_top + row_index * 64).saturating_sub(scroll);
            rounded_rect(
                buffer,
                width,
                height,
                x + 8,
                row_y,
                panel_width.saturating_sub(48),
                56,
                10,
                0x0022262c,
            );
            let avatar_key = format!("friend:{}", friend.user_id);
            if let Some((avatar_width, avatar_height, pixels)) =
                avatars.get(&avatar_key)
            {
                draw_scaled_image(
                    buffer,
                    width,
                    height,
                    *avatar_width,
                    *avatar_height,
                    pixels,
                    x + 18,
                    row_y + 10,
                    36,
                    36,
                );
            } else {
                rounded_rect(
                    buffer,
                    width,
                    height,
                    x + 18,
                    row_y + 10,
                    36,
                    36,
                    18,
                    0x00303b2b,
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
                    x + 31,
                    row_y + 21,
                    &initial,
                    0x00a8e66b,
                    1,
                );
            }
            text_clipped(
                buffer,
                width,
                height,
                x + 64,
                row_y + 10,
                &friend.username,
                0x00f1f3f5,
                1,
                panel_width.saturating_sub(300),
            );
            text(
                buffer,
                width,
                height,
                x + 64,
                row_y + 31,
                &format!("Received {}", relative_play_time(friend.created)),
                0x007b838d,
                1,
            );
            let accept_x = x + panel_width.saturating_sub(220);
            let pending_accept = accepting.contains(&friend.user_id);
            rounded_rect(
                buffer,
                width,
                height,
                accept_x,
                row_y + 12,
                92,
                32,
                8,
                if pending_accept {
                    0x002e333a
                } else {
                    0x00303b2b
                },
            );
            text(
                buffer,
                width,
                height,
                accept_x + if pending_accept { 16 } else { 25 },
                row_y + 22,
                if pending_accept { "SAVING" } else { "ACCEPT" },
                if pending_accept {
                    0x007b838d
                } else {
                    0x00a8e66b
                },
                1,
            );
            let pending_remove = removing.contains(&friend.user_id);
            let ignore_x = x + panel_width.saturating_sub(116);
            rounded_rect(
                buffer,
                width,
                height,
                ignore_x,
                row_y + 12,
                92,
                32,
                8,
                if pending_remove {
                    0x002e333a
                } else {
                    0x00343a42
                },
            );
            text(
                buffer,
                width,
                height,
                ignore_x + if pending_remove { 17 } else { 23 },
                row_y + 22,
                if pending_remove { "SAVING" } else { "IGNORE" },
                if pending_remove {
                    0x007b838d
                } else {
                    0x00f1f3f5
                },
                1,
            );
        }
    }
    crate::render::set_clip_rect(None);
    if let Some(error) = error {
        text_clipped(
            buffer,
            width,
            height,
            x + 24,
            y + panel_height.saturating_sub(54),
            error,
            0x00e68a6b,
            1,
            panel_width.saturating_sub(48),
        );
    }
    text(
        buffer,
        width,
        height,
        x + panel_width.saturating_sub(84),
        y + panel_height.saturating_sub(32),
        "ESC TO CLOSE",
        0x007b838d,
        1,
    );
}

pub(super) fn draw_friend_remove_confirmation(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    friend: &native_backend::NativeFriend,
    error: Option<&str>,
    pending: bool,
) {
    let panel_width = 440usize.min(width.saturating_sub(32));
    let panel_height = 210usize.min(height.saturating_sub(32));
    let x = width.saturating_sub(panel_width) / 2;
    let y = height.saturating_sub(panel_height) / 2;
    rect(buffer, width, height, 0, 0, width, height, 0x00101215);
    rounded_rect(
        buffer,
        width,
        height,
        x,
        y,
        panel_width,
        panel_height,
        16,
        0x001b1d22,
    );
    text(
        buffer,
        width,
        height,
        x + 24,
        y + 22,
        "REMOVE FRIEND?",
        0x00f1f3f5,
        2,
    );
    text(
        buffer,
        width,
        height,
        x + 24,
        y + 64,
        &format!("Remove {} from your Modrinth friends?", friend.username),
        0x00bdc2ca,
        1,
    );
    text(
        buffer,
        width,
        height,
        x + 24,
        y + 86,
        "This does not block the user.",
        0x007b838d,
        1,
    );
    if let Some(error) = error {
        text_clipped(
            buffer,
            width,
            height,
            x + 24,
            y + 112,
            error,
            0x00e68a6b,
            1,
            panel_width.saturating_sub(48),
        );
    }
    rounded_rect(
        buffer,
        width,
        height,
        x + 220,
        y + 150,
        96,
        36,
        8,
        0x003b302b,
    );
    text(
        buffer,
        width,
        height,
        x + 236,
        y + 162,
        if pending { "REMOVING" } else { "REMOVE" },
        0x00e68a6b,
        1,
    );
    rounded_rect(
        buffer,
        width,
        height,
        x + 324,
        y + 150,
        96,
        36,
        8,
        0x00343a42,
    );
    text(
        buffer,
        width,
        height,
        x + 343,
        y + 162,
        "CANCEL",
        0x00f1f3f5,
        1,
    );
}
