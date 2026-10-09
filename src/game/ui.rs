//! Flat overlay copied from the original raylib HUD and character select.
//! Positions, sizes, and colors follow draw_hud and the menu block in main.

use super as sim;
use super::app::Sim;
use super::session::ROUND_OPTIONS;
use super::view;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Component)]
pub(crate) struct Hud;

pub(crate) fn draw_ui(
    mut commands: Commands,
    sim: Res<Sim>,
    pose: Res<view::CamPose>,
    windows: Query<&Window, With<PrimaryWindow>>,
    old: Query<Entity, With<Hud>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let s = &sim.0;
    if s.selecting {
        menu(&mut commands, s);
    } else {
        let aspect = windows
            .iter()
            .next()
            .map(|w| w.width() / w.height().max(1.0))
            .unwrap_or(1280.0 / 720.0);
        fight(&mut commands, s, &pose, aspect);
    }
    if s.world.fx.flash > 0.02 {
        rect(&mut commands, 0.0, 0.0, 1280.0, 720.0, c(255, 255, 255, (s.world.fx.flash.min(1.0) * 200.0) as u8));
    }
}

fn menu(commands: &mut Commands, s: &sim::session::Session) {
    let theme = sim::THEMES[s.theme_idx];
    center(commands, "PAUSI", 8.0, 56.0, sim::Color::WHITE);
    center(
        commands,
        &format!("STARTING MAP: {}  ({})   - press M to change", theme.name(), theme.blurb()),
        70.0,
        22.0,
        sim::Color::YELLOW,
    );
    let cpu = [s.cpu_mode == 2, s.cpu_mode >= 1];
    for i in 0..2 {
        let px = 40.0 + i as f32 * 640.0;
        let py = 100.0;
        rect(commands, px, py, 600.0, 460.0, c(0, 0, 0, 170));
        frame(commands, px, py, 600.0, 460.0, sim::Color::WHITE);
        let label = if cpu[i] {
            format!("PLAYER {} (CPU: {})", i + 1, sim::CPU_NAMES[s.cpu_prof[i]])
        } else {
            format!("PLAYER {}", i + 1)
        };
        text(commands, &label, px + 16.0, py + 8.0, 28.0, s.previews[i].kind.color());
        if s.ready[i] || cpu[i] {
            text(commands, "READY!", px + 440.0, py + 8.0, 30.0, sim::Color::LIME);
        }
        let setup = s.setups[i];
        let values = [
            sim::KINDS[setup.kind].name(),
            sim::ABILITIES[setup.abil[0]].name(),
            sim::ABILITIES[setup.abil[1]].name(),
            sim::TRAIL_STYLES[setup.trail].name(),
            sim::TRAIL_COLOR_NAMES[setup.color],
            sim::DEATH_NAMES[setup.death],
            sim::VICTORY_NAMES[setup.victory],
        ];
        for r in 0..sim::ROWS {
            let ry = py + 44.0 + r as f32 * 36.0;
            let active = s.row[i] == r && !s.ready[i];
            if active {
                rect(commands, px + 8.0, ry - 3.0, 584.0, 32.0, c(255, 255, 255, 60));
            }
            text(commands, sim::ROW_NAMES[r], px + 20.0, ry, 22.0, sim::Color::WHITE);
            let val = if active { format!("<  {}  >", values[r]) } else { values[r].to_string() };
            text(commands, &val, px + 190.0, ry, 24.0, if active { sim::Color::YELLOW } else { sim::Color::LIGHTGRAY });
        }
        let kind = sim::KINDS[setup.kind];
        let ks = if i == 0 { ("F", "G") } else { ("COMMA", "PERIOD") };
        let a1 = sim::ABILITIES[setup.abil[0]];
        let a2 = sim::ABILITIES[setup.abil[1]];
        let by = py + 304.0;
        text(commands, &format!("{}: {} - {}", ks.0, a1.name(), a1.blurb()), px + 20.0, by, 20.0, sim::Color::WHITE);
        text(commands, &format!("{}: {} - {}", ks.1, a2.name(), a2.blurb()), px + 20.0, by + 24.0, 20.0, sim::Color::WHITE);
        let (knife, block, ult) = if i == 0 { ("E", "Q", "V") } else { ("SLASH", "R-SHIFT", "R-CTRL") };
        text(commands, &format!("{}: KNIFE   {}: BLOCK (tap = parry)", knife, block), px + 20.0, by + 48.0, 20.0, sim::Color::WHITE);
        text(
            commands,
            &format!("{}: ULT {} - {}", ult, kind.ult_name(), kind.ult_blurb()),
            px + 20.0,
            by + 72.0,
            20.0,
            c(255, 220, 90, 255),
        );
        text(commands, kind.passive(), px + 20.0, by + 96.0, 20.0, c(150, 255, 170, 255));
        let (tag, blurb) = match s.row[i] {
            5 => ("DEATH", sim::DEATH_BLURBS[setup.death]),
            6 => ("VICTORY", sim::VICTORY_BLURBS[setup.victory]),
            _ => ("", ""),
        };
        let note = if blurb.is_empty() && cpu[i] {
            format!("CPU {} ({}): {}", sim::CPU_NAMES[s.cpu_prof[i]], if i == 0 { "key 1" } else { "key 2" }, sim::CPU_BLURBS[s.cpu_prof[i]])
        } else if blurb.is_empty() {
            format!("DEATH: {}   VICTORY: {}", sim::DEATH_NAMES[setup.death], sim::VICTORY_NAMES[setup.victory])
        } else {
            format!("{tag}: {blurb}")
        };
        text(commands, &note, px + 20.0, by + 120.0, 20.0, c(255, 170, 230, 255));
    }
    text(commands, "P1: W/S pick row  A/D change  F ready", 40.0, 572.0, 24.0, sim::Color::YELLOW);
    text(commands, "P2: Up/Down pick row  Left/Right change  L ready", 640.0, 572.0, 24.0, sim::Color::YELLOW);
    let mode = sim::MODES[s.mode_idx];
    let players = match s.cpu_mode {
        0 => "HUMAN vs HUMAN",
        1 => "HUMAN vs CPU",
        _ => "CPU vs CPU",
    };
    center(
        commands,
        &format!("ROUNDS: {} (N)    MODE: {} (B)    PLAYERS: {} (C)", ROUND_OPTIONS[s.rounds_idx], mode.name(), players),
        604.0,
        26.0,
        sim::Color::WHITE,
    );
    center(commands, mode.blurb(), 636.0, 22.0, sim::Color::LIGHTGRAY);
    let go = match s.cpu_mode {
        0 => "Every round switches to the next map. Both ready = fight!",
        1 => "Press F to fight the CPU!  (key 2 changes its personality)",
        _ => "CPU vs CPU: keys 1 and 2 change their personalities - press F or L to start and watch",
    };
    center(commands, go, 664.0, 22.0, sim::Color::LIME);
}

