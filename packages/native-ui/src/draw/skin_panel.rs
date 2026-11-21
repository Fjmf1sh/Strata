use super::*;

pub(super) fn draw_skin_panel(
    buffer: &mut [u32],
    w: usize,
    layout_width: usize,
    h: usize,
    skins: &[theseus::minecraft_skins::Skin],
    previews: &HashMap<String, (u32, u32, Vec<u32>)>,
    skin_scroll_pixels: f64,
    variant: theseus::minecraft_skins::MinecraftSkinVariant,
    rotation: f32,
    username: Option<&str>,
    hide_nametag: bool,
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
    text(buffer, w, h, 292, 84, "BACK TO LIBRARY", 0x00a8e66b, 1);
    text(buffer, w, h, 292, 120, "SKIN SELECTOR", 0x00f1f3f5, 2);
    text(
        buffer,
        w,
        h,
        292,
        144,
        "Minecraft profile skins",
        0x00bdc2ca,
        1,
    );
    let add_bounds = (layout_width.saturating_sub(152), 82, 132, 34);
    rect(
        buffer,
        w,
        h,
        add_bounds.0,
        add_bounds.1,
        add_bounds.2,
        add_bounds.3,
        0x00a8e66b,
    );
    text(
        buffer,
        w,
        h,
        add_bounds.0 + 18,
        add_bounds.1 + 12,
        "ADD SKIN",
        0x001b1d22,
        1,
    );
    for (button_x, label) in [
        (layout_width.saturating_sub(388), "L"),
        (layout_width.saturating_sub(344), "R"),
    ] {
        rect(buffer, w, h, button_x, 82, 36, 34, 0x0022262c);
        text(buffer, w, h, button_x + 14, 94, label, 0x00f1f3f5, 1);
    }
    let variant_label = match variant {
        theseus::minecraft_skins::MinecraftSkinVariant::Classic => {
            "MODEL: CLASSIC"
        }
        theseus::minecraft_skins::MinecraftSkinVariant::Slim => "MODEL: SLIM",
        theseus::minecraft_skins::MinecraftSkinVariant::Unknown => {
            "MODEL: UNKNOWN"
        }
    };
    rect(
        buffer,
        w,
        h,
        layout_width.saturating_sub(304),
        82,
        140,
        34,
        0x0022262c,
    );
    text(
        buffer,
        w,
        h,
        layout_width.saturating_sub(296),
        94,
        variant_label,
        0x00f1f3f5,
        1,
    );
    let (columns, card_width, card_height, gap) =
        minecraft_skin_grid_layout(layout_width);
    let start_x = 292;
    let start_y = 166;
    let visible_rows = h.saturating_sub(start_y + 42) / (card_height + gap);
    let stride = card_height + gap;
    let skin_offset = (skin_scroll_pixels / stride as f64).floor() as usize;
    let start = skin_offset * columns;
    for (visible_index, skin) in skins
        .iter()
        .skip(start)
        .take(
            (visible_rows
                + usize::from(
                    skin_scroll_pixels.rem_euclid((card_height + gap) as f64)
                        > 0.0,
                ))
                * columns,
        )
        .enumerate()
    {
        let row = visible_index / columns;
        let column = visible_index % columns;
        let x = start_x + column * (card_width + gap);
        let scroll_remainder = skin_scroll_pixels
            .rem_euclid((card_height + gap) as f64)
            .round() as usize;
        let y = (start_y + row * stride).saturating_sub(scroll_remainder);
        rounded_rect(
            buffer,
            w,
            h,
            x,
            y,
            card_width,
            card_height,
            10,
            0x0022262c,
        );
        let preview_key = skin.texture_key.to_string();
        if skin.is_equipped && !hide_nametag {
            if let Some(username) = username {
                text_clipped(
                    buffer,
                    w,
                    h,
                    x + 14,
                    y + 2,
                    username,
                    0x00bdc2ca,
                    1,
                    card_width.saturating_sub(28),
                );
            }
        }
        if let Some(preview) = previews.get(&preview_key) {
            draw_minecraft_skin_model(
                buffer,
                w,
                h,
                preview.0,
                preview.1,
                &preview.2,
                x + 14,
                y + 12,
                104.min(card_width.saturating_sub(28)),
                104,
                rotation,
                skin.variant,
            );
        } else {
            rect(
                buffer,
                w,
                h,
                x + 14,
                y + 12,
                104.min(card_width.saturating_sub(28)),
                104,
                0x001b1d22,
            );
            text(buffer, w, h, x + 26, y + 58, "LOADING", 0x007b838d, 1);
        }
        let skin_name = skin.name.as_deref().unwrap_or("Minecraft skin");
        text_clipped(
            buffer,
            w,
            h,
            x + 14,
            y + 130,
            skin_name,
            0x00f1f3f5,
            1,
            card_width.saturating_sub(28),
        );
        let detail = format!(
            "{}  •  {}",
            match skin.variant {
                theseus::minecraft_skins::MinecraftSkinVariant::Classic =>
                    "CLASSIC",
                theseus::minecraft_skins::MinecraftSkinVariant::Slim => "SLIM",
                theseus::minecraft_skins::MinecraftSkinVariant::Unknown =>
                    "UNKNOWN",
            },
            match &skin.source {
                theseus::minecraft_skins::SkinSource::Default => "DEFAULT",
                theseus::minecraft_skins::SkinSource::CustomExternal =>
                    "PROFILE",
                theseus::minecraft_skins::SkinSource::Custom => "SAVED",
            }
        );
        text(buffer, w, h, x + 14, y + 150, &detail, 0x007b838d, 1);
        let button_color = if skin.is_equipped {
            0x00303b2b
        } else {
            0x00343a42
        };
        let is_saved = matches!(
            &skin.source,
            theseus::minecraft_skins::SkinSource::Custom
        );
        let action_width =
            card_width.saturating_sub(if is_saved { 82 } else { 28 });
        rect(
            buffer,
            w,
            h,
            x + 14,
            y + 178,
            action_width,
            32,
            button_color,
        );
        text(
            buffer,
            w,
            h,
            x + 24,
            y + 189,
            if skin.is_equipped {
                "EQUIPPED"
            } else {
                "APPLY SKIN"
            },
            if skin.is_equipped {
                0x00a8e66b
            } else {
                0x00f1f3f5
            },
            1,
        );
        if is_saved {
            rect(
                buffer,
                w,
                h,
                x + card_width.saturating_sub(58),
                y + 178,
                44,
                32,
                0x003b302b,
            );
            text(
                buffer,
                w,
                h,
                x + card_width.saturating_sub(44),
                y + 189,
                "×",
                0x00e68a6b,
                1,
            );
        }
    }
    if skins.is_empty() {
        text(
            buffer,
            w,
            h,
            292,
            190,
            "No skins are available yet.",
            0x00bdc2ca,
            1,
        );
    }
}

