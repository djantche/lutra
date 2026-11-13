//! The picture: the core's frame uploaded as a texture and drawn at its true shape, centred,
//! with black bars around it. OpenGL rather than a pixel buffer, so cores that render with the
//! GPU (N64, PSP, Dreamcast...) can later draw into Lutra's context.

use glow::HasContext;

use super::libretro::{Frame, PixelFormat};
use crate::emulation::profile::{Effect, Scaling, VideoSettings};

const VERTEX: &str = r#"#version 330 core
out vec2 uv;
void main() {
    // Two triangles covering the viewport, without any vertex buffer.
    vec2 corner = vec2(float((gl_VertexID & 1) << 1), float(gl_VertexID & 2));
    uv = vec2(corner.x * 0.5, 1.0 - corner.y * 0.5);
    gl_Position = vec4(corner - 1.0, 0.0, 1.0);
}
"#;

const FRAGMENT: &str = r#"#version 330 core
in vec2 uv;
out vec4 color;
uniform sampler2D frame;
uniform vec2 source;
// 0: sharp pixels, 1: smoothed.
uniform int smoothing;
// 0: none, 1: scanlines, 2: LCD grid.
uniform int effect;
void main() {
    vec2 texel = uv * source;
    vec2 sample_at = uv;
    if (smoothing == 0) {
        // Sharp pixels that do not shimmer at non-integer scales: each texel is a flat
        // square, softened only on its edges.
        vec2 scale = max(fwidth(texel), vec2(1e-5));
        vec2 center = floor(texel) + 0.5;
        vec2 offset = clamp((texel - center) / scale, -0.5, 0.5) + 0.5;
        sample_at = (floor(texel) + offset) / source;
    }
    vec3 rgb = texture(frame, sample_at).rgb;
    vec2 inside = fract(texel) - 0.5;
    if (effect == 1) {
        // Bright in the middle of each row, dark between rows, as on a tube.
        rgb *= 1.0 - 0.45 * pow(abs(inside.y) * 2.0, 2.0);
        rgb *= 1.12;
    } else if (effect == 2) {
        // The thin gaps between a handheld screen's cells.
        vec2 edge = smoothstep(0.38, 0.5, abs(inside));
        rgb *= 1.0 - 0.28 * max(edge.x, edge.y);
    }
    color = vec4(rgb, 1.0);
}
"#;

pub struct Renderer {
    program: glow::Program,
    vertex_array: glow::VertexArray,
    texture: glow::Texture,
    source: Option<glow::UniformLocation>,
    smoothing: Option<glow::UniformLocation>,
    effect: Option<glow::UniformLocation>,
    size: (usize, usize),
    format: Option<PixelFormat>,
}

/// The rectangle of whole multiples of a `size` picture (shaped `aspect`) that fits a window,
/// centred: every console pixel gets the same number of screen pixels.
pub fn fit_integer(window: (u32, u32), size: (usize, usize), aspect: f32) -> (i32, i32, i32, i32) {
    // The console's rows set the scale; the width follows the picture's shape.
    let height = size.1.max(1) as f32;
    let shaped_width = height * aspect;
    let scale = (window.0 as f32 / shaped_width)
        .min(window.1 as f32 / height)
        .floor()
        .max(1.0);
    let (w, h) = (shaped_width * scale, height * scale);
    (
        ((window.0 as f32 - w) / 2.0).round() as i32,
        ((window.1 as f32 - h) / 2.0).round() as i32,
        w.round() as i32,
        h.round() as i32,
    )
}

/// The rectangle a picture of shape `aspect` takes in a window, centred, as large as fits.
pub fn fit(window: (u32, u32), aspect: f32) -> (i32, i32, i32, i32) {
    let (width, height) = (window.0 as f32, window.1 as f32);
    let (w, h) = if width / height > aspect {
        (height * aspect, height)
    } else {
        (width, width / aspect)
    };
    (
        ((width - w) / 2.0).round() as i32,
        ((height - h) / 2.0).round() as i32,
        w.round() as i32,
        h.round() as i32,
    )
}

