//! The 2D game, rebuilt as a low-poly diorama.
//! One world unit is one pixel. Gameplay stays on the original plane. Shapes keep
//! the 2D silhouette and palette, with real thickness, a key light, and a camera
//! pitched down and off to the side. No fog.

use super as sim;
use super::app::Sim;
use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::Face;
use bevy::render::view::Tonemapping;
use bevy::prelude::*;
use std::collections::HashMap;

pub(crate) const CAM_DIST: f32 = 1760.0;
const CAM_FOV_DEG: f32 = 28.0;
const CAM_PITCH_DEG: f32 = 22.0;
const CAM_YAW_DEG: f32 = 19.0;

const W: f32 = 1280.0;
const H: f32 = 720.0;

#[derive(Component)]
pub(crate) struct Scenery;

#[derive(Component)]
pub(crate) struct Cam;

#[derive(Resource)]
pub(crate) struct Gfx {
    cube: Handle<Mesh>,
    disk: Handle<Mesh>,
    ring: Handle<Mesh>,
    mats: HashMap<(u32, bool), Handle<StandardMaterial>>,
    built_map: u32,
}

#[derive(Resource, Default)]
pub(crate) struct FxPool {
    ents: Vec<Entity>,
    cursor: usize,
}

#[derive(Resource)]
pub(crate) struct TriPool {
    ents: Vec<Entity>,
    handles: Vec<Handle<Mesh>>,
    cursor: usize,
}

fn key_of(col: sim::Color, exact: bool) -> u32 {
    let c = if exact {
        col
    } else {
        let q = |v: u8| v & 0xF8;
        let a = if col.a > 248 { 255 } else { (col.a & 0xF0).max(16) };
        sim::Color::new(q(col.r), q(col.g), q(col.b), a)
    };
    u32::from(c.r) | (u32::from(c.g) << 8) | (u32::from(c.b) << 16) | (u32::from(c.a) << 24)
}

fn material(
    cache: &mut HashMap<(u32, bool), Handle<StandardMaterial>>,
    materials: &mut Assets<StandardMaterial>,
    col: sim::Color,
    exact: bool,
    lit: bool,
) -> Handle<StandardMaterial> {
    let k = key_of(col, exact);
    let lit = lit && col.a >= 250;
    if let Some(h) = cache.get(&(k, lit)) {
        return h.clone();
    }
    let (r, g, b, a) = ((k & 255) as u8, ((k >> 8) & 255) as u8, ((k >> 16) & 255) as u8, (k >> 24) as u8);
    let lin = LinearRgba::from(Color::srgba_u8(r, g, b, a));
    // A little of the base hue in the shadows, so the key light shades faces
    // without pulling the palette toward gray.
    let emissive = if lit {
        LinearRgba::rgb(lin.red * 0.16, lin.green * 0.16, lin.blue * 0.16)
    } else {
        LinearRgba::BLACK
    };
    let h = materials.add(StandardMaterial {
        base_color: Color::srgba_u8(r, g, b, a),
        emissive,
        perceptual_roughness: 0.86,
        metallic: 0.0,
        reflectance: 0.04,
        unlit: !lit,
        alpha_mode: if a < 250 { AlphaMode::Blend } else { AlphaMode::Opaque },
        // Outward faces only. The hidden back of a slab was landing on the next
        // surface and flickering with it.
        cull_mode: Some(Face::Back),
        ..default()
    });
    cache.insert((k, lit), h.clone());
    h
}

/// Push distant scenery back so the angled camera gets real parallax.
/// Offsets of a few units inside one layer stay small; the sky sits much farther.
fn layer(z: f32) -> f32 {
    if z >= -45.0 {
        z
    } else if z >= -130.0 {
        -45.0 + (z + 45.0) * 2.2
    } else {
        let mid = -45.0 + (-130.0 + 45.0) * 2.2;
        mid + (z + 130.0) * 10.0
    }
}

fn place(front: f32, depth: f32) -> f32 {
    layer(front) - depth * 0.5
}

fn vsub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn vcross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn vdot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn vnorm(a: [f32; 3]) -> [f32; 3] {
    let len = vdot(a, a).sqrt().max(1.0e-6);
    [a[0] / len, a[1] / len, a[2] / len]
}

fn prism(a: Vec2, b: Vec2, c: Vec2, front: f32, depth: f32) -> Mesh {
    let f = |p: Vec2, z: f32| [p.x - W * 0.5, H * 0.5 - p.y, z];
    let z1 = layer(front);
    let z0 = z1 - depth;
    // Screen y is down, so the 2D winding flips in world y. a,c,b faces the camera (+Z).
    let mut pos = vec![f(a, z1), f(c, z1), f(b, z1), f(a, z0), f(b, z0), f(c, z0)];
    let mut nrm = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0]];
    let mut idx = vec![0, 1, 2, 3, 4, 5];
    let cx = (a.x + b.x + c.x) / 3.0;
    let cy = (a.y + b.y + c.y) / 3.0;
    let centroid = f(Vec2::new(cx, cy), (z0 + z1) * 0.5);
    let pts = [a, b, c];
    for i in 0..3 {
        let j = (i + 1) % 3;
        let base = pos.len() as u32;
        let p0 = f(pts[i], z1);
        let p1 = f(pts[j], z1);
        let p2 = f(pts[j], z0);
        let p3 = f(pts[i], z0);
        pos.push(p0);
        pos.push(p1);
        pos.push(p2);
        pos.push(p3);
        let mut n = vnorm(vcross(vsub(p1, p0), vsub(p2, p0)));
        let mid = [(p0[0] + p2[0]) * 0.5, (p0[1] + p2[1]) * 0.5, (p0[2] + p2[2]) * 0.5];
        if vdot(n, vsub(mid, centroid)) < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        for _ in 0..4 {
            nrm.push(n);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let uvs = vec![[0.0, 0.0]; pos.len()];
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nrm);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(idx));
    mesh
}

fn blank_mesh() -> Mesh {
    prism(Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0), 0.0, 1.0)
}

struct Ink<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    mats: &'a mut HashMap<(u32, bool), Handle<StandardMaterial>>,
    materials: &'a mut Assets<StandardMaterial>,
    meshes: &'a mut Assets<Mesh>,
    cube: Handle<Mesh>,
    disk: Handle<Mesh>,
    ring: Handle<Mesh>,
    pool: Option<&'a mut FxPool>,
    tris: Option<&'a mut TriPool>,
}

