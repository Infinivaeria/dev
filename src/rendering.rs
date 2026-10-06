//! Full-scene supersampling, with layout and input kept in window coordinates.

use std::ops::{Deref, DerefMut};

use raylib::{ffi, prelude::*};

const SAMPLE_SCALE: i32 = 2;

#[derive(Default)]
pub struct Renderer {
    target: Option<RenderTexture2D>,
}

impl Renderer {
    pub fn begin<'a, 'b>(
        &'b mut self,
        rl: &'a mut RaylibHandle,
        thread: &RaylibThread,
    ) -> Result<Frame<'a, 'b>, String> {
        let logical = (rl.get_screen_width().max(1), rl.get_screen_height().max(1));
        let physical = (rl.get_render_width().max(1), rl.get_render_height().max(1));
        let size = target_size(physical)?;
        if self
            .target
            .as_ref()
            .is_none_or(|target| (target.texture().width(), target.texture().height()) != size)
        {
            let target = rl
                .load_render_texture(thread, size.0 as u32, size.1 as u32)
                .map_err(|error| format!("Could not allocate anti-aliasing buffer: {error}"))?;
            // SAFETY: the window's GL context is current, and target owns
            // a live framebuffer. Check completeness as well as its IDs.
            if !target.is_render_texture_valid()
                || !unsafe { ffi::rlFramebufferComplete(target.id()) }
            {
                return Err(format!(
                    "Invalid {}x{} anti-aliasing buffer",
                    size.0, size.1
                ));
            }
            target
                .texture()
                .set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            self.target = Some(target);
        }
        let target = self.target.as_ref().expect("render target initialized");
        let draw = rl.begin_drawing(thread);
        // SAFETY: the draw handle owns the current GL context; Frame borrows
        // the target until EndTextureMode runs, before EndDrawing.
        unsafe {
            ffi::BeginTextureMode(*target.as_ref());
            projection(logical);
        }
        Ok(Frame {
            draw,
            target,
            logical,
            physical,
        })
    }
}

fn target_size((width, height): (i32, i32)) -> Result<(i32, i32), String> {
    let scale = |value: i32| {
        value
            .max(1)
            .checked_mul(SAMPLE_SCALE)
            .ok_or_else(|| "Window is too large for anti-aliasing".to_owned())
    };
    Ok((scale(width)?, scale(height)?))
}

// Scale via projection, not a 2D camera: EndMode3D restores this projection
// and overlays continue to use the same window coordinates afterwards.
unsafe fn projection((width, height): (i32, i32)) {
    ffi::rlMatrixMode(ffi::RL_PROJECTION as i32);
    ffi::rlLoadIdentity();
    ffi::rlOrtho(0.0, width as f64, height as f64, 0.0, 0.0, 1.0);
    ffi::rlMatrixMode(ffi::RL_MODELVIEW as i32);
    ffi::rlLoadIdentity();
}

pub struct Frame<'a, 'b> {
    draw: RaylibDrawHandle<'a>,
    target: &'b RenderTexture2D,
    logical: (i32, i32),
    physical: (i32, i32),
}

impl<'a> Deref for Frame<'a, '_> {
    type Target = RaylibDrawHandle<'a>;
    fn deref(&self) -> &Self::Target {
        &self.draw
    }
}

impl DerefMut for Frame<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.draw
    }
}

impl RaylibDraw for Frame<'_, '_> {}

impl Drop for Frame<'_, '_> {
    fn drop(&mut self) {
        // SAFETY: paired with BeginTextureMode in Renderer::begin; the owned
        // draw handle is dropped only after this resolve pass finishes.
        unsafe {
            ffi::EndTextureMode();
            // Refresh the default framebuffer viewport explicitly on every
            // frame, including maximize/restore and monitor DPI changes.
            ffi::rlViewport(0, 0, self.physical.0, self.physical.1);
            projection(self.logical);
        }
        self.draw.clear_background(Color::BLACK);
        self.draw.draw_texture_pro(
            self.target.texture(),
            Rectangle::new(
                0.0,
                0.0,
                self.target.texture().width() as f32,
                -self.target.texture().height() as f32,
            ),
            Rectangle::new(0.0, 0.0, self.logical.0 as f32, self.logical.1 as f32),
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );
    }
}

/// Scissor coordinates are framebuffer pixels, even when projection uses
/// logical window coordinates. Raylib handles DPI itself on the main buffer.
pub fn scissor_rect(rect: Rectangle) -> (i32, i32, i32, i32) {
    // SAFETY: called only during drawing with a live window/GL context.
    unsafe {
        let scale = if ffi::rlGetActiveFramebuffer() != 0 {
            (
                ffi::rlGetFramebufferWidth() as f32 / ffi::GetScreenWidth().max(1) as f32,
                ffi::rlGetFramebufferHeight() as f32 / ffi::GetScreenHeight().max(1) as f32,
            )
        } else {
            (1.0, 1.0)
        };
        scaled_scissor(rect, scale)
    }
}

fn scaled_scissor(rect: Rectangle, scale: (f32, f32)) -> (i32, i32, i32, i32) {
    let x = (rect.x * scale.0).floor() as i32;
    let y = (rect.y * scale.1).floor() as i32;
    let right = ((rect.x + rect.width) * scale.0).ceil() as i32;
    let bottom = ((rect.y + rect.height) * scale.1).ceil() as i32;
    (x, y, (right - x).max(0), (bottom - y).max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffers_follow_maximize_restore_and_minimize() {
        for (input, expected) in [
            ((960, 720), (1920, 1440)),
            ((4986, 2742), (9972, 5484)),
            ((960, 720), (1920, 1440)),
            ((0, 0), (2, 2)),
        ] {
            assert_eq!(target_size(input).unwrap(), expected);
        }
        assert!(target_size((i32::MAX, 720)).is_err());
    }

    #[test]
    fn clipping_scales_both_edges_without_losing_fractional_pixels() {
        assert_eq!(
            scaled_scissor(Rectangle::new(10.25, 20.5, 30.5, 40.25), (2.0, 3.0)),
            (20, 61, 62, 122)
        );
        assert_eq!(
            scaled_scissor(Rectangle::new(10.0, 20.0, 30.0, 40.0), (1.0, 1.0)),
            (10, 20, 30, 40)
        );
    }
}