pub(super) fn redraw_skin_header(
    buffer: &mut [u32],
    w: usize,
    layout_width: usize,
    h: usize,
    variant: theseus::minecraft_skins::MinecraftSkinVariant,
) {
    rect(
        buffer,
        w,
        h,
        248,
        56,
        w.saturating_sub(248),
        110,
        0x001b1d22,
    );
    text(buffer, w, h, 292, 84, "BACK TO LIBRARY", 0x00a8e66b, 1);
    text(buffer, w, h, 292, 120, "SKIN SELECTOR", 0x00f1f3f5, 2);
    text(
        buffer,
        w,
        h,
        292,
        144,
        "Minecraft profile skins",
        0x00bdc2ca,
        1,
    );
    let add_x = layout_width.saturating_sub(152);
    rect(buffer, w, h, add_x, 82, 132, 34, 0x00a8e66b);
    text(buffer, w, h, add_x + 18, 94, "ADD SKIN", 0x001b1d22, 1);
    for (button_x, label) in [
        (layout_width.saturating_sub(388), "L"),
        (layout_width.saturating_sub(344), "R"),
    ] {
        rect(buffer, w, h, button_x, 82, 36, 34, 0x0022262c);
        text(buffer, w, h, button_x + 14, 94, label, 0x00f1f3f5, 1);
    }
    rect(
        buffer,
        w,
        h,
        layout_width.saturating_sub(304),
        82,
        140,
        34,
        0x0022262c,
    );
    text(
        buffer,
        w,
        h,
        layout_width.saturating_sub(296),
        94,
        match variant {
            theseus::minecraft_skins::MinecraftSkinVariant::Classic => {
                "MODEL: CLASSIC"
            }
            theseus::minecraft_skins::MinecraftSkinVariant::Slim => {
                "MODEL: SLIM"
            }
            theseus::minecraft_skins::MinecraftSkinVariant::Unknown => {
                "MODEL: UNKNOWN"
            }
        },
        0x00f1f3f5,
        1,
    );
}