impl Ink<'_, '_, '_> {
    fn put(
        &mut self,
        mesh: Handle<Mesh>,
        cx: f32,
        cy: f32,
        center_z: f32,
        sx: f32,
        sy: f32,
        sz: f32,
        rot: Quat,
        col: sim::Color,
        exact: bool,
        front: f32,
    ) {
        if sx < 0.2 || sy < 0.2 || sz < 0.2 || col.a == 0 {
            return;
        }
        let lit = front > -210.0;
        let mat = material(self.mats, self.materials, col, exact, lit);
        let tf = Transform {
            translation: Vec3::new(cx - W * 0.5, H * 0.5 - cy, center_z),
            rotation: rot,
            scale: Vec3::new(sx, sy, sz),
        };
        let opaque = col.a >= 250;
        let cast = opaque && exact && sz >= 16.0 && front > -30.0;
        let receive = opaque && front > -80.0;
        let spawn = |commands: &mut Commands, extra: Option<Scenery>| {
            let e = match extra {
                Some(tag) => commands.spawn((tag, Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), tf, Visibility::Visible)).id(),
                None => commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), tf, Visibility::Visible)).id(),
            };
            e
        };
        let e = if let Some(pool) = self.pool.as_mut() {
            if pool.cursor >= pool.ents.len() {
                let e = spawn(self.commands, None);
                pool.ents.push(e);
                pool.cursor += 1;
                e
            } else {
                let e = pool.ents[pool.cursor];
                pool.cursor += 1;
                self.commands
                    .entity(e)
                    .insert((Mesh3d(mesh), MeshMaterial3d(mat), tf, Visibility::Visible));
                e
            }
        } else {
            spawn(self.commands, Some(Scenery))
        };
        if cast {
            self.commands.entity(e).remove::<NotShadowCaster>();
        } else {
            self.commands.entity(e).insert(NotShadowCaster);
        }
        if receive {
            self.commands.entity(e).remove::<NotShadowReceiver>();
        } else {
            self.commands.entity(e).insert(NotShadowReceiver);
        }
    }

    /// `z` is the front face, toward the camera. Thickness extends back into the scene.
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, z: f32, depth: f32, col: sim::Color) {
        self.rect_proud(x, y, w, h, z, depth, 0.0, col);
    }

    /// `proud` pushes the front face toward the camera in world units, after parallax.
    fn rect_proud(&mut self, x: f32, y: f32, w: f32, h: f32, z: f32, depth: f32, proud: f32, col: sim::Color) {
        let front = layer(z) + proud;
        self.put(
            self.cube.clone(),
            x + w * 0.5,
            y + h * 0.5,
            front - depth * 0.5,
            w.abs(),
            h.abs(),
            depth,
            Quat::IDENTITY,
            col,
            true,
            z,
        );
    }

    fn rect_rot(&mut self, cx: f32, cy: f32, w: f32, h: f32, rot_deg: f32, z: f32, depth: f32, col: sim::Color, exact: bool) {
        let rot = Quat::from_rotation_z(-rot_deg.to_radians());
        let front = layer(z);
        self.put(self.cube.clone(), cx, cy, front - depth * 0.5, w.abs(), h.abs(), depth, rot, col, exact, z);
    }

    fn disk(&mut self, cx: f32, cy: f32, r: f32, z: f32, col: sim::Color, exact: bool) {
        self.disk_proud(cx, cy, r, z, 0.0, col, exact);
    }

    fn disk_proud(&mut self, cx: f32, cy: f32, r: f32, z: f32, proud: f32, col: sim::Color, exact: bool) {
        if r < 0.4 {
            return;
        }
        let d = (r * 2.0).max(0.8);
        let thick = if col.a < 250 {
            (r * 0.14).clamp(3.0, 16.0)
        } else {
            (r * 0.5).clamp(8.0, 64.0)
        };
        let front = layer(z) + proud;
        let rot = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        self.put(self.disk.clone(), cx, cy, front - thick * 0.5, d, thick, d, rot, col, exact, z);
    }

    fn ellipse(&mut self, cx: f32, cy: f32, rh: f32, rv: f32, z: f32, col: sim::Color) {
        let thick = if col.a < 250 {
            6.0
        } else {
            (rh.min(rv) * 0.42).clamp(12.0, 120.0)
        };
        let rot = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let front = layer(z);
        self.put(
            self.disk.clone(),
            cx,
            cy,
            front - thick * 0.5,
            (rh * 2.0).max(0.8),
            thick,
            (rv * 2.0).max(0.8),
            rot,
            col,
            true,
            z,
        );
    }

    fn ring(&mut self, cx: f32, cy: f32, r: f32, z: f32, col: sim::Color) {
        if r < 1.0 {
            return;
        }
        let rot = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        // The torus tube is 0.035 of the outer radius. Stretch it along local Y
        // (world Z after the tilt) so the ring has thickness without a fatter stroke.
        let tube = (r * 0.2).clamp(5.0, 22.0);
        let y = tube / 0.035;
        self.put(self.ring.clone(), cx, cy, place(z, tube), r, y, r, rot, col, true, z);
    }

    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thick: f32, z: f32, col: sim::Color) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt().max(0.5);
        let ang = dy.atan2(dx).to_degrees();
        let depth = (thick * 2.2).clamp(6.0, 36.0);
        self.rect_rot((x0 + x1) * 0.5, (y0 + y1) * 0.5, len, thick.max(1.0), ang, z, depth, col, true);
    }

    fn frame(&mut self, x: f32, y: f32, w: f32, h: f32, t: f32, z: f32, col: sim::Color) {
        self.frame_proud(x, y, w, h, t, z, 0.0, col);
    }

    fn frame_proud(&mut self, x: f32, y: f32, w: f32, h: f32, t: f32, z: f32, proud: f32, col: sim::Color) {
        let d = 5.0;
        self.rect_proud(x, y, w, t, z, d, proud, col);
        self.rect_proud(x, y + h - t, w, t, z, d, proud, col);
        self.rect_proud(x, y, t, h, z, d, proud, col);
        self.rect_proud(x + w - t, y, t, h, z, d, proud, col);
    }

    fn grad(&mut self, x: f32, y: f32, w: f32, h: f32, top: sim::Color, bot: sim::Color, bands: i32, z: f32) {
        let (x, y, w, h) = if z <= -180.0 {
            // Big enough that the angled camera, including the victory orbit, never sees past it.
            self.rect(-4000.0, -4200.0, 12000.0, 10400.0, z - 4.0, 12.0, bot);
            (-3600.0, -3800.0, 11000.0, 9600.0)
        } else {
            (x, y, w, h)
        };
        let bands = bands.max(1);
        for i in 0..bands {
            let tm = (i as f32 + 0.5) / bands as f32;
            let col = lerp(top, bot, tm);
            let y0 = y + h * (i as f32) / bands as f32;
            // A small overlap hides cracks. Each band sits clearly in front of the
            // one behind it, so the overlap cannot shimmer.
            self.rect(x, y0, w, h / bands as f32 + 0.6, z + i as f32 * 2.5, 8.0, col);
        }
    }

    fn tri(&mut self, ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32, z: f32, col: sim::Color) {
        if col.a == 0 {
            return;
        }
        let span = ((ax - bx).hypot(ay - by)).max((bx - cx).hypot(by - cy)).max((cx - ax).hypot(cy - ay));
        let depth = if col.a < 250 { 8.0 } else { (span * 0.26).clamp(12.0, 160.0) };
        let mesh = prism(Vec2::new(ax, ay), Vec2::new(bx, by), Vec2::new(cx, cy), z, depth);
        let mat = material(self.mats, self.materials, col, true, z > -210.0);
        let tf = Transform::IDENTITY;
        let e = if let Some(tris) = self.tris.as_mut() {
            if tris.cursor >= tris.ents.len() {
                let handle = self.meshes.add(mesh);
                let e = self
                    .commands
                    .spawn((Mesh3d(handle.clone()), MeshMaterial3d(mat), tf, Visibility::Visible))
                    .id();
                tris.handles.push(handle);
                tris.ents.push(e);
                tris.cursor += 1;
                e
            } else {
                let i = tris.cursor;
                tris.cursor += 1;
                let handle = tris.handles[i].clone();
                let e = tris.ents[i];
                if let Some(mut existing) = self.meshes.get_mut(&handle) {
                    *existing = mesh;
                }
                self.commands
                    .entity(e)
                    .insert((Mesh3d(handle), MeshMaterial3d(mat), tf, Visibility::Visible));
                e
            }
        } else {
            let handle = self.meshes.add(mesh);
            self.commands
                .spawn((Scenery, Mesh3d(handle), MeshMaterial3d(mat), tf, Visibility::Visible))
                .id()
        };
        // Prisms are scenery and effects. Fighters and platforms carry the shadows.
        self.commands.entity(e).insert(NotShadowCaster);
        if z > -80.0 && col.a >= 250 {
            self.commands.entity(e).remove::<NotShadowReceiver>();
        } else {
            self.commands.entity(e).insert(NotShadowReceiver);
        }
    }
}

fn lerp(a: sim::Color, b: sim::Color, t: f32) -> sim::Color {
    let m = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * t).round().clamp(0.0, 255.0) as u8;
    sim::Color::new(m(a.r, b.r), m(a.g, b.g), m(a.b, b.b), m(a.a, b.a))
}

fn c(r: u8, g: u8, b: u8, a: u8) -> sim::Color {
    sim::Color::new(r, g, b, a)
}

fn knife(ink: &mut Ink, px: f32, py: f32, angle: f32, facing: f32, scale: f32, z: f32) {
    let wood = c(150, 90, 40, 255);
    let dark = c(100, 55, 25, 255);
    let steel = c(190, 198, 205, 255);
    let parts = [
        (-6.0, -24.0, 6.0, 2.0, sim::Color::BLACK),
        (-4.0, -22.0, 4.0, 0.0, wood),
        (-12.0, -30.0, 12.0, -22.0, sim::Color::BLACK),
        (-10.0, -28.0, 10.0, -24.0, dark),
        (-8.0, -80.0, 8.0, -28.0, sim::Color::BLACK),
        (-5.0, -92.0, 5.0, -80.0, sim::Color::BLACK),
        (-5.0, -78.0, 5.0, -30.0, steel),
        (1.0, -78.0, 5.0, -30.0, sim::Color::WHITE),
        (-3.0, -90.0, 3.0, -78.0, steel),
    ];
    let (s, co) = angle.to_radians().sin_cos();
    for (i, (x0, y0, x1, y1, col)) in parts.iter().enumerate() {
        let (w, h) = ((x1 - x0) * scale, (y1 - y0) * scale);
        let lx = (x0 + x1) * 0.5 * facing * scale;
        let ly = (y0 + y1) * 0.5 * scale;
        let cx = px + lx * co - ly * s;
        let cy = py + lx * s + ly * co;
        // The white strip is the cutting edge: thinner, and proud of the blade, so the side reads as a bevel.
        let thick = (18.0 * scale).clamp(8.0, 96.0);
        let edge = i == 7;
        let depth = if edge { thick * 0.34 } else { thick };
        // Each part sits just in front of the one under it. The white edge is
        // proud of the steel so the side of the blade reads as a bevel.
        let front = z + i as f32 * 1.4 + if edge { thick * 0.36 } else { 0.0 };
        ink.rect_rot(cx, cy, w, h, angle, front, depth, *col, true);
    }
}

