//! HUD and the character-select menu, laid out in the original 1280x720 space.

use super as sim;
use super::app::Sim;
use super::session::ROUND_OPTIONS;
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct Hud;

pub(crate) fn draw_ui(mut commands: Commands, sim: Res<Sim>, old: Query<Entity, With<Hud>>) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let s = &sim.0;
    if s.selecting {
        menu(&mut commands, s);
    } else {
        fight(&mut commands, s);
    }
    if s.world.fx.flash > 0.02 {
        rect(&mut commands, 0.0, 0.0, 1280.0, 720.0, Color::srgba(1.0, 1.0, 1.0, s.world.fx.flash * 0.75));
    }
}

fn menu(commands: &mut Commands, s: &sim::session::Session) {
    rect(commands, 0.0, 0.0, 1280.0, 720.0, Color::srgba(0.02, 0.03, 0.06, 0.35));
    label(commands, "PAUSI", 36.0, 18.0, 54.0, Color::srgb_u8(255, 236, 180));
    label(commands, "3D", 250.0, 34.0, 28.0, Color::srgb_u8(255, 120, 60));
    let theme = sim::THEMES[s.theme_idx];
    label(commands, theme.name(), 340.0, 22.0, 22.0, Color::srgb_u8(190, 230, 255));
    label(commands, theme.blurb(), 340.0, 48.0, 14.0, Color::srgb_u8(180, 190, 200));

    for i in 0..2 {
        let x = 28.0 + i as f32 * 664.0;
        panel(commands, s, i, x);
    }

    let mode = sim::MODES[s.mode_idx];
    let players = ["HUMAN VS HUMAN", "HUMAN VS CPU", "CPU VS CPU"][s.cpu_mode];
    let footer = format!(
        "M map   N rounds {}   B {}   C {}     1 / 2 cpu personality     Space next round     R menu     Esc quit",
        ROUND_OPTIONS[s.rounds_idx],
        mode.name(),
        players
    );
    rect(commands, 0.0, 676.0, 1280.0, 44.0, Color::srgba(0.0, 0.0, 0.0, 0.55));
    label(commands, footer, 20.0, 688.0, 14.0, Color::srgb_u8(210, 214, 220));
}

fn panel(commands: &mut Commands, s: &sim::session::Session, i: usize, x: f32) {
    let setup = &s.setups[i];
    let kind = sim::KINDS[setup.kind];
    let accent = px_color(kind.color());
    rect(commands, x, 86.0, 560.0, 575.0, Color::srgba(0.03, 0.04, 0.07, 0.62));
    rect(commands, x, 86.0, 8.0, 575.0, accent);
    let cpu = is_cpu(s.cpu_mode, i);
    let who = if cpu { format!("P{}  CPU", i + 1) } else { format!("P{}  HUMAN", i + 1) };
    label(commands, who, x + 22.0, 96.0, 16.0, Color::srgb_u8(160, 170, 180));
    if cpu {
        label(commands, sim::CPU_NAMES[s.cpu_prof[i]], x + 150.0, 96.0, 16.0, Color::srgb_u8(255, 210, 120));
    }
    label(commands, kind.name(), x + 22.0, 118.0, 32.0, accent);
    label(commands, kind.passive(), x + 22.0, 156.0, 14.0, Color::srgb_u8(220, 224, 230));
    label(commands, format!("ULT  {}  —  {}", kind.ult_name(), kind.ult_blurb()), x + 22.0, 176.0, 13.0, Color::srgb_u8(255, 196, 90));

    for row in 0..sim::ROWS {
        let y = 214.0 + row as f32 * 52.0;
        let on = s.row[i] == row && !s.ready[i];
        if on {
            rect(commands, x + 16.0, y - 4.0, 528.0, 48.0, Color::srgba(1.0, 1.0, 1.0, 0.08));
        }
        label(commands, sim::ROW_NAMES[row], x + 24.0, y, 12.0, Color::srgb_u8(140, 148, 160));
        let val = row_value(setup, row);
        label(commands, format!("<  {val}  >"), x + 24.0, y + 16.0, 20.0, if on { Color::WHITE } else { Color::srgb_u8(230, 232, 236) });
    }
    let blurb = row_blurb(setup, s.row[i]);
    label(commands, blurb, x + 24.0, 588.0, 13.0, Color::srgb_u8(190, 198, 206));
    if s.ready[i] {
        label(commands, "READY", x + 420.0, 600.0, 22.0, Color::srgb_u8(80, 230, 120));
    } else {
        let key = if i == 0 { "F ready" } else { "L ready" };
        label(commands, key, x + 430.0, 604.0, 16.0, Color::srgb_u8(255, 220, 140));
    }
}

