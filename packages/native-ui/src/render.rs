use std::cell::RefCell;
use std::collections::HashMap;

static ACTIVE_PALETTE: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(0);

struct TextCommand {
    x: usize,
    y: usize,
    value: String,
    color: u32,
    scale: usize,
    semibold: bool,
    italic: bool,
    max_width: Option<usize>,
    tail_overflow: bool,
    clip: Option<(usize, usize, usize, usize)>,
}

thread_local! {
    static ACTIVE_CLIP: RefCell<Option<(usize, usize, usize, usize)>> = const { RefCell::new(None) };
    static ACTIVE_PAGE_LAYOUT: RefCell<Option<usize>> = const { RefCell::new(None) };
    static ACTIVE_PAGE_SCROLL: RefCell<Option<(usize, usize)>> = const { RefCell::new(None) };
    static ACTIVE_PAGE_CLIP: RefCell<Option<(usize, usize, usize, usize)>> = const { RefCell::new(None) };
}

pub(crate) fn set_page_horizontal_layout(content_right: Option<usize>) {
    ACTIVE_PAGE_LAYOUT.with(|active| *active.borrow_mut() = content_right);
}

pub(crate) fn set_page_vertical_layout(
    scroll_offset: Option<usize>,
    clip: Option<(usize, usize, usize, usize)>,
) {
    let viewport_top = clip.map_or(64, |bounds| bounds.1);
    ACTIVE_PAGE_SCROLL.with(|active| {
        *active.borrow_mut() =
            scroll_offset.map(|offset| (viewport_top, offset));
    });
    ACTIVE_PAGE_CLIP.with(|active| *active.borrow_mut() = clip);
}

fn map_page_y(y: usize) -> usize {
    ACTIVE_PAGE_SCROLL.with(|active| {
        active.borrow().map_or(y, |(viewport_top, offset)| {
            if y >= viewport_top {
                y.saturating_sub(offset)
            } else {
                y
            }
        })
    })
}

pub(crate) fn page_y_to_logical(y: f64, scroll_offset: usize) -> f64 {
    if y >= 64.0 {
        y + scroll_offset as f64
    } else {
        y
    }
}

pub(crate) fn map_page_vertical_span(
    y: usize,
    height: usize,
) -> (usize, usize) {
    let start = map_page_y(y);
    let end = map_page_y(y.saturating_add(height));
    (start, end.saturating_sub(start))
}

pub(crate) fn page_x_to_logical(x: f64, content_right: usize) -> f64 {
    let right = content_right as f64;
    if right <= 292.0 || x < 64.0 || x >= right {
        return x;
    }
    if x <= 88.0 {
        248.0 + (x - 64.0) * (44.0 / 24.0)
    } else {
        292.0 + (x - 88.0) * ((right - 292.0) / (right - 88.0))
    }
}

pub(crate) fn pointer_x_to_logical(
    x: f64,
    content_right: usize,
    fixed_settings_overlay: bool,
) -> f64 {
    if fixed_settings_overlay {
        x
    } else {
        page_x_to_logical(x, content_right)
    }
}

fn map_page_x(x: usize, content_right: usize) -> usize {
    let x = x as f64;
    let right = content_right as f64;
    if right <= 292.0 || x < 248.0 || x >= right {
        return x.round() as usize;
    }
    if x <= 292.0 {
        (64.0 + (x - 248.0) * (24.0 / 44.0)).round() as usize
    } else {
        (88.0 + (x - 292.0) * ((right - 88.0) / (right - 292.0))).round()
            as usize
    }
}

pub(super) fn map_page_span(
    x: usize,
    width: usize,
    content_right: usize,
) -> (usize, usize) {
    let start = map_page_x(x, content_right);
    let end = map_page_x(x.saturating_add(width), content_right);
    (start, end.saturating_sub(start))
}

pub(crate) fn map_page_layout_span(x: usize, width: usize) -> (usize, usize) {
    ACTIVE_PAGE_LAYOUT.with(|active| {
        active
            .borrow()
            .map_or((x, width), |right| map_page_span(x, width, right))
    })
}

pub(crate) fn set_clip_rect(clip: Option<(usize, usize, usize, usize)>) {
    let clip = clip.map(|(x, y, width, height)| {
        let (x, width) = ACTIVE_PAGE_LAYOUT.with(|active| {
            active
                .borrow()
                .map_or((x, width), |right| map_page_span(x, width, right))
        });
        let (y, height) = map_page_vertical_span(y, height);
        (x, y, width, height)
    });
    set_clip_rect_physical(clip);
}

fn set_clip_rect_physical(clip: Option<(usize, usize, usize, usize)>) {
    ACTIVE_CLIP.with(|active| *active.borrow_mut() = clip);
}

pub(crate) fn active_clip_rect() -> Option<(usize, usize, usize, usize)> {
    let active = ACTIVE_CLIP.with(|active| *active.borrow());
    let page = ACTIVE_PAGE_CLIP.with(|clip| *clip.borrow());
    match (active, page) {
        (Some(a), Some(b)) => {
            let left = a.0.max(b.0);
            let top = a.1.max(b.1);
            let right = a.0.saturating_add(a.2).min(b.0.saturating_add(b.2));
            let bottom = a.1.saturating_add(a.3).min(b.1.saturating_add(b.3));
            Some((
                left,
                top,
                right.saturating_sub(left),
                bottom.saturating_sub(top),
            ))
        }
        (Some(active), None) => Some(active),
        (None, Some(page)) => Some(page),
        (None, None) => None,
    }
}

thread_local! {
    static TEXT_COMMANDS: RefCell<Vec<TextCommand>> = const { RefCell::new(Vec::new()) };
    static TEXT_RENDERER: RefCell<Option<TextRenderer>> = const { RefCell::new(None) };
}

struct RasterizedGlyph {
    metrics: fontdue::Metrics,
    coverage: Vec<u8>,
}

struct TextRenderer {
    regular: fontdue::Font,
    semibold: fontdue::Font,
    glyphs: HashMap<(bool, u8, char), RasterizedGlyph>,
}

impl TextRenderer {
    fn new() -> Option<Self> {
        let regular = fontdue::Font::from_bytes(
            &include_bytes!("../assets/fonts/Inter-Regular.ttf")[..],
            fontdue::FontSettings::default(),
        )
        .ok()?;
        let semibold = fontdue::Font::from_bytes(
            &include_bytes!("../assets/fonts/Inter-SemiBold.ttf")[..],
            fontdue::FontSettings::default(),
        )
        .ok()?;
        Some(Self {
            regular,
            semibold,
            glyphs: HashMap::new(),
        })
    }

