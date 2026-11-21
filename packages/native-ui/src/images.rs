use std::{collections::HashMap, sync::Arc};

use image;

pub(super) const INSTANCE_ICON_CACHE_BUDGET: usize = 2 * 1024 * 1024;
pub(super) const BROWSE_IMAGE_CACHE_BUDGET: usize = 5 * 1024 * 1024;
pub(super) const SCREENSHOT_THUMBNAIL_CACHE_BUDGET: usize = 4 * 1024 * 1024;
pub(super) const SKIN_PREVIEW_CACHE_BUDGET: usize = 256 * 1024;

static IMAGE_DECODE_PERMITS: std::sync::LazyLock<Arc<tokio::sync::Semaphore>> =
    std::sync::LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(1)));

pub(super) async fn spawn_image_decode<F, T>(decode: F) -> Result<T, String>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let permit = Arc::clone(&IMAGE_DECODE_PERMITS)
        .acquire_owned()
        .await
        .map_err(|error| error.to_string())?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        decode()
    })
    .await
    .map_err(|error| error.to_string())
}

pub(super) fn cap_image_cache(
    cache: &mut HashMap<String, (u32, u32, Vec<u32>)>,
    keep: &str,
    byte_budget: usize,
) {
    let mut bytes = cache
        .values()
        .map(|(_, _, pixels)| {
            pixels.len().saturating_mul(std::mem::size_of::<u32>())
        })
        .sum::<usize>();
    while bytes > byte_budget {
        let Some(victim) =
            cache.keys().find(|key| key.as_str() != keep).cloned()
        else {
            break;
        };
        if let Some((_, _, pixels)) = cache.remove(&victim) {
            bytes = bytes.saturating_sub(
                pixels.len().saturating_mul(std::mem::size_of::<u32>()),
            );
        }
    }
}

fn composite_rgba_pixel(background: u32, foreground: u32) -> u32 {
    let alpha = (foreground >> 24) & 0xff;
    if alpha == 0xff {
        return foreground & 0x00ff_ffff;
    }
    if alpha == 0 {
        return background & 0x00ff_ffff;
    }
    let inverse_alpha = 0xff - alpha;
    let blend = |shift: u32| {
        let front = (foreground >> shift) & 0xffu32;
        let back = (background >> shift) & 0xffu32;
        (front * alpha + back * inverse_alpha + 127) / 255
    };
    (blend(16) << 16) | (blend(8) << 8) | blend(0)
}

pub(crate) fn decode_skin_texture(
    bytes: &[u8],
) -> Result<(u32, u32, Vec<u32>), String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| error.to_string())?
        .thumbnail(128, 128)
        .to_rgba8();
    let (width, height) = image.dimensions();
    let pixels = image
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

pub(crate) fn draw_scaled_image(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    image_width: u32,
    image_height: u32,
    pixels: &[u32],
    viewport_x: usize,
    viewport_y: usize,
    viewport_width: usize,
    viewport_height: usize,
) {
    let (viewport_x, viewport_width) =
        crate::render::map_page_layout_span(viewport_x, viewport_width);
    let (viewport_y, viewport_height) =
        crate::render::map_page_vertical_span(viewport_y, viewport_height);
    if image_width == 0
        || image_height == 0
        || viewport_width == 0
        || viewport_height == 0
    {
        return;
    }
    let scale = (viewport_width as f64 / f64::from(image_width))
        .min(viewport_height as f64 / f64::from(image_height));
    let drawn_width = (f64::from(image_width) * scale).round() as usize;
    let drawn_height = (f64::from(image_height) * scale).round() as usize;
    let origin_x = viewport_x + viewport_width.saturating_sub(drawn_width) / 2;
    let origin_y =
        viewport_y + viewport_height.saturating_sub(drawn_height) / 2;
    let clip = crate::render::active_clip_rect();
    for y in 0..drawn_height {
        let target_y = origin_y + y;
        if target_y >= buffer_height
            || clip.is_some_and(|c| target_y >= c.1.saturating_add(c.3))
        {
            break;
        }
        if clip.is_some_and(|c| target_y < c.1) {
            continue;
        }
        let source_y = y * image_height as usize / drawn_height.max(1);
        for x in 0..drawn_width {
            let target_x = origin_x + x;
            if target_x >= buffer_width
                || clip.is_some_and(|c| target_x >= c.0.saturating_add(c.2))
            {
                break;
            }
            if clip.is_some_and(|c| target_x < c.0) {
                continue;
            }
            let source_x = x * image_width as usize / drawn_width.max(1);
            if let Some(color) =
                pixels.get(source_y * image_width as usize + source_x)
            {
                let target = target_y * buffer_width + target_x;
                buffer[target] = composite_rgba_pixel(buffer[target], *color);
            }
        }
    }
}

