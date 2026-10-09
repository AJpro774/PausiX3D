//! Bevy app: input, the simulation step, the 3D view, and the benchmark.

use super::config::Cfg;
use super::session::{FrameInput, Pad, Session};
use super::{ui, view};
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::WinitSettings;

#[derive(Resource)]
pub(crate) struct Sim(pub Session);

#[derive(Resource, Default)]
struct Pads(FrameInput);

#[derive(Resource)]
struct Bench {
    secs: f32,
    warmup: f32,
    stable: f32,
    measuring: bool,
    samples: Vec<f32>,
    finished: bool,
}

#[derive(Resource)]
struct Shot {
    after: f32,
    path: String,
    elapsed: f32,
    taken: bool,
    wait: f32,
}

pub(crate) fn run(cfg: Cfg) {
    let bench = cfg.benchmark_secs.map(|n| n as f32);
    let mut session = Session::new();
    session.apply(&cfg);
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "PausiX3D".into(),
            resolution: WindowResolution::new(1280, 720),
            present_mode: if bench.is_some() { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync },
            resizable: true,
            ..default()
        }),
        ..default()
    }));
    if bench.is_some() {
        app.insert_resource(WinitSettings::continuous());
    }
    app.insert_resource(Sim(session))
        .insert_resource(Pads::default())
        .insert_resource(ClearColor(Color::srgb_u8(20, 24, 32)))
        .add_systems(Startup, view::setup)
        .add_systems(
            Update,
            (
                read_input,
                drive,
                view::sync_stage,
                view::sync_actors,
                view::sync_fx,
                view::bob_system,
                view::sync_camera,
                ui::draw_ui,
                bench_system,
                shot_system,
            )
                .chain(),
        );
    if let Some(secs) = bench {
        app.insert_resource(Bench { secs, warmup: 0.0, stable: 0.0, measuring: false, samples: Vec::new(), finished: false });
    }
    if bench.is_none() {
        if let (Some(after), Some(path)) = (cfg.screenshot_after, cfg.screenshot_path) {
            app.insert_resource(Shot { after, path, elapsed: 0.0, taken: false, wait: 0.0 });
        }
    }
    app.run();
}

fn read_input(keys: Res<ButtonInput<KeyCode>>, mut pads: ResMut<Pads>) {
    let p1 = Pad {
        left: keys.pressed(KeyCode::KeyA),
        right: keys.pressed(KeyCode::KeyD),
        down: keys.pressed(KeyCode::KeyS),
        jump: keys.just_pressed(KeyCode::KeyW),
        a1: keys.just_pressed(KeyCode::KeyF),
        a2: keys.just_pressed(KeyCode::KeyG),
        melee: keys.just_pressed(KeyCode::KeyE),
        block: keys.pressed(KeyCode::KeyQ),
        ult: keys.just_pressed(KeyCode::KeyV),
        up_p: keys.just_pressed(KeyCode::KeyW),
        down_p: keys.just_pressed(KeyCode::KeyS),
        left_p: keys.just_pressed(KeyCode::KeyA),
        right_p: keys.just_pressed(KeyCode::KeyD),
    };
    let p2 = Pad {
        left: keys.pressed(KeyCode::ArrowLeft),
        right: keys.pressed(KeyCode::ArrowRight),
        down: keys.pressed(KeyCode::ArrowDown),
        jump: keys.just_pressed(KeyCode::ArrowUp),
        a1: keys.just_pressed(KeyCode::Comma),
        a2: keys.just_pressed(KeyCode::Period),
        melee: keys.just_pressed(KeyCode::Slash),
        block: keys.pressed(KeyCode::ShiftRight),
        ult: keys.just_pressed(KeyCode::ControlRight),
        up_p: keys.just_pressed(KeyCode::ArrowUp),
        down_p: keys.just_pressed(KeyCode::ArrowDown),
        left_p: keys.just_pressed(KeyCode::ArrowLeft),
        right_p: keys.just_pressed(KeyCode::ArrowRight),
    };
    pads.0 = FrameInput {
        p1,
        p2,
        ready: [keys.just_pressed(KeyCode::KeyF), keys.just_pressed(KeyCode::KeyL)],
        map_key: keys.just_pressed(KeyCode::KeyM),
        rounds_key: keys.just_pressed(KeyCode::KeyN),
        mode_key: keys.just_pressed(KeyCode::KeyB),
        players_key: keys.just_pressed(KeyCode::KeyC),
        cpu_key: [keys.just_pressed(KeyCode::Digit1), keys.just_pressed(KeyCode::Digit2)],
        space: keys.just_pressed(KeyCode::Space),
        enter: keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter),
        back: keys.just_pressed(KeyCode::KeyR),
        esc: keys.just_pressed(KeyCode::Escape),
    };
}