    fn draw_text(
        &mut self,
        buffer: &mut [u32],
        width: usize,
        height: usize,
        command: &TextCommand,
    ) {
        let semibold = command.semibold;
        let px = match command.scale {
            4 => 14.0,
            8 => 16.0,
            12.. => 24.0,
            // Match the upstream app's Inter scale more closely: the native
            // UI uses this style for section headings and navigation labels,
            // not display-sized 24 px text.
            value if value > 1 => 18.0,
            _ => 14.0,
        };
        let font = if semibold {
            &self.semibold
        } else {
            &self.regular
        };
        let line_metrics = font.horizontal_line_metrics(px);
        let ascent = line_metrics.map_or(px * 0.8, |metrics| metrics.ascent);
        let line_height =
            line_metrics.map_or(px * 1.2, |metrics| metrics.new_line_size);
        let start_x = command.x as f32;
        let mut cursor_x = start_x;
        let mut baseline = command.y as f32 + ascent;
        let mut previous_character = None;
        let available_width = width.saturating_sub(command.x);
        let max_width =
            command.max_width.map_or(available_width, |max_width| {
                max_width.min(available_width)
            });
        let fitted_value = Some(max_width).map(|max_width| {
            if command.tail_overflow {
                fit_text_to_width_tail(font, &command.value, px, max_width)
            } else {
                fit_text_to_width(font, &command.value, px, max_width)
            }
        });
        let value = fitted_value.as_deref().unwrap_or(&command.value);
        let clip = command.clip.unwrap_or((0, 0, width, height));
        for character in value.chars() {
            if character == '\n' {
                cursor_x = start_x;
                baseline += line_height;
                previous_character = None;
                continue;
            }
            if let Some(previous) = previous_character {
                cursor_x += font
                    .horizontal_kern(previous, character, px)
                    .unwrap_or_default();
            }
            let key = (semibold, px as u8, character);
            if !self.glyphs.contains_key(&key) {
                let (metrics, coverage) = font.rasterize(character, px);
                self.glyphs
                    .insert(key, RasterizedGlyph { metrics, coverage });
            }
            let Some(glyph) = self.glyphs.get(&key) else {
                continue;
            };
            let glyph_x = cursor_x.floor() as i32 + glyph.metrics.xmin;
            let glyph_y = baseline.floor() as i32
                - (glyph.metrics.ymin + glyph.metrics.height as i32);
            for row in 0..glyph.metrics.height {
                let y = glyph_y + row as i32;
                if !(clip.1 as i32..clip.1.saturating_add(clip.3) as i32)
                    .contains(&y)
                    || !(0..height as i32).contains(&y)
                {
                    continue;
                }
                for column in 0..glyph.metrics.width {
                    let italic_shift = if command.italic {
                        ((glyph.metrics.height - row) as f32 * 0.18).round()
                            as i32
                    } else {
                        0
                    };
                    let x = glyph_x + column as i32 + italic_shift;
                    if !(clip.0 as i32..clip.0.saturating_add(clip.2) as i32)
                        .contains(&x)
                        || !(0..width as i32).contains(&x)
                    {
                        continue;
                    }
                    let coverage = glyph.coverage
                        [row * glyph.metrics.width + column]
                        as u32;
                    if coverage == 0 {
                        continue;
                    }
                    let pixel = &mut buffer[y as usize * width + x as usize];
                    *pixel = blend_color(*pixel, command.color, coverage);
                }
            }
            cursor_x += glyph.metrics.advance_width;
            previous_character = Some(character);
        }
    }

    fn measure_text(&self, value: &str, scale: usize) -> usize {
        let px = match scale {
            4 => 14.0,
            8 => 16.0,
            12.. => 24.0,
            value if value > 1 => 18.0,
            _ => 14.0,
        };
        let font = if scale > 1 {
            &self.semibold
        } else {
            &self.regular
        };
        let mut width = 0.0f32;
        let mut previous_character = None;
        for character in display_text(value).chars() {
            if character == '\n' {
                previous_character = None;
                continue;
            }
            if let Some(previous) = previous_character {
                width += font
                    .horizontal_kern(previous, character, px)
                    .unwrap_or_default();
            }
            width += font.metrics(character, px).advance_width;
            previous_character = Some(character);
        }
        width.ceil().max(0.0) as usize
    }
}