fn draw_player(ink: &mut Ink, p: &sim::Player, t: f32) {
    let w = sim::SIZE;
    let h = (sim::SIZE - p.crouch).max(8.0);
    let cx = p.x + sim::SIZE * 0.5;
    let cy = p.y + sim::SIZE - h * 0.5;
    let body = if p.hurt_t > 0.0 { sim::Color::WHITE } else { p.kind.color() };
    let z = 36.0;
    if p.power_t > 0.0 {
        ink.disk(cx, cy, sim::SIZE * 0.85, z - 8.0, c(255, 70, 60, 60), true);
    }
    if p.haste_t > 0.0 {
        ink.disk(cx, cy, sim::SIZE * 0.85, z - 7.0, c(60, 200, 255, 60), true);
    }
    if p.over_t > 0.0 {
        ink.disk(cx, cy, sim::SIZE, z - 6.0, c(255, 150, 40, 90), true);
    }
    if p.ult >= sim::ULT_MAX {
        let pulse = 0.5 + 0.5 * (t * 8.0).sin();
        ink.ring(cx, cy, sim::SIZE * 0.8 + 6.0 * pulse, z - 5.0, c(255, 230, 80, 220));
    }
    ink.rect_rot(cx, cy, w, h, p.rot, z, sim::SIZE, body, true);

    let (look_x, look_y, ew, eh) = match p.eyes {
        sim::Eyes::Forward => (0.0, 0.0, 2.0, 2.0),
        sim::Eyes::Left => (-2.0, 0.0, 2.0, 2.0),
        sim::Eyes::Right => (2.0, 0.0, 2.0, 2.0),
        sim::Eyes::UpWide => (0.0, -1.5, 3.0, 3.0),
        sim::Eyes::Squint => (0.0, 0.0, 3.0, 1.0),
    };
    let eye_col = if p.hurt_t > 0.0 {
        sim::Color::BLACK
    } else if p.kind == sim::Kind::Ghost {
        c(60, 50, 90, 255)
    } else {
        sim::Color::WHITE
    };
    let big = sim::SIZE / 15.0;
    let (s, co) = p.rot.to_radians().sin_cos();
    for side in [-1.0_f32, 1.0] {
        let ox = (side * 3.0 + look_x) * big;
        let oy = (-2.0 + look_y) * big - p.crouch * 0.3;
        let ex = cx + ox * co - oy * s;
        let ey = cy + ox * s + oy * co;
        ink.rect_rot(ex, ey, ew * big, eh * big, p.rot, z + 5.0, 6.0, eye_col, true);
    }
    if p.frozen_t > 0.0 {
        ink.rect_rot(cx, cy, w + 8.0, h + 8.0, p.rot, z + 8.0, sim::SIZE + 18.0, c(170, 230, 255, 160), true);
        ink.rect_rot(cx - 10.0, cy - 12.0, 8.0, 26.0, p.rot + 20.0, z + 9.0, 3.0, c(255, 255, 255, 190), true);
    }
    if p.stun_t > 0.0 {
        for k in 0..3 {
            let a = t * 9.0 + k as f32 * 2.09;
            ink.disk(cx + a.cos() * 26.0, p.y - 10.0 + a.sin() * 6.0, 5.0, z + 12.0, c(255, 230, 60, 255), true);
        }
    }
    if p.block_held {
        let gx = cx + p.facing * sim::SIZE * 0.62;
        let col = if p.parry_t > 0.0 { sim::Color::WHITE } else { c(200, 230, 255, 220) };
        ink.disk(gx, cy, 40.0, z + 2.0, c(200, 230, 255, 50), true);
        ink.ring(gx, cy, 40.0, z + 10.0, col);
    }
    let melee = p.melee_t;
    if melee > 0.0 {
        let progress = 1.0 - melee / sim::MELEE_TIME;
        let angle = (-60.0 + 150.0 * progress) * p.facing;
        let kx = cx + p.facing * sim::SIZE * 0.35;
        let ky = cy + sim::SIZE * 0.1;
        knife(ink, kx, ky, angle, p.facing, 1.0, z + 14.0);
        if progress > 0.2 && progress < 0.9 {
            let a = (1.0 - progress) * 200.0;
            ink.ring(kx, ky, 70.0, z + 13.0, c(255, 255, 255, a as u8));
        }
    }
    if p.shield_t > 0.0 {
        ink.disk(cx, cy, sim::SIZE * 0.95, z - 4.0, c(120, 220, 255, 70), true);
        ink.ring(cx, cy, sim::SIZE * 0.95, z + 11.0, c(120, 220, 255, 255));
        for k in 0..6 {
            let a = p.shield_t * 6.0 + k as f32 * 1.047;
            ink.disk(cx + a.cos() * sim::SIZE * 0.95, cy + a.sin() * sim::SIZE * 0.95, 4.0, z + 12.0, sim::Color::WHITE, true);
        }
    }
    if p.dash_t > 0.0 {
        for i in 1..4 {
            let tx = cx - p.facing * i as f32 * 25.0;
            ink.rect(tx - w * 0.5, cy - h * 0.5, w, h, z - 2.0 - i as f32, 6.0, c(255, 165, 0, (110 - i * 30) as u8));
        }
    }
}

fn draw_particles(ink: &mut Ink, parts: &[sim::Particle]) {
    let n = parts.len();
    let start = n.saturating_sub(420);
    for q in &parts[start..] {
        let a = (q.life / q.max).clamp(0.0, 1.0);
        let col = sim::with_alpha(q.color, (q.color.a as f32 * a) as u8);
        match q.shape {
            sim::Shape::Circle => {
                let r = if q.grow { q.size * (1.0 + (1.0 - a)) } else { q.size * (0.4 + 0.6 * a) };
                ink.disk(q.x, q.y, r, 80.0, col, false);
            }
            sim::Shape::Ghost => ink.rect_rot(q.x, q.y, sim::SIZE, sim::SIZE, q.rot, 78.0, 6.0, col, false),
            sim::Shape::Square => {
                let s = q.size * (0.5 + 0.5 * a);
                ink.rect_rot(q.x, q.y, s, s, q.rot + q.life * 300.0, 80.0, 4.0, col, false);
            }
            sim::Shape::Streak => {
                ink.line(q.x, q.y, q.x - q.vx * 0.06, q.y - q.vy * 0.06, q.size.max(1.5), 80.0, col);
            }
            sim::Shape::Star => {
                let s = q.size * (0.5 + 0.5 * a);
                for rot in [0.0, 45.0] {
                    ink.rect_rot(q.x, q.y, s * 2.0, 3.0, rot, 82.0, 2.0, col, false);
                    ink.rect_rot(q.x, q.y, 3.0, s * 2.0, rot, 82.0, 2.0, col, false);
                }
            }
        }
    }
}

