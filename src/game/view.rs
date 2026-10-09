//! Procedural 3D view of the 1280x720 simulation.
//! Meshes are Bevy primitives. Particles are pooled and do not cast shadows.

use super as sim;
use super::app::Sim;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use std::collections::HashMap;

const SCALE: f32 = 0.012;

#[derive(Component)]
pub(crate) struct StageRoot;

#[derive(Component)]
pub(crate) struct Actor {
    slot: u8,
}

#[derive(Component)]
pub(crate) struct Bob {
    base_y: f32,
    amp: f32,
    speed: f32,
    phase: f32,
}

#[derive(Clone, Copy)]
enum MeshId {
    Cube,
    Sphere,
    Cone,
    Cyl,
    Torus,
}

#[derive(Resource, Default)]
pub(crate) struct FxPool {
    ents: Vec<Entity>,
    cursor: usize,
}

#[derive(Resource)]
pub(crate) struct Gfx {
    cube: Handle<Mesh>,
    sphere: Handle<Mesh>,
    cone: Handle<Mesh>,
    cyl: Handle<Mesh>,
    torus: Handle<Mesh>,
    mats: HashMap<(u8, u8, u8, u8, u8), Handle<StandardMaterial>>,
    built_map: u32,
}

impl Gfx {
    fn mesh(&self, id: MeshId) -> Handle<Mesh> {
        match id {
            MeshId::Cube => self.cube.clone(),
            MeshId::Sphere => self.sphere.clone(),
            MeshId::Cone => self.cone.clone(),
            MeshId::Cyl => self.cyl.clone(),
            MeshId::Torus => self.torus.clone(),
        }
    }

    fn material(&mut self, materials: &mut Assets<StandardMaterial>, col: sim::Color, kind: u8) -> Handle<StandardMaterial> {
        let col = if kind <= 2 {
            let q = |v: u8| v & 0xF0;
            sim::Color::new(q(col.r), q(col.g), q(col.b), if kind == 2 { q(col.a).max(0x20) } else { 255 })
        } else {
            col
        };
        let key = (col.r, col.g, col.b, col.a, kind);
        if let Some(h) = self.mats.get(&key) {
            return h.clone();
        }
        let base = Color::srgba_u8(col.r, col.g, col.b, col.a);
        let emissive = if kind == 3 || kind == 4 {
            let s = if kind == 4 { 1.4 } else { 0.35 };
            LinearRgba::rgb(col.r as f32 / 255.0 * s, col.g as f32 / 255.0 * s, col.b as f32 / 255.0 * s)
        } else {
            LinearRgba::BLACK
        };
        let h = materials.add(StandardMaterial {
            base_color: base,
            emissive,
            unlit: kind == 1 || kind == 2 || kind == 4,
            alpha_mode: if kind == 2 { AlphaMode::Blend } else { AlphaMode::Opaque },
            cull_mode: None,
            perceptual_roughness: if kind == 3 { 0.42 } else { 0.86 },
            metallic: if kind == 3 { 0.04 } else { 0.0 },
            ..default()
        });
        self.mats.insert(key, h.clone());
        h
    }
}

fn px(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new((x - sim::W as f32 * 0.5) * SCALE, (sim::H as f32 * 0.5 - y) * SCALE, z * SCALE)
}

