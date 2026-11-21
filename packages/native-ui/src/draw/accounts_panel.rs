use super::*;

pub(super) fn draw_accounts_panel(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    message: &str,
    account_tab: u8,
    modrinth_login_pending: bool,
    content_accounts: &[native_backend::ContentServiceAccount],
    login_flow_active: bool,
    login_code_input: &str,
    minecraft_accounts: &[native_backend::MinecraftAccount],
    account_offset: usize,
    account_scroll_pixels: f64,
    default_account: Option<uuid::Uuid>,
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
    text(buffer, w, h, 292, 95, "ACCOUNTS", 0x00f1f3f5, 2);
    rect(
        buffer,
        w,
        h,
        292,
        119,
        126,
        25,
        if account_tab == 0 {
            0x00303b2b
        } else {
            0x0022262c
        },
    );
    text(
        buffer,
        w,
        h,
        303,
        128,
        "MINECRAFT",
        if account_tab == 0 {
            0x00a8e66b
        } else {
            0x007b838d
        },
        1,
    );
    rect(
        buffer,
        w,
        h,
        424,
        119,
        126,
        25,
        if account_tab == 1 {
            0x00303b2b
        } else {
            0x0022262c
        },
    );
    text(
        buffer,
        w,
        h,
        435,
        128,
        "MODRINTH",
        if account_tab == 1 {
            0x00a8e66b
        } else {
            0x007b838d
        },
        1,
    );
    rect(buffer, w, h, w.saturating_sub(142), 82, 112, 32, 0x0022262c);
    text(
        buffer,
        w,
        h,
        w.saturating_sub(128),
        94,
        "CLOSE",
        0x00a8e66b,
        1,
    );
    text_clipped(
        buffer,
        w,
        h,
        292,
        139,
        message,
        0x00bdc2ca,
        1,
        w.saturating_sub(316),
    );
    if account_tab == 1 {
        rect(buffer, w, h, 292, 154, 166, 34, 0x00303b2b);
        text(
            buffer,
            w,
            h,
            302,
            167,
            if modrinth_login_pending {
                "CANCEL LOGIN"
            } else {
                "ADD ACCOUNT"
            },
            0x00a8e66b,
            1,
        );
        if modrinth_login_pending {
            text(
                buffer,
                w,
                h,
                292,
                191,
                "Finish approval in your system browser.",
                0x00bdc2ca,
                1,
            );
        }
        if content_accounts.is_empty() {
            text(
                buffer,
                w,
                h,
                292,
                202,
                "No Modrinth accounts are saved yet.",
                0x00bdc2ca,
                1,
            );
        }
        let (list_x, list_width) =
            render::map_page_layout_span(292, w.saturating_sub(324));
        render::set_clip_rect(Some((
            list_x,
            204,
            list_width,
            h.saturating_sub(252),
        )));
        render::set_page_vertical_layout(
            Some(
                (account_scroll_pixels.floor() as usize)
                    .saturating_sub(account_offset.saturating_mul(58)),
            ),
            Some((204, 204, list_width, h.saturating_sub(252))),
        );
        for (index, account) in content_accounts
            .iter()
            .skip(account_offset)
            .enumerate()
            .take((h.saturating_sub(258) / 58).min(12) + 1)
        {
            let y = 204 + index * 58;
            rect(buffer, w, h, 292, y, w.saturating_sub(324), 48, 0x0022262c);
            text(buffer, w, h, 306, y + 9, &account.user_id, 0x00f1f3f5, 1);
            text(
                buffer,
                w,
                h,
                306,
                y + 28,
                if account.active {
                    "ACTIVE ACCOUNT"
                } else {
                    "MODRINTH ACCOUNT"
                },
                0x007b838d,
                1,
            );
            if !account.active {
                rect(
                    buffer,
                    w,
                    h,
                    w.saturating_sub(350),
                    y + 9,
                    132,
                    30,
                    0x00303b2b,
                );
                text(
                    buffer,
                    w,
                    h,
                    w.saturating_sub(340),
                    y + 19,
                    "SET ACTIVE",
                    0x00a8e66b,
                    1,
                );
            }
            rect(
                buffer,
                w,
                h,
                w.saturating_sub(212),
                y + 9,
                164,
                30,
                0x00372a2a,
            );
            text(
                buffer,
                w,
                h,
                w.saturating_sub(183),
                y + 19,
                "REMOVE",
                0x00ff8b82,
                1,
            );
        }
        render::set_page_vertical_layout(None, None);
        render::set_clip_rect(None);
    } else if login_flow_active {
        text(
            buffer,
            w,
            h,
            292,
            157,
            "Complete Microsoft sign-in in your browser.",
            0x00bdc2ca,
            1,
        );
        text(
            buffer,
            w,
            h,
            292,
            172,
            "Paste the code or full callback URL shown after approval.",
            0x007b838d,
            1,
        );
        rect(buffer, w, h, 292, 190, 330, 34, 0x0022262c);
        text_input_clipped(
            buffer,
            w,
            h,
            302,
            203,
            if login_code_input.is_empty() {
                "AUTHORIZATION CODE"
            } else {
                login_code_input
            },
            if login_code_input.is_empty() {
                0x007b838d
            } else {
                0x00f1f3f5
            },
            1,
            320,
        );
        rect(buffer, w, h, 632, 190, 62, 34, 0x0022262c);
        text(buffer, w, h, 641, 203, "PASTE", 0x00a8e66b, 1);
        rect(buffer, w, h, 700, 190, 62, 34, 0x00303b2b);
        text(buffer, w, h, 705, 203, "FINISH", 0x00a8e66b, 1);
        rect(buffer, w, h, 292, 236, 96, 30, 0x0022262c);
        text(buffer, w, h, 306, 246, "CANCEL", 0x00bdc2ca, 1);
    } else {
        rect(buffer, w, h, 292, 154, 166, 34, 0x00303b2b);
        text(buffer, w, h, 302, 167, "ADD ACCOUNT", 0x00a8e66b, 1);
        if minecraft_accounts.is_empty() {
            text(
                buffer,
                w,
                h,
                292,
                202,
                "No Minecraft accounts are saved yet.",
                0x00bdc2ca,
                1,
            );
        }
        let (list_x, list_width) =
            render::map_page_layout_span(292, w.saturating_sub(324));
        render::set_clip_rect(Some((
            list_x,
            204,
            list_width,
            h.saturating_sub(252),
        )));
        render::set_page_vertical_layout(
            Some(
                (account_scroll_pixels.floor() as usize)
                    .saturating_sub(account_offset.saturating_mul(58)),
            ),
            Some((204, 204, list_width, h.saturating_sub(252))),
        );
        for (index, account) in minecraft_accounts
            .iter()
            .skip(account_offset)
            .take((h.saturating_sub(258) / 58).min(12) + 1)
            .enumerate()
        {
            let y = 204 + index * 58;
            rect(buffer, w, h, 292, y, w.saturating_sub(324), 48, 0x0022262c);
            text(buffer, w, h, 306, y + 9, &account.username, 0x00f1f3f5, 1);
            let active = default_account == Some(account.id);
            text(
                buffer,
                w,
                h,
                306,
                y + 28,
                if active {
                    "DEFAULT ACCOUNT"
                } else {
                    "MICROSOFT ACCOUNT"
                },
                0x007b838d,
                1,
            );
            if !active {
                rect(
                    buffer,
                    w,
                    h,
                    w.saturating_sub(350),
                    y + 9,
                    132,
                    30,
                    0x00303b2b,
                );
                text(
                    buffer,
                    w,
                    h,
                    w.saturating_sub(340),
                    y + 19,
                    "SET DEFAULT",
                    0x00a8e66b,
                    1,
                );
            }
            rect(
                buffer,
                w,
                h,
                w.saturating_sub(212),
                y + 9,
                164,
                30,
                0x00372a2a,
            );
            text(
                buffer,
                w,
                h,
                w.saturating_sub(183),
                y + 19,
                "REMOVE",
                0x00ff8b82,
                1,
            );
        }
        render::set_page_vertical_layout(None, None);
        render::set_clip_rect(None);
    }
}