fn fight(commands: &mut Commands, s: &sim::session::Session, pose: &view::CamPose, aspect: f32) {
    let world_text = |commands: &mut Commands, msg: &str, x: f32, y: f32, size: f32, col: sim::Color| {
        let Some((sx, sy, depth)) = view::project_screen(pose, x, y, 40.0, aspect) else { return };
        let scale = (view::CAM_DIST / depth).clamp(0.35, 3.0);
        text(commands, msg, sx, sy, size * scale, col);
    };

    if s.vic.is_none() {
        let cpu = [s.cpu_mode == 2, s.cpu_mode >= 1];
        let tags = [
            if cpu[0] { Some(sim::CPU_NAMES[s.cpu_now[0]]) } else { None },
            if cpu[1] { Some(sim::CPU_NAMES[s.cpu_now[1]]) } else { None },
        ];
        hud(commands, s, &tags);
        let mode = sim::MODES[s.mode_idx];
        let theme = sim::THEMES[s.theme_idx];
        center(commands, &format!("ROUND {} / {}", s.round_no, ROUND_OPTIONS[s.rounds_idx]), 12.0, 24.0, sim::Color::WHITE);
        center(commands, &format!("{}  -  {}", s.wins[0], s.wins[1]), 42.0, 44.0, sim::Color::YELLOW);
        center(commands, &format!("{} - {}", theme.name(), mode.name()), 92.0, 18.0, sim::Color::LIGHTGRAY);
        match mode {
            sim::Mode::Timed => {
                let left = (sim::TIME_LIMIT - s.rd.time).max(0.0);
                let col = if left < 10.0 { c(255, 90, 80, 255) } else { sim::Color::WHITE };
                center(commands, &format!("{:.0}", left.ceil()), 118.0, 40.0, col);
            }
            sim::Mode::Hill => {
                let left = (sim::HILL_TIME - s.rd.time).max(0.0);
                center(
                    commands,
                    &format!("HILL  {:.0}  -  {:.0}   ({:.0}s)", s.rd.hill[0], s.rd.hill[1], left.ceil()),
                    118.0,
                    26.0,
                    c(255, 230, 90, 255),
                );
                center(commands, &format!("first to {:.0}", sim::HILL_TARGET), 148.0, 18.0, sim::Color::LIGHTGRAY);
            }
            _ => {}
        }
        let wind_name = match theme {
            sim::Theme::Frozen => "BLIZZARD",
            sim::Theme::Sea => "CURRENT",
            _ => "WIND",
        };
        if s.rd.wind_warn {
            center(commands, &format!("{wind_name} INCOMING!"), 150.0, 28.0, sim::Color::WHITE);
        } else if s.rd.wind != 0.0 {
            let arrow = if s.rd.wind > 0.0 { format!("{wind_name}  >>>") } else { format!("<<<  {wind_name}") };
            center(commands, &arrow, 150.0, 28.0, sim::Color::WHITE);
        }
        if theme == sim::Theme::Space {
            if s.rd.zerog_warn {
                center(commands, "ZERO-G INCOMING!", 150.0, 28.0, sim::Color::WHITE);
            } else if s.rd.zerog {
                center(commands, "ZERO-G!", 150.0, 28.0, c(160, 220, 255, 255));
            }
        }
        let rise = sim::lava_rise(theme, s.rd.time);
        if theme == sim::Theme::Volcano && rise > 5.0 {
            center(commands, "LAVA SURGE!", 176.0, 28.0, c(255, 130, 40, 255));
        }
        text(commands, "P1: A/D move  W jump (again = double, walls too)  S charge", 15.0, 720.0 - 56.0, 20.0, sim::Color::WHITE);
        text(commands, "F/G abilities  E knife  Q block  V ultimate", 15.0, 720.0 - 30.0, 20.0, sim::Color::WHITE);
        let p2a = "P2: arrows move/jump/charge";
        let p2b = "COMMA/PERIOD abilities  SLASH knife  R-SHIFT block  R-CTRL ult";
        text(commands, p2a, 1280.0 - width(p2a, 20.0) - 15.0, 720.0 - 56.0, 20.0, sim::Color::WHITE);
        text(commands, p2b, 1280.0 - width(p2b, 20.0) - 15.0, 720.0 - 30.0, 20.0, sim::Color::WHITE);
        for i in 0..2 {
            if s.players[i].dead_t > 0.0 && s.result.is_none() {
                let x = if i == 0 { 1280.0 / 4.0 } else { 3.0 * 1280.0 / 4.0 };
                let msg = format!("P{} RESPAWN {:.1}", i + 1, s.players[i].dead_t);
                text(commands, &msg, x - width(&msg, 28.0) / 2.0, 360.0, 28.0, sim::Color::WHITE);
            }
        }
    }

    if let Some(v) = s.vic.as_ref() {
        let bar = 90.0 * sim::ease(v.t / 0.4);
        rect(commands, 0.0, 0.0, 1280.0, bar, sim::Color::BLACK);
        rect(commands, 0.0, 720.0 - bar, 1280.0, bar + 1.0, sim::Color::BLACK);
        let who = v.who;
        let title = format!("P{} {}  -  {}", who + 1, s.players[who].kind.name(), sim::VICTORY_NAMES[v.idx]);
        center(commands, &title, 28.0, 30.0, sim::Color::YELLOW);
        center(commands, "SPACE = skip", 720.0 - 60.0, 20.0, sim::Color::LIGHTGRAY);
    }

    if let Some((name, col, left)) = &s.world.banner {
        let a = (left / 0.4).clamp(0.0, 1.0);
        rect(commands, 0.0, 250.0, 1280.0, 100.0, c(0, 0, 0, (150.0 * a) as u8));
        let bw = width(name, 70.0);
        text(commands, name, 640.0 - bw / 2.0, 265.0, 70.0, sim::with_alpha(*col, (255.0 * a) as u8));
    }

    if s.ko_timer > 0.0 {
        let elapsed = sim::KO_TIME - s.ko_timer;
        let bar = 90.0 * sim::ease(elapsed / 0.4);
        rect(commands, 0.0, 0.0, 1280.0, bar, sim::Color::BLACK);
        rect(commands, 0.0, 720.0 - bar, 1280.0, bar + 1.0, sim::Color::BLACK);
        let size = 170.0 - 60.0 * sim::ease(elapsed / 0.3);
        let jx = (s.t * 40.0).sin() * 4.0;
        text(commands, "K.O.!", 640.0 - width("K.O.!", size) / 2.0 + jx, 360.0 - size / 2.0, size, sim::Color::YELLOW);
    }

    if s.ko_timer <= 0.0 && s.vic.is_none() && s.pending_vic.is_none() {
        if let Some(msg) = &s.result {
            rect(commands, 0.0, 190.0, 1280.0, 260.0, c(0, 0, 0, 150));
            center(commands, msg, 205.0, 50.0, sim::Color::YELLOW);
            center(commands, &format!("TALLY   P1 {}  -  {} P2", s.wins[0], s.wins[1]), 285.0, 40.0, sim::Color::WHITE);
            if s.series_over {
                let final_text = if s.wins[0] > s.wins[1] {
                    format!("P1 WINS THE MATCH {} - {}", s.wins[0], s.wins[1])
                } else if s.wins[1] > s.wins[0] {
                    format!("P2 WINS THE MATCH {} - {}", s.wins[1], s.wins[0])
                } else {
                    format!("MATCH DRAWN {} - {}", s.wins[0], s.wins[1])
                };
                center(commands, &final_text, 345.0, 48.0, sim::Color::LIME);
                center(commands, "Press R to go back to the menu", 405.0, 28.0, sim::Color::WHITE);
            } else {
                let next = sim::THEMES[sim::cycle(s.theme_idx, sim::THEMES.len(), 1)].name();
                center(commands, &format!("Next map: {next}"), 350.0, 30.0, sim::Color::LIGHTGRAY);
                center(commands, "Press SPACE for the next round  (R = menu)", 395.0, 28.0, sim::Color::WHITE);
            }
        }
    }

    let ground = 720.0 - sim::FLOOR_H;
    for st in &s.world.strikes {
        if !st.struck {
            world_text(commands, "!", st.x - 8.0, 24.0, 44.0, c(255, 70, 60, 255));
        }
    }
    for m in &s.world.meteors {
        if !m.falling {
            world_text(commands, "!", m.tx - 7.0, ground - 70.0, 40.0, c(255, 90, 60, 255));
        }
    }
    for l in &s.world.lasers {
        if !l.fired {
            world_text(commands, "!", 14.0, l.y - 22.0, 44.0, c(255, 80, 80, 255));
            world_text(commands, "!", 1280.0 - 34.0, l.y - 22.0, 44.0, c(255, 80, 80, 255));
        }
    }
    if sim::MODES[s.mode_idx] == sim::Mode::Hill {
        if let Some(z) = s.map.zones.get(s.rd.zone_idx) {
            world_text(commands, "HILL", z.x + z.width * 0.5 - 30.0, z.y + 6.0, 26.0, c(255, 240, 120, 255));
        }
    }
    for pk in &s.world.pickups {
        let bob = (pk.age * 4.0).sin() * 5.0;
        world_text(commands, pk.kind.label(), pk.x - 8.0, pk.y + bob - 13.0, 28.0, sim::Color::WHITE);
    }
}

