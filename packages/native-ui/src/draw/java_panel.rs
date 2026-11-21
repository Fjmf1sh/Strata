use super::*;

pub(super) fn draw_java_panel(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    java_versions: &[theseus::data::JavaVersion],
    java_busy: bool,
    message: &str,
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
    text(buffer, w, h, 292, 95, "JAVA RUNTIMES", 0x00f1f3f5, 2);
    let close_button = settings_close_button(w);
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
        w.saturating_sub(128),
        94,
        "BACK",
        0x00a8e66b,
        1,
    );
    rect(buffer, w, h, 292, 146, 210, 36, 0x00303b2b);
    text(buffer, w, h, 305, 158, "ADD EXISTING JAVA", 0x00a8e66b, 1);
    text(
        buffer,
        w,
        h,
        292,
        190,
        "Choose a Java executable to validate and register it.",
        0x007b838d,
        1,
    );
    for (index, major) in [25u32, 21, 17, 8].iter().enumerate() {
        let y = 214 + index * 58;
        rect(buffer, w, h, 292, y, w.saturating_sub(324), 48, 0x0022262c);
        text(
            buffer,
            w,
            h,
            306,
            y + 8,
            &format!("JAVA {major}"),
            0x00f1f3f5,
            1,
        );
        if let Some(installed) = java_versions
            .iter()
            .find(|item| item.parsed_version == *major)
        {
            text(
                buffer,
                w,
                h,
                306,
                y + 27,
                &format!("{} · {}", installed.version, installed.architecture),
                0x007b838d,
                1,
            );
        } else {
            text(buffer, w, h, 306, y + 27, "Not installed", 0x007b838d, 1);
            rect(
                buffer,
                w,
                h,
                w.saturating_sub(212),
                y + 9,
                164,
                30,
                if java_busy { 0x0022262c } else { 0x00303b2b },
            );
            text(
                buffer,
                w,
                h,
                w.saturating_sub(193),
                y + 19,
                if java_busy { "WORKING…" } else { "INSTALL" },
                0x00a8e66b,
                1,
            );
        }
    }
    text_clipped(
        buffer,
        w,
        h,
        292,
        464,
        message,
        0x00bdc2ca,
        1,
        w.saturating_sub(316),
    );
}
