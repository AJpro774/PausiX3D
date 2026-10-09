//! Match flow: menus, rounds, KO cinematics, and victory cutscenes.
//! The step order matches the original raylib loop.

use super::*;

pub(crate) const ROUND_OPTIONS: [u32; 5] = [1, 3, 5, 10, 15];

#[derive(Clone, Copy, Default)]
pub(crate) struct Pad {
    pub left: bool,
    pub right: bool,
    pub down: bool,
    pub jump: bool,
    pub a1: bool,
    pub a2: bool,
    pub melee: bool,
    pub block: bool,
    pub ult: bool,
    pub up_p: bool,
    pub down_p: bool,
    pub left_p: bool,
    pub right_p: bool,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct FrameInput {
    pub p1: Pad,
    pub p2: Pad,
    pub ready: [bool; 2],
    pub map_key: bool,
    pub rounds_key: bool,
    pub mode_key: bool,
    pub players_key: bool,
    pub cpu_key: [bool; 2],
    pub space: bool,
    pub enter: bool,
    pub back: bool,
    pub esc: bool,
}

pub(crate) struct Session {
    pub world: World,
    pub theme_idx: usize,
    pub map: Map,
    pub selecting: bool,
    pub setups: [Setup; 2],
    pub row: [usize; 2],
    pub ready: [bool; 2],
    pub previews: [Player; 2],
    pub players: [Player; 2],
    pub result: Option<String>,
    pub rounds_idx: usize,
    pub mode_idx: usize,
    pub cpu_mode: usize,
    pub idle_t: f32,
    pub cpu_prof: [usize; 2],
    pub cpu_now: [usize; 2],
    pub start_theme: usize,
    pub round_no: u32,
    pub wins: [u32; 2],
    pub series_over: bool,
    pub rd: Round,
    pub ko_timer: f32,
    pub ko_focus: (f32, f32),
    pub vic: Option<Vic>,
    pub pending_vic: Option<usize>,
    pub vic_done: bool,
    pub t: f32,
    pub map_serial: u32,
    pub quit: bool,
}

impl Session {
    pub(crate) fn new() -> Self {
        let setups = [Setup::new(0), Setup::new(1)];
        let map = make_map(THEMES[0]);
        let spawns = map.spawns;
        let keys = Keys;
        Self {
            world: World::new(),
            theme_idx: 0,
            map,
            selecting: true,
            setups,
            row: [0, 0],
            ready: [false, false],
            previews: [
                Player::new(&setups[0], 0.0, 1.0, keys),
                Player::new(&setups[1], 0.0, -1.0, keys),
            ],
            players: [
                Player::new(&setups[0], spawns[0], 1.0, keys),
                Player::new(&setups[1], spawns[1], -1.0, keys),
            ],
            result: None,
            rounds_idx: 1,
            mode_idx: 0,
            cpu_mode: 0,
            idle_t: 0.0,
            cpu_prof: [0, 7],
            cpu_now: [0, 7],
            start_theme: 0,
            round_no: 1,
            wins: [0, 0],
            series_over: false,
            rd: Round::new(),
            ko_timer: 0.0,
            ko_focus: (W as f32 / 2.0, H as f32 / 2.0),
            vic: None,
            pending_vic: None,
            vic_done: false,
            t: 0.0,
            map_serial: 1,
            quit: false,
        }
    }