fn draw_world_fx(ink: &mut Ink, s: &sim::session::Session, t: f32) {
    let theme = sim::THEMES[s.theme_idx];
    let ground = H - sim::FLOOR_H;
    let rise = sim::lava_rise(theme, s.rd.time);
    if let Some(hz) = s.map.hazard {
        let hz = sim::Rectangle::new(hz.x, hz.y - rise, hz.width, hz.height + rise);
        let pulse = 0.5 + 0.5 * (t * 4.0).sin();
        ink.rect(hz.x - 6.0, hz.y - 6.0, hz.width + 12.0, hz.height + 10.0, 8.0, 10.0, c(255, (90.0 + 60.0 * pulse) as u8, 20, 255));
        ink.rect(hz.x, hz.y - 3.0, hz.width, 8.0, 12.0, 4.0, c(255, 220, 90, 230));
        for i in 0..8 {
            let bx = hz.x + 20.0 + i as f32 * 40.0;
            let by = hz.y - 6.0 - ((t * 20.0 + i as f32 * 9.0) % 14.0);
            ink.disk(bx, by, 3.0, 16.0, c(255, 200, 60, 220), true);
        }
    }
    if sim::MODES[s.mode_idx] == sim::Mode::Hill {
        if let Some(z) = s.map.zones.get(s.rd.zone_idx) {
            let pulse = 0.5 + 0.5 * (t * 4.0).sin();
            ink.rect(z.x, z.y, z.width, z.height, 6.0, 4.0, c(255, 220, 60, (50.0 + 40.0 * pulse) as u8));
            ink.frame(z.x, z.y, z.width, z.height, 2.0, 7.0, c(255, 230, 90, 255));
        }
    }
    for m in &s.world.meteors {
        if !m.falling {
            let prog = (1.0 - m.warn).clamp(0.0, 1.0);
            let pulse = 0.5 + 0.5 * (t * 20.0).sin();
            ink.line(m.tx - 300.0, -60.0, m.tx, ground, 2.0, 18.0, c(255, 80, 60, (60.0 + 100.0 * prog) as u8));
            ink.disk(m.tx, ground - 2.0, 18.0 + 22.0 * prog, 18.0, c(255, 70, 50, (80.0 + 80.0 * pulse) as u8), true);
        } else {
            ink.disk(m.x, m.y, 26.0, 18.0, c(255, 120, 30, 110), true);
            ink.disk(m.x, m.y, 16.0, 19.0, c(90, 60, 50, 255), true);
            ink.disk(m.x - 4.0, m.y - 4.0, 7.0, 20.0, c(255, 170, 60, 255), true);
        }
    }
    for l in &s.world.lasers {
        if !l.fired {
            let prog = (1.0 - l.t / 1.1).clamp(0.0, 1.0);
            let pulse = 0.5 + 0.5 * (t * 20.0).sin();
            ink.rect(0.0, l.y - 22.0 * prog, W, 44.0 * prog + 1.0, 18.0, 2.0, c(255, 60, 120, (30.0 + 50.0 * pulse) as u8));
            ink.rect(0.0, l.y - 2.0, W, 4.0, 19.0, 2.0, c(255, 60, 80, (80.0 + 150.0 * prog) as u8));
        } else {
            let a = (l.vis / 0.4).clamp(0.0, 1.0);
            ink.rect(0.0, l.y - 26.0 * a, W, 52.0 * a + 1.0, 18.0, 2.0, c(255, 80, 220, (130.0 * a) as u8));
            ink.rect(0.0, l.y - 9.0 * a, W, 18.0 * a + 1.0, 20.0, 2.0, c(255, 255, 255, (255.0 * a) as u8));
        }
    }
    for st in &s.world.strikes {
        if !st.struck {
            let prog = (1.0 - st.t / sim::STRIKE_DELAY).clamp(0.0, 1.0);
            let pulse = 0.5 + 0.5 * (t * 25.0).sin();
            let alpha = (25.0 + 110.0 * prog + 40.0 * pulse) as u8;
            ink.rect(st.x - 35.0, 0.0, 70.0, ground, 17.0, 2.0, c(255, 240, 120, alpha));
            ink.rect(st.x - 2.0, 0.0, 4.0, ground, 18.0, 2.0, c(255, 255, 255, (120.0 + 120.0 * prog) as u8));
            ink.disk(st.x, ground - 2.0, 20.0 + 25.0 * prog, 18.0, c(255, 240, 120, 120), true);
        } else {
            let a = (st.vis / 0.35).clamp(0.0, 1.0);
            for i in 1..st.pts.len() {
                let p0 = st.pts[i - 1];
                let p1 = st.pts[i];
                ink.line(p0.x, p0.y, p1.x, p1.y, 22.0 * a + 2.0, 21.0, c(170, 210, 255, (110.0 * a) as u8));
                ink.line(p0.x, p0.y, p1.x, p1.y, 8.0 * a + 1.0, 22.0, c(255, 255, 255, (255.0 * a) as u8));
            }
        }
    }
    for pk in &s.world.pickups {
        let bob = (pk.age * 4.0).sin() * 5.0;
        let col = pk.kind.color();
        ink.disk(pk.x, pk.y + bob, 30.0, 24.0, sim::with_alpha(col, 70), true);
        ink.disk(pk.x, pk.y + bob, 20.0, 25.0, col, true);
        ink.ring(pk.x, pk.y + bob, 20.0, 26.0, sim::Color::BLACK);
    }
    for b in &s.world.projs {
        match b.kind {
            sim::ProjKind::Bomb => {
                ink.disk(b.x, b.y, 9.0, 30.0, sim::Color::BLACK, true);
                ink.disk(b.x, b.y, 4.0, 32.0, sim::Color::RED, true);
            }
            sim::ProjKind::Fire => {
                ink.disk(b.x, b.y, 16.0, 30.0, c(255, 120, 30, 120), true);
                ink.disk(b.x, b.y, 10.0, 31.0, c(255, 170, 40, 255), true);
                ink.disk(b.x, b.y, 5.0, 32.0, c(255, 245, 170, 255), true);
            }
            sim::ProjKind::Frost => {
                ink.disk(b.x, b.y, 15.0, 30.0, c(150, 230, 255, 110), true);
                ink.rect_rot(b.x, b.y, 18.0, 18.0, 45.0 + t * 200.0, 32.0, 4.0, c(200, 245, 255, 255), true);
            }
        }
    }
    for b in &s.world.booms {
        let p = (b.t / sim::BOOM_TIME).min(1.0);
        let r = b.r * (0.4 + 0.6 * p);
        let fade = (255.0 * (1.0 - p)) as u8;
        ink.disk(b.x, b.y, r, 34.0, sim::with_alpha(b.color, fade / 4), false);
        ink.ring(b.x, b.y, r.max(2.0), 35.0, sim::with_alpha(b.color, fade));
        ink.ring(b.x, b.y, (r * 0.85).max(2.0), 36.0, sim::with_alpha(sim::Color::WHITE, fade / 2));
    }
}

fn victory(ink: &mut Ink, s: &sim::session::Session, t: f32) {
    let Some(v) = s.vic.as_ref() else { return };
    let ground = H - sim::FLOOR_H;
    let p = &s.players[v.who];
    let (cx, cy) = p.center();
    let vt = v.t;
    match v.idx {
        0 => {
            let scale = 2.4;
            let len = 92.0 * scale;
            let kx = cx + 130.0;
            let y0 = -len - 40.0;
            let y1 = ground + 22.0 - len;
            let hy = if vt < 0.9 { y0 + (y1 - y0) * sim::ease(vt / 0.9) } else { y1 };
            if vt < 0.9 {
                for k in 0..4 {
                    let lx = kx - 20.0 + k as f32 * 13.0;
                    ink.line(lx, hy - 140.0, lx, hy, 3.0, 60.0, c(255, 255, 255, 120));
                }
            }
            knife(ink, kx, hy, 180.0, 1.0, scale, 64.0);
            if vt >= 0.9 {
                let a = (1.0 - (vt - 0.9) / 1.2).clamp(0.0, 1.0);
                ink.ring(kx, ground, 30.0 + 160.0 * (1.0 - a), 62.0, c(255, 255, 255, (200.0 * a) as u8));
            }
        }
        3 => {
            ink.rect(cx - 45.0, 0.0, 90.0, cy, 58.0, 4.0, c(255, 255, 220, 40));
            for (k, r) in [50.0, 70.0, 90.0].iter().enumerate() {
                let pulse = (t * 6.0 + k as f32).sin() * 5.0;
                ink.ring(cx, cy, r + pulse, 60.0, c(255, 230, 120, 200));
            }
        }
        4 => {
            let flap = (t * 8.0).sin();
            for side in [-1.0_f32, 1.0] {
                for (len, ang, col) in [
                    (170.0, -40.0 + 20.0 * flap, c(255, 110, 30, 200)),
                    (135.0, -10.0 + 15.0 * flap, c(255, 170, 40, 200)),
                    (100.0, 22.0 + 10.0 * flap, c(255, 225, 100, 200)),
                ] {
                    let a = ang.to_radians();
                    let base_x = cx + side * 16.0;
                    let tip_x = cx + side * (20.0 + len * a.cos());
                    let tip_y = cy + len * a.sin();
                    ink.tri(base_x, cy, tip_x, tip_y, base_x, cy + 34.0, 60.0, col);
                }
            }
        }
        6 => {
            let a = (vt / 0.4).min(1.0);
            ink.disk(cx, cy, 220.0, 55.0, c(0, 0, 0, (205.0 * a) as u8), true);
            let beam = c(255, 240, 160, (34.0 * a) as u8);
            ink.tri(cx - 30.0, -20.0, cx - 150.0, ground, cx, ground, 57.0, beam);
            ink.tri(cx + 30.0, -20.0, cx, ground, cx + 150.0, ground, 57.0, beam);
            ink.tri(cx - 30.0, -20.0, cx + 30.0, -20.0, cx, ground, 57.0, beam);
        }
        8 => {
            if vt >= 0.3 && vt < 1.1 {
                let f = sim::ease((vt - 0.3) / 0.65);
                knife(ink, cx - 60.0, cy - 40.0, -80.0 + 180.0 * f, 1.0, 7.0, 70.0);
            }
            if vt >= 0.95 {
                let a = (1.0 - (vt - 0.95) / 1.4).clamp(0.0, 1.0);
                let th = 50.0 * a + 4.0;
                ink.line(0.0, 40.0, W, H - 40.0, th * 1.8, 66.0, c(120, 230, 255, (120.0 * a) as u8));
                ink.line(0.0, 40.0, W, H - 40.0, th, 68.0, c(255, 255, 255, (255.0 * a) as u8));
            }
        }
        _ => {}
    }
}

