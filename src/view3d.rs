//! 3D grid view: an orbit camera that looks at the grid lying on the
//! ground plane, plus ray picking and textured tile drawing.
//!
//! Cell `(col, row)` occupies the unit square `x ∈ [col, col+1]`,
//! `z ∈ [row, row+1]` on the `y = 0` plane, so rows grow towards the viewer
//! just like they grow downwards in the 2D view.

use raylib::prelude::*;

const MIN_PITCH: f32 = 0.10;
const MAX_PITCH: f32 = 1.55;
const MIN_DISTANCE: f32 = 1.5;
const MAX_DISTANCE: f32 = 600.0;
/// Cells further than this (in cells) from the orbit target are not drawn.
pub const MAX_RADIUS: i64 = 64;
const ROTATE_SPEED: f32 = 0.008;
const RL_QUADS: i32 = 0x0007;

#[derive(Clone, Debug)]
pub struct Orbit {
    target: Vector3,
    goal: Vector3,
    /// Rotation around the vertical axis, radians.
    pub yaw: f32,
    /// Elevation above the ground plane, radians.
    pub pitch: f32,
    pub distance: f32,
    pub fovy: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        let center = Vector3::new(0.5, 0.0, 0.5);
        Self {
            target: center,
            goal: center,
            yaw: 0.6,
            pitch: 0.85,
            distance: 14.0,
            fovy: 45.0,
        }
    }
}

impl Orbit {
    /// A fresh view of the origin that keeps the current viewing angles.
    pub fn recentered(&self) -> Self {
        Self {
            yaw: self.yaw,
            pitch: self.pitch,
            ..Self::default()
        }
    }

    pub fn target(&self) -> Vector3 {
        self.target
    }

    pub fn position(&self) -> Vector3 {
        let horizontal = self.pitch.cos() * self.distance;
        Vector3::new(
            self.target.x + horizontal * self.yaw.sin(),
            self.target.y + self.pitch.sin() * self.distance,
            self.target.z + horizontal * self.yaw.cos(),
        )
    }