    pub(crate) fn apply(&mut self, cfg: &crate::game::config::Cfg) {
        if let Some(m) = cfg.mode {
            self.mode_idx = m;
        }
        if let Some(map) = cfg.map {
            self.theme_idx = map;
            self.map = make_map(THEMES[map]);
            self.map_serial += 1;
        }
        if let Some(r) = cfg.rounds {
            self.rounds_idx = r;
        }
        if let Some(p) = cfg.players {
            self.cpu_mode = p;
        }
        for i in 0..2 {
            let explicit_loadout = cfg.p[i].a1.is_some() || cfg.p[i].a2.is_some();
            if let Some(k) = cfg.p[i].kind {
                self.setups[i].kind = k;
                if !explicit_loadout {
                    self.setups[i].abil = KINDS[k].default_loadout();
                }
            }
            if let Some(a) = cfg.p[i].a1 {
                self.setups[i].abil[0] = a;
            }
            if let Some(a) = cfg.p[i].a2 {
                self.setups[i].abil[1] = a;
            }
            if let Some(tr) = cfg.p[i].trail {
                self.setups[i].trail = tr;
            }
            if let Some(c) = cfg.p[i].color {
                self.setups[i].color = c;
            }
            if let Some(d) = cfg.p[i].death {
                self.setups[i].death = d;
            }
            if let Some(v) = cfg.p[i].victory {
                self.setups[i].victory = v;
            }
            if let Some(cpu) = cfg.p[i].cpu {
                self.cpu_prof[i] = cpu;
                self.cpu_now[i] = if cpu >= 8 { 0 } else { cpu };
            }
            self.previews[i].apply_setup(&self.setups[i]);
        }
        if cfg.start || cfg.demo_victory.is_some() || cfg.benchmark_secs.is_some() {
            self.begin_match();
        }
        if cfg.benchmark_secs.is_some() {
            self.configure_benchmark();
        }
        if let Some(idx) = cfg.demo_victory {
            let (px, py) = self.players[1].center();
            let col = self.players[1].kind.color();
            play_death(self.setups[1].death, px, py, col, &mut self.world);
            self.players[1].hp = 0.0;
            self.players[1].dead_t = 99.0;
            self.wins[0] = 1;
            self.vic = Some(victory_start(idx, 0, &mut self.players, &self.map, &mut self.world));
            self.result = Some(format!(
                "P1 ({}) WINS THE ROUND",
                self.players[0].kind.name()
            ));
            self.pending_vic = None;
            self.selecting = false;
        }
    }

    /// CPU vs CPU on Volcano with the noisiest loadouts: carpet bombs, lightning,
    /// dense trails, and full ultimate meters so the measured minute is busy.
    fn configure_benchmark(&mut self) {
        self.cpu_mode = 2;
        self.mode_idx = 0;
        self.rounds_idx = 4; // 15 rounds, so a one-minute sample stays in the match
        self.theme_idx = THEMES.iter().position(|t| *t == Theme::Volcano).unwrap_or(2);
        self.map = make_map(THEMES[self.theme_idx]);
        self.map_serial = self.map_serial.wrapping_add(1);
        self.setups[0] = Setup::new(1); // Bomber
        self.setups[0].abil = [2, 11]; // Bomb, Fireball
        self.setups[0].trail = 4; // Smoke
        self.setups[0].color = 2; // Red
        self.setups[1] = Setup::new(5); // Spark
        self.setups[1].abil = [8, 0]; // Thunder, Dash
        self.setups[1].trail = 3; // Sparkle
        self.setups[1].color = 3; // Yellow
        self.cpu_prof = [5, 5]; // Berserker
        self.begin_match();
        for p in &mut self.players {
            p.ult = ULT_MAX;
        }
    }

    pub(crate) fn begin_match(&mut self) {
        let keys = Keys;
        self.players = [
            Player::new(&self.setups[0], self.map.spawns[0], 1.0, keys),
            Player::new(&self.setups[1], self.map.spawns[1], -1.0, keys),
        ];
        self.world.clear();
        self.result = None;
        self.start_theme = self.theme_idx;
        self.round_no = 1;
        self.wins = [0, 0];
        self.series_over = false;
        self.ko_timer = 0.0;
        self.vic = None;
        self.pending_vic = None;
        self.vic_done = false;
        self.rd = Round::new();
        for i in 0..2 {
            self.cpu_now[i] = if self.cpu_prof[i] >= 8 {
                (self.world.rng.next() * 8.0) as usize % 8
            } else {
                self.cpu_prof[i]
            };
        }
        self.idle_t = 0.0;
        self.selecting = false;
    }