fn fight(commands: &mut Commands, s: &sim::session::Session) {
    for i in 0..2 {
        let p = &s.players[i];
        let x = if i == 0 { 24.0 } else { 1280.0 - 24.0 - 400.0 };
        let cpu = is_cpu(s.cpu_mode, i);
        let tag = if cpu { format!("P{} {}  {}", i + 1, p.kind.name(), sim::CPU_NAMES[s.cpu_now[i]]) } else { format!("P{} {}", i + 1, p.kind.name()) };
        label(commands, tag, x, 14.0, 16.0, px_color(p.kind.color()));
        bar(commands, x, 36.0, 400.0, 16.0, (p.hp / p.max_hp).clamp(0.0, 1.0), Color::srgb_u8(28, 30, 34), hp_color(p.hp / p.max_hp));
        bar(commands, x, 56.0, 400.0, 8.0, (p.ult / sim::ULT_MAX).clamp(0.0, 1.0), Color::srgb_u8(28, 30, 34), Color::srgb_u8(255, 196, 40));
        for a in 0..2 {
            let ax = x + a as f32 * 204.0;
            let cd = if a == 0 { p.cd1 } else { p.cd2 };
            let max = p.abilities[a].cooldown();
            let ready = 1.0 - (cd / max).clamp(0.0, 1.0);
            rect(commands, ax, 70.0, 196.0, 28.0, Color::srgba(0.0, 0.0, 0.0, 0.45));
            rect(commands, ax, 94.0, 196.0 * ready, 4.0, if cd <= 0.0 { Color::srgb_u8(80, 220, 255) } else { Color::srgb_u8(80, 90, 110) });
            let name = if cd > 0.05 { format!("{}  {:.1}", p.abilities[a].name(), cd) } else { p.abilities[a].name().to_string() };
            label(commands, name, ax + 6.0, 74.0, 14.0, Color::WHITE);
        }
        if sim::MODES[s.mode_idx] == sim::Mode::Stock {
            let stocks = s.rd.stocks[i].max(0);
            label(commands, format!("STOCK {stocks}"), x, 104.0, 14.0, Color::srgb_u8(255, 230, 160));
        }
        if sim::MODES[s.mode_idx] == sim::Mode::Hill {
            label(commands, format!("HILL {:.0}", s.rd.hill[i]), x, 104.0, 14.0, Color::srgb_u8(255, 220, 80));
        }
        if s.rd.combo[i] >= 2 && s.rd.combo_show[i] > 0.0 {
            let cx = if i == 0 { 80.0 } else { 980.0 };
            label(commands, format!("{} HIT", s.rd.combo[i]), cx, 220.0, 28.0, Color::srgb_u8(255, 230, 120));
        }
    }

    let mode = sim::MODES[s.mode_idx];
    let rounds = ROUND_OPTIONS[s.rounds_idx];
    let mid = format!("R{} / {}    {} – {}    {}", s.round_no, rounds, s.wins[0], s.wins[1], mode.name());
    label(commands, mid, 470.0, 12.0, 16.0, Color::srgb_u8(230, 234, 240));
    if mode == sim::Mode::Timed {
        let left = (sim::TIME_LIMIT - s.rd.time).max(0.0);
        label(commands, format!("{left:.0}"), 620.0, 32.0, 18.0, Color::WHITE);
    } else if mode == sim::Mode::Hill {
        let left = (sim::HILL_TIME - s.rd.time).max(0.0);
        label(commands, format!("{left:.0}"), 620.0, 32.0, 18.0, Color::WHITE);
    }
    if s.rd.wind_warn {
        label(commands, "WIND INCOMING", 540.0, 120.0, 18.0, Color::srgb_u8(200, 230, 255));
    }
    if s.rd.zerog_warn {
        label(commands, "ZERO-G", 600.0, 120.0, 18.0, Color::srgb_u8(180, 180, 255));
    }
    if let Some((text, col, _)) = &s.world.banner {
        label(commands, text, 360.0, 150.0, 36.0, px_color(*col));
    }
    if s.ko_timer > 0.0 {
        label(commands, "K.O.", 560.0, 250.0, 72.0, Color::srgb_u8(255, 80, 70));
    }
    if let Some(text) = &s.result {
        label(commands, text, 280.0, 340.0, 28.0, Color::WHITE);
    }
    if let Some(v) = &s.vic {
        label(commands, sim::VICTORY_NAMES[v.idx], 360.0, 80.0, 26.0, Color::srgb_u8(255, 220, 120));
        rect(commands, 0.0, 0.0, 1280.0, 36.0, Color::BLACK);
        rect(commands, 0.0, 684.0, 1280.0, 36.0, Color::BLACK);
    }
    label(commands, sim::THEMES[s.theme_idx].name(), 560.0, 692.0, 13.0, Color::srgba(1.0, 1.0, 1.0, 0.7));
}