pub(crate) fn set_active_palette(palette: u8) {
    ACTIVE_PALETTE.store(palette, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn active_palette() -> u8 {
    ACTIVE_PALETTE.load(std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn measure_text_width(value: &str, scale: usize) -> usize {
    TEXT_RENDERER.with(|renderer| {
        let mut renderer = renderer.borrow_mut();
        if renderer.is_none() {
            *renderer = TextRenderer::new();
        }
        renderer.as_ref().map_or_else(
            || value.chars().count() * 7 * scale,
            |renderer| renderer.measure_text(value, scale),
        )
    })
}

pub(crate) fn clear_text_commands() {
    TEXT_COMMANDS.with(|commands| commands.borrow_mut().clear());
}

pub(crate) fn themed_color(color: u32) -> u32 {
    let palette = ACTIVE_PALETTE.load(std::sync::atomic::Ordering::Relaxed);
    resolved_palette_color(color, palette)
}

pub(crate) fn contrasting_text_color(color: u32) -> u32 {
    let linear_channel = |shift: u32| {
        let channel = ((color >> shift) & 0xff) as f64 / 255.0;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = linear_channel(16) * 0.2126
        + linear_channel(8) * 0.7152
        + linear_channel(0) * 0.0722;
    if luminance > 0.179 {
        0x001b1d22
    } else {
        0x00f1f3f5
    }
}

fn resolved_palette_color(color: u32, palette: u8) -> u32 {
    let color = palette_color(color, palette);
    if palette != 0 && !(palette >= 16 && (palette - 16) / 16 == 0) {
        return color;
    }
    match color {
        0x001b1d22 | 0x00141619 => 0x0016181c,
        0x00202429 => 0x001d1f23,
        0x0022262c => 0x0027292e,
        0x002e333a | 0x00343a42 => 0x0034363c,
        0x00303b2b => 0x001a3326,
        0x00363b42 => 0x0042444a,
        0x00372a2a => 0x003b202c,
        0x003b302b => 0x00402a1b,
        0x007b838d => 0x0096a2b0,
        0x007f8791 | 0x00999fa8 => 0x0096a2b0,
        0x00a8adb5 | 0x00aeb4bc => 0x00b0bac5,
        0x00d3d7dd | 0x00d5d9df => 0x00f1f3f5,
        0x00a8e66b => 0x001bd96a,
        0x00bdc2ca => 0x00b0bac5,
        0x00e68a6b => 0x00ffa347,
        0x00f1f3f5 => 0x00ffffff,
        0x00ff8b82 => 0x00ff496e,
        _ => color,
    }
}

pub(crate) fn palette_color(color: u32, palette: u8) -> u32 {
    if palette >= 16 {
        let combined = palette - 16;
        let base_theme = combined / 16;
        let colorway = combined % 16;
        let themed = base_palette_color(color, base_theme);
        return match color {
            // Keep the selected Light, Dark, OLED, or Retro surfaces while
            // applying the chosen Northstar colorway to semantic accents.
            0x00a8e66b | 0x00e68a6b | 0x00ff8b82 => {
                base_palette_color(color, colorway + 2)
            }
            _ => themed,
        };
    }
    base_palette_color(color, palette)
}

fn base_palette_color(color: u32, palette: u8) -> u32 {
    match palette {
        1 => match color {
            0x00101215 | 0x00141619 => 0x00ffffff,
            0x001b1d22 => 0x00f4f6f8,
            0x00202429 => 0x00ffffff,
            0x0022262c => 0x00e4e9ee,
            0x002e333a => 0x00d3d9df,
            0x00303b2b => 0x00d7eee0,
            0x00343a42 => 0x00dbe1e6,
            0x00363b42 => 0x00cbd2d9,
            0x00372a2a => 0x00f3dddd,
            0x003b302b => 0x00f5dfd7,
            0x007b838d => 0x0065707a,
            0x007f8791 | 0x00999fa8 | 0x00a8adb5 | 0x00aeb4bc => 0x0065707a,
            0x00d3d7dd | 0x00d5d9df | 0x00e4e9ee => 0x00414b55,
            0x00a8e66b => 0x00247750,
            0x00bdc2ca => 0x00414b55,
            0x00e68a6b => 0x00b44631,
            0x00f1f3f5 => 0x001d252b,
            0x00ff8b82 => 0x00b4323b,
            _ => color,
        },
        2 => match color {
            0x001b1d22 | 0x00141619 => 0x00000000,
            0x00202429 => 0x00070707,
            0x0022262c => 0x00111111,
            0x002e333a => 0x001b1b1b,
            0x00303b2b => 0x001d2b19,
            0x00343a42 => 0x00282828,
            0x00363b42 => 0x00323232,
            0x00372a2a => 0x003b2020,
            0x003b302b => 0x003b241b,
            0x007b838d => 0x0088949f,
            0x007f8791 | 0x00999fa8 | 0x00a8adb5 | 0x00aeb4bc => 0x0088949f,
            0x00d3d7dd | 0x00d5d9df | 0x00e4e9ee => 0x00c2c8cf,
            0x00a8e66b => 0x009fdb68,
            0x00bdc2ca => 0x00c2c8cf,
            0x00e68a6b => 0x00f08c6c,
            0x00f1f3f5 => 0x00f4f6f8,
            _ => color,
        },
        3 => match color {
            0x00101215 => 0x00251e14,
            0x00141619 => 0x00251e14,
            0x001b1d22 => 0x002e2519,
            0x00202429 => 0x00392e1f,
            0x0022262c => 0x00463827,
            0x002e333a => 0x00524430,
            0x00303b2b => 0x0052472b,
            0x00343a42 => 0x00604e35,
            0x00363b42 => 0x00685439,
            0x00372a2a => 0x00533328,
            0x003b302b => 0x00533829,
            0x007b838d => 0x00b7a982,
            0x007f8791 | 0x00999fa8 | 0x00a8adb5 | 0x00aeb4bc => 0x00b7a982,
            0x00d3d7dd | 0x00d5d9df | 0x00e4e9ee => 0x00d9cba9,
            0x00a8e66b => 0x00e5bd59,
            0x00bdc2ca => 0x00d9cba9,
            0x00e68a6b => 0x00ee8256,
            0x00f1f3f5 => 0x00f4e6c5,
            0x00ff8b82 => 0x00ff967b,
            _ => color,
        },
        4 => match color {
            0x00101215 | 0x00141619 => 0x00171f1b,
            0x001b1d22 => 0x00212b24,
            0x00202429 => 0x002c3830,
            0x0022262c => 0x0035443a,
            0x002e333a => 0x00404e43,
            0x00303b2b => 0x003d503e,
            0x00343a42 => 0x004b5c4e,
            0x00363b42 => 0x00536555,
            0x00372a2a => 0x004a3028,
            0x003b302b => 0x004a392c,
            0x007b838d => 0x0099a794,
            0x00a8e66b => 0x0098d995,
            0x00bdc2ca => 0x00c3d2c2,
            0x00e68a6b => 0x00e29a65,
            0x00f1f3f5 => 0x00e6f0df,
            0x00ff8b82 => 0x00f47c70,
            _ => color,
        },
        5 => match color {
            0x00101215 | 0x00141619 => 0x00121c28,
            0x001b1d22 => 0x001a2837,
            0x00202429 => 0x00233445,
            0x0022262c => 0x002b4053,
            0x002e333a => 0x00344d62,
            0x00303b2b => 0x0029444c,
            0x00343a42 => 0x003b586c,
            0x00363b42 => 0x00435f73,
            0x00372a2a => 0x0046303d,
            0x003b302b => 0x00463538,
            0x007b838d => 0x008eabb8,
            0x00a8e66b => 0x006cd9c5,
            0x00bdc2ca => 0x00bdd0dc,
            0x00e68a6b => 0x00f08c70,
            0x00f1f3f5 => 0x00e5f0f8,
            0x00ff8b82 => 0x00ff7788,
            _ => color,
        },
        6 => match color {
            0x00101215 | 0x00141619 => 0x001d1928,
            0x001b1d22 => 0x00272034,
            0x00202429 => 0x00332842,
            0x0022262c => 0x003f3350,
            0x002e333a => 0x004b3b5e,
            0x00303b2b => 0x00433b58,
            0x00343a42 => 0x00524368,
            0x00363b42 => 0x005d4b74,
            0x00372a2a => 0x00543a4a,
            0x003b302b => 0x00534446,
            0x007b838d => 0x00b0a1bb,
            0x00a8e66b => 0x00d2a7f0,
            0x00bdc2ca => 0x00d7cce2,
            0x00e68a6b => 0x00f19a87,
            0x00f1f3f5 => 0x00f0eaf6,
            0x00ff8b82 => 0x00ff829a,
            _ => color,
        },
        7 => match color {
            0x00101215 | 0x00141619 => 0x00241918,
            0x001b1d22 => 0x002e201e,
            0x00202429 => 0x003b2925,
            0x0022262c => 0x004a332b,
            0x002e333a => 0x00553b30,
            0x00303b2b => 0x00543b27,
            0x00343a42 => 0x00644734,
            0x00363b42 => 0x006f4d39,
            0x00372a2a => 0x005a3029,
            0x003b302b => 0x005b3828,
            0x007b838d => 0x00c2a393,
            0x00a8e66b => 0x00ffa04e,
            0x00bdc2ca => 0x00e0cbbb,
            0x00e68a6b => 0x00ff8b4f,
            0x00f1f3f5 => 0x00f6e9db,
            0x00ff8b82 => 0x00ff7167,
            _ => color,
        },
        8 => match color {
            0x00101215 | 0x00141619 => 0x00231720,
            0x001b1d22 => 0x0030202c,
            0x00202429 => 0x003e2938,
            0x0022262c => 0x004a3042,
            0x002e333a => 0x00583a4e,
            0x00303b2b => 0x004d3443,
            0x00343a42 => 0x0067475b,
            0x00363b42 => 0x00745165,
            0x00372a2a => 0x005b303e,
            0x003b302b => 0x005a3942,
            0x007b838d => 0x00c1a1b2,
            0x00a8e66b => 0x00f18bb8,
            0x00bdc2ca => 0x00e1cbd7,
            0x00e68a6b => 0x00ff9a8b,
            0x00f1f3f5 => 0x00f8eaf1,
            0x00ff8b82 => 0x00ff718b,
            _ => color,
        },
        9 => match color {
            0x00101215 | 0x00141619 => 0x00131d27,
            0x001b1d22 => 0x001b2935,
            0x00202429 => 0x00243340,
            0x0022262c => 0x002c3d4b,
            0x002e333a => 0x00364b59,
            0x00303b2b => 0x002b4948,
            0x00343a42 => 0x003e5664,
            0x00363b42 => 0x00476370,
            0x00372a2a => 0x0041313b,
            0x003b302b => 0x0040393c,
            0x007b838d => 0x009bbbc8,
            0x00a8e66b => 0x007bd9df,
            0x00bdc2ca => 0x00c7e0e9,
            0x00e68a6b => 0x00f2a57e,
            0x00f1f3f5 => 0x00e9f5f8,
            0x00ff8b82 => 0x00ff8999,
            _ => color,
        },
        10 => match color {
            0x00101215 | 0x00141619 => 0x00090f0b,
            0x001b1d22 => 0x000d1710,
            0x00202429 => 0x00121f16,
            0x0022262c => 0x0018271c,
            0x002e333a => 0x001f3022,
            0x00303b2b => 0x001d3520,
            0x00343a42 => 0x00263829,
            0x00363b42 => 0x002c402e,
            0x00372a2a => 0x00352519,
            0x003b302b => 0x00342a18,
            0x007b838d => 0x007b9a73,
            0x00a8e66b => 0x007dff70,
            0x00bdc2ca => 0x00c0d8b8,
            0x00e68a6b => 0x00e9c95e,
            0x00f1f3f5 => 0x00d7f2cf,
            0x00ff8b82 => 0x00ff775e,
            _ => color,
        },
        _ => color,
    }
}

pub(crate) fn draw_scaled_instance_icon(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    size: usize,
    image: &(u32, u32, Vec<u32>),
) {
    draw_scaled_icon_with_radius(
        buffer,
        width,
        height,
        x,
        y,
        size,
        image,
        20.min(size / 2),
    );
}

pub(crate) fn draw_scaled_project_icon(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    size: usize,
    image: &(u32, u32, Vec<u32>),
) {
    draw_scaled_icon_with_radius(
        buffer,
        width,
        height,
        x,
        y,
        size,
        image,
        12.min(size / 4),
    );
}

pub(crate) fn draw_navigation_icon(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    center_x: usize,
    center_y: usize,
    icon: u8,
    color: u32,
) {
    // Navigation glyph geometry follows the Lucide icons used by the
    // upstream shell (ISC; see packages/assets/icons for source assets).
    let color = themed_color(color);
    macro_rules! path {
        ($points:expr, $close:expr $(,)?) => {{
            let points = $points;
            for pair in points.windows(2) {
                draw_line(buffer, width, height, pair[0], pair[1], color);
                draw_line(
                    buffer,
                    width,
                    height,
                    (pair[0].0, pair[0].1 + 1),
                    (pair[1].0, pair[1].1 + 1),
                    color,
                );
            }
            if $close && points.len() > 2 {
                draw_line(
                    buffer,
                    width,
                    height,
                    *points.last().unwrap(),
                    points[0],
                    color,
                );
                draw_line(
                    buffer,
                    width,
                    height,
                    (points.last().unwrap().0, points.last().unwrap().1 + 1),
                    (points[0].0, points[0].1 + 1),
                    color,
                );
            }
        }};
    }
    let cx = center_x as isize;
    let cy = center_y as isize;
    match icon {
        0 => {
            path!(&[(cx - 9, cy - 1), (cx, cy - 9), (cx + 9, cy - 1)], false);
            path!(
                &[
                    (cx - 7, cy - 2),
                    (cx - 7, cy + 8),
                    (cx + 7, cy + 8),
                    (cx + 7, cy - 2)
                ],
                false,
            );
            path!(
                &[
                    (cx - 2, cy + 8),
                    (cx - 2, cy + 2),
                    (cx + 2, cy + 2),
                    (cx + 2, cy + 8)
                ],
                false
            );
        }
        1 => {
            draw_outline_circle(buffer, width, height, cx, cy, 9, color);
            path!(
                &[
                    (cx + 4, cy - 4),
                    (cx + 2, cy + 1),
                    (cx - 4, cy + 4),
                    (cx - 2, cy - 1),
                    (cx - 1, cy - 2),
                ],
                true
            );
        }
        2 => path!(
            &[
                (cx - 4, cy - 8),
                (cx - 9, cy - 4),
                (cx - 6, cy),
                (cx - 3, cy - 2),
                (cx - 3, cy + 8),
                (cx + 3, cy + 8),
                (cx + 3, cy - 2),
                (cx + 6, cy),
                (cx + 9, cy - 4),
                (cx + 4, cy - 8),
                (cx + 2, cy - 4),
                (cx - 2, cy - 4),
            ],
            true,
        ),
        3 => {
            path!(
                &[
                    (cx - 8, cy - 8),
                    (cx + 8, cy - 8),
                    (cx + 8, cy + 8),
                    (cx - 8, cy + 8),
                ],
                true,
            );
            draw_outline_circle(
                buffer,
                width,
                height,
                cx - 3,
                cy - 3,
                1,
                color,
            );
            path!(
                &[
                    (cx - 6, cy + 7),
                    (cx + 2, cy - 1),
                    (cx + 4, cy - 1),
                    (cx + 7, cy + 2),
                ],
                false,
            );
        }
        4 => {
            path!(&[(cx, cy - 8), (cx, cy + 8)], false);
            path!(&[(cx - 8, cy), (cx + 8, cy)], false);
        }
        5 => {
            path!(
                &[
                    (cx, cy - 9),
                    (cx + 2, cy - 8),
                    (cx + 3, cy - 6),
                    (cx + 5, cy - 5),
                    (cx + 7, cy - 6),
                    (cx + 9, cy - 3),
                    (cx + 7, cy - 1),
                    (cx + 7, cy + 1),
                    (cx + 9, cy + 3),
                    (cx + 7, cy + 6),
                    (cx + 5, cy + 5),
                    (cx + 3, cy + 6),
                    (cx + 2, cy + 8),
                    (cx, cy + 9),
                    (cx - 2, cy + 8),
                    (cx - 3, cy + 6),
                    (cx - 5, cy + 5),
                    (cx - 7, cy + 6),
                    (cx - 9, cy + 3),
                    (cx - 7, cy + 1),
                    (cx - 7, cy - 1),
                    (cx - 9, cy - 3),
                    (cx - 7, cy - 6),
                    (cx - 5, cy - 5),
                    (cx - 3, cy - 6),
                    (cx - 2, cy - 8),
                ],
                true,
            );
            draw_outline_circle(buffer, width, height, cx, cy, 3, color);
        }
        7 => {
            draw_outline_circle(buffer, width, height, cx, cy, 8, color);
            path!(&[(cx - 2, cy - 5), (cx + 5, cy), (cx - 2, cy + 5)], true,);
        }
        8 => {
            for offset in [-5, 4] {
                path!(
                    &[
                        (cx - 9, cy + offset - 3),
                        (cx + 9, cy + offset - 3),
                        (cx + 9, cy + offset + 3),
                        (cx - 9, cy + offset + 3)
                    ],
                    true
                );
                draw_outline_circle(
                    buffer,
                    width,
                    height,
                    cx - 4,
                    cy + offset,
                    1,
                    color,
                );
                draw_line(
                    buffer,
                    width,
                    height,
                    (cx, cy + offset),
                    (cx + 5, cy + offset),
                    color,
                );
            }
        }
        9 => path!(&[(cx - 5, cy - 8), (cx + 7, cy), (cx - 5, cy + 8),], true,),
        _ => {
            draw_outline_circle(buffer, width, height, cx, cy - 4, 4, color);
            path!(
                &[
                    (cx - 9, cy + 8),
                    (cx - 8, cy + 4),
                    (cx - 5, cy + 2),
                    (cx + 5, cy + 2),
                    (cx + 8, cy + 4),
                    (cx + 9, cy + 8)
                ],
                false,
            );
        }
    }
}

pub(crate) fn draw_search_icon(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    center_x: usize,
    center_y: usize,
    color: u32,
) {
    let color = themed_color(color);
    let center = (center_x as isize, center_y as isize);
    draw_outline_circle(buffer, width, height, center.0, center.1, 5, color);
    draw_line(
        buffer,
        width,
        height,
        (center.0 + 4, center.1 + 4),
        (center.0 + 8, center.1 + 8),
        color,
    );
}

pub(crate) fn draw_line(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    (start_x, start_y): (isize, isize),
    (end_x, end_y): (isize, isize),
    color: u32,
) {
    let (start_x, end_x) = ACTIVE_PAGE_LAYOUT.with(|active| {
        active.borrow().map_or((start_x, end_x), |right| {
            let map = |x: isize| {
                if x < 0 {
                    x
                } else {
                    map_page_x(x as usize, right) as isize
                }
            };
            (map(start_x), map(end_x))
        })
    });
    let start_y = map_page_y(start_y.max(0) as usize) as isize;
    let end_y = map_page_y(end_y.max(0) as usize) as isize;
    let clip = active_clip_rect();
    let dx = (end_x - start_x).abs();
    let sx = if start_x < end_x { 1 } else { -1 };
    let dy = -(end_y - start_y).abs();
    let sy = if start_y < end_y { 1 } else { -1 };
    let mut error = dx + dy;
    let (mut x, mut y) = (start_x, start_y);
    loop {
        if x >= 0
            && y >= 0
            && x < width as isize
            && y < height as isize
            && clip.is_none_or(|c| {
                x >= c.0 as isize
                    && y >= c.1 as isize
                    && x < c.0.saturating_add(c.2) as isize
                    && y < c.1.saturating_add(c.3) as isize
            })
        {
            buffer[y as usize * width + x as usize] = color;
        }
        if x == end_x && y == end_y {
            break;
        }
        let twice_error = 2 * error;
        if twice_error >= dy {
            error += dy;
            x += sx;
        }
        if twice_error <= dx {
            error += dx;
            y += sy;
        }
    }
}

fn draw_outline_circle(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    center_x: isize,
    center_y: isize,
    radius: isize,
    color: u32,
) {
    let segments = (radius * 8).max(16);
    let mut previous = None;
    for segment in 0..=segments {
        let angle = std::f64::consts::TAU * segment as f64 / segments as f64;
        let point = (
            center_x + (radius as f64 * angle.cos()).round() as isize,
            center_y + (radius as f64 * angle.sin()).round() as isize,
        );
        if let Some(previous) = previous {
            draw_line(buffer, width, height, previous, point, color);
        }
        previous = Some(point);
    }
}

fn draw_scaled_icon_with_radius(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    size: usize,
    image: &(u32, u32, Vec<u32>),
    radius: usize,
) {
    let original_size = size;
    let (x, size) = map_page_layout_span(x, size);
    let radius = if original_size == 0 {
        radius
    } else {
        radius.saturating_mul(size) / original_size
    };
    let (source_width, source_height, pixels) = image;
    if *source_width == 0 || *source_height == 0 || size == 0 {
        return;
    }
    let scale = (size as f64 / *source_width as f64)
        .min(size as f64 / *source_height as f64);
    let draw_width = (*source_width as f64 * scale).round().max(1.0) as usize;
    let draw_height = (*source_height as f64 * scale).round().max(1.0) as usize;
    let offset_x = x + (size.saturating_sub(draw_width)) / 2;
    let offset_y = y + (size.saturating_sub(draw_height)) / 2;
    let clip = ACTIVE_CLIP.with(|active| *active.borrow());
    for destination_y in 0..draw_height {
        let target_y = offset_y + destination_y;
        if target_y >= height
            || clip.is_some_and(|c| {
                target_y < c.1 || target_y >= c.1.saturating_add(c.3)
            })
        {
            continue;
        }
        let source_y = destination_y * *source_height as usize / draw_height;
        for destination_x in 0..draw_width {
            let target_x = offset_x + destination_x;
            if target_x >= width
                || clip.is_some_and(|c| {
                    target_x < c.0 || target_x >= c.0.saturating_add(c.2)
                })
            {
                continue;
            }
            let local_x = target_x.saturating_sub(x);
            let local_y = target_y.saturating_sub(y);
            let dx = if local_x < radius {
                radius - local_x
            } else if local_x >= size.saturating_sub(radius) {
                local_x.saturating_sub(size.saturating_sub(radius + 1))
            } else {
                0
            };
            let dy = if local_y < radius {
                radius - local_y
            } else if local_y >= size.saturating_sub(radius) {
                local_y.saturating_sub(size.saturating_sub(radius + 1))
            } else {
                0
            };
            if dx.saturating_mul(dx) + dy.saturating_mul(dy)
                > radius.saturating_mul(radius)
            {
                continue;
            }
            let source_x = destination_x * *source_width as usize / draw_width;
            let Some(&source) =
                pixels.get(source_y * *source_width as usize + source_x)
            else {
                continue;
            };
            let alpha = (source >> 24) & 0xff;
            if alpha == 0 {
                continue;
            }
            let background = buffer[target_y * width + target_x];
            let blend = |shift: u32| {
                let foreground = (source >> shift) & 0xff;
                let background = (background >> shift) & 0xff;
                (foreground * alpha + background * (255 - alpha)) / 255
            };
            buffer[target_y * width + target_x] =
                (blend(16) << 16) | (blend(8) << 8) | blend(0);
        }
    }
}

pub(crate) fn draw_northstar_mark(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    size: usize,
) {
    for local_y in 0..size {
        let ny = (local_y as f32 + 0.5) / size.max(1) as f32 * 2.0 - 1.0;
        for local_x in 0..size {
            let nx = (local_x as f32 + 0.5) / size.max(1) as f32 * 2.0 - 1.0;
            let angle = ny.atan2(nx);
            let radius = 0.30 + 0.58 * (2.0 * angle).cos().abs();
            if nx.hypot(ny) <= radius {
                let target_x = x + local_x;
                let target_y = y + local_y;
                if target_x < width && target_y < height {
                    buffer[target_y * width + target_x] =
                        themed_color(0x00a8e66b);
                }
            }
        }
    }
}

pub(crate) fn northstar_window_icon() -> Option<winit::window::Icon> {
    const SIZE: usize = 32;
    let mut rgba = vec![0; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let nx = (x as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let ny = (y as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let index = (y * SIZE + x) * 4;
            let corner_x = (nx.abs() - 0.78).max(0.0);
            let corner_y = (ny.abs() - 0.78).max(0.0);
            if nx.abs() < 0.78
                || ny.abs() < 0.78
                || corner_x.hypot(corner_y) < 0.22
            {
                rgba[index..index + 4].copy_from_slice(&[20, 22, 25, 255]);
            }
            let angle = ny.atan2(nx);
            let radius = 0.30 + 0.58 * (2.0 * angle).cos().abs();
            if nx.hypot(ny) <= radius * 0.70 {
                rgba[index..index + 4].copy_from_slice(&[168, 230, 107, 255]);
            }
        }
    }
    winit::window::Icon::from_rgba(rgba, SIZE as u32, SIZE as u32).ok()
}

pub(crate) fn rect(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    rw: usize,
    rh: usize,
    color: u32,
) {
    let radius = if rw < w && rh < h && rw >= 20 && rh >= 20 {
        10.min(rw / 2).min(rh / 2)
    } else {
        0
    };
    rounded_rect(buffer, w, h, x, y, rw, rh, radius, color);
}

pub(crate) fn rect_square(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    rw: usize,
    rh: usize,
    color: u32,
) {
    rounded_rect(buffer, w, h, x, y, rw, rh, 0, color);
}

pub(crate) fn rounded_rect(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    rw: usize,
    rh: usize,
    radius: usize,
    color: u32,
) {
    let (x, rw) = ACTIVE_PAGE_LAYOUT.with(|active| {
        active
            .borrow()
            .map_or((x, rw), |right| map_page_span(x, rw, right))
    });
    let (y, rh) = map_page_vertical_span(y, rh);
    let color = themed_color(color);
    let radius = radius.min(rw / 2).min(rh / 2);
    let clip = active_clip_rect();
    let (clip_x, clip_y, clip_w, clip_h) = clip.unwrap_or((0, 0, w, h));
    for yy in y.max(clip_y).min(h)
        ..y.saturating_add(rh)
            .min(h)
            .min(clip_y.saturating_add(clip_h))
    {
        for xx in x.max(clip_x).min(w)
            ..x.saturating_add(rw)
                .min(w)
                .min(clip_x.saturating_add(clip_w))
        {
            if radius > 0 {
                let dx = if xx < x + radius {
                    x + radius - xx
                } else if xx >= x.saturating_add(rw).saturating_sub(radius) {
                    xx - (x.saturating_add(rw).saturating_sub(radius + 1))
                } else {
                    0
                };
                let dy = if yy < y + radius {
                    y + radius - yy
                } else if yy >= y.saturating_add(rh).saturating_sub(radius) {
                    yy - (y.saturating_add(rh).saturating_sub(radius + 1))
                } else {
                    0
                };
                if dx.saturating_mul(dx) + dy.saturating_mul(dy)
                    > radius.saturating_mul(radius)
                {
                    continue;
                }
            }
            buffer[yy * w + xx] = color;
        }
    }
}

pub(crate) fn brighten_rounded_rect(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    rw: usize,
    rh: usize,
    radius: usize,
    coverage: u32,
) {
    let (x, rw) = ACTIVE_PAGE_LAYOUT.with(|active| {
        active
            .borrow()
            .map_or((x, rw), |right| map_page_span(x, rw, right))
    });
    let (y, rh) = map_page_vertical_span(y, rh);
    let radius = radius.min(rw / 2).min(rh / 2);
    let clip = active_clip_rect();
    let (clip_x, clip_y, clip_w, clip_h) = clip.unwrap_or((0, 0, w, h));
    for yy in y.max(clip_y).min(h)
        ..y.saturating_add(rh)
            .min(h)
            .min(clip_y.saturating_add(clip_h))
    {
        for xx in x.max(clip_x).min(w)
            ..x.saturating_add(rw)
                .min(w)
                .min(clip_x.saturating_add(clip_w))
        {
            if radius > 0 {
                let dx = if xx < x + radius {
                    x + radius - xx
                } else if xx >= x.saturating_add(rw).saturating_sub(radius) {
                    xx - (x.saturating_add(rw).saturating_sub(radius + 1))
                } else {
                    0
                };
                let dy = if yy < y + radius {
                    y + radius - yy
                } else if yy >= y.saturating_add(rh).saturating_sub(radius) {
                    yy - (y.saturating_add(rh).saturating_sub(radius + 1))
                } else {
                    0
                };
                if dx.saturating_mul(dx) + dy.saturating_mul(dy)
                    > radius.saturating_mul(radius)
                {
                    continue;
                }
            }
            let index = yy * w + xx;
            buffer[index] = blend_color(buffer[index], 0x00ffffff, coverage);
        }
    }
}

pub(crate) fn text(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    value: &str,
    color: u32,
    scale: usize,
) {
    text_styled(buffer, w, h, x, y, value, color, scale, scale > 1, false);
}

pub(crate) fn text_styled(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    value: &str,
    color: u32,
    scale: usize,
    semibold: bool,
    italic: bool,
) {
    let color = themed_color(color);
    let display_value = display_text(value);
    let (x, available_width) = ACTIVE_PAGE_LAYOUT.with(|active| {
        active.borrow().map_or((x, w.saturating_sub(x)), |right| {
            let x = map_page_x(x, right);
            (x, right.saturating_sub(x))
        })
    });
    let y = map_page_y(y);
    let clip = active_clip_rect();
    let max_width = clip.map_or(available_width, |bounds| {
        available_width.min(bounds.0.saturating_add(bounds.2).saturating_sub(x))
    });
    let command = TextCommand {
        x,
        y,
        value: display_value,
        color,
        scale,
        semibold,
        italic,
        max_width: Some(max_width),
        tail_overflow: false,
        clip,
    };
    TEXT_COMMANDS.with(|commands| commands.borrow_mut().push(command));
    let _ = (buffer, w, h);
}

pub(crate) fn text_regular(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    value: &str,
    color: u32,
    scale: usize,
    max_width: usize,
) {
    let color = themed_color(color);
    let (x, max_width) = ACTIVE_PAGE_LAYOUT.with(|active| {
        active
            .borrow()
            .map_or((x, max_width), |right| map_page_span(x, max_width, right))
    });
    let y = map_page_y(y);
    let command = TextCommand {
        x,
        y,
        value: display_text(value),
        color,
        scale,
        semibold: false,
        italic: false,
        max_width: Some(max_width),
        tail_overflow: false,
        clip: active_clip_rect(),
    };
    TEXT_COMMANDS.with(|commands| commands.borrow_mut().push(command));
    let _ = (buffer, w, h);
}

pub(crate) fn text_clipped(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    value: &str,
    color: u32,
    scale: usize,
    max_width: usize,
) {
    let color = themed_color(color);
    let (x, max_width) = ACTIVE_PAGE_LAYOUT.with(|active| {
        active
            .borrow()
            .map_or((x, max_width), |right| map_page_span(x, max_width, right))
    });
    let y = map_page_y(y);
    let command = TextCommand {
        x,
        y,
        value: display_text(value),
        color,
        scale,
        semibold: scale > 1,
        italic: false,
        max_width: Some(max_width),
        tail_overflow: false,
        clip: active_clip_rect(),
    };
    TEXT_COMMANDS.with(|commands| commands.borrow_mut().push(command));
    let _ = (buffer, w, h);
}

pub(crate) fn wrap_text_to_width(
    value: &str,
    scale: usize,
    max_width: usize,
    max_lines: usize,
) -> String {
    if max_width == 0 || max_lines == 0 {
        return String::new();
    }
    TEXT_RENDERER.with(|renderer| {
        let mut renderer = renderer.borrow_mut();
        if renderer.is_none() {
            *renderer = TextRenderer::new();
        }
        let Some(renderer) = renderer.as_ref() else {
            return value.to_owned();
        };
        let px = match scale {
            4 => 14.0,
            8 => 16.0,
            12.. => 24.0,
            value if value > 1 => 18.0,
            _ => 14.0,
        };
        let font = if scale > 1 {
            &renderer.semibold
        } else {
            &renderer.regular
        };
        let mut lines = Vec::new();
        let mut line = String::new();
        let mut previous = None;
        let mut width = 0.0f32;
        let mut truncated = false;

        'paragraphs: for (paragraph_index, paragraph) in
            value.split('\n').enumerate()
        {
            if paragraph_index > 0 {
                lines.push(std::mem::take(&mut line));
                previous = None;
                width = 0.0;
                if lines.len() >= max_lines {
                    truncated = true;
                    break;
                }
            }
            for word in paragraph.split_whitespace() {
                let word_width = word
                    .chars()
                    .fold((0.0f32, None), |(width, previous), character| {
                        let kern = previous
                            .and_then(|left| {
                                font.horizontal_kern(left, character, px)
                            })
                            .unwrap_or_default();
                        (
                            width
                                + kern
                                + font.metrics(character, px).advance_width,
                            Some(character),
                        )
                    })
                    .0;
                let space_width = if line.is_empty() {
                    0.0
                } else {
                    font.metrics(' ', px).advance_width
                };
                let first_kern = if line.is_empty() {
                    0.0
                } else {
                    previous
                        .and_then(|left| {
                            font.horizontal_kern(
                                left,
                                word.chars().next().unwrap_or(' '),
                                px,
                            )
                        })
                        .unwrap_or_default()
                };
                if !line.is_empty()
                    && width + space_width + first_kern + word_width
                        > max_width as f32
                {
                    lines.push(std::mem::take(&mut line));
                    previous = None;
                    width = 0.0;
                    if lines.len() >= max_lines {
                        truncated = true;
                        break 'paragraphs;
                    }
                }
                if !line.is_empty() {
                    line.push(' ');
                    width += font.metrics(' ', px).advance_width;
                }
                for character in word.chars() {
                    let mut kern = previous
                        .and_then(|left| {
                            font.horizontal_kern(left, character, px)
                        })
                        .unwrap_or_default();
                    let advance = font.metrics(character, px).advance_width;
                    if !line.is_empty()
                        && width + kern + advance > max_width as f32
                    {
                        lines.push(std::mem::take(&mut line));
                        width = 0.0;
                        kern = 0.0;
                        if lines.len() >= max_lines {
                            truncated = true;
                            break 'paragraphs;
                        }
                    }
                    line.push(character);
                    width += advance + kern;
                    previous = Some(character);
                }
            }
        }
        if !line.is_empty() && lines.len() < max_lines {
            lines.push(line);
        }
        if truncated && let Some(last) = lines.last_mut() {
            *last = fit_text_line(font, &format!("{last}…"), px, max_width);
        }
        lines.join("\n")
    })
}

pub(crate) fn text_input_clipped(
    buffer: &mut [u32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    value: &str,
    color: u32,
    scale: usize,
    max_width: usize,
) {
    let color = themed_color(color);
    let (x, max_width) = ACTIVE_PAGE_LAYOUT.with(|active| {
        active
            .borrow()
            .map_or((x, max_width), |right| map_page_span(x, max_width, right))
    });
    let y = map_page_y(y);
    let command = TextCommand {
        x,
        y,
        value: value.to_owned(),
        color,
        scale,
        semibold: scale > 1,
        italic: false,
        max_width: Some(max_width),
        tail_overflow: true,
        clip: active_clip_rect(),
    };
    TEXT_COMMANDS.with(|commands| commands.borrow_mut().push(command));
    let _ = (buffer, w, h);
}

fn fit_text_to_width(
    font: &fontdue::Font,
    value: &str,
    px: f32,
    max_width: usize,
) -> String {
    value
        .split('\n')
        .map(|line| fit_text_line(font, line, px, max_width))
        .collect::<Vec<_>>()
        .join("\n")
}

fn fit_text_to_width_tail(
    font: &fontdue::Font,
    value: &str,
    px: f32,
    max_width: usize,
) -> String {
    let Some(line) = value.rsplit('\n').next() else {
        return String::new();
    };
    let full_width: f32 = line
        .chars()
        .map(|character| font.metrics(character, px).advance_width)
        .sum();
    if full_width <= max_width as f32 {
        return line.to_owned();
    }
    let ellipsis = '…';
    let ellipsis_width = font.metrics(ellipsis, px).advance_width;
    let budget = (max_width as f32 - ellipsis_width).max(0.0);
    let mut width = 0.0;
    let mut trailing = Vec::new();
    for character in line.chars().rev() {
        let advance = font.metrics(character, px).advance_width;
        if width + advance > budget {
            break;
        }
        width += advance;
        trailing.push(character);
    }
    trailing.reverse();
    let mut output = String::new();
    if max_width as f32 >= ellipsis_width {
        output.push(ellipsis);
    }
    output.extend(trailing);
    output
}

fn fit_text_line(
    font: &fontdue::Font,
    line: &str,
    px: f32,
    max_width: usize,
) -> String {
    let ellipsis = '…';
    let ellipsis_width = font.metrics(ellipsis, px).advance_width;
    let full_width = line
        .chars()
        .fold((0.0, None), |(width, previous), character| {
            let kern = previous
                .and_then(|left| font.horizontal_kern(left, character, px))
                .unwrap_or_default();
            (
                width + kern + font.metrics(character, px).advance_width,
                Some(character),
            )
        })
        .0;
    if full_width <= max_width as f32 {
        return line.to_owned();
    }
    let budget = (max_width as f32 - ellipsis_width).max(0.0);
    let mut width = 0.0;
    let mut previous = None;
    let mut output = String::new();
    for character in line.chars() {
        let kern = previous
            .and_then(|left| font.horizontal_kern(left, character, px))
            .unwrap_or_default();
        let advance = font.metrics(character, px).advance_width;
        if width + kern + advance > budget {
            let ellipsis_kern = previous
                .and_then(|left| font.horizontal_kern(left, ellipsis, px))
                .unwrap_or_default();
            if width + ellipsis_kern + ellipsis_width <= max_width as f32 {
                output.push(ellipsis);
            }
            return output;
        }
        width += kern + advance;
        output.push(character);
        previous = Some(character);
    }
    output
}

fn display_text(value: &str) -> String {
    if value.chars().any(char::is_lowercase) {
        return value.to_owned();
    }
    let mut words = value.split_whitespace();
    let Some(first) = words.next() else {
        return String::new();
    };
    let mut output = capitalize_word(first.to_lowercase());
    for word in words {
        output.push(' ');
        output.push_str(&styled_word(word));
    }
    output
}

fn capitalize_word(value: String) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return value;
    };
    first.to_uppercase().chain(chars).collect()
}

fn styled_word(value: &str) -> String {
    let lower = value.to_lowercase();
    match lower.as_str() {
        "java" => "Java".into(),
        "minecraft" => "Minecraft".into(),
        "modrinth" => "Modrinth".into(),
        "neoforge" => "NeoForge".into(),
        "api" => "API".into(),
        "jvm" => "JVM".into(),
        "cpu" => "CPU".into(),
        "gpu" => "GPU".into(),
        "ram" => "RAM".into(),
        "fps" => "FPS".into(),
        "id" => "ID".into(),
        "url" => "URL".into(),
        "json" => "JSON".into(),
        _ => lower,
    }
}

pub(crate) fn flush_native_text(
    buffer: &mut [u32],
    width: usize,
    height: usize,
) {
    TEXT_COMMANDS.with(|commands| {
        let commands = commands.borrow();
        TEXT_RENDERER.with(|renderer| {
            let mut renderer = renderer.borrow_mut();
            let previous_clip = active_clip_rect();
            if renderer.is_none() {
                *renderer = TextRenderer::new();
            }
            if let Some(renderer) = renderer.as_mut() {
                for command in commands.iter() {
                    set_clip_rect_physical(command.clip);
                    renderer.draw_text(buffer, width, height, command);
                }
            } else {
                for command in commands.iter() {
                    set_clip_rect_physical(command.clip);
                    draw_bitmap_text(buffer, width, height, command);
                }
            }
            set_clip_rect_physical(previous_clip);
        });
    });
}

pub(crate) fn blend_color(
    background: u32,
    foreground: u32,
    coverage: u32,
) -> u32 {
    let inverse_coverage = 255 - coverage;
    let blend_channel = |shift: u32| {
        ((((foreground >> shift) & 0xff) * coverage
            + ((background >> shift) & 0xff) * inverse_coverage
            + 127)
            / 255)
            << shift
    };
    blend_channel(16) | blend_channel(8) | blend_channel(0)
}

fn draw_bitmap_text(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    command: &TextCommand,
) {
    let mut cursor_x = command.x;
    let max_x = command
        .x
        .saturating_add(
            command
                .max_width
                .unwrap_or_else(|| width.saturating_sub(command.x)),
        )
        .min(width);
    for character in command.value.chars() {
        if character == '\n' {
            continue;
        }
        let glyph = glyph(character.to_ascii_uppercase());
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    if cursor_x.saturating_add(column * command.scale) >= max_x
                    {
                        continue;
                    }
                    rect(
                        buffer,
                        width,
                        height,
                        cursor_x + column * command.scale,
                        command.y + row * command.scale,
                        command.scale,
                        command.scale,
                        command.color,
                    );
                }
            }
        }
        cursor_x += 6 * command.scale;
    }
}

fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 4, 4],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod clipping_tests {
    use super::*;

    #[test]
    fn selection_check_color_contrasts_with_light_and_dark_accents() {
        assert_eq!(contrasting_text_color(0x001bd96a), 0x001b1d22);
        assert_eq!(contrasting_text_color(0x00247750), 0x00f1f3f5);
        assert_eq!(contrasting_text_color(0x00e5bd59), 0x001b1d22);
    }

    #[test]
    fn secondary_text_aliases_follow_light_dark_and_retro_palettes() {
        assert_eq!(resolved_palette_color(0x007f8791, 0), 0x0096a2b0);
        assert_eq!(palette_color(0x00d3d7dd, 1), 0x00414b55);
        assert_eq!(palette_color(0x00a8adb5, 1), 0x0065707a);
        assert_eq!(palette_color(0x007f8791, 3), 0x00b7a982);
        assert_eq!(palette_color(0x00d3d7dd, 3), 0x00d9cba9);
    }

    #[test]
    fn hover_brightness_respects_rounded_card_corners() {
        set_clip_rect(None);
        set_page_horizontal_layout(None);
        set_page_vertical_layout(None, None);
        let mut buffer = vec![0; 12 * 12];
        rounded_rect(&mut buffer, 12, 12, 1, 1, 10, 10, 4, 0x0022262c);
        let original_center = buffer[5 * 12 + 5];
        let original_corner = buffer[1 * 12 + 1];
        brighten_rounded_rect(&mut buffer, 12, 12, 1, 1, 10, 10, 4, 26);
        assert_eq!(buffer[1 * 12 + 1], original_corner);
        assert_eq!(
            buffer[5 * 12 + 5],
            blend_color(original_center, 0x00ffffff, 26)
        );
    }

    #[test]
    fn composed_dark_colorways_keep_default_dark_surfaces() {
        let amber_dark_palette = 16 + 1;
        assert_eq!(
            resolved_palette_color(0x001b1d22, amber_dark_palette),
            0x0016181c
        );
        assert_eq!(
            resolved_palette_color(0x00a8e66b, amber_dark_palette),
            base_palette_color(0x00a8e66b, 3)
        );
    }

    #[test]
    fn rectangle_writes_only_inside_active_clip() {
        let mut buffer = vec![0; 36];
        set_clip_rect(Some((2, 2, 2, 2)));
        rect(&mut buffer, 6, 6, 0, 0, 6, 6, 0x0000ff00);
        set_clip_rect(None);

        assert_eq!(buffer.iter().filter(|pixel| **pixel != 0).count(), 4);
        assert_eq!(buffer[2 * 6 + 2], themed_color(0x0000ff00));
        assert_eq!(buffer[1 * 6 + 2], 0);
        assert_eq!(buffer[2 * 6 + 1], 0);
    }

    #[test]
    fn page_scroll_translates_content_and_clips_above_the_navigation_rail() {
        set_clip_rect(None);
        set_page_vertical_layout(Some(80), Some((64, 64, 160, 96)));
        assert_eq!(map_page_vertical_span(128, 24), (48, 24));
        assert_eq!(page_y_to_logical(64.0, 80), 144.0);

        let mut buffer = vec![0; 20 * 20];
        rounded_rect(&mut buffer, 20, 20, 0, 120, 20, 20, 0, 0x0000ff00);
        assert_eq!(buffer.iter().filter(|pixel| **pixel != 0).count(), 0);

        set_page_vertical_layout(None, None);
    }

    #[test]
    fn page_layout_expands_the_content_after_the_navigation_rail() {
        assert_eq!(map_page_x(248, 1024), 64);
        assert_eq!(map_page_x(292, 1024), 88);
        assert_eq!(map_page_x(1024, 1024), 1024);
        assert_eq!(map_page_span(248, 776, 1024), (64, 960));
        for (screen_x, logical_x) in
            [(64.0, 248.0), (88.0, 292.0), (500.0, 616.0)]
        {
            let round_trip = page_x_to_logical(
                map_page_x(logical_x as usize, 1024) as f64,
                1024,
            );
            assert!((round_trip - logical_x).abs() < 1.0, "{screen_x}");
        }
    }

    #[test]
    fn fixed_settings_overlay_keeps_pointer_coordinates_aligned_with_drawn_controls()
     {
        assert_eq!(pointer_x_to_logical(500.0, 1024, true), 500.0);
        assert_eq!(
            pointer_x_to_logical(500.0, 1024, false),
            page_x_to_logical(500.0, 1024)
        );
    }

    #[test]
    fn navigation_play_glyph_matches_the_upstream_home_icon_without_a_ring() {
        set_active_palette(0);
        set_clip_rect(None);
        let (width, height, center_x, center_y) = (64, 64, 32, 32);
        let mut home = vec![0; width * height];
        draw_navigation_icon(
            &mut home, width, height, center_x, center_y, 9, 0x00f1f3f5,
        );
        let at = |buffer: &[u32], x: usize, y: usize| buffer[y * width + x];
        assert_ne!(at(&home, center_x - 5, center_y - 8), 0);
        assert_eq!(at(&home, center_x, center_y - 8), 0);

        let mut circular_action = vec![0; width * height];
        draw_navigation_icon(
            &mut circular_action,
            width,
            height,
            center_x,
            center_y,
            7,
            0x00f1f3f5,
        );
        assert_ne!(at(&circular_action, center_x, center_y - 8), 0);
    }
}