pub(crate) fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let gfx = Gfx {
        cube: meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0))),
        sphere: meshes.add(Sphere::new(0.5).mesh().ico(1).unwrap()),
        cone: meshes.add(Mesh::from(Cone::new(0.5, 1.0))),
        cyl: meshes.add(Mesh::from(Cylinder::new(0.5, 1.0))),
        torus: meshes.add(Mesh::from(Torus::new(0.78, 1.0))),
        mats: HashMap::new(),
        built_map: 0,
    };
    let body = gfx.cube.clone();
    let white = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.5,
        ..default()
    });
    commands.insert_resource(gfx);
    commands.insert_resource(FxPool::default());
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.4, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    for slot in 0..4 {
        commands.spawn((
            Actor { slot },
            Mesh3d(body.clone()),
            MeshMaterial3d(white.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

struct Pen<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    gfx: &'a mut Gfx,
    materials: &'a mut Assets<StandardMaterial>,
}

impl Pen<'_, '_, '_> {
    fn put(
        &mut self,
        mesh: MeshId,
        x: f32,
        y: f32,
        z: f32,
        sx: f32,
        sy: f32,
        sz: f32,
        rot: Quat,
        col: sim::Color,
        kind: u8,
        shadow: bool,
        bob: Option<(f32, f32, f32)>,
    ) {
        let Pen { commands, gfx, materials } = self;
        let handle = gfx.mesh(mesh).clone();
        let mat = gfx.material(materials, col, kind);
        let scale = if matches!(mesh, MeshId::Torus) {
            Vec3::splat(sx * SCALE)
        } else {
            Vec3::new(sx * SCALE, sy * SCALE, sz * SCALE)
        };
        let tf = Transform {
            translation: px(x, y, z),
            rotation: rot,
            scale,
        };
        let mut e = commands.spawn((StageRoot, Mesh3d(handle), MeshMaterial3d(mat), tf));
        if !shadow {
            e.insert((NotShadowCaster, NotShadowReceiver));
        }
        if let Some((amp, speed, phase)) = bob {
            e.insert(Bob {
                base_y: tf.translation.y,
                amp,
                speed,
                phase,
            });
        }
    }

    fn cube(&mut self, x: f32, y: f32, z: f32, w: f32, h: f32, d: f32, col: sim::Color, kind: u8, shadow: bool) {
        self.put(MeshId::Cube, x, y, z, w, h, d, Quat::IDENTITY, col, kind, shadow, None);
    }

    fn sphere(&mut self, x: f32, y: f32, z: f32, d: f32, col: sim::Color, kind: u8, shadow: bool) {
        self.put(MeshId::Sphere, x, y, z, d, d, d, Quat::IDENTITY, col, kind, shadow, None);
    }
}

struct Look {
    sky: [u8; 3],
    ground: sim::Color,
    plat: sim::Color,
    top: sim::Color,
    accent: sim::Color,
    ambient: f32,
    sun: [u8; 3],
    lux: f32,
}

fn look(theme: sim::Theme) -> Look {
    use sim::Color;
    use sim::Theme::*;
    match theme {
        Meadow => Look { sky: [120, 196, 255], ground: Color::new(70, 160, 60, 255), plat: Color::new(92, 176, 72, 255), top: Color::new(130, 210, 90, 255), accent: Color::new(255, 214, 80, 255), ambient: 140.0, sun: [255, 244, 220], lux: 14000.0 },
        Skyline => Look { sky: [10, 14, 36], ground: Color::new(28, 30, 42, 255), plat: Color::new(36, 40, 58, 255), top: Color::new(70, 80, 120, 255), accent: Color::new(255, 196, 80, 255), ambient: 55.0, sun: [180, 200, 255], lux: 4500.0 },
        Volcano => Look { sky: [48, 18, 22], ground: Color::new(50, 28, 26, 255), plat: Color::new(78, 42, 36, 255), top: Color::new(120, 64, 48, 255), accent: Color::new(255, 90, 20, 255), ambient: 70.0, sun: [255, 140, 80], lux: 7000.0 },
        Frozen => Look { sky: [186, 214, 232], ground: Color::new(210, 226, 236, 255), plat: Color::new(170, 200, 220, 255), top: Color::new(236, 246, 255, 255), accent: Color::new(140, 200, 255, 255), ambient: 150.0, sun: [230, 240, 255], lux: 12000.0 },
        Space => Look { sky: [6, 8, 20], ground: Color::new(30, 32, 48, 255), plat: Color::new(48, 52, 78, 255), top: Color::new(90, 100, 150, 255), accent: Color::new(120, 170, 255, 255), ambient: 40.0, sun: [200, 210, 255], lux: 3500.0 },
        Sea => Look { sky: [8, 60, 110], ground: Color::new(20, 80, 90, 255), plat: Color::new(30, 110, 100, 255), top: Color::new(70, 170, 140, 255), accent: Color::new(80, 220, 200, 255), ambient: 80.0, sun: [140, 220, 255], lux: 6000.0 },
        Sand => Look { sky: [255, 186, 110], ground: Color::new(196, 150, 78, 255), plat: Color::new(176, 130, 70, 255), top: Color::new(230, 190, 110, 255), accent: Color::new(255, 120, 50, 255), ambient: 150.0, sun: [255, 220, 170], lux: 15000.0 },
        Neon => Look { sky: [18, 0, 36], ground: Color::new(16, 8, 32, 255), plat: Color::new(28, 10, 48, 255), top: Color::new(255, 40, 160, 255), accent: Color::new(40, 255, 220, 255), ambient: 60.0, sun: [255, 80, 200], lux: 4000.0 },
    }
}

pub(crate) fn sync_stage(
    mut commands: Commands,
    sim: Res<Sim>,
    mut gfx: ResMut<Gfx>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut clear: ResMut<ClearColor>,
    old: Query<Entity, With<StageRoot>>,
) {
    if gfx.built_map == sim.0.map_serial {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let theme = sim::THEMES[sim.0.theme_idx];
    let look = look(theme);
    clear.0 = Color::srgb_u8(look.sky[0], look.sky[1], look.sky[2]);
    ambient.color = Color::srgb_u8(look.sky[0], look.sky[1], look.sky[2]);
    ambient.brightness = look.ambient;
    let serial = sim.0.map_serial;
    gfx.built_map = serial;

    commands.spawn((
        StageRoot,
        DirectionalLight {
            color: Color::srgb_u8(look.sun[0], look.sun[1], look.sun[2]),
            illuminance: look.lux,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-4.0, 12.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        StageRoot,
        PointLight {
            color: Color::srgb_u8(255, 244, 230),
            intensity: 1_200_000.0,
            range: 48.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2.5, 9.0),
    ));

    let mut pen = Pen { commands: &mut commands, gfx: &mut gfx, materials: &mut materials };
    let sky = sim::Color::new(look.sky[0], look.sky[1], look.sky[2], 255);
    pen.cube(sim::W as f32 * 0.5, sim::H as f32 * 0.5, -220.0, 1800.0, 1200.0, 8.0, sky, 1, false);

    for solid in &sim.0.map.solids {
        let r = solid.r;
        let floor = r.y >= sim::H as f32 - sim::FLOOR_H - 1.0 && r.height >= sim::FLOOR_H - 1.0;
        let col = if floor { look.ground } else { look.plat };
        let depth = if floor { 160.0 } else { 70.0 };
        let z = if floor { -10.0 } else { 0.0 };
        pen.cube(r.x + r.width * 0.5, r.y + r.height * 0.5, z, r.width, r.height, depth, col, 0, true);
        let lip = 8.0_f32.min(r.height);
        pen.cube(r.x + r.width * 0.5, r.y + lip * 0.5, z + 8.0, r.width, lip, depth * 0.92, look.top, if theme == sim::Theme::Neon { 4 } else { 0 }, false);
    }
    dress(&mut pen, theme, look.accent);
}

fn dress(pen: &mut Pen, theme: sim::Theme, accent: sim::Color) {
    use sim::Theme::*;
    let c = |r, g, b| sim::Color::new(r, g, b, 255);
    match theme {
        Meadow => {
            pen.sphere(1080.0, 90.0, -80.0, 90.0, c(255, 220, 90), 4, false);
            for (i, (x, y)) in [(180.0, 120.0), (420.0, 80.0), (760.0, 140.0), (980.0, 100.0)].iter().enumerate() {
                let ph = i as f32;
                pen.put(MeshId::Sphere, *x, *y, -90.0, 70.0, 40.0, 36.0, Quat::IDENTITY, c(255, 255, 255), 1, false, Some((0.15, 0.6, ph)));
                pen.put(MeshId::Sphere, *x + 28.0, *y + 6.0, -86.0, 48.0, 32.0, 28.0, Quat::IDENTITY, c(245, 248, 255), 1, false, Some((0.12, 0.7, ph + 1.0)));
            }
            for (x, s) in [(160.0, 180.0), (640.0, 240.0), (1040.0, 200.0)] {
                pen.put(MeshId::Cone, x, 560.0, -140.0, s, 160.0, s, Quat::IDENTITY, c(60, 140, 55), 0, true, None);
            }
            for x in [220.0, 480.0, 860.0, 1120.0] {
                pen.put(MeshId::Cyl, x, 600.0, -40.0, 16.0, 70.0, 16.0, Quat::IDENTITY, c(110, 70, 40), 0, true, None);
                pen.sphere(x, 540.0, -40.0, 54.0, c(40, 150, 55), 0, true);
            }
        }
        Skyline => {
            pen.sphere(180.0, 110.0, -100.0, 70.0, c(230, 230, 210), 4, false);
            for i in 0..36 {
                let x = 40.0 + (i * 97 % 1200) as f32;
                let y = 20.0 + (i * 53 % 280) as f32;
                pen.sphere(x, y, -160.0, 4.0 + (i % 3) as f32, c(255, 255, 230), 4, false);
            }
            for (x, h) in [(80.0, 220.0), (250.0, 300.0), (1050.0, 260.0), (1180.0, 180.0)] {
                pen.cube(x, 500.0 - h * 0.15, -120.0, 50.0, h, 40.0, c(22, 26, 40), 0, true);
            }
        }
        Volcano => {
            pen.put(MeshId::Cone, 180.0, 430.0, -150.0, 280.0, 320.0, 280.0, Quat::IDENTITY, c(70, 36, 32), 0, true, None);
            pen.put(MeshId::Cone, 1100.0, 460.0, -160.0, 320.0, 280.0, 320.0, Quat::IDENTITY, c(60, 30, 28), 0, true, None);
            pen.sphere(640.0, 80.0, -40.0, 50.0, accent, 4, false);
            for i in 0..14 {
                let x = 80.0 + (i * 90) as f32;
                pen.put(MeshId::Sphere, x, 160.0 + (i % 5) as f32 * 30.0, -60.0, 10.0, 10.0, 10.0, Quat::IDENTITY, c(255, 120, 30), 4, false, Some((0.4, 1.2 + (i % 3) as f32 * 0.2, i as f32)));
            }
        }
        Frozen => {
            for (x, h) in [(200.0, 260.0), (640.0, 320.0), (1080.0, 240.0)] {
                pen.put(MeshId::Cone, x, 420.0, -150.0, 260.0, h, 260.0, Quat::IDENTITY, c(200, 220, 235), 0, true, None);
            }
            for x in [140.0, 400.0, 900.0, 1160.0] {
                pen.put(MeshId::Cone, x, 560.0, -30.0, 46.0, 90.0, 46.0, Quat::IDENTITY, c(30, 110, 70), 0, true, None);
                pen.put(MeshId::Cone, x, 520.0, -30.0, 64.0, 70.0, 64.0, Quat::IDENTITY, c(40, 130, 80), 0, true, None);
            }
            for i in 0..18 {
                pen.put(MeshId::Sphere, 30.0 + (i * 70) as f32, 40.0 + (i * 37 % 200) as f32, -20.0, 6.0, 6.0, 6.0, Quat::IDENTITY, c(255, 255, 255), 1, false, Some((0.25, 0.8, i as f32)));
            }
        }
        Space => {
            pen.sphere(980.0, 160.0, -140.0, 150.0, c(70, 90, 180), 0, false);
            pen.put(MeshId::Torus, 980.0, 160.0, -130.0, 120.0, 120.0, 120.0, Quat::from_rotation_x(0.4), c(180, 170, 140), 0, false, None);
            pen.sphere(250.0, 200.0, -80.0, 40.0, c(200, 200, 190), 4, false);
            for i in 0..40 {
                let x = (i * 83 % 1280) as f32;
                let y = (i * 47 % 500) as f32;
                pen.sphere(x, y, -180.0, 3.0 + (i % 4) as f32, c(255, 255, 240), 4, false);
            }
        }
        Sea => {
            for i in 0..8 {
                let x = 80.0 + i as f32 * 150.0;
                pen.put(MeshId::Cyl, x, 640.0, -20.0, 10.0, 80.0, 10.0, Quat::from_rotation_z((i as f32) * 0.2), c(20, 140, 90), 0, false, Some((0.1, 1.5, i as f32)));
            }
            for (x, y) in [(200.0, 180.0), (500.0, 140.0), (900.0, 200.0), (1100.0, 120.0)] {
                pen.sphere(x, y, -30.0, 28.0, c(255, 140, 60), 0, false);
                pen.put(MeshId::Cone, x + 22.0, y, -30.0, 16.0, 22.0, 12.0, Quat::from_rotation_z(-1.2), c(255, 160, 70), 0, false, None);
            }
            for i in 0..12 {
                pen.put(MeshId::Sphere, 60.0 + (i * 100) as f32, 80.0 + (i * 40 % 220) as f32, 10.0, 12.0, 12.0, 12.0, Quat::IDENTITY, c(200, 240, 255), 2, false, Some((0.3, 0.9, i as f32 * 0.7)));
            }
        }
        Sand => {
            pen.sphere(160.0, 80.0, -60.0, 80.0, c(255, 210, 90), 4, false);
            pen.put(MeshId::Cone, 280.0, 500.0, -100.0, 220.0, 180.0, 220.0, Quat::IDENTITY, c(190, 140, 70), 0, true, None);
            pen.put(MeshId::Cone, 1000.0, 520.0, -110.0, 260.0, 200.0, 260.0, Quat::IDENTITY, c(176, 126, 64), 0, true, None);
            for x in [360.0, 920.0] {
                pen.put(MeshId::Cyl, x, 560.0, -20.0, 14.0, 90.0, 14.0, Quat::IDENTITY, c(120, 80, 40), 0, true, None);
                pen.sphere(x, 500.0, -16.0, 48.0, c(40, 130, 50), 0, true);
            }
        }
        Neon => {
            for i in 0..8 {
                let x = 80.0 + i as f32 * 150.0;
                pen.cube(x, sim::H as f32 - 24.0, 20.0, 6.0, 4.0, 140.0, accent, 4, false);
            }
            for (x, h, col) in [(100.0, 240.0, c(255, 40, 140)), (300.0, 180.0, c(40, 255, 220)), (1000.0, 220.0, c(140, 60, 255)), (1180.0, 160.0, c(255, 220, 40))] {
                pen.cube(x, 480.0, -130.0, 36.0, h, 28.0, col, 4, false);
            }
            pen.put(MeshId::Torus, 640.0, 90.0, -40.0, 70.0, 70.0, 70.0, Quat::IDENTITY, c(255, 40, 180), 4, false, None);
        }
    }
}

pub(crate) fn bob_system(time: Res<Time>, mut q: Query<(&Bob, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (b, mut tf) in &mut q {
        tf.translation.y = b.base_y + (t * b.speed + b.phase).sin() * b.amp;
    }
}

pub(crate) fn sync_camera(sim: Res<Sim>, mut cam: Query<&mut Transform, With<Camera3d>>) {
    let Some(mut tf) = cam.iter_mut().next() else { return };
    let s = &sim.0;
    let mut zoom = 1.0 + s.world.fx.punch;
    let mut focus = (sim::W as f32 * 0.5, sim::H as f32 * 0.5);
    let mut weight = 0.0;
    if s.ko_timer > 0.0 {
        let prog = sim::ease(1.0 - s.ko_timer / sim::KO_TIME);
        zoom += 0.45 * prog;
        focus = s.ko_focus;
        weight = prog;
    }
    if let Some(v) = &s.vic {
        let wp = sim::ease((v.t / 0.45).min(1.0));
        zoom = 1.0 + (sim::VIC_ZOOM[v.idx] - 1.0) * wp + s.world.fx.punch;
        focus = s.players[v.who].center();
        weight = wp;
        if v.idx == 3 {
            focus.1 -= 80.0;
        }
    }
    zoom = zoom.max(0.9);
    let half_w = sim::W as f32 / (2.0 * zoom);
    let half_h = sim::H as f32 / (2.0 * zoom);
    let tx = (sim::W as f32 * 0.5 + (focus.0 - sim::W as f32 * 0.5) * weight).clamp(half_w, sim::W as f32 - half_w);
    let ty = (sim::H as f32 * 0.5 + (focus.1 - sim::H as f32 * 0.5) * weight).clamp(half_h, sim::H as f32 - half_h);
    let target = px(tx, ty, 0.0);
    let shake = s.world.fx.shake * SCALE;
    let sx = (s.t * 47.0).sin() * shake;
    let sy = (s.t * 63.0).cos() * shake;
    let mut orbit = 0.0;
    let ult_banner = s.world.banner.as_ref().is_some_and(|b| b.0 != "PARRY!");
    if s.vic.is_some() {
        orbit = (s.t * 0.55).sin() * 1.5;
    } else if ult_banner && !s.selecting {
        orbit = (s.t * 1.15).sin() * 0.75;
    }
    let dist = 12.2 / zoom;
    let eye = target + Vec3::new(orbit + sx, 2.15 / zoom + sy, dist);
    *tf = Transform::from_translation(eye).looking_at(target + Vec3::new(0.0, 0.2, 0.0), Vec3::Y);
}

pub(crate) fn sync_actors(
    sim: Res<Sim>,
    mut gfx: ResMut<Gfx>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(&Actor, &mut Transform, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (actor, mut tf, mut vis, mut mat) in &mut q {
        let slot = actor.slot as usize;
        let (p, show) = if slot < 2 {
            (&sim.0.players[slot], !sim.0.selecting && sim.0.players[slot].dead_t < 0.08)
        } else {
            (&sim.0.previews[slot - 2], sim.0.selecting)
        };
        if !show {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        let crouch = p.crouch.min(1.0);
        let h = sim::SIZE * (1.0 - crouch * 0.42);
        let cx = p.x + sim::SIZE * 0.5;
        let foot = p.y + sim::SIZE;
        let cy = foot - h * 0.5;
        tf.translation = px(cx, cy, 30.0);
        tf.rotation = Quat::from_rotation_z(-p.rot.to_radians());
        tf.scale = Vec3::new(sim::SIZE * SCALE, h * SCALE, sim::SIZE * 0.85 * SCALE);
        let col = body_color(p);
        let kind = if p.kind == sim::Kind::Ghost { 2 } else { 3 };
        mat.0 = gfx.material(&mut materials, col, kind);
    }
}

fn body_color(p: &sim::Player) -> sim::Color {
    if p.hurt_t > 0.12 {
        return sim::Color::WHITE;
    }
    if p.frozen_t > 0.0 {
        return sim::Color::new(170, 220, 255, 255);
    }
    let mut c = p.kind.color();
    if p.kind == sim::Kind::Ghost {
        c.a = 160;
    }
    c
}

struct Fx<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    gfx: &'a mut Gfx,
    materials: &'a mut Assets<StandardMaterial>,
    pool: &'a mut FxPool,
}

impl Fx<'_, '_, '_> {
    fn begin(&mut self) {
        self.pool.cursor = 0;
    }
    fn end(&mut self) {
        let Fx { commands, pool, .. } = self;
        for e in pool.ents.iter().skip(pool.cursor) {
            commands.entity(*e).insert(Visibility::Hidden);
        }
    }
    fn draw(&mut self, mesh: MeshId, x: f32, y: f32, z: f32, sx: f32, sy: f32, sz: f32, rot: Quat, col: sim::Color, kind: u8) {
        if col.a == 0 {
            return;
        }
        let Fx { commands, gfx, materials, pool } = self;
        let handle = gfx.mesh(mesh).clone();
        let mat = gfx.material(materials, col, kind);
        let scale = if matches!(mesh, MeshId::Torus) {
            Vec3::splat(sx * SCALE)
        } else {
            Vec3::new(sx * SCALE, sy * SCALE, sz * SCALE)
        };
        let tf = Transform { translation: px(x, y, z), rotation: rot, scale };
        if pool.cursor < pool.ents.len() {
            let e = pool.ents[pool.cursor];
            pool.cursor += 1;
            commands.entity(e).insert((Mesh3d(handle), MeshMaterial3d(mat), tf, Visibility::Visible));
        } else {
            let e = commands.spawn((
                Mesh3d(handle),
                MeshMaterial3d(mat),
                tf,
                Visibility::Visible,
                NotShadowCaster,
                NotShadowReceiver,
            )).id();
            pool.ents.push(e);
            pool.cursor += 1;
        }
    }
}

pub(crate) fn sync_fx(
    mut commands: Commands,
    sim: Res<Sim>,
    mut gfx: ResMut<Gfx>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pool: ResMut<FxPool>,
) {
    let mut fx = Fx { commands: &mut commands, gfx: &mut gfx, materials: &mut materials, pool: &mut pool };
    fx.begin();
    let s = &sim.0;
    let fighters: Vec<&sim::Player> = if s.selecting { s.previews.iter().collect() } else { s.players.iter().collect() };
    for p in fighters {
        if !s.selecting && p.dead_t > 0.08 {
            continue;
        }
        draw_fighter(&mut fx, p);
    }
    if !s.selecting {
        draw_world(&mut fx, s);
        draw_victory(&mut fx, s);
    }
    let parts = &s.world.parts;
    let start = parts.len().saturating_sub(1400);
    for q in &parts[start..] {
        draw_particle(&mut fx, q);
    }
    fx.end();
}

fn draw_fighter(fx: &mut Fx, p: &sim::Player) {
    let (cx, cy) = p.center();
    let z = 48.0;
    let eye = match p.eyes {
        sim::Eyes::Squint => 5.0,
        sim::Eyes::UpWide => 9.0,
        _ => 7.0,
    };
    let look = match p.eyes {
        sim::Eyes::Left => -5.0,
        sim::Eyes::Right => 5.0,
        _ => 0.0,
    };
    let eye_y = cy - sim::SIZE * 0.12;
    let white = sim::Color::new(245, 245, 250, 255);
    let pupil = sim::Color::new(20, 18, 28, 255);
    for side in [-1.0_f32, 1.0] {
        let ex = cx + side * 12.0 + look;
        fx.draw(MeshId::Sphere, ex, eye_y, z, eye, eye, eye, Quat::IDENTITY, white, 1);
        fx.draw(MeshId::Sphere, ex + look * 0.15, eye_y, z + 4.0, eye * 0.45, eye * 0.45, eye * 0.45, Quat::IDENTITY, pupil, 1);
    }
    let prog = if p.melee_t > 0.0 { 1.0 - p.melee_t / sim::MELEE_TIME } else { 0.0 };
    let ang = if p.melee_t > 0.0 { (-70.0 + 160.0 * prog) * p.facing } else { -18.0 * p.facing };
    let kx = cx + p.facing * 34.0;
    let ky = cy + 6.0;
    let knife_rot = Quat::from_rotation_z((-ang).to_radians());
    fx.draw(MeshId::Cube, kx, ky, 36.0, 8.0, 36.0, 4.0, knife_rot, sim::Color::new(210, 220, 230, 255), 3);
    fx.draw(MeshId::Cube, kx - ang.to_radians().sin() * 10.0, ky - 16.0, 36.0, 7.0, 12.0, 5.0, knife_rot, sim::Color::new(90, 60, 40, 255), 0);

    if p.shield_t > 0.0 {
        fx.draw(MeshId::Sphere, cx, cy, 30.0, sim::SIZE * 2.1, sim::SIZE * 2.1, sim::SIZE * 2.1, Quat::IDENTITY, sim::Color::new(80, 220, 255, 90), 2);
    }
    if p.block_held {
        let ring = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(p.facing * 0.4);
        fx.draw(MeshId::Torus, cx + p.facing * 40.0, cy, 40.0, 34.0, 34.0, 34.0, ring, sim::Color::new(180, 220, 255, 200), 2);
    }
    if p.parry_t > 0.0 {
        fx.draw(MeshId::Torus, cx, cy, 40.0, 48.0, 48.0, 48.0, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), sim::Color::WHITE, 4);
    }
    if p.ult >= sim::ULT_MAX {
        fx.draw(MeshId::Torus, cx, cy + 8.0, 20.0, 46.0, 46.0, 46.0, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), sim::Color::new(255, 210, 60, 255), 4);
    }
    if p.dash_t > 0.0 {
        for i in 1..4 {
            let back = p.facing * -18.0 * i as f32;
            let a = 140 - i * 35;
            let mut col = p.kind.color();
            col.a = a as u8;
            fx.draw(MeshId::Cube, cx + back, cy, 24.0, sim::SIZE * 0.9, sim::SIZE * 0.9, 30.0, Quat::IDENTITY, col, 2);
        }
    }
    if p.over_t > 0.0 {
        fx.draw(MeshId::Sphere, cx, cy, 20.0, sim::SIZE * 1.8, sim::SIZE * 1.8, sim::SIZE * 1.8, Quat::IDENTITY, sim::Color::new(255, 150, 40, 50), 2);
    }
    if p.frozen_t > 0.0 {
        fx.draw(MeshId::Cube, cx, cy, 30.0, sim::SIZE * 1.25, sim::SIZE * 1.25, sim::SIZE, Quat::IDENTITY, sim::Color::new(180, 230, 255, 70), 2);
    }
}

fn draw_world(fx: &mut Fx, s: &sim::session::Session) {
    let theme = sim::THEMES[s.theme_idx];
    let rise = sim::lava_rise(theme, s.rd.time);
    if let Some(hz) = s.map.hazard {
        let y = hz.y - rise;
        let h = hz.height + rise;
        fx.draw(MeshId::Cube, hz.x + hz.width * 0.5, y + h * 0.5, 8.0, hz.width, h.max(8.0), 90.0, Quat::IDENTITY, sim::Color::new(255, 80, 20, 230), 4);
    }
    if sim::MODES[s.mode_idx] == sim::Mode::Hill {
        if let Some(z) = s.map.zones.get(s.rd.zone_idx) {
            fx.draw(MeshId::Cube, z.x + z.width * 0.5, z.y + 6.0, 24.0, z.width, 8.0, 60.0, Quat::IDENTITY, sim::Color::new(255, 220, 60, 160), 2);
        }
    }
    for b in &s.world.booms {
        let k = (b.t / sim::BOOM_TIME).clamp(0.0, 1.0);
        let mut col = b.color;
        col.a = ((1.0 - k) * 220.0) as u8;
        let rot = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        fx.draw(MeshId::Torus, b.x, b.y, 16.0, b.r * (0.25 + k), 1.0, 1.0, rot, col, 2);
    }
    for p in &s.world.projs {
        let (d, col) = match p.kind {
            sim::ProjKind::Bomb => (22.0, sim::Color::new(40, 40, 46, 255)),
            sim::ProjKind::Frost => (16.0, sim::Color::new(140, 220, 255, 255)),
            sim::ProjKind::Fire => (18.0, sim::Color::new(255, 120, 30, 255)),
        };
        fx.draw(MeshId::Sphere, p.x, p.y, 36.0, d, d, d, Quat::IDENTITY, col, 4);
    }
    for m in &s.world.meteors {
        if m.falling {
            fx.draw(MeshId::Sphere, m.x, m.y, 40.0, 28.0, 28.0, 28.0, Quat::IDENTITY, sim::Color::new(255, 90, 30, 255), 4);
        } else {
            fx.draw(MeshId::Torus, m.tx, sim::H as f32 - sim::FLOOR_H - 6.0, 20.0, 28.0, 28.0, 28.0, Quat::from_rotation_x(1.1), sim::Color::new(255, 60, 40, 180), 2);
        }
    }
    for laser in &s.world.lasers {
        let col = if laser.fired { sim::Color::new(255, 240, 255, 210) } else { sim::Color::new(255, 40, 80, 90) };
        let thick = if laser.fired { 18.0 } else { 4.0 };
        fx.draw(MeshId::Cube, sim::W as f32 * 0.5, laser.y, 50.0, sim::W as f32, thick, 8.0, Quat::IDENTITY, col, 2);
    }
    for st in &s.world.strikes {
        if !st.struck {
            fx.draw(MeshId::Cube, st.x, sim::H as f32 * 0.45, 10.0, 10.0, sim::H as f32 * 0.7, 6.0, Quat::IDENTITY, sim::Color::new(255, 255, 180, 40), 2);
        } else {
            let n = st.pts.len().min(10);
            for i in 0..n {
                let p = st.pts[i];
                fx.draw(MeshId::Cube, p.x, p.y, 55.0, 8.0, 28.0, 8.0, Quat::IDENTITY, sim::Color::new(255, 250, 180, 230), 4);
            }
        }
    }
    for pk in &s.world.pickups {
        let bob = (pk.age * 3.0).sin() * 8.0;
        fx.draw(MeshId::Sphere, pk.x, pk.y + bob, 36.0, 22.0, 22.0, 22.0, Quat::IDENTITY, pk.kind.color(), 4);
    }
}

fn draw_victory(fx: &mut Fx, s: &sim::session::Session) {
    let Some(v) = &s.vic else { return };
    let p = &s.players[v.who];
    let (cx, cy) = p.center();
    let ground = sim::H as f32 - sim::FLOOR_H;
    match v.idx {
        0 => {
            let scale = 2.4;
            let len = 92.0 * scale;
            let kx = cx + 130.0;
            let y0 = -len - 40.0;
            let y1 = ground + 22.0 - len;
            let hy = if v.t < 0.9 { y0 + (y1 - y0) * sim::ease(v.t / 0.9) } else { y1 };
            fx.draw(MeshId::Cube, kx, hy + len * 0.5, 50.0, 16.0 * scale * 0.35, len, 10.0, Quat::IDENTITY, sim::Color::new(230, 236, 245, 255), 3);
            fx.draw(MeshId::Cube, kx, hy + 18.0, 50.0, 22.0, 28.0, 14.0, Quat::IDENTITY, sim::Color::new(90, 58, 36, 255), 0);
        }
        4 => {
            let spread = 70.0 + 20.0 * (v.t * 2.0).sin();
            let wing = Quat::from_rotation_z(0.7);
            fx.draw(MeshId::Cube, cx - spread, cy + 10.0, 20.0, 90.0, 18.0, 8.0, wing, sim::Color::new(255, 120, 30, 200), 2);
            fx.draw(MeshId::Cube, cx + spread, cy + 10.0, 20.0, 90.0, 18.0, 8.0, Quat::from_rotation_z(-0.7), sim::Color::new(255, 90, 20, 200), 2);
        }
        6 => {
            fx.draw(MeshId::Cube, cx, cy - 80.0, 10.0, 80.0, 280.0, 80.0, Quat::IDENTITY, sim::Color::new(255, 230, 140, 28), 2);
            fx.draw(MeshId::Torus, cx, ground - 8.0, 24.0, 70.0, 70.0, 70.0, Quat::from_rotation_x(1.15), sim::Color::new(255, 210, 80, 180), 4);
        }
        8 if v.t >= 0.95 && v.t < 1.7 => {
            let ang = (sim::H as f32).atan2(sim::W as f32);
            let len = (sim::W as f32).hypot(sim::H as f32);
            fx.draw(MeshId::Cube, sim::W as f32 * 0.5, sim::H as f32 * 0.5, 60.0, 16.0, len, 8.0, Quat::from_rotation_z(-ang), sim::Color::new(200, 245, 255, 230), 4);
        }
        _ => {
            let _ = (cx, cy);
        }
    }
}

fn draw_particle(fx: &mut Fx, q: &sim::Particle) {
    let fade = (q.life / q.max).clamp(0.0, 1.0);
    let mut col = q.color;
    col.a = ((col.a as f32) * fade) as u8;
    if col.a < 8 {
        return;
    }
    let grow = if q.grow { 1.0 + (1.0 - fade) } else { 1.0 };
    let size = q.size * grow;
    let kind = if col.a < 250 { 2 } else { 1 };
    match q.shape {
        sim::Shape::Circle => fx.draw(MeshId::Sphere, q.x, q.y, 42.0, size * 2.0, size * 2.0, size * 2.0, Quat::IDENTITY, col, kind),
        sim::Shape::Square => fx.draw(MeshId::Cube, q.x, q.y, 42.0, size, size, size * 0.6, Quat::from_rotation_z(-q.rot.to_radians()), col, kind),
        sim::Shape::Star => {
            let rot = Quat::from_rotation_z(-q.rot.to_radians());
            fx.draw(MeshId::Cube, q.x, q.y, 44.0, size * 1.6, size * 0.45, size * 0.4, rot, col, 4);
        }
        sim::Shape::Streak => {
            let vel = Vec3::new(q.vx, -q.vy, 0.0);
            let speed = vel.length().max(1.0);
            let dir = vel / speed;
            let len = (speed * 0.06).max(size);
            let rot = Quat::from_rotation_arc(Vec3::Y, dir);
            fx.draw(MeshId::Cube, q.x, q.y, 42.0, size * 0.45, len, size * 0.45, rot, col, kind);
        }
        sim::Shape::Ghost => {
            col.a = col.a.min(150);
            fx.draw(MeshId::Cube, q.x, q.y, 28.0, sim::SIZE, sim::SIZE, sim::SIZE * 0.7, Quat::from_rotation_z(-q.rot.to_radians()), col, 2);
        }
    }
}