fn hud(commands: &mut Commands, s: &sim::session::Session, tags: &[Option<&str>; 2]) {
    let mode = sim::MODES[s.mode_idx];
    let labels = [("F", "G"), ("COMMA", "PERIOD")];
    let ult_keys = ["V", "R-CTRL"];
    for (i, p) in s.players.iter().enumerate() {
        let bar_w = 420.0;
        let x = if i == 0 { 20.0 } else { 1280.0 - 20.0 - bar_w };
        rect(commands, x - 3.0, 17.0, bar_w + 6.0, 30.0, sim::Color::DARKGRAY);
        let fill = bar_w * (p.hp / p.max_hp).clamp(0.0, 1.0);
        let fx = if i == 0 { x } else { x + bar_w - fill };
        rect(commands, fx, 20.0, fill, 24.0, p.kind.color());
        plain(commands, &format!("{:.0}", p.hp.ceil()), x + 6.0, 22.0, 20.0, sim::Color::WHITE);
        let name = match tags[i] {
            Some(tag) => format!("P{} {} [CPU {}]", i + 1, p.kind.name(), tag),
            None => format!("P{} {}", i + 1, p.kind.name()),
        };
        text(commands, &name, x, 52.0, if tags[i].is_some() { 22.0 } else { 28.0 }, sim::Color::WHITE);
        if mode == sim::Mode::Stock {
            for k in 0..sim::STOCKS {
                let col = if k < s.rd.stocks[i] { c(255, 90, 90, 255) } else { c(60, 60, 60, 255) };
                let dx = if i == 0 { x + 300.0 + k as f32 * 24.0 } else { x + 330.0 - k as f32 * 24.0 };
                dot(commands, dx, 66.0, 9.0, col);
            }
        }
        let cds = [
            (p.cd1, p.abilities[0].cooldown(), format!("{}: {}", labels[i].0, p.abilities[0].name())),
            (p.cd2, p.abilities[1].cooldown(), format!("{}: {}", labels[i].1, p.abilities[1].name())),
        ];
        for (k, (cd, max, label)) in cds.iter().enumerate() {
            let px = x + k as f32 * 215.0;
            rect(commands, px - 2.0, 82.0, 205.0, 32.0, sim::Color::BLACK);
            let ready = 1.0 - cd / max;
            let col = if *cd <= 0.0 { c(40, 160, 40, 255) } else { sim::Color::GRAY };
            rect(commands, px, 84.0, 201.0 * ready, 28.0, col);
            text(commands, label, px + 8.0, 86.0, 22.0, sim::Color::WHITE);
        }
        rect(commands, x - 2.0, 118.0, 424.0, 18.0, sim::Color::BLACK);
        let full = p.ult >= sim::ULT_MAX;
        let ucol = if full { sim::hsv(s.t * 300.0) } else { c(230, 190, 40, 255) };
        rect(commands, x, 120.0, 420.0 * p.ult / sim::ULT_MAX, 14.0, ucol);
        let label = if full {
            format!("{}: {} READY!", ult_keys[i], p.kind.ult_name())
        } else {
            format!("ULTIMATE {:.0}%", p.ult)
        };
        text(commands, &label, x + 6.0, 118.0, 16.0, sim::Color::WHITE);
        let mut by = 142.0;
        if p.power_t > 0.0 {
            text(commands, &format!("POWER {:.0}s", p.power_t.ceil()), x, by, 20.0, c(255, 90, 80, 255));
            by += 22.0;
        }
        if p.haste_t > 0.0 {
            text(commands, &format!("HASTE {:.0}s", p.haste_t.ceil()), x, by, 20.0, c(80, 210, 255, 255));
            by += 22.0;
        }
        if s.rd.combo_show[i] > 0.0 && s.rd.combo[i] >= 2 {
            text(commands, &format!("x{} COMBO!", s.rd.combo[i]), x, by, 30.0, sim::Color::YELLOW);
        }
    }
}