/// Body under the trim, cap on top. They meet at an edge and do not share a face.
const SLAB_DEPTH: f32 = 120.0;
const CAP_FRONT: f32 = 8.0;
const CAP_DEPTH: f32 = 136.0;

fn slab(ink: &mut Ink, x: f32, y: f32, w: f32, h: f32, body: sim::Color, cap_h: f32, cap: sim::Color) {
    let cap_h = cap_h.min(h - 1.0).max(2.0);
    ink.rect(x, y + cap_h, w, h - cap_h, 0.0, SLAB_DEPTH, body);
    // The cap owns the top face. It is proud of the front and deep enough to
    // cover the body, so the grass or trim is one surface instead of two.
    ink.rect(x, y, w, cap_h, CAP_FRONT, CAP_DEPTH, cap);
}

fn platforms(ink: &mut Ink, theme: sim::Theme, solids: &[sim::Rectangle]) {
    for r in solids {
        let (x, y, wd, h) = (r.x, r.y, r.width, r.height);
        let xi = x as i32;
        let yi = y as i32;
        let wi = wd as i32;
        let hi = h as i32;
        match theme {
            sim::Theme::Meadow => slab(ink, x, y, wd, h, c(115, 80, 50, 255), 9.0, c(85, 175, 65, 255)),
            sim::Theme::Skyline => {
                slab(ink, x, y, wd, h, c(28, 32, 62, 255), 5.0, c(100, 110, 170, 255));
                if hi >= 100 && wi < sim::W - 10 {
                    for wy in 0..((hi - 14) / 28) {
                        for wx in 0..((wi - 10) / 24) {
                            if (wx * 7 + wy * 13 + xi / 10) % 3 != 0 {
                                ink.rect((xi + 10 + wx * 24) as f32, (yi + 16 + wy * 28) as f32, 12.0, 16.0, 14.0, 4.0, c(255, 215, 100, 255));
                            }
                        }
                    }
                }
            }
            sim::Theme::Volcano => slab(ink, x, y, wd, h, c(52, 36, 38, 255), 5.0, c(215, 90, 25, 255)),
            sim::Theme::Frozen => {
                slab(ink, x, y, wd, h, c(150, 195, 225, 255), 8.0, c(240, 250, 255, 255));
                ink.rect(x + 4.0, y + 12.0, (wd - 8.0).max(0.0), 2.0, 16.0, 4.0, c(200, 230, 250, 255));
            }
            sim::Theme::Space => {
                slab(ink, x, y, wd, h, c(48, 52, 80, 255), 4.0, c(110, 200, 255, 255));
                ink.frame(x, y, wd, h, 2.0, 18.0, c(110, 200, 255, 90));
            }
            sim::Theme::Sea => {
                slab(ink, x, y, wd, h, c(55, 85, 100, 255), 6.0, c(90, 175, 150, 255));
                let kmax = (wi / 36).max(0);
                for k in 0..kmax {
                    ink.disk((xi + 18 + k * 36) as f32, y + 3.0, 3.0, 20.0, c(240, 140, 160, 255), true);
                }
            }
            sim::Theme::Sand => {
                slab(ink, x, y, wd, h, c(186, 146, 88, 255), 8.0, c(232, 205, 135, 255));
                let mut ly = yi + 20;
                while ly < yi + hi {
                    ink.rect(x, ly as f32, wd, 2.0, 16.0, 4.0, c(160, 120, 70, 255));
                    ly += 20;
                }
            }
            sim::Theme::Neon => {
                slab(ink, x, y, wd, h, c(22, 8, 48, 255), 4.0, c(255, 60, 200, 255));
                ink.frame(x, y, wd, h, 2.0, 18.0, c(0, 255, 255, 160));
            }
        }
    }
}