    pub fn camera(&self) -> Camera3D {
        Camera3D::perspective(
            self.position(),
            self.target,
            Vector3::new(0.0, 1.0, 0.0),
            self.fovy,
        )
    }

    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw = (self.yaw - dx * ROTATE_SPEED).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + dy * ROTATE_SPEED).clamp(MIN_PITCH, MAX_PITCH);
    }

    pub fn rotate_by(&mut self, yaw: f32, pitch: f32) {
        self.yaw = (self.yaw + yaw).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + pitch).clamp(MIN_PITCH, MAX_PITCH);
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn set_angles(&mut self, yaw_degrees: f64, pitch_degrees: f64, distance: f64) {
        self.yaw = (yaw_degrees as f32)
            .to_radians()
            .rem_euclid(std::f32::consts::TAU);
        self.pitch = (pitch_degrees as f32)
            .to_radians()
            .clamp(MIN_PITCH, MAX_PITCH);
        if distance > 0.0 {
            self.distance = (distance as f32).clamp(MIN_DISTANCE, MAX_DISTANCE);
        }
    }

    /// Drag the ground plane by a mouse delta so the grid follows the cursor.
    pub fn pan(&mut self, dx: f32, dy: f32, screen_height: f32) {
        let units_per_pixel =
            2.0 * self.distance * (self.fovy.to_radians() / 2.0).tan() / screen_height.max(1.0);
        let right = Vector3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let forward = Vector3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        // Looking down steeply, a vertical drag covers less ground.
        let vertical = units_per_pixel / self.pitch.sin().max(0.35);
        let shift = right * (-dx * units_per_pixel) + forward * (dy * vertical);
        self.target += shift;
        self.goal += shift;
    }

    pub fn zoom(&mut self, wheel: f32) {
        let factor = (1.0 - wheel * 0.12).clamp(0.5, 1.5);
        self.distance = (self.distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    /// Smoothly glide the orbit target to the centre of a cell.
    pub fn focus(&mut self, col: i64, row: i64) {
        self.goal = Vector3::new(col as f32 + 0.5, 0.0, row as f32 + 0.5);
    }

    pub fn animate(&mut self, dt: f32) {
        let t = (dt * 10.0).clamp(0.0, 1.0);
        self.target = self.target.lerp(self.goal, t);
    }

    pub fn center_cell(&self) -> (i64, i64) {
        (self.target.x.floor() as i64, self.target.z.floor() as i64)
    }

    /// Inclusive `(min_col, max_col, min_row, max_row)` around the target,
    /// wide enough to reach the horizon at the current zoom.
    pub fn visible_span(&self) -> (i64, i64, i64, i64) {
        let reach = self.distance * (1.0 + 1.6 / self.pitch.sin().max(0.2));
        let radius = (reach.ceil() as i64 + 2).clamp(4, MAX_RADIUS);
        let (col, row) = self.center_cell();
        (col - radius, col + radius, row - radius, row + radius)
    }

    /// Approximate on-screen size in pixels of one cell at `point`.
    pub fn pixels_per_unit(&self, point: Vector3, screen_height: f32) -> f32 {
        let distance = (point - self.position()).length().max(0.01);
        screen_height / (2.0 * distance * (self.fovy.to_radians() / 2.0).tan())
    }
}

/// Where a ray hits the ground plane, as a cell coordinate.
pub fn pick_ground(ray: Ray) -> Option<(i64, i64)> {
    if ray.direction.y.abs() < 1e-6 {
        return None;
    }
    let t = -ray.position.y / ray.direction.y;
    if t < 0.0 {
        return None;
    }
    let hit = ray.position + ray.direction * t;
    Some((hit.x.floor() as i64, hit.z.floor() as i64))
}

/// Nearest raised block hit by the ray, falling back to the ground plane.
pub fn pick_cell(ray: Ray, blocks: impl Iterator<Item = (i64, i64, f32)>) -> Option<(i64, i64)> {
    let mut best: Option<(f32, (i64, i64))> = None;
    for (col, row, height) in blocks {
        let (x, z) = (col as f32, row as f32);
        let bounds = BoundingBox::new(
            Vector3::new(x + 0.04, 0.0, z + 0.04),
            Vector3::new(x + 0.96, height, z + 0.96),
        );
        if let Some(distance) = ray_box_distance(&ray, &bounds) {
            if best.is_none_or(|(current, _)| distance < current) {
                best = Some((distance, (col, row)));
            }
        }
    }
    best.map(|(_, cell)| cell).or_else(|| pick_ground(ray))
}

fn ray_box_distance(ray: &Ray, bounds: &BoundingBox) -> Option<f32> {
    let origin = [ray.position.x, ray.position.y, ray.position.z];
    let direction = [ray.direction.x, ray.direction.y, ray.direction.z];
    let min = [bounds.min.x, bounds.min.y, bounds.min.z];
    let max = [bounds.max.x, bounds.max.y, bounds.max.z];
    let (mut near, mut far) = (f32::NEG_INFINITY, f32::INFINITY);
    for axis in 0..3 {
        if direction[axis].abs() < 1e-8 {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return None;
            }
            continue;
        }
        let a = (min[axis] - origin[axis]) / direction[axis];
        let b = (max[axis] - origin[axis]) / direction[axis];
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far && far >= 0.0).then_some(near.max(0.0))
}