fn drive(mut sim: ResMut<Sim>, pads: Res<Pads>, time: Res<Time>, mut exit: MessageWriter<AppExit>) {
    sim.0.tick(&pads.0, time.delta_secs());
    if sim.0.quit {
        exit.write(AppExit::Success);
    }
}

fn bench_system(time: Res<Time>, mut bench: Option<ResMut<Bench>>, windows: Query<&Window>, mut exit: MessageWriter<AppExit>) {
    let Some(bench) = bench.as_mut() else { return };
    if bench.finished {
        return;
    }
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    if !bench.measuring {
        bench.warmup += dt;
        if dt < 0.05 {
            bench.stable += dt;
        } else {
            bench.stable = 0.0;
        }
        let settled = bench.stable >= 1.0 && bench.warmup >= 1.0;
        if settled || bench.warmup >= 15.0 {
            bench.measuring = true;
        }
        return;
    }
    bench.samples.push(dt);
    let elapsed: f32 = bench.samples.iter().sum();
    if elapsed < bench.secs {
        return;
    }
    bench.finished = true;
    let present = windows.iter().next().map(|w| format!("{:?}", w.present_mode)).unwrap_or_else(|| "unknown".into());
    let report = report(&bench.samples, bench.warmup, &present);
    println!("\n{report}");
    let path = std::env::current_dir().map(|d| d.join("benchmark.txt")).unwrap_or_else(|_| "benchmark.txt".into());
    match std::fs::write(&path, &report) {
        Ok(()) => {
            println!("wrote {}", path.display());
            exit.write(AppExit::Success);
        }
        Err(e) => {
            eprintln!("could not write {}: {e}", path.display());
            exit.write(AppExit::error());
        }
    }
}

fn report(samples: &[f32], warmup: f32, present: &str) -> String {
    let n = samples.len().max(1);
    let sum: f32 = samples.iter().copied().sum();
    let avg = sum / n as f32;
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min_dt = sorted.first().copied().unwrap_or(avg);
    let max_dt = sorted.last().copied().unwrap_or(avg);
    let p99_i = ((n as f32) * 0.99).ceil() as usize - 1;
    let p99 = sorted[p99_i.min(n - 1)];
    let worst_n = ((n as f32) * 0.01).ceil().max(1.0) as usize;
    let worst = &sorted[n - worst_n..];
    let worst_avg = worst.iter().sum::<f32>() / worst.len() as f32;
    let fps = |dt: f32| if dt > 1e-6 { 1.0 / dt } else { 0.0 };
    format!(
        "\
PausiX3D benchmark
map: VOLCANO
match: CPU BERSERKER Bomber (Bomb/Fireball, smoke trail) vs CPU BERSERKER Spark (Thunder/Dash, sparkle trail)
both ultimates start full; carpet bombs, lightning, lava, meteors, and dense trails
resolution: 1280x720
present mode: {present}
vsync requested: off (AutoNoVsync)
warmup excluded: {warmup:.2} s (shader compile, until frames settle or 15s)
measured: {sum:.2} s
frames: {n}
average fps: {avg_fps:.1}
1% low fps: {low:.1}
min fps: {min_fps:.1}
frame time avg ms: {avg_ms:.2}
frame time min ms: {min_ms:.2}
frame time max ms: {max_ms:.2}
frame time p99 ms: {p99_ms:.2}
1% low is the average fps of the slowest 1% of frames
",
        avg_fps = fps(avg),
        low = fps(worst_avg),
        min_fps = fps(max_dt),
        avg_ms = avg * 1000.0,
        min_ms = min_dt * 1000.0,
        max_ms = max_dt * 1000.0,
        p99_ms = p99 * 1000.0,
    )
}

fn shot_system(time: Res<Time>, mut shot: Option<ResMut<Shot>>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    let Some(shot) = shot.as_mut() else { return };
    let dt = time.delta_secs();
    if !shot.taken {
        shot.elapsed += dt;
        if shot.elapsed >= shot.after {
            let path = shot.path.clone();
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
            shot.taken = true;
        }
        return;
    }
    shot.wait += dt;
    let ready = std::path::Path::new(&shot.path).is_file() && shot.wait > 0.4;
    if ready || shot.wait > 4.0 {
        println!("screenshot {}", shot.path);
        exit.write(AppExit::Success);
    }
}