    pub(crate) fn tick(&mut self, input: &FrameInput, dt_raw: f32) {
        let dt = dt_raw.min(0.05);
        self.t += dt;
        if input.esc {
            self.quit = true;
            return;
        }
        self.decay_fx(dt);
        if self.selecting {
            self.tick_menu(input, dt);
        } else {
            self.tick_match(input, dt);
        }
    }

    fn decay_fx(&mut self, dt: f32) {
        {
            let w = &mut self.world;
            w.fx.shake *= 0.86f32.powf(dt * 60.0);
            if w.fx.shake < 0.3 {
                w.fx.shake = 0.0;
            }
            w.fx.punch *= 0.88f32.powf(dt * 60.0);
            w.fx.flash = (w.fx.flash - dt * 2.2).max(0.0);
            if let Some((_, _, left)) = w.banner.as_mut() {
                *left -= dt;
            }
            if matches!(w.banner, Some((_, _, left)) if left <= 0.0) {
                w.banner = None;
            }
        }
        for i in 0..2 {
            self.rd.combo_show[i] = (self.rd.combo_show[i] - dt).max(0.0);
        }
    }

    fn tick_menu(&mut self, input: &FrameInput, dt: f32) {
        let t = self.t;
        let pads = [input.p1, input.p2];
        let mut preview_death: Option<(usize, usize)> = None;
        for i in 0..2 {
            if !self.ready[i] {
                if pads[i].up_p {
                    self.row[i] = cycle(self.row[i], ROWS, -1);
                }
                if pads[i].down_p {
                    self.row[i] = cycle(self.row[i], ROWS, 1);
                }
                let dir = pads[i].right_p as i32 - pads[i].left_p as i32;
                if dir != 0 {
                    let s = &mut self.setups[i];
                    match self.row[i] {
                        0 => {
                            s.kind = cycle(s.kind, KINDS.len(), dir);
                            s.abil = KINDS[s.kind].default_loadout();
                        }
                        1 => s.abil[0] = cycle(s.abil[0], ABILITIES.len(), dir),
                        2 => s.abil[1] = cycle(s.abil[1], ABILITIES.len(), dir),
                        3 => s.trail = cycle(s.trail, TRAIL_STYLES.len(), dir),
                        4 => s.color = cycle(s.color, TRAIL_COLOR_NAMES.len(), dir),
                        5 => {
                            s.death = cycle(s.death, DEATH_NAMES.len(), dir);
                            preview_death = Some((i, s.death));
                        }
                        _ => s.victory = cycle(s.victory, VICTORY_NAMES.len(), dir),
                    }
                }
            }
            if input.ready[i] {
                self.ready[i] = !self.ready[i];
            }
        }
        if let Some((i, choice)) = preview_death {
            let (px, py) = self.previews[i].center();
            let col = self.previews[i].kind.color();
            play_death(choice, px, py, col, &mut self.world);
        }
        let both = match self.cpu_mode {
            0 => self.ready[0] && self.ready[1],
            1 => self.ready[0],
            _ => self.ready[0] || self.ready[1],
        };
        if !both {
            if input.map_key {
                self.theme_idx = cycle(self.theme_idx, THEMES.len(), 1);
                self.map = make_map(THEMES[self.theme_idx]);
                self.map_serial += 1;
            }
            if input.rounds_key {
                self.rounds_idx = cycle(self.rounds_idx, ROUND_OPTIONS.len(), 1);
            }
            if input.mode_key {
                self.mode_idx = cycle(self.mode_idx, MODES.len(), 1);
            }
            if input.cpu_key[0] {
                self.cpu_prof[0] = cycle(self.cpu_prof[0], CPU_NAMES.len(), 1);
            }
            if input.cpu_key[1] {
                self.cpu_prof[1] = cycle(self.cpu_prof[1], CPU_NAMES.len(), 1);
            }
            if input.players_key {
                self.cpu_mode = (self.cpu_mode + 1) % 3;
            }
        }
        for i in 0..2 {
            let setup = self.setups[i];
            let panel_x = 40.0 + i as f32 * 640.0;
            let pv = &mut self.previews[i];
            let world = &mut self.world;
            pv.apply_setup(&setup);
            pv.x = panel_x + 500.0 + (t * 1.6).sin() * 55.0;
            pv.y = 220.0;
            pv.vx = (t * 1.6).cos() * 55.0 * 1.6;
            pv.vy = 0.0;
            pv.facing = if pv.vx >= 0.0 { 1.0 } else { -1.0 };
            pv.rot += pv.vx * dt / (SIZE / 2.0) * 57.3;
            emit_trail(pv, world, t, dt);
        }
        update_particles(&mut self.world.parts, dt);
        if both {
            self.begin_match();
        }
    }