/// Draw `texture` lying flat at height `y`, fitted inside the given ground
/// rectangle with its aspect ratio preserved. The image's top edge faces
/// the lower row numbers, matching the 2D view.
pub fn draw_texture_flat<D: RaylibDraw3D>(
    _d: &mut D,
    texture: &Texture2D,
    x: f32,
    z: f32,
    size: f32,
    y: f32,
) {
    let (width, height) = (texture.width.max(1) as f32, texture.height.max(1) as f32);
    let scale = (size / width).min(size / height);
    let (w, h) = (width * scale, height * scale);
    let x0 = x + (size - w) / 2.0;
    let z0 = z + (size - h) / 2.0;
    let (x1, z1) = (x0 + w, z0 + h);
    // SAFETY: called inside an active 3D mode (enforced by the
    // `RaylibDraw3D` bound); rlgl immediate-mode calls are balanced and the
    // texture binding is reset afterwards.
    unsafe {
        raylib::ffi::rlSetTexture(texture.id);
        raylib::ffi::rlBegin(RL_QUADS);
        raylib::ffi::rlColor4ub(255, 255, 255, 255);
        raylib::ffi::rlNormal3f(0.0, 1.0, 0.0);
        raylib::ffi::rlTexCoord2f(0.0, 0.0);
        raylib::ffi::rlVertex3f(x0, y, z0);
        raylib::ffi::rlTexCoord2f(0.0, 1.0);
        raylib::ffi::rlVertex3f(x0, y, z1);
        raylib::ffi::rlTexCoord2f(1.0, 1.0);
        raylib::ffi::rlVertex3f(x1, y, z1);
        raylib::ffi::rlTexCoord2f(1.0, 0.0);
        raylib::ffi::rlVertex3f(x1, y, z0);
        raylib::ffi::rlEnd();
        raylib::ffi::rlSetTexture(0);
    }
}

/// Outline of a ground rectangle at height `y`.
pub fn draw_square_outline<D: RaylibDraw3D>(
    d: &mut D,
    x: f32,
    z: f32,
    size: f32,
    y: f32,
    color: Color,
) {
    let corners = [
        Vector3::new(x, y, z),
        Vector3::new(x + size, y, z),
        Vector3::new(x + size, y, z + size),
        Vector3::new(x, y, z + size),
    ];
    for index in 0..4 {
        d.draw_line3D(corners[index], corners[(index + 1) % 4], color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ray(from: Vector3, to: Vector3) -> Ray {
        Ray::new(from, (to - from).normalize())
    }

    #[test]
    fn picks_ground_cells_under_the_ray() {
        let hit = pick_ground(ray(
            Vector3::new(2.5, 10.0, -3.5),
            Vector3::new(2.5, 0.0, -3.5),
        ));
        assert_eq!(hit, Some((2, -4)));
        let up = ray(Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, 5.0, 0.0));
        assert_eq!(pick_ground(up), None);
    }

    #[test]
    fn raised_blocks_win_over_the_ground_behind_them() {
        // A shallow ray that would land on the ground at cell (3, 0) but
        // passes through the tall block in cell (1, 0) first.
        let r = ray(Vector3::new(-2.0, 1.0, 0.5), Vector3::new(4.0, 0.0, 0.5));
        assert_eq!(pick_ground(r), Some((4, 0)));
        assert_eq!(pick_cell(r, [(1, 0, 1.0)].into_iter()), Some((1, 0)));
        assert_eq!(pick_cell(r, [(1, 5, 1.0)].into_iter()), Some((4, 0)));
    }

    #[test]
    fn orbit_clamps_and_pans() {
        let mut orbit = Orbit::default();
        orbit.rotate(0.0, 10_000.0);
        assert!(orbit.pitch <= MAX_PITCH);
        orbit.zoom(100.0);
        assert!(orbit.distance >= MIN_DISTANCE);
        let before = orbit.target();
        orbit.pan(100.0, 0.0, 720.0);
        assert!((orbit.target() - before).length() > 0.0);
        assert!(orbit.position().y > orbit.target().y);
        orbit.focus(10, -4);
        for _ in 0..200 {
            orbit.animate(1.0 / 60.0);
        }
        assert_eq!(orbit.center_cell(), (10, -4));
        let (min_col, max_col, ..) = orbit.visible_span();
        assert!(min_col < 10 && max_col > 10 && max_col - min_col <= 2 * MAX_RADIUS);
    }
}