pub(crate) fn draw_cover_image(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    image_width: u32,
    image_height: u32,
    pixels: &[u32],
    viewport_x: usize,
    viewport_y: usize,
    viewport_width: usize,
    viewport_height: usize,
) {
    let (viewport_x, viewport_width) =
        crate::render::map_page_layout_span(viewport_x, viewport_width);
    let (viewport_y, viewport_height) =
        crate::render::map_page_vertical_span(viewport_y, viewport_height);
    if image_width == 0
        || image_height == 0
        || viewport_width == 0
        || viewport_height == 0
    {
        return;
    }

    let viewport_ratio = viewport_width as f64 / viewport_height as f64;
    let image_ratio = f64::from(image_width) / f64::from(image_height);
    let (crop_x, crop_y, crop_width, crop_height) =
        if image_ratio > viewport_ratio {
            let crop_width =
                (f64::from(image_height) * viewport_ratio).round() as usize;
            (
                (image_width as usize).saturating_sub(crop_width) / 2,
                0,
                crop_width.max(1),
                image_height as usize,
            )
        } else {
            let crop_height =
                (f64::from(image_width) / viewport_ratio).round() as usize;
            (
                0,
                (image_height as usize).saturating_sub(crop_height) / 2,
                image_width as usize,
                crop_height.max(1),
            )
        };
    let clip = crate::render::active_clip_rect();
    for y in 0..viewport_height {
        let target_y = viewport_y + y;
        if target_y >= buffer_height
            || clip.is_some_and(|c| target_y >= c.1.saturating_add(c.3))
        {
            break;
        }
        if clip.is_some_and(|c| target_y < c.1) {
            continue;
        }
        let source_y = crop_y + y * crop_height / viewport_height;
        for x in 0..viewport_width {
            let target_x = viewport_x + x;
            if target_x >= buffer_width
                || clip.is_some_and(|c| target_x >= c.0.saturating_add(c.2))
            {
                break;
            }
            if clip.is_some_and(|c| target_x < c.0) {
                continue;
            }
            let source_x = crop_x + x * crop_width / viewport_width;
            if let Some(color) =
                pixels.get(source_y * image_width as usize + source_x)
            {
                let target = target_y * buffer_width + target_x;
                buffer[target] = composite_rgba_pixel(buffer[target], *color);
            }
        }
    }
}