    fn tick_match(&mut self, input: &FrameInput, dt: f32) {
        let t = self.t;
        let mode = MODES[self.mode_idx];
        let cpu = [self.cpu_mode == 2, self.cpu_mode >= 1];
        let rounds_target = ROUND_OPTIONS[self.rounds_idx];
        let pads = [input.p1, input.p2];

        let Session {
            players,
            world,
            map,
            rd,
            setups,
            result,
            wins,
            ko_timer,
            ko_focus,
            vic,
            pending_vic,
            vic_done,
            series_over,
            round_no,
            theme_idx,
            cpu_now,
            cpu_prof,
            idle_t,
            start_theme,
            map_serial,
            selecting,
            ready,
            ..
        } = self;

        let ko_active = *ko_timer > 0.0;
        let mut dt_sim = dt;
        if ko_active {
            *ko_timer = (*ko_timer - dt).max(0.0);
            dt_sim = dt * KO_SLOW;
        }
        if world.fx.hitstop > 0.0 {
            world.fx.hitstop -= dt;
            dt_sim = 0.0;
        }

        if *vic_done {
            *vic = None;
            *vic_done = false;
        }

        if let Some(wi) = *pending_vic {
            if *ko_timer <= 0.0 && world.fx.hitstop <= 0.0 && vic.is_none() {
                let li = 1 - wi;
                if players[li].dead_t < 50.0 {
                    let (px, py) = players[li].center();
                    let col = players[li].kind.color();
                    play_death(setups[li].death, px, py, col, world);
                    players[li].dead_t = 99.0;
                }
                *vic = Some(victory_start(setups[wi].victory, wi, players, map, world));
                *pending_vic = None;
            }
        }

        if let Some(v) = vic.as_mut() {
            let skip = input.space || input.enter;
            let finished = victory_update(v, players, map, world, dt);
            let who = v.who;
            if finished || skip {
                victory_finish(who, players, map);
                *vic_done = true;
            }
        } else if result.is_none() || ko_active {
            if dt_sim > 0.0 {
                let hp_before = [players[0].hp, players[1].hp];
                let live = result.is_none();
                if live {
                    rd.time += dt_sim;
                }
                for i in 0..2 {
                    if live && players[i].dead_t > 0.0 {
                        players[i].dead_t -= dt_sim;
                        if players[i].dead_t <= 0.0 {
                            let facing = if i == 0 { 1.0 } else { -1.0 };
                            respawn(&mut players[i], &setups[i], map.spawns[i], facing, Keys, world);
                        }
                    }
                }
                let inputs = [
                    if cpu[0] {
                        bot_input(players, 0, &mut world.rng, &cpu_profile(cpu_now[0]), t, 0.0)
                    } else {
                        pad_input(&pads[0])
                    },
                    if cpu[1] {
                        bot_input(players, 1, &mut world.rng, &cpu_profile(cpu_now[1]), t, 3.7)
                    } else {
                        pad_input(&pads[1])
                    },
                ];
                for i in 0..2 {
                    if players[i].dead_t > 0.0 {
                        continue;
                    }
                    let (a, b) = players.split_at_mut(1);
                    let (me, foe) = if i == 0 {
                        (&mut a[0], &mut b[0])
                    } else {
                        (&mut b[0], &mut a[0])
                    };
                    update_player(&inputs[i], me, foe, i, &map.solids, world, dt_sim);
                }
                collide_players(players, &map.solids);
                for p in players.iter_mut() {
                    clamp_to_arena(p);
                }
                for i in 0..2 {
                    let (a, b) = players.split_at_mut(1);
                    let (me, foe) = if i == 0 {
                        (&mut a[0], &mut b[0])
                    } else {
                        (&mut b[0], &mut a[0])
                    };
                    slam_land(me, foe, world);
                }
                if live {
                    update_events(THEMES[*theme_idx], rd, players, &map.solids, world, dt_sim);
                    update_pickups(players, &map.solids, rd, world, dt_sim);
                }
                let rise = lava_rise(THEMES[*theme_idx], rd.time);
                let hazard_now = map
                    .hazard
                    .map(|hz| Rectangle::new(hz.x, hz.y - rise, hz.width, hz.height + rise));
                if let Some(hz) = hazard_now {
                    for p in players.iter_mut() {
                        if p.dead_t <= 0.0 && overlaps(&p.rect(), &hz) {
                            p.hp = (p.hp - LAVA_DPS * dt_sim).max(0.0);
                            p.hurt_t = 0.1;
                            if world.rng.next() < 0.5 {
                                let (cx, cy) = p.center();
                                let mut q = Particle::new(
                                    cx + world.rng.range(-20.0, 20.0),
                                    cy + 25.0,
                                    world.rng.range(-40.0, 40.0),
                                    -120.0,
                                    0.5,
                                    6.0,
                                    Color::new(255, 150, 30, 255),
                                    Shape::Circle,
                                );
                                q.grav = 400.0;
                                world.parts.push(q);
                            }
                        }
                    }
                }
                update_projs(players, &map.solids, world, dt_sim);
                update_strikes(players, world, dt_sim);
                update_booms(&mut world.booms, dt_sim);
                for p in players.iter_mut() {
                    emit_trail(p, world, t, dt_sim);
                }
                update_particles(&mut world.parts, dt_sim);

                for i in 0..2 {
                    let (px, py) = players[i].center();
                    if players[i].parried {
                        players[i].parried = false;
                        let att = 1 - i;
                        players[att].stun_t = 0.8;
                        players[att].vx = 0.0;
                        players[i].ult = (players[i].ult + 20.0).min(ULT_MAX);
                        world.ring(px, py, 160.0, Color::WHITE);
                        world.ring(px, py, 100.0, Color::new(255, 230, 90, 255));
                        world.burst(px, py, 20, Color::WHITE, 500.0, 0.5, 7.0, Shape::Star, 0.0);
                        world.fx.hitstop = world.fx.hitstop.max(0.18);
                        world.fx.flash = world.fx.flash.max(0.35);
                        world.shake(12.0);
                        world.banner = Some(("PARRY!".to_string(), Color::WHITE, 0.7));
                    }
                    if players[i].blocked {
                        players[i].blocked = false;
                        let f = players[i].facing;
                        world.burst(
                            px + f * 40.0,
                            py,
                            10,
                            Color::new(200, 230, 255, 255),
                            350.0,
                            0.3,
                            4.0,
                            Shape::Streak,
                            0.0,
                        );
                        world.ring(px + f * 40.0, py, 50.0, Color::new(200, 230, 255, 255));
                        world.shake(4.0);
                    }
                }

                for i in 0..2 {
                    let drop = hp_before[i] - players[i].hp;
                    if drop >= 3.0 && players[i].hurt_t > 0.2 {
                        let att = 1 - i;
                        if rd.combo_t[att] > 0.0 {
                            rd.combo[att] += 1;
                        } else {
                            rd.combo[att] = 1;
                        }
                        rd.combo_t[att] = COMBO_WINDOW;
                        rd.combo_show[att] = 1.6;
                        let mut bonus = ((rd.combo[att] as f32 - 1.0) * 0.1).min(0.5);
                        if players[att].power_t > 0.0 {
                            bonus += 0.5;
                        }
                        let extra = drop * bonus;
                        if extra > 0.0 {
                            players[i].hp = (players[i].hp - extra).max(0.0);
                        }
                        let total = drop + extra;
                        players[att].ult = (players[att].ult + total * 1.2).min(ULT_MAX);
                        players[i].ult = (players[i].ult + total * 0.6).min(ULT_MAX);
                        if total >= 18.0 && !players[i].block_held {
                            players[i].stun_t = players[i].stun_t.max(0.3);
                        }
                        let (px, py) = players[i].center();
                        world.shake((5.0 + total * 0.6).min(24.0));
                        world.fx.hitstop = world.fx.hitstop.max((0.03 + total * 0.003).min(0.12));
                        world.fx.punch = world.fx.punch.max((0.02 + total * 0.002).min(0.08));
                        if total >= 18.0 {
                            world.fx.flash = world.fx.flash.max(0.25);
                        }
                        world.burst(
                            px,
                            py,
                            10 + total as usize,
                            Color::WHITE,
                            420.0,
                            0.35,
                            4.0,
                            Shape::Streak,
                            0.0,
                        );
                        world.burst(px, py, 6, players[i].kind.color(), 300.0, 0.5, 9.0, Shape::Square, 500.0);
                        world.ring(px, py, 60.0 + total * 2.0, Color::WHITE);
                    }
                }
                for i in 0..2 {
                    rd.combo_t[i] = (rd.combo_t[i] - dt_sim).max(0.0);
                }

                if live && (mode == Mode::Stock || mode == Mode::Hill) {
                    for i in 0..2 {
                        if players[i].hp <= 0.0 && players[i].dead_t <= 0.0 {
                            players[i].dead_t = RESPAWN_TIME;
                            let (px, py) = players[i].center();
                            let col = players[i].kind.color();
                            play_death(setups[i].death, px, py, col, world);
                            if mode == Mode::Stock {
                                rd.stocks[i] -= 1;
                            } else {
                                rd.hill[1 - i] += 5.0;
                            }
                        }
                    }
                }

                if live && mode == Mode::Hill {
                    rd.zone_t -= dt_sim;
                    if rd.zone_t <= 0.0 {
                        rd.zone_t = HILL_MOVE;
                        rd.zone_idx = (rd.zone_idx + 1) % map.zones.len();
                        let z = map.zones[rd.zone_idx];
                        world.ring(
                            z.x + z.width / 2.0,
                            z.y + z.height / 2.0,
                            140.0,
                            Color::new(255, 230, 90, 255),
                        );
                    }
                    let zone = map.zones[rd.zone_idx];
                    let inside = [
                        players[0].dead_t <= 0.0 && overlaps(&players[0].rect(), &zone),
                        players[1].dead_t <= 0.0 && overlaps(&players[1].rect(), &zone),
                    ];
                    if inside[0] && !inside[1] {
                        rd.hill[0] += dt_sim;
                    } else if inside[1] && !inside[0] {
                        rd.hill[1] += dt_sim;
                    }
                }

                if result.is_none() {
                    let dead = [players[0].hp <= 0.0, players[1].hp <= 0.0];
                    let by_hp = |a: f32, b: f32| -> Option<usize> {
                        if (a - b).abs() < 0.01 {
                            None
                        } else if a > b {
                            Some(0)
                        } else {
                            Some(1)
                        }
                    };
                    let mut end: Option<(Option<usize>, bool)> = None;
                    match mode {
                        Mode::Classic | Mode::Timed => {
                            end = match dead {
                                [true, true] => Some((None, true)),
                                [true, false] => Some((Some(1), true)),
                                [false, true] => Some((Some(0), true)),
                                _ => None,
                            };
                            if end.is_none() && mode == Mode::Timed && rd.time >= TIME_LIMIT {
                                end = Some((by_hp(players[0].hp, players[1].hp), false));
                            }
                        }
                        Mode::Stock => {
                            if rd.stocks[0] <= 0 || rd.stocks[1] <= 0 {
                                let win = if rd.stocks[0] <= 0 && rd.stocks[1] <= 0 {
                                    None
                                } else if rd.stocks[0] <= 0 {
                                    Some(1)
                                } else {
                                    Some(0)
                                };
                                end = Some((win, true));
                            }
                        }
                        Mode::Hill => {
                            if rd.hill[0] >= HILL_TARGET || rd.hill[1] >= HILL_TARGET || rd.time >= HILL_TIME {
                                end = Some((by_hp(rd.hill[0], rd.hill[1]), false));
                            }
                        }
                    }
                    if let Some((win, ko)) = end {
                        if let Some(i) = win {
                            wins[i] += 1;
                        }
                        *series_over = *round_no >= rounds_target;
                        let reason = if ko {
                            ""
                        } else if mode == Mode::Hill {
                            " (HILL POINTS)"
                        } else {
                            " (TIME UP)"
                        };
                        *result = Some(match win {
                            Some(i) => format!("P{} ({}) WINS THE ROUND{}", i + 1, players[i].kind.name(), reason),
                            None => format!("ROUND DRAWN{}", reason),
                        });
                        world.fx.flash = 0.9;
                        world.shake(26.0);
                        world.fx.hitstop = 0.18;
                        *pending_vic = win;
                        if ko {
                            *ko_timer = KO_TIME;
                            let mut fx = 0.0;
                            let mut fy = 0.0;
                            let mut n = 0.0;
                            for i in 0..2 {
                                if dead[i] || (mode == Mode::Stock && rd.stocks[i] <= 0) {
                                    players[i].hp = 0.0;
                                    let (px, py) = players[i].center();
                                    let col = players[i].kind.color();
                                    if players[i].dead_t <= 0.0 {
                                        play_death(setups[i].death, px, py, col, world);
                                    }
                                    players[i].dead_t = 99.0;
                                    fx += px;
                                    fy += py;
                                    n += 1.0;
                                }
                            }
                            if n > 0.0 {
                                *ko_focus = (fx / n, fy / n);
                            }
                        }
                    }
                }
            }
        } else {
            update_booms(&mut world.booms, dt);
            update_particles(&mut world.parts, dt);
            update_strikes(players, world, dt);
            if pending_vic.is_none() && cpu[0] && cpu[1] {
                *idle_t += dt;
            } else {
                *idle_t = 0.0;
            }
            let auto_next = *idle_t > 3.5;
            let auto_menu = *idle_t > 6.0;
            if pending_vic.is_none() && (input.back || (*series_over && auto_menu)) {
                *idle_t = 0.0;
                *selecting = true;
                ready[0] = false;
                ready[1] = false;
                world.clear();
                *vic = None;
                *theme_idx = *start_theme;
                *map = make_map(THEMES[*theme_idx]);
                *map_serial += 1;
            } else if pending_vic.is_none()
                && !*series_over
                && (input.space || input.enter || auto_next)
            {
                *idle_t = 0.0;
                *round_no += 1;
                *theme_idx = cycle(*theme_idx, THEMES.len(), 1);
                *map = make_map(THEMES[*theme_idx]);
                *map_serial += 1;
                let keys = Keys;
                *players = [
                    Player::new(&setups[0], map.spawns[0], 1.0, keys),
                    Player::new(&setups[1], map.spawns[1], -1.0, keys),
                ];
                world.clear();
                *result = None;
                *rd = Round::new();
                for i in 0..2 {
                    cpu_now[i] = if cpu_prof[i] >= 8 {
                        (world.rng.next() * 8.0) as usize % 8
                    } else {
                        cpu_prof[i]
                    };
                }
            }
        }
    }

    pub(crate) fn theme(&self) -> Theme {
        THEMES[self.theme_idx]
    }

    pub(crate) fn mode(&self) -> Mode {
        MODES[self.mode_idx]
    }
}

fn pad_input(p: &Pad) -> Input {
    Input {
        left: p.left,
        right: p.right,
        down: p.down,
        jump: p.jump,
        a1: p.a1,
        a2: p.a2,
        melee: p.melee,
        block_down: p.block,
        ult: p.ult,
    }
}

/// Same brain as the original, but indexed so both players stay borrowed together.
fn bot_input(players: &[Player; 2], me_i: usize, rng: &mut Rng, prof: &Profile, t: f32, seed: f32) -> Input {
    let foe_i = 1 - me_i;
    super::bot_input(&players[me_i], &players[foe_i], rng, prof, t, seed)
}