fn width(s: &str, size: f32) -> f32 {
    s.len() as f32 * size * 0.55
}

fn c(r: u8, g: u8, b: u8, a: u8) -> sim::Color {
    sim::Color::new(r, g, b, a)
}

fn bevy(col: sim::Color) -> Color {
    Color::srgba_u8(col.r, col.g, col.b, col.a)
}

fn text(commands: &mut Commands, s: &str, x: f32, y: f32, size: f32, col: sim::Color) {
    for (ox, oy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0), (-2.0, -2.0), (2.0, 2.0), (-2.0, 2.0), (2.0, -2.0)] {
        plain(commands, s, x + ox, y + oy, size, sim::Color::BLACK);
    }
    plain(commands, s, x, y, size, col);
}

fn center(commands: &mut Commands, s: &str, y: f32, size: f32, col: sim::Color) {
    text(commands, s, 640.0 - width(s, size) / 2.0, y, size, col);
}

fn plain(commands: &mut Commands, s: &str, x: f32, y: f32, size: f32, col: sim::Color) {
    commands.spawn((
        Hud,
        Text::new(s),
        TextFont::from_font_size(size),
        TextColor(bevy(col)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            ..default()
        },
    ));
}

fn rect(commands: &mut Commands, x: f32, y: f32, w: f32, h: f32, col: sim::Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    commands.spawn((
        Hud,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(w),
            height: Val::Px(h),
            ..default()
        },
        BackgroundColor(bevy(col)),
    ));
}

fn frame(commands: &mut Commands, x: f32, y: f32, w: f32, h: f32, col: sim::Color) {
    rect(commands, x, y, w, 2.0, col);
    rect(commands, x, y + h - 2.0, w, 2.0, col);
    rect(commands, x, y, 2.0, h, col);
    rect(commands, x + w - 2.0, y, 2.0, h, col);
}

fn dot(commands: &mut Commands, cx: f32, cy: f32, r: f32, col: sim::Color) {
    commands.spawn((
        Hud,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cx - r),
            top: Val::Px(cy - r),
            width: Val::Px(r * 2.0),
            height: Val::Px(r * 2.0),
            border_radius: BorderRadius::all(Val::Px(r)),
            ..default()
        },
        BackgroundColor(bevy(col)),
    ));
}