fn is_cpu(mode: usize, i: usize) -> bool {
    mode == 2 || (mode == 1 && i == 1)
}

fn row_value(setup: &sim::Setup, row: usize) -> &'static str {
    match row {
        0 => sim::KINDS[setup.kind].name(),
        1 => sim::ABILITIES[setup.abil[0]].name(),
        2 => sim::ABILITIES[setup.abil[1]].name(),
        3 => sim::TRAIL_STYLES[setup.trail].name(),
        4 => sim::TRAIL_COLOR_NAMES[setup.color],
        5 => sim::DEATH_NAMES[setup.death],
        _ => sim::VICTORY_NAMES[setup.victory],
    }
}

fn row_blurb(setup: &sim::Setup, row: usize) -> &'static str {
    match row {
        0 => sim::KINDS[setup.kind].ult_blurb(),
        1 => sim::ABILITIES[setup.abil[0]].blurb(),
        2 => sim::ABILITIES[setup.abil[1]].blurb(),
        3 => sim::TRAIL_STYLES[setup.trail].name(),
        5 => sim::DEATH_BLURBS[setup.death],
        6 => sim::VICTORY_BLURBS[setup.victory],
        _ => "trail color",
    }
}

fn hp_color(frac: f32) -> Color {
    if frac > 0.5 {
        Color::srgb_u8(70, 210, 90)
    } else if frac > 0.25 {
        Color::srgb_u8(240, 190, 40)
    } else {
        Color::srgb_u8(230, 60, 50)
    }
}

fn px_color(c: sim::Color) -> Color {
    Color::srgba_u8(c.r, c.g, c.b, c.a)
}

fn label(commands: &mut Commands, text: impl Into<String>, x: f32, y: f32, size: f32, color: Color) {
    commands.spawn((
        Hud,
        Text::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
        TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            ..default()
        },
    ));
}

fn rect(commands: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color) {
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
        BackgroundColor(color),
    ));
}

fn bar(commands: &mut Commands, x: f32, y: f32, w: f32, h: f32, frac: f32, bg: Color, fg: Color) {
    rect(commands, x, y, w, h, bg);
    rect(commands, x + 2.0, y + 2.0, (w - 4.0) * frac.clamp(0.0, 1.0), (h - 4.0).max(1.0), fg);
}