fn scenery_static(ink: &mut Ink, theme: sim::Theme) {
    let ground = H - sim::FLOOR_H;
    match theme {
        sim::Theme::Meadow => {
            ink.grad(0.0, 0.0, W, H, c(90, 160, 235, 255), c(205, 232, 255, 255), 16, -220.0);
            for (r, a) in [(130.0, 25u8), (95.0, 45), (62.0, 255)] {
                let col = if a == 255 { c(255, 232, 110, 255) } else { c(255, 240, 150, a) };
                ink.disk(1070.0, 120.0, r, -200.0, col, true);
            }
            ink.ellipse(180.0, ground + 40.0, 420.0, 230.0, -160.0, c(120, 185, 130, 255));
            ink.ellipse(700.0, ground + 60.0, 520.0, 190.0, -150.0, c(105, 175, 120, 255));
            ink.ellipse(1150.0, ground + 40.0, 380.0, 220.0, -150.0, c(120, 185, 130, 255));
            ink.ellipse(430.0, ground + 30.0, 380.0, 110.0, -140.0, c(70, 150, 80, 255));
            ink.ellipse(960.0, ground + 30.0, 400.0, 130.0, -140.0, c(65, 145, 75, 255));
            for tx in [95.0, 310.0, 1010.0, 1195.0] {
                ink.rect(tx - 8.0, ground - 70.0, 16.0, 70.0, -100.0, 36.0, c(110, 70, 40, 255));
                ink.disk(tx, ground - 95.0, 42.0, -96.0, c(40, 120, 50, 255), true);
                ink.disk(tx - 22.0, ground - 70.0, 30.0, -84.0, c(50, 135, 60, 255), true);
                ink.disk(tx + 22.0, ground - 72.0, 30.0, -74.0, c(50, 135, 60, 255), true);
            }
            for i in 0..30 {
                let fx = (i * 43 + 11) as f32;
                let col = match i % 3 {
                    0 => c(235, 60, 70, 255),
                    1 => c(255, 220, 60, 255),
                    _ => c(255, 130, 200, 255),
                };
                ink.rect(fx, ground - 10.0, 2.0, 10.0, -80.0, 3.0, c(50, 130, 50, 255));
                ink.disk(fx + 1.0, ground - 12.0, 4.0, -78.0, col, true);
            }
        }
        sim::Theme::Skyline => {
            ink.grad(0.0, 0.0, W, H, c(8, 10, 38, 255), c(75, 40, 115, 255), 16, -220.0);
            for i in 0..80 {
                let sx = (i * 977) % sim::W;
                let sy = (i * 389) % (sim::H - 260);
                let sz = if i % 5 == 0 { 3.0 } else { 2.0 };
                let tw = 160 + (i * 37) % 80;
                ink.rect(sx as f32, sy as f32, sz, sz, -200.0, 2.0, c(255, 255, 230, tw as u8));
            }
            ink.disk(190.0, 130.0, 85.0, -190.0, c(240, 240, 210, 25), true);
            ink.disk(190.0, 130.0, 56.0, -188.0, c(240, 240, 210, 255), true);
            for (cx, cy, r) in [(172.0, 115.0, 11.0), (210.0, 140.0, 8.0), (185.0, 150.0, 6.0), (205.0, 105.0, 5.0)] {
                ink.disk(cx, cy, r, -186.0, c(205, 205, 175, 255), true);
            }
            for layer in 0..2 {
                let base = if layer == 0 { c(35, 32, 85, 255) } else { c(22, 22, 60, 255) };
                for i in 0..13 {
                    let bw = 80 + ((i * 37 + layer * 11) % 50);
                    let bh = 130 + ((i * 53 + layer * 71) % 190) + layer * 40;
                    let bx = i * 100 - 20 + layer * 45;
                    let by = ground as i32 - bh;
                    let z = if layer == 0 { -170.0 } else { -130.0 };
                    let depth = if layer == 0 { 90.0 } else { 70.0 };
                    ink.rect(bx as f32, by as f32, bw as f32, bh as f32, z, depth, base);
                    for wy in 0..(bh / 26) {
                        for wx in 0..(bw / 22) {
                            if (wx * 7 + wy * 13 + i * 5 + layer) % 3 != 0 {
                                ink.rect_proud((bx + 8 + wx * 22) as f32, (by + 10 + wy * 26) as f32, 10.0, 14.0, z, 4.0, 7.0, c(255, 220, 110, 200));
                            }
                        }
                    }
                }
            }
        }
        sim::Theme::Volcano => {
            ink.grad(0.0, 0.0, W, H, c(40, 6, 10, 255), c(235, 105, 30, 255), 16, -220.0);
            ink.tri(-60.0, ground, 300.0, ground, 110.0, 360.0, -180.0, c(70, 30, 30, 255));
            ink.tri(980.0, ground, 1340.0, ground, 1170.0, 330.0, -180.0, c(70, 30, 30, 255));
            ink.tri(130.0, ground, 1150.0, ground, 640.0, 190.0, -160.0, c(42, 24, 30, 255));
            ink.ellipse(640.0, 195.0, 78.0, 18.0, -150.0, c(255, 150, 40, 255));
            ink.ellipse(640.0, 197.0, 56.0, 11.0, -148.0, c(255, 220, 90, 255));
            for (x1, y1, x2, y2) in [(610.0, 205.0, 520.0, 520.0), (650.0, 205.0, 740.0, 470.0), (635.0, 205.0, 640.0, 600.0)] {
                ink.line(x1, y1, x2, y2, 5.0, -145.0, c(255, 120, 30, 220));
            }
        }
        sim::Theme::Frozen => {
            ink.grad(0.0, 0.0, W, H, c(12, 22, 70, 255), c(150, 200, 235, 255), 16, -220.0);
            for i in 0..40 {
                ink.rect(((i * 541) % sim::W) as f32, ((i * 263) % 250) as f32, 2.0, 2.0, -200.0, 2.0, c(255, 255, 255, 180));
            }
            let snow = c(240, 248, 255, 255);
            ink.tri(-120.0, ground, 420.0, ground, 150.0, 250.0, -170.0, c(140, 170, 210, 255));
            ink.tri(150.0, 250.0, 95.0, 340.0, 205.0, 340.0, -168.0, snow);
            ink.tri(300.0, ground, 900.0, ground, 600.0, 190.0, -170.0, c(120, 155, 200, 255));
            ink.tri(600.0, 190.0, 535.0, 290.0, 665.0, 290.0, -168.0, snow);
            ink.tri(760.0, ground, 1400.0, ground, 1090.0, 230.0, -170.0, c(140, 170, 210, 255));
            ink.tri(1090.0, 230.0, 1030.0, 320.0, 1150.0, 320.0, -168.0, snow);
            ink.ellipse(300.0, ground + 50.0, 420.0, 90.0, -140.0, c(225, 238, 250, 255));
            ink.ellipse(980.0, ground + 50.0, 460.0, 100.0, -140.0, c(225, 238, 250, 255));
            for px in [70.0_f32, 230.0, 1050.0, 1215.0] {
                let green = c(25, 80, 70, 255);
                ink.tri(px, ground - 130.0, px - 38.0, ground - 75.0, px + 38.0, ground - 75.0, -110.0, green);
                ink.tri(px, ground - 100.0, px - 48.0, ground - 35.0, px + 48.0, ground - 35.0, -108.0, green);
                ink.tri(px, ground - 65.0, px - 58.0, ground, px + 58.0, ground, -106.0, green);
                ink.tri(px, ground - 130.0, px - 14.0, ground - 112.0, px + 14.0, ground - 112.0, -104.0, snow);
            }
        }
        sim::Theme::Space => {
            ink.grad(0.0, 0.0, W, H, c(2, 2, 16, 255), c(45, 12, 70, 255), 16, -220.0);
            for (cx, cy, r, col) in [
                (300.0, 220.0, 230.0, c(120, 40, 170, 26)),
                (900.0, 160.0, 260.0, c(200, 60, 120, 20)),
                (1050.0, 470.0, 220.0, c(60, 90, 200, 24)),
                (200.0, 520.0, 200.0, c(60, 160, 170, 18)),
            ] {
                ink.disk(cx, cy, r, -200.0, col, true);
                ink.disk(cx, cy, r * 0.65, -198.0, col, true);
            }
            for i in 0..150 {
                let sz = if i % 11 == 0 { 3.0 } else if i % 4 == 0 { 2.0 } else { 1.0 };
                ink.rect(((i * 677) % sim::W) as f32, ((i * 353) % (sim::H - 40)) as f32, sz, sz, -190.0, 2.0, c(255, 255, 255, 190));
            }
            ink.disk(930.0, 170.0, 115.0, -170.0, c(205, 125, 75, 255), true);
            for k in 0..5 {
                let y = 170.0 + (k as f32 - 2.0) * 34.0;
                let rh = (115.0f32 * 115.0 - (y - 170.0) * (y - 170.0)).max(0.0).sqrt();
                let band = if k % 2 == 0 { c(225, 160, 100, 255) } else { c(180, 100, 60, 255) };
                ink.ellipse(930.0, y, rh, 9.0, -168.0, band);
            }
            for (rh, rv) in [(200.0, 38.0), (185.0, 34.0), (170.0, 30.0)] {
                ink.ellipse(930.0, 175.0, rh, rv, -160.0, c(230, 200, 160, 50));
                ink.ellipse(930.0, 175.0, rh - 6.0, (rv - 4.0).max(2.0), -158.0, c(40, 20, 30, 255));
            }
            ink.disk(170.0, 120.0, 38.0, -150.0, c(205, 205, 215, 255), true);
            ink.disk(158.0, 110.0, 8.0, -148.0, c(170, 170, 185, 255), true);
            ink.disk(182.0, 132.0, 6.0, -148.0, c(170, 170, 185, 255), true);
            for i in 0..14 {
                let bw = 60 + (i * 31) % 40;
                let bh = 40 + (i * 37) % 70;
                let x = i * 95 - 10;
                ink.rect(x as f32, ground - bh as f32, bw as f32, bh as f32, -120.0, 48.0, c(25, 28, 52, 255));
            }
        }
        sim::Theme::Sea => {
            ink.grad(0.0, 0.0, W, H, c(30, 125, 175, 255), c(4, 18, 48, 255), 16, -220.0);
            ink.ellipse(200.0, ground + 20.0, 280.0, 140.0, -160.0, c(10, 50, 80, 255));
            ink.ellipse(930.0, ground + 20.0, 320.0, 150.0, -160.0, c(10, 50, 80, 255));
        }
        sim::Theme::Sand => {
            ink.grad(0.0, 0.0, W, H, c(255, 160, 80, 255), c(250, 225, 165, 255), 16, -220.0);
            ink.disk(640.0, 250.0, 150.0, -190.0, c(255, 230, 170, 50), true);
            ink.disk(640.0, 250.0, 105.0, -188.0, c(255, 240, 200, 120), true);
            ink.disk(640.0, 250.0, 70.0, -186.0, c(255, 250, 230, 255), true);
            let lit = c(214, 165, 95, 255);
            let shade = c(180, 132, 72, 255);
            ink.tri(80.0, ground, 520.0, ground, 300.0, 300.0, -160.0, lit);
            ink.tri(300.0, 300.0, 520.0, ground, 300.0, ground, -160.0, shade);
            ink.tri(780.0, ground, 1260.0, ground, 1020.0, 350.0, -160.0, lit);
            ink.tri(1020.0, 350.0, 1260.0, ground, 1020.0, ground, -160.0, shade);
            ink.ellipse(160.0, ground + 30.0, 420.0, 100.0, -140.0, c(236, 192, 122, 255));
            ink.ellipse(700.0, ground + 40.0, 560.0, 110.0, -140.0, c(226, 180, 108, 255));
            ink.ellipse(1180.0, ground + 30.0, 420.0, 105.0, -140.0, c(236, 192, 122, 255));
            ink.ellipse(420.0, ground + 36.0, 380.0, 70.0, -130.0, c(214, 168, 98, 255));
            ink.ellipse(950.0, ground + 36.0, 380.0, 70.0, -130.0, c(214, 168, 98, 255));
            for cx in [60.0_f32, 390.0, 1220.0] {
                let green = c(50, 140, 70, 255);
                ink.rect(cx - 7.0, ground - 70.0, 14.0, 70.0, -100.0, 32.0, green);
                ink.rect(cx - 24.0, ground - 48.0, 17.0, 8.0, -98.0, 6.0, green);
                ink.rect(cx - 24.0, ground - 60.0, 8.0, 20.0, -98.0, 6.0, green);
                ink.rect(cx + 7.0, ground - 38.0, 17.0, 8.0, -98.0, 6.0, green);
                ink.rect(cx + 16.0, ground - 52.0, 8.0, 22.0, -98.0, 6.0, green);
            }
        }
        sim::Theme::Neon => {
            ink.grad(0.0, 0.0, W, H, c(14, 0, 34, 255), c(105, 0, 105, 255), 16, -220.0);
            for i in 0..60 {
                ink.rect(((i * 613) % sim::W) as f32, ((i * 197) % 260) as f32, 2.0, 2.0, -200.0, 2.0, c(255, 255, 255, 180));
            }
            for (i, (r, col)) in [(170.0, c(255, 40, 150, 255)), (120.0, c(255, 110, 70, 255)), (70.0, c(255, 200, 80, 255)), (36.0, c(255, 230, 90, 255))]
                .into_iter()
                .enumerate()
            {
                ink.disk(640.0, 320.0, r, -186.0 + i as f32 * 4.0, col, true);
            }
            for k in 0..7 {
                let y = 330.0 + k as f32 * 24.0;
                let half = (170.0f32 * 170.0 - (y - 320.0) * (y - 320.0)).max(0.0).sqrt();
                ink.rect(640.0 - half, y, half * 2.0, (3 + k * 2) as f32, -170.0, 3.0, c(50, 0, 70, 255));
            }
            for i in 0..12 {
                let bh = 100 + (i * 47) % 150;
                let x = i * 108 - 5;
                let neon = if i % 2 == 0 { c(0, 240, 255, 255) } else { c(255, 60, 200, 255) };
                ink.rect(x as f32, (430 - bh) as f32, 90.0, bh as f32, -140.0, 76.0, c(18, 4, 40, 255));
                ink.frame_proud(x as f32, (430 - bh) as f32, 90.0, bh as f32, 2.0, -140.0, 8.0, neon);
            }
            ink.grad(0.0, 430.0, W, H - 430.0, c(30, 0, 60, 255), c(90, 0, 120, 255), 8, -120.0);
            for i in -14..=14 {
                ink.line(640.0 + i as f32 * 28.0, 430.0, 640.0 + i as f32 * 190.0, H, 2.0, -110.0, c(0, 240, 255, 110));
            }
        }
    }
}