pub(crate) fn draw_contained_image_zoomed(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    image_width: u32,
    image_height: u32,
    pixels: &[u32],
    viewport_x: usize,
    viewport_y: usize,
    viewport_width: usize,
    viewport_height: usize,
    zoom: f32,
) {
    let (viewport_x, viewport_width) =
        crate::render::map_page_layout_span(viewport_x, viewport_width);
    let (viewport_y, viewport_height) =
        crate::render::map_page_vertical_span(viewport_y, viewport_height);
    if image_width == 0
        || image_height == 0
        || viewport_width == 0
        || viewport_height == 0
    {
        return;
    }
    let scale = (viewport_width as f64 / f64::from(image_width))
        .min(viewport_height as f64 / f64::from(image_height))
        * f64::from(zoom.clamp(1.0, 5.0));
    let drawn_width =
        (f64::from(image_width) * scale).round().max(1.0) as isize;
    let drawn_height =
        (f64::from(image_height) * scale).round().max(1.0) as isize;
    let origin_x =
        viewport_x as isize + (viewport_width as isize - drawn_width) / 2;
    let origin_y =
        viewport_y as isize + (viewport_height as isize - drawn_height) / 2;
    let clip = crate::render::active_clip_rect();
    for y in 0..viewport_height {
        let target_y = viewport_y + y;
        if target_y >= buffer_height
            || clip.is_some_and(|c| target_y >= c.1.saturating_add(c.3))
        {
            break;
        }
        if clip.is_some_and(|c| target_y < c.1) {
            continue;
        }
        let source_y = target_y as isize - origin_y;
        if !(0..drawn_height).contains(&source_y) {
            continue;
        }
        let source_y =
            source_y as usize * image_height as usize / drawn_height as usize;
        for x in 0..viewport_width {
            let target_x = viewport_x + x;
            if target_x >= buffer_width
                || clip.is_some_and(|c| target_x >= c.0.saturating_add(c.2))
            {
                break;
            }
            if clip.is_some_and(|c| target_x < c.0) {
                continue;
            }
            let source_x = target_x as isize - origin_x;
            if !(0..drawn_width).contains(&source_x) {
                continue;
            }
            let source_x =
                source_x as usize * image_width as usize / drawn_width as usize;
            if let Some(color) =
                pixels.get(source_y * image_width as usize + source_x)
            {
                let target = target_y * buffer_width + target_x;
                buffer[target] = composite_rgba_pixel(buffer[target], *color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::composite_rgba_pixel;

    #[test]
    fn rgba_pixels_composite_on_the_existing_surface_color() {
        assert_eq!(composite_rgba_pixel(0x0012_3456, 0x00ff_0000), 0x0012_3456);
        assert_eq!(composite_rgba_pixel(0x0012_3456, 0xffff_0000), 0x00ff_0000);
        assert_eq!(composite_rgba_pixel(0x0020_4060, 0x80c0_8040), 0x0070_6050);
    }
}

type SkinUvRect = (f32, f32, f32, f32);

fn skin_uv(x: f32, y: f32, width: f32, height: f32) -> SkinUvRect {
    (x, y, width, height)
}

pub(crate) fn draw_minecraft_skin_model(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    texture_width: u32,
    texture_height: u32,
    texture: &[u32],
    viewport_x: usize,
    viewport_y: usize,
    viewport_width: usize,
    viewport_height: usize,
    rotation: f32,
    variant: theseus::minecraft_skins::MinecraftSkinVariant,
) {
    let (viewport_x, viewport_width) =
        crate::render::map_page_layout_span(viewport_x, viewport_width);
    let (viewport_y, viewport_height) =
        crate::render::map_page_vertical_span(viewport_y, viewport_height);
    if texture_width == 0
        || texture_height == 0
        || viewport_width == 0
        || viewport_height == 0
    {
        return;
    }
    let mut depth = vec![f32::NEG_INFINITY; viewport_width * viewport_height];
    let classic = [
        skin_uv(8.0, 8.0, 8.0, 8.0),
        skin_uv(24.0, 8.0, 8.0, 8.0),
        skin_uv(0.0, 8.0, 8.0, 8.0),
        skin_uv(16.0, 8.0, 8.0, 8.0),
        skin_uv(8.0, 0.0, 8.0, 8.0),
        skin_uv(16.0, 0.0, 8.0, 8.0),
    ];
    draw_skin_cuboid(
        buffer,
        buffer_width,
        buffer_height,
        &mut depth,
        viewport_x,
        viewport_y,
        viewport_width,
        viewport_height,
        texture_width,
        texture_height,
        texture,
        rotation,
        [-4.0, 24.0, -4.0],
        [4.0, 32.0, 4.0],
        classic,
    );
    let head_overlay = [
        skin_uv(32.0, 8.0, 8.0, 8.0),
        skin_uv(56.0, 8.0, 8.0, 8.0),
        skin_uv(32.0, 8.0, 8.0, 8.0),
        skin_uv(48.0, 8.0, 8.0, 8.0),
        skin_uv(40.0, 0.0, 8.0, 8.0),
        skin_uv(48.0, 0.0, 8.0, 8.0),
    ];
    draw_skin_cuboid(
        buffer,
        buffer_width,
        buffer_height,
        &mut depth,
        viewport_x,
        viewport_y,
        viewport_width,
        viewport_height,
        texture_width,
        texture_height,
        texture,
        rotation,
        [-4.35, 24.0, -4.35],
        [4.35, 32.35, 4.35],
        head_overlay,
    );
    let torso = [
        skin_uv(20.0, 20.0, 8.0, 12.0),
        skin_uv(32.0, 20.0, 8.0, 12.0),
        skin_uv(16.0, 20.0, 4.0, 12.0),
        skin_uv(28.0, 20.0, 4.0, 12.0),
        skin_uv(20.0, 16.0, 8.0, 4.0),
        skin_uv(28.0, 16.0, 8.0, 4.0),
    ];
    draw_skin_cuboid(
        buffer,
        buffer_width,
        buffer_height,
        &mut depth,
        viewport_x,
        viewport_y,
        viewport_width,
        viewport_height,
        texture_width,
        texture_height,
        texture,
        rotation,
        [-4.0, 12.0, -2.0],
        [4.0, 24.0, 2.0],
        torso,
    );
    let torso_overlay = [
        skin_uv(20.0, 36.0, 8.0, 12.0),
        skin_uv(32.0, 36.0, 8.0, 12.0),
        skin_uv(16.0, 36.0, 4.0, 12.0),
        skin_uv(28.0, 36.0, 4.0, 12.0),
        skin_uv(20.0, 32.0, 8.0, 4.0),
        skin_uv(28.0, 32.0, 8.0, 4.0),
    ];
    draw_skin_cuboid(
        buffer,
        buffer_width,
        buffer_height,
        &mut depth,
        viewport_x,
        viewport_y,
        viewport_width,
        viewport_height,
        texture_width,
        texture_height,
        texture,
        rotation,
        [-4.25, 12.0, -2.25],
        [4.25, 24.25, 2.25],
        torso_overlay,
    );
    let slim = matches!(
        variant,
        theseus::minecraft_skins::MinecraftSkinVariant::Slim
    );
    let arm_width = if slim { 3.0 } else { 4.0 };
    let right_arm = [
        skin_uv(44.0, 20.0, 4.0, 12.0),
        skin_uv(52.0, 20.0, 4.0, 12.0),
        skin_uv(40.0, 20.0, 4.0, 12.0),
        skin_uv(48.0, 20.0, 4.0, 12.0),
        skin_uv(44.0, 16.0, 4.0, 4.0),
        skin_uv(48.0, 16.0, 4.0, 4.0),
    ];
    let left_arm = [
        skin_uv(36.0, 52.0, 4.0, 12.0),
        skin_uv(44.0, 52.0, 4.0, 12.0),
        skin_uv(32.0, 52.0, 4.0, 12.0),
        skin_uv(40.0, 52.0, 4.0, 12.0),
        skin_uv(36.0, 48.0, 4.0, 4.0),
        skin_uv(40.0, 48.0, 4.0, 4.0),
    ];
    for (x0, x1, uv) in [
        (4.0, 4.0 + arm_width, right_arm),
        (-4.0 - arm_width, -4.0, left_arm),
    ] {
        draw_skin_cuboid(
            buffer,
            buffer_width,
            buffer_height,
            &mut depth,
            viewport_x,
            viewport_y,
            viewport_width,
            viewport_height,
            texture_width,
            texture_height,
            texture,
            rotation,
            [x0, 12.0, -2.0],
            [x1, 24.0, 2.0],
            uv,
        );
    }
    let right_leg = [
        skin_uv(4.0, 20.0, 4.0, 12.0),
        skin_uv(12.0, 20.0, 4.0, 12.0),
        skin_uv(0.0, 20.0, 4.0, 12.0),
        skin_uv(8.0, 20.0, 4.0, 12.0),
        skin_uv(4.0, 16.0, 4.0, 4.0),
        skin_uv(8.0, 16.0, 4.0, 4.0),
    ];
    let left_leg = [
        skin_uv(20.0, 52.0, 4.0, 12.0),
        skin_uv(28.0, 52.0, 4.0, 12.0),
        skin_uv(16.0, 52.0, 4.0, 12.0),
        skin_uv(24.0, 52.0, 4.0, 12.0),
        skin_uv(20.0, 48.0, 4.0, 4.0),
        skin_uv(24.0, 48.0, 4.0, 4.0),
    ];
    for (x0, x1, uv) in [(-4.0, 0.0, right_leg), (0.0, 4.0, left_leg)] {
        draw_skin_cuboid(
            buffer,
            buffer_width,
            buffer_height,
            &mut depth,
            viewport_x,
            viewport_y,
            viewport_width,
            viewport_height,
            texture_width,
            texture_height,
            texture,
            rotation,
            [x0, 0.0, -2.0],
            [x1, 12.0, 2.0],
            uv,
        );
    }
}

fn draw_skin_cuboid(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    depth: &mut [f32],
    viewport_x: usize,
    viewport_y: usize,
    viewport_width: usize,
    viewport_height: usize,
    texture_width: u32,
    texture_height: u32,
    texture: &[u32],
    rotation: f32,
    min: [f32; 3],
    max: [f32; 3],
    uv: [SkinUvRect; 6],
) {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let faces = [
        (
            [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            uv[0],
        ),
        (
            [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
            uv[1],
        ),
        (
            [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
            uv[2],
        ),
        (
            [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
            uv[3],
        ),
        (
            [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
            uv[4],
        ),
        (
            [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
            uv[5],
        ),
    ];
    for (points, atlas) in faces {
        draw_skin_face(
            buffer,
            buffer_width,
            buffer_height,
            depth,
            viewport_x,
            viewport_y,
            viewport_width,
            viewport_height,
            texture_width,
            texture_height,
            texture,
            rotation,
            points,
            atlas,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_skin_face(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    depth: &mut [f32],
    viewport_x: usize,
    viewport_y: usize,
    viewport_width: usize,
    viewport_height: usize,
    texture_width: u32,
    texture_height: u32,
    texture: &[u32],
    rotation: f32,
    points: [[f32; 3]; 4],
    (u, v, face_width, face_height): SkinUvRect,
) {
    let corners = [
        (u, v + face_height),
        (u + face_width, v + face_height),
        (u + face_width, v),
        (u, v),
    ];
    let angle_sin = rotation.sin();
    let angle_cos = rotation.cos();
    let pitch_sin = 0.12_f32;
    let pitch_cos = (1.0 - pitch_sin * pitch_sin).sqrt();
    let scale = viewport_width as f32 / 36.0;
    let center_x = viewport_width as f32 * 0.5;
    let center_y = viewport_height as f32 * 0.57;
    let vertices = std::array::from_fn::<_, 4, _>(|index| {
        let [x, y, z] = points[index];
        let rotated_x = x * angle_cos + z * angle_sin;
        let rotated_z = -x * angle_sin + z * angle_cos;
        let screen_y = -y * pitch_cos + rotated_z * pitch_sin;
        let depth = y * pitch_sin + rotated_z * pitch_cos;
        (
            center_x + rotated_x * scale,
            center_y + screen_y * scale,
            depth,
            corners[index].0,
            corners[index].1,
        )
    });
    raster_skin_triangle(
        buffer,
        buffer_width,
        buffer_height,
        depth,
        viewport_x,
        viewport_y,
        viewport_width,
        viewport_height,
        texture_width,
        texture_height,
        texture,
        [vertices[0], vertices[1], vertices[2]],
    );
    raster_skin_triangle(
        buffer,
        buffer_width,
        buffer_height,
        depth,
        viewport_x,
        viewport_y,
        viewport_width,
        viewport_height,
        texture_width,
        texture_height,
        texture,
        [vertices[0], vertices[2], vertices[3]],
    );
}

#[allow(clippy::too_many_arguments)]
fn raster_skin_triangle(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    depth_buffer: &mut [f32],
    viewport_x: usize,
    viewport_y: usize,
    viewport_width: usize,
    viewport_height: usize,
    texture_width: u32,
    texture_height: u32,
    texture: &[u32],
    vertices: [(f32, f32, f32, f32, f32); 3],
) {
    let [
        (x0, y0, z0, u0, v0),
        (x1, y1, z1, u1, v1),
        (x2, y2, z2, u2, v2),
    ] = vertices;
    let area = (x1 - x0) * (y2 - y0) - (y1 - y0) * (x2 - x0);
    if area.abs() < 0.0001 {
        return;
    }
    let min_x = x0.min(x1).min(x2).floor().max(0.0) as usize;
    let max_x =
        x0.max(x1).max(x2).ceil().min(viewport_width as f32 - 1.0) as usize;
    let min_y = y0.min(y1).min(y2).floor().max(0.0) as usize;
    let max_y =
        y0.max(y1).max(y2).ceil().min(viewport_height as f32 - 1.0) as usize;
    if min_x > max_x || min_y > max_y {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = ((x1 - px) * (y2 - py) - (y1 - py) * (x2 - px)) / area;
            let w1 = ((x2 - px) * (y0 - py) - (y2 - py) * (x0 - px)) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                continue;
            }
            let pixel_depth = w0 * z0 + w1 * z1 + w2 * z2;
            let depth_index = y * viewport_width + x;
            if pixel_depth <= depth_buffer[depth_index] {
                continue;
            }
            let texture_x =
                ((w0 * u0 + w1 * u1 + w2 * u2) * texture_width as f32 / 64.0)
                    .floor()
                    .clamp(0.0, texture_width.saturating_sub(1) as f32)
                    as usize;
            let texture_y =
                ((w0 * v0 + w1 * v1 + w2 * v2) * texture_height as f32 / 64.0)
                    .floor()
                    .clamp(0.0, texture_height.saturating_sub(1) as f32)
                    as usize;
            if let Some(color) =
                texture.get(texture_y * texture_width as usize + texture_x)
            {
                let target_x = viewport_x + x;
                let target_y = viewport_y + y;
                if target_x < buffer_width && target_y < buffer_height {
                    buffer[target_y * buffer_width + target_x] = *color;
                    depth_buffer[depth_index] = pixel_depth;
                }
            }
        }
    }
}