fn gl_format(format: PixelFormat) -> (u32, u32, u32) {
    match format {
        PixelFormat::Xrgb8888 => (glow::RGB8, glow::BGRA, glow::UNSIGNED_INT_8_8_8_8_REV),
        PixelFormat::Rgb565 => (glow::RGB8, glow::RGB, glow::UNSIGNED_SHORT_5_6_5),
        PixelFormat::Rgb1555 => (glow::RGB8, glow::BGRA, glow::UNSIGNED_SHORT_1_5_5_5_REV),
    }
}

impl Renderer {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        // SAFETY: plain OpenGL calls on the current context.
        unsafe {
            let program = gl.create_program()?;
            for (kind, source) in [
                (glow::VERTEX_SHADER, VERTEX),
                (glow::FRAGMENT_SHADER, FRAGMENT),
            ] {
                let shader = gl.create_shader(kind)?;
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    return Err(gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }
            let vertex_array = gl.create_vertex_array()?;
            let texture = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for (parameter, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, value as i32);
            }
            let source = gl.get_uniform_location(program, "source");
            let smoothing = gl.get_uniform_location(program, "smoothing");
            let effect = gl.get_uniform_location(program, "effect");
            Ok(Self {
                program,
                vertex_array,
                texture,
                source,
                smoothing,
                effect,
                size: (0, 0),
                format: None,
            })
        }
    }

    /// Sends a new frame to the GPU.
    pub fn upload(&mut self, gl: &glow::Context, frame: &Frame) {
        if frame.width == 0 || frame.height == 0 {
            return;
        }
        let (internal, format, kind) = gl_format(frame.format);
        // SAFETY: `frame.pixels` holds `height` tightly packed rows of `width` pixels.
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            let pixels = glow::PixelUnpackData::Slice(Some(&frame.pixels));
            if self.size != (frame.width, frame.height) || self.format != Some(frame.format) {
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    internal as i32,
                    frame.width as i32,
                    frame.height as i32,
                    0,
                    format,
                    kind,
                    pixels,
                );
                self.size = (frame.width, frame.height);
                self.format = Some(frame.format);
            } else {
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    frame.width as i32,
                    frame.height as i32,
                    format,
                    kind,
                    pixels,
                );
            }
        }
    }

    /// Draws the last frame into a window of `window` pixels, at shape `aspect` (the console's),
    /// as `video` asks.
    pub fn draw(&self, gl: &glow::Context, window: (u32, u32), aspect: f32, video: &VideoSettings) {
        // SAFETY: plain OpenGL calls on the current context.
        unsafe {
            gl.viewport(0, 0, window.0 as i32, window.1 as i32);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            if self.size.0 == 0 {
                return;
            }
            let aspect = match video.aspect {
                crate::emulation::profile::Aspect::Console => aspect,
                crate::emulation::profile::Aspect::Square => {
                    self.size.0 as f32 / self.size.1 as f32
                }
                crate::emulation::profile::Aspect::Stretch => {
                    window.0 as f32 / window.1.max(1) as f32
                }
            };
            let (x, y, width, height) = if video.scaling == Scaling::Integer {
                fit_integer(window, self.size, aspect)
            } else {
                fit(window, aspect)
            };
            gl.viewport(x, y, width, height);
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(self.source.as_ref(), self.size.0 as f32, self.size.1 as f32);
            gl.uniform_1_i32(
                self.smoothing.as_ref(),
                i32::from(video.scaling == Scaling::Smooth),
            );
            gl.uniform_1_i32(
                self.effect.as_ref(),
                match video.effect {
                    Effect::None => 0,
                    Effect::Scanlines => 1,
                    Effect::Lcd => 2,
                },
            );
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_keeps_its_shape_with_bars_on_the_sides_or_above() {
        // A 3:2 Game Boy Advance picture on a 16:9 screen: bars left and right.
        assert_eq!(fit((1920, 1080), 1.5), (150, 0, 1620, 1080));
        // A 4:3 picture in a tall window: bars above and below.
        assert_eq!(fit((800, 1000), 4.0 / 3.0), (0, 200, 800, 600));
    }

    #[test]
    fn integer_scaling_gives_every_console_pixel_the_same_size() {
        // 240x160 on 1920x1080: 6 times (1440x960), centred.
        assert_eq!(
            fit_integer((1920, 1080), (240, 160), 1.5),
            (240, 60, 1440, 960)
        );
        // Never smaller than the console itself.
        assert_eq!(fit_integer((100, 100), (240, 160), 1.5).2, 240);
    }
}