fn scenery_live(ink: &mut Ink, theme: sim::Theme, t: f32) {
    let ground = H - sim::FLOOR_H;
    match theme {
        sim::Theme::Meadow => {
            for i in 0..5 {
                let x = ((i as f32 * 320.0 + t * 14.0) % (W + 300.0)) - 150.0;
                let y = 70.0 + ((i * 53) % 150) as f32;
                let white = c(255, 255, 255, 230);
                ink.disk(x, y, 30.0, -120.0, white, true);
                ink.disk(x + 34.0, y - 12.0, 38.0, -118.0, white, true);
                ink.disk(x + 72.0, y, 28.0, -118.0, white, true);
                ink.rect(x, y, 72.0, 28.0, -116.0, 6.0, white);
            }
        }
        sim::Theme::Volcano => {
            for i in 0..7 {
                let sx = 640.0 + (t * 0.5 + i as f32).sin() * 25.0 + i as f32 * 14.0;
                let sy = 165.0 - i as f32 * 32.0;
                ink.disk(sx, sy, 30.0 + i as f32 * 8.0, -140.0, c(60, 50, 55, (170 - i * 20) as u8), true);
            }
            for i in 0..45 {
                let ex = ((i * 131) as f32 + (t * 0.7 + i as f32).sin() * 30.0).rem_euclid(W);
                let ey = H - ((t * 40.0 + (i * 57) as f32) % H);
                ink.disk(ex, ey, 2.0, -100.0, c(255, 170, 50, 210), true);
            }
        }
        sim::Theme::Frozen => {
            for i in 0..58 {
                let x = i as f32 * 22.0;
                for band in 0..2 {
                    let bf = band as f32;
                    let y = 70.0 + bf * 45.0 + 28.0 * (t * 0.6 + i as f32 * 0.22 + bf).sin();
                    let h = 70.0 + 30.0 * (t * 0.4 + i as f32 * 0.15).sin();
                    let col = if band == 0 { c(70, 255, 160, 70) } else { c(160, 100, 255, 70) };
                    ink.rect(x, y, 24.0, h, -190.0, 2.0, col);
                }
            }
            for i in 0..90 {
                let fx = ((i * 97) as f32 + (t * 0.8 + i as f32).sin() * 30.0).rem_euclid(W);
                let fy = (t * 70.0 + (i * 41) as f32) % H;
                ink.disk(fx, fy, 1.5 + (i % 3) as f32, -60.0, c(255, 255, 255, 220), true);
            }
        }
        sim::Theme::Space => {
            let sx = (t * 350.0) % (W + 500.0) - 250.0;
            let sy = 60.0 + (t * 90.0) % 260.0;
            ink.line(sx, sy, sx - 120.0, sy - 40.0, 3.0, -140.0, c(255, 255, 255, 200));
            for i in 0..14 {
                let bw = 60 + (i * 31) % 40;
                let bh = 40 + (i * 37) % 70;
                let x = i * 95 - 10;
                if ((t * 2.0 + i as f32) % 2.0) < 1.0 {
                    ink.rect((x + bw / 2) as f32, ground - bh as f32 - 8.0, 4.0, 8.0, -110.0, 3.0, c(255, 60, 60, 255));
                }
            }
        }
        sim::Theme::Sea => {
            for i in 0..7 {
                let x = 60.0 + i as f32 * 190.0 + (t * 0.3 + i as f32).sin() * 40.0;
                ink.tri(x, 0.0, x + 60.0, 0.0, x + 170.0, ground, -180.0, c(200, 240, 255, 16));
            }
            for i in 0..6 {
                let x = (t * (50.0 + i as f32 * 8.0) + i as f32 * 230.0) % (W + 240.0) - 120.0;
                let y = 130.0 + i as f32 * 68.0 + (t * 1.5 + i as f32).sin() * 12.0;
                let col = match i % 3 {
                    0 => c(255, 170, 60, 230),
                    1 => c(255, 220, 90, 230),
                    _ => c(110, 190, 255, 230),
                };
                ink.ellipse(x, y, 18.0, 9.0, -80.0, col);
                ink.tri(x - 14.0, y, x - 32.0, y - 9.0, x - 32.0, y + 9.0, -78.0, col);
                ink.disk(x + 9.0, y - 2.0, 2.0, -76.0, sim::Color::BLACK, true);
            }
            for i in 0..3 {
                let jx = 250.0 + i as f32 * 380.0;
                let jy = 200.0 + (t + i as f32).sin() * 40.0;
                ink.disk(jx, jy, 22.0, -70.0, c(255, 150, 220, 120), true);
                for k in -2..=2 {
                    let kx = jx + k as f32 * 7.0;
                    ink.line(kx, jy + 10.0, kx + (t * 2.0 + k as f32).sin() * 6.0, jy + 52.0, 2.0, -68.0, c(255, 170, 230, 120));
                }
            }
            for i in 0..40 {
                let bx = ((i * 83) as f32 + (t + i as f32).sin() * 15.0).rem_euclid(W);
                let by = H - ((t * 35.0 + (i * 47) as f32) % H);
                ink.ring(bx, by, 2.0 + (i % 4) as f32, -50.0, c(220, 245, 255, 170));
            }
            let mut x = 0;
            while x < sim::W {
                for s in 0..5 {
                    let sx = x as f32 + (t * 1.4 + x as f32 * 0.04 + s as f32 * 0.6).sin() * 5.0 * s as f32;
                    ink.rect(sx, ground - 18.0 * (s as f32 + 1.0), 8.0, 18.0, -40.0, 4.0, c(40, 150, 90, 255));
                }
                x += 60;
            }
        }
        sim::Theme::Sand => {
            for i in 0..4 {
                let bx = (t * 70.0 + i as f32 * 300.0) % (W + 100.0) - 50.0;
                let by = 100.0 + i as f32 * 30.0 + (t * 3.0 + i as f32).sin() * 8.0;
                ink.line(bx - 10.0, by, bx, by + 5.0, 2.0, -100.0, c(70, 50, 40, 255));
                ink.line(bx, by + 5.0, bx + 10.0, by, 2.0, -100.0, c(70, 50, 40, 255));
            }
            for i in 0..25 {
                let x = (t * 180.0 + i as f32 * 53.0) % W;
                let y = ground - 10.0 - ((i * 13) % 160) as f32;
                ink.line(x, y, x + 14.0, y + 1.0, 1.5, -90.0, c(255, 235, 190, 150));
            }
        }
        sim::Theme::Neon => {
            for k in 0..9 {
                let f = ((k as f32 + (t * 0.5) % 1.0) / 9.0).powi(2);
                let y = 430.0 + f * (H - 430.0);
                ink.line(0.0, y, W, y, 2.0, -100.0, c(255, 60, 200, 100));
            }
        }
        sim::Theme::Skyline => {}
    }
}

fn shake_px(s: &sim::session::Session) -> (f32, f32) {
    let mag = s.world.fx.shake;
    ((s.t * 91.7).sin() * mag, (s.t * 67.3).cos() * mag)
}

pub(crate) fn camera_frame(s: &sim::session::Session) -> (f32, f32, f32) {
    let mut zoom = 1.0 + s.world.fx.punch;
    let mut focus = s.ko_focus;
    let mut weight = 0.0;
    if s.ko_timer > 0.0 {
        let prog = sim::ease(1.0 - s.ko_timer / sim::KO_TIME);
        zoom += 0.45 * prog;
        weight = prog;
    }
    if let Some(v) = s.vic.as_ref() {
        let wp = sim::ease(v.t / 0.5);
        zoom = 1.0 + (sim::VIC_ZOOM[v.idx] - 1.0) * wp + s.world.fx.punch;
        focus = s.players[v.who].center();
        weight = wp;
    }
    let zoom = zoom.max(1.0);
    let tx = W * 0.5 + (focus.0 - W * 0.5) * weight;
    let ty = H * 0.5 + (focus.1 - H * 0.5) * weight;
    let half_w = W / (2.0 * zoom);
    let half_h = H / (2.0 * zoom);
    let cx = tx.clamp(half_w, W - half_w);
    let cy = ty.clamp(half_h, H - half_h);
    let (sx, sy) = shake_px(s);
    (cx + sx, cy + sy, zoom)
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct CamPose {
    pub eye: Vec3,
    pub look: Vec3,
    pub fov: f32,
    pub dist: f32,
}

impl Default for CamPose {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 400.0, 1600.0),
            look: Vec3::ZERO,
            fov: CAM_FOV_DEG.to_radians(),
            dist: CAM_DIST,
        }
    }
}

/// Project a gameplay-plane point into the 1280×720 overlay.
pub(crate) fn project_screen(pose: &CamPose, x: f32, y: f32, z: f32, aspect: f32) -> Option<(f32, f32, f32)> {
    let world = Vec3::new(x - W * 0.5, H * 0.5 - y, z);
    let view = Transform::from_translation(pose.eye)
        .looking_at(pose.look, Vec3::Y)
        .to_matrix()
        .inverse();
    let cam = view.transform_point3(world);
    let depth = -cam.z;
    if depth < 1.0 {
        return None;
    }
    let t = (pose.fov * 0.5).tan().max(0.05);
    let ndc_x = cam.x / (depth * t * aspect.max(0.1));
    let ndc_y = cam.y / (depth * t);
    let sx = (ndc_x + 1.0) * 0.5 * W;
    let sy = (1.0 - (ndc_y + 1.0) * 0.5) * H;
    Some((sx, sy, depth))
}

pub(crate) fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let ring = meshes.add(
        Torus::new(0.93, 1.0)
            .mesh()
            .major_resolution(28)
            .minor_resolution(6)
            .build(),
    );
    let gfx = Gfx {
        cube: meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0))),
        disk: meshes.add(Mesh::from(Cylinder::new(0.5, 1.0))),
        ring,
        mats: HashMap::new(),
        built_map: 0,
    };
    let mut tri_ents = Vec::new();
    let mut tri_handles = Vec::new();
    let blank_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });
    for _ in 0..48 {
        let handle = meshes.add(blank_mesh());
        let e = commands
            .spawn((
                Mesh3d(handle.clone()),
                MeshMaterial3d(blank_mat.clone()),
                Transform::IDENTITY,
                Visibility::Hidden,
            ))
            .id();
        tri_handles.push(handle);
        tri_ents.push(e);
    }
    commands.insert_resource(gfx);
    commands.insert_resource(FxPool::default());
    commands.insert_resource(TriPool { ents: tri_ents, handles: tri_handles, cursor: 0 });
    commands.insert_resource(CamPose::default());
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.98, 0.95),
        brightness: 260.0,
        affects_lightmapped_meshes: true,
    });
    commands.insert_resource(DirectionalLightShadowMap { size: 2048 });
    let light_yaw = 40.0_f32.to_radians();
    let light_pitch = 54.0_f32.to_radians();
    let light_pos = Vec3::new(
        light_yaw.sin() * light_pitch.cos(),
        light_pitch.sin(),
        light_yaw.cos() * light_pitch.cos(),
    ) * 2400.0;
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.96, 0.90),
            illuminance: 3200.0,
            shadow_maps_enabled: true,
            // Constant bias kills acne on the pixel-scale slabs. Normal bias stays
            // small so shadows stay attached to the cubes instead of floating.
            shadow_depth_bias: 2.4,
            shadow_normal_bias: 0.6,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            minimum_distance: 80.0,
            first_cascade_far_bound: 4200.0,
            maximum_distance: 4200.0,
            overlap_proportion: 0.1,
        }
        .build(),
        Transform::from_translation(light_pos).looking_at(Vec3::new(0.0, 0.0, -40.0), Vec3::Y),
    ));
    commands.spawn((
        Cam,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: CAM_FOV_DEG.to_radians(),
            near: 8.0,
            far: 7000.0,
            aspect_ratio: W / H,
            ..default()
        }),
        Tonemapping::ReinhardLuminance,
        Exposure { ev100: 9.5 },
        Transform::from_xyz(500.0, 600.0, 1500.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub(crate) fn sync_stage(
    mut commands: Commands,
    sim: Res<Sim>,
    mut gfx: ResMut<Gfx>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut clear: ResMut<ClearColor>,
    old: Query<Entity, With<Scenery>>,
) {
    if gfx.built_map == sim.0.map_serial {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    gfx.built_map = sim.0.map_serial;
    let theme = sim::THEMES[sim.0.theme_idx];
    clear.0 = Color::BLACK;
    let solids: Vec<sim::Rectangle> = sim.0.map.solids.iter().map(|s| s.r).collect();
    let cube = gfx.cube.clone();
    let disk = gfx.disk.clone();
    let ring = gfx.ring.clone();
    let mut ink = Ink {
        commands: &mut commands,
        mats: &mut gfx.mats,
        materials: &mut materials,
        meshes: &mut meshes,
        cube,
        disk,
        ring,
        pool: None,
        tris: None,
    };
    scenery_static(&mut ink, theme);
    platforms(&mut ink, theme, &solids);
}

pub(crate) fn sync_actors() {}

pub(crate) fn bob_system() {}

pub(crate) fn sync_fx(
    mut commands: Commands,
    sim: Res<Sim>,
    mut gfx: ResMut<Gfx>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pool: ResMut<FxPool>,
    mut tris: ResMut<TriPool>,
) {
    pool.cursor = 0;
    tris.cursor = 0;
    {
        let cube = gfx.cube.clone();
        let disk = gfx.disk.clone();
        let ring = gfx.ring.clone();
        let mut ink = Ink {
            commands: &mut commands,
            mats: &mut gfx.mats,
            materials: &mut materials,
            meshes: &mut meshes,
            cube,
            disk,
            ring,
            pool: Some(&mut pool),
            tris: Some(&mut tris),
        };
        let t = sim.0.t;
        let theme = sim::THEMES[sim.0.theme_idx];
        scenery_live(&mut ink, theme, t);
        if !sim.0.selecting {
            draw_world_fx(&mut ink, &sim.0, t);
        }
        draw_particles(&mut ink, &sim.0.world.parts);
        if sim.0.selecting {
            for p in &sim.0.previews {
                draw_player(&mut ink, p, t);
            }
        } else {
            for p in &sim.0.players {
                if p.hp > 0.0 && p.dead_t <= 0.0 {
                    draw_player(&mut ink, p, t);
                }
            }
            victory(&mut ink, &sim.0, t);
        }
    }
    for e in pool.ents.iter().skip(pool.cursor) {
        commands.entity(*e).insert(Visibility::Hidden);
    }
    for e in tris.ents.iter().skip(tris.cursor) {
        commands.entity(*e).insert(Visibility::Hidden);
    }
}

pub(crate) fn sync_camera(sim: Res<Sim>, mut pose: ResMut<CamPose>, mut cam: Query<(&mut Transform, &mut Projection), With<Cam>>) {
    let Some((mut tf, mut proj)) = cam.iter_mut().next() else { return };
    let (cx, cy, zoom) = camera_frame(&sim.0);
    let look = Vec3::new(cx - W * 0.5, H * 0.5 - cy, 8.0);
    let mut yaw = CAM_YAW_DEG.to_radians();
    let mut pitch = CAM_PITCH_DEG.to_radians();
    if sim.0.ko_timer > 0.0 {
        let prog = 1.0 - sim.0.ko_timer / sim::KO_TIME;
        yaw += 0.16 * prog;
        pitch += 0.05 * prog;
    }
    if let Some(v) = sim.0.vic.as_ref() {
        let prog = (v.t / 0.55).clamp(0.0, 1.0);
        yaw += 0.20 * prog.sin();
        pitch += 0.04 * prog;
    }
    let dist = CAM_DIST / zoom.max(0.35);
    let eye = look
        + Vec3::new(
            dist * yaw.sin() * pitch.cos(),
            dist * pitch.sin(),
            dist * yaw.cos() * pitch.cos(),
        );
    tf.translation = eye;
    tf.look_at(look, Vec3::Y);
    let fov = CAM_FOV_DEG.to_radians();
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = fov;
        p.near = 8.0;
        p.far = 7000.0;
    }
    *pose = CamPose { eye, look, fov, dist };
}
