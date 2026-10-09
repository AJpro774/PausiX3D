//! Pausi simulation, ported from the original raylib game.
//! Coordinates stay in the original 1280x720, y-down pixel space.

#![allow(dead_code, clippy::too_many_arguments, clippy::needless_range_loop)]

#[derive(Clone, Copy, Debug)]
pub(crate) struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub(crate) const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub(crate) const WHITE: Self = Self::new(255, 255, 255, 255);
    pub(crate) const BLACK: Self = Self::new(0, 0, 0, 255);
    pub(crate) const RED: Self = Self::new(230, 41, 55, 255);
    pub(crate) const ORANGE: Self = Self::new(255, 161, 0, 255);
    pub(crate) const YELLOW: Self = Self::new(253, 249, 0, 255);
    pub(crate) const LIME: Self = Self::new(0, 158, 47, 255);
    pub(crate) const DARKGRAY: Self = Self::new(80, 80, 80, 255);
    pub(crate) const GRAY: Self = Self::new(130, 130, 130, 255);
    pub(crate) const LIGHTGRAY: Self = Self::new(200, 200, 200, 255);
}

impl Default for Color {
    fn default() -> Self { Self::WHITE }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Vector2 {
    pub x: f32,
    pub y: f32,
}
impl Vector2 {
    pub(crate) fn new(x: f32, y: f32) -> Self { Self { x, y } }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rectangle {
    pub(crate) fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}


// ---- Alexander's movement numbers (unchanged feel) ----
pub(crate) const W: i32 = 1280;
pub(crate) const H: i32 = 720;
pub(crate) const SIZE: f32 = 60.0; // smaller than the solo prototype so two fit in an arena
pub(crate) const TOP_SPEED: f32 = 350.0; // px per second
pub(crate) const ICE: f32 = 400.0; // slide after letting go
pub(crate) const JUMP: f32 = 200.0; // normal jump height in pixels
pub(crate) const MAX_CHARGE: f32 = 3.5; // full charge = 3.5x jump
pub(crate) const GRAVITY: f32 = 800.0; // higher = snappier, lower = floatier (was 250)
pub(crate) const TAP_CROUCH: f32 = 0.15;
pub(crate) const CROUCH: f32 = 5.0;

// ---- duel numbers ----
pub(crate) const MAX_HP: f32 = 100.0;
pub(crate) const FLOOR_H: f32 = 40.0;
pub(crate) const DASH_SPEED: f32 = 900.0;
pub(crate) const DASH_TIME: f32 = 0.2;
pub(crate) const SLAM_SPEED: f32 = 900.0;
pub(crate) const BOMB_GRAVITY: f32 = 600.0;
pub(crate) const BLAST_R: f32 = 120.0;
pub(crate) const SHIELD_TIME: f32 = 1.2;
pub(crate) const BOOM_TIME: f32 = 0.3;
pub(crate) const MELEE_TIME: f32 = 0.25; // length of the knife swing
pub(crate) const MELEE_CD: f32 = 0.0; // no cooldown (you still finish the swing in progress)
pub(crate) const MELEE_REACH: f32 = 80.0;
pub(crate) const LAVA_DPS: f32 = 35.0;
pub(crate) const MAX_SPEED: f32 = 1500.0; // knockback never launches anyone faster than this
pub(crate) const BLINK_DIST: f32 = 260.0;
pub(crate) const STRIKE_DELAY: f32 = 0.9;
pub(crate) const FROZEN_TIME: f32 = 1.3;
pub(crate) const BURN_TIME: f32 = 2.5;
pub(crate) const KO_TIME: f32 = 1.8; // slow-motion KO cinematic
pub(crate) const KO_SLOW: f32 = 0.25;

// ---- movement tech ----
pub(crate) const WALL_SLIDE: f32 = 140.0; // max fall speed while sliding down a wall
pub(crate) const WALL_KICK: f32 = 430.0; // sideways push off a wall jump

// ---- combat depth ----
pub(crate) const BLOCK_MAX: f32 = 1.6; // longest you can hold a guard
pub(crate) const PARRY_WINDOW: f32 = 0.16; // block right as a hit lands to parry it
pub(crate) const ULT_MAX: f32 = 100.0;
pub(crate) const COMBO_WINDOW: f32 = 1.8;

// ---- modes ----
pub(crate) const TIME_LIMIT: f32 = 60.0;
pub(crate) const HILL_TIME: f32 = 90.0;
pub(crate) const HILL_TARGET: f32 = 30.0;
pub(crate) const HILL_MOVE: f32 = 15.0;
pub(crate) const STOCKS: i32 = 3;
pub(crate) const RESPAWN_TIME: f32 = 1.6;

// =====================================================================
// small helpers
// =====================================================================

pub(crate) struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 10000) as f32 / 10000.0
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

pub(crate) fn cycle(v: usize, n: usize, dir: i32) -> usize {
    (v as i32 + dir).rem_euclid(n as i32) as usize
}

pub(crate) fn with_alpha(c: Color, a: u8) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

pub(crate) fn hsv(h: f32) -> Color {
    let h = h.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as i32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Color::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255)
}

pub(crate) fn overlaps(a: &Rectangle, b: &Rectangle) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

pub(crate) fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

// raylib wants triangles wound counter-clockwise, so fix the order for it

// =====================================================================
// characters, abilities, trails
// =====================================================================

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Eyes {
    Forward,
    Left,
    Right,
    UpWide,
    Squint,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Dasher,
    Bomber,
    Shielder,
    Titan,
    Ghost,
    Spark,
}

pub(crate) const KINDS: [Kind; 6] = [Kind::Dasher, Kind::Bomber, Kind::Shielder, Kind::Titan, Kind::Ghost, Kind::Spark];

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Dasher => "DASHER",
            Kind::Bomber => "BOMBER",
            Kind::Shielder => "SHIELDER",
            Kind::Titan => "TITAN",
            Kind::Ghost => "GHOST",
            Kind::Spark => "SPARK",
        }
    }
    fn color(self) -> Color {
        match self {
            Kind::Dasher => Color::ORANGE,
            Kind::Bomber => Color::new(220, 60, 60, 255),
            Kind::Shielder => Color::new(40, 170, 160, 255),
            Kind::Titan => Color::new(110, 120, 160, 255),
            Kind::Ghost => Color::new(225, 220, 255, 255),
            Kind::Spark => Color::new(255, 230, 60, 255),
        }
    }
    // indices into ABILITIES
    fn default_loadout(self) -> [usize; 2] {
        match self {
            Kind::Dasher => [0, 1],
            Kind::Bomber => [2, 3],
            Kind::Shielder => [4, 5],
            Kind::Titan => [1, 5],
            Kind::Ghost => [6, 7],
            Kind::Spark => [8, 0],
        }
    }
    // ---- passive traits ----
    fn max_hp(self) -> f32 {
        match self {
            Kind::Titan => 130.0,
            Kind::Ghost => 85.0,
            Kind::Spark => 90.0,
            _ => MAX_HP,
        }
    }
    fn speed(self) -> f32 {
        match self {
            Kind::Dasher => 1.1,
            Kind::Titan => 0.85,
            Kind::Spark => 1.25,
            _ => 1.0,
        }
    }
    fn kb_resist(self) -> f32 {
        if self == Kind::Titan {
            0.55
        } else {
            1.0
        }
    }
    fn dmg_taken(self) -> f32 {
        if self == Kind::Shielder {
            0.85
        } else {
            1.0
        }
    }
    fn grav(self) -> f32 {
        match self {
            Kind::Ghost => 0.75,
            Kind::Titan => 1.1,
            _ => 1.0,
        }
    }
    fn air_jumps(self) -> i32 {
        match self {
            Kind::Titan => 0,
            Kind::Ghost => 2,
            _ => 1,
        }
    }
    fn melee_dmg(self) -> f32 {
        match self {
            Kind::Spark => 14.0,
            Kind::Titan => 12.0,
            _ => 10.0,
        }
    }
    fn passive(self) -> &'static str {
        match self {
            Kind::Dasher => "PASSIVE: +10% speed",
            Kind::Bomber => "PASSIVE: bombs hit 20% harder",
            Kind::Shielder => "PASSIVE: takes 15% less damage",
            Kind::Titan => "PASSIVE: 130 HP, resists knockback, slow, no double jump",
            Kind::Ghost => "PASSIVE: 85 HP, floaty, two air jumps",
            Kind::Spark => "PASSIVE: 90 HP, +25% speed, stronger knife",
        }
    }
    fn ult_name(self) -> &'static str {
        match self {
            Kind::Dasher => "OVERDRIVE",
            Kind::Bomber => "CARPET BOMB",
            Kind::Shielder => "AEGIS NOVA",
            Kind::Titan => "EARTHQUAKE",
            Kind::Ghost => "DEEP FREEZE",
            Kind::Spark => "LIGHTNING STORM",
        }
    }
    fn ult_blurb(self) -> &'static str {
        match self {
            Kind::Dasher => "6s faster, cooldowns x3, knife x1.8",
            Kind::Bomber => "bombs rain across the arena",
            Kind::Shielder => "3s invincible + blast wave",
            Kind::Titan => "foe on the ground: 25 dmg + launch",
            Kind::Ghost => "freeze them 2s from anywhere",
            Kind::Spark => "5 bolts strike around them",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Ability {
    Dash,
    Slam,
    Bomb,
    Hop,
    Shield,
    Pulse,
    Blink,
    Frost,
    Thunder,
    Magnet,
    Mend,
    Fireball,
}

pub(crate) const ABILITIES: [Ability; 12] = [
    Ability::Dash,
    Ability::Slam,
    Ability::Bomb,
    Ability::Hop,
    Ability::Shield,
    Ability::Pulse,
    Ability::Blink,
    Ability::Frost,
    Ability::Thunder,
    Ability::Magnet,
    Ability::Mend,
    Ability::Fireball,
];

impl Ability {
    fn name(self) -> &'static str {
        match self {
            Ability::Dash => "DASH",
            Ability::Slam => "SLAM",
            Ability::Bomb => "BOMB",
            Ability::Hop => "HOP",
            Ability::Shield => "SHIELD",
            Ability::Pulse => "PULSE",
            Ability::Blink => "BLINK",
            Ability::Frost => "FROST",
            Ability::Thunder => "THUNDER",
            Ability::Magnet => "MAGNET",
            Ability::Mend => "MEND",
            Ability::Fireball => "FIREBALL",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Ability::Dash => "ram forward, 15 dmg",
            Ability::Slam => "leap and crash down, 20 dmg",
            Ability::Bomb => "thrown, 25 dmg, launches you",
            Ability::Hop => "boost upward, even in the air",
            Ability::Shield => "blocks damage, reflects shots",
            Ability::Pulse => "12 dmg, big knockback",
            Ability::Blink => "teleport forward past danger",
            Ability::Frost => "freezes them for 1.3s, 5 dmg",
            Ability::Thunder => "lightning on their spot, 22 dmg",
            Ability::Magnet => "yank them toward you",
            Ability::Mend => "heal 18 HP over 2 seconds",
            Ability::Fireball => "14 dmg and burns for 2.5s",
        }
    }
    fn cooldown(self) -> f32 {
        match self {
            Ability::Dash => 2.0,
            Ability::Slam => 3.0,
            Ability::Bomb => 1.5,
            Ability::Hop => 2.5,
            Ability::Shield => 3.0,
            Ability::Pulse => 1.5,
            Ability::Blink => 2.5,
            Ability::Frost => 3.5,
            Ability::Thunder => 5.0,
            Ability::Magnet => 3.5,
            Ability::Mend => 9.0,
            Ability::Fireball => 2.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Trail {
    Off,
    Dots,
    Ghost,
    Sparkle,
    Smoke,
}

pub(crate) const TRAIL_STYLES: [Trail; 5] = [Trail::Off, Trail::Dots, Trail::Ghost, Trail::Sparkle, Trail::Smoke];

impl Trail {
    fn name(self) -> &'static str {
        match self {
            Trail::Off => "NONE",
            Trail::Dots => "DOTS",
            Trail::Ghost => "GHOST",
            Trail::Sparkle => "SPARKLE",
            Trail::Smoke => "SMOKE",
        }
    }
}

pub(crate) const TRAIL_COLOR_NAMES: [&str; 9] = ["BODY", "WHITE", "RED", "YELLOW", "GREEN", "CYAN", "PURPLE", "PINK", "RAINBOW"];

pub(crate) fn trail_color(idx: usize, t: f32, body: Color, spread: f32) -> Color {
    match idx {
        0 => body,
        1 => Color::WHITE,
        2 => Color::new(235, 50, 50, 255),
        3 => Color::new(255, 230, 40, 255),
        4 => Color::new(60, 220, 70, 255),
        5 => Color::new(0, 220, 255, 255),
        6 => Color::new(160, 80, 230, 255),
        7 => Color::new(255, 120, 190, 255),
        _ => hsv(t * 200.0 + spread),
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Setup {
    kind: usize,
    abil: [usize; 2],
    trail: usize,
    color: usize,
    death: usize,   // index into DEATH_NAMES (10 = random)
    victory: usize, // index into VICTORY_NAMES (10 = random)
}

impl Setup {
    fn new(kind: usize) -> Self {
        Setup { kind, abil: KINDS[kind].default_loadout(), trail: 1, color: 0, death: 0, victory: 0 }
    }
}

// =====================================================================
// game modes
// =====================================================================

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Mode {
    Classic,
    Timed,
    Stock,
    Hill,
}

pub(crate) const MODES: [Mode; 4] = [Mode::Classic, Mode::Timed, Mode::Stock, Mode::Hill];

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Classic => "CLASSIC",
            Mode::Timed => "TIMED",
            Mode::Stock => "STOCK",
            Mode::Hill => "KING OF THE HILL",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Mode::Classic => "a round is won when the other square is knocked out",
            Mode::Timed => "60 second clock - KO wins, otherwise most HP wins",
            Mode::Stock => "3 lives per round, you respawn until they run out",
            Mode::Hill => "stand in the glowing zone to score, 30 points or most in 90s",
        }
    }
}

// =====================================================================
// effects: particles, rings, screen shake, hit-stop ("cinematics")
// =====================================================================

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Shape {
    Circle,
    Ghost,
    Star,
    Streak,
    Square,
}

pub(crate) struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max: f32,
    size: f32,
    rot: f32,
    color: Color,
    shape: Shape,
    grow: bool,
    grav: f32,
    spin: f32, // degrees per second
}

impl Particle {
    fn new(x: f32, y: f32, vx: f32, vy: f32, life: f32, size: f32, color: Color, shape: Shape) -> Self {
        Particle { x, y, vx, vy, life, max: life, size, rot: 0.0, color, shape, grow: false, grav: 0.0, spin: 0.0 }
    }
}

pub(crate) struct Boom {
    x: f32,
    y: f32,
    r: f32,
    t: f32,
    color: Color,
}

pub(crate) enum ProjKind {
    Bomb,
    Frost,
    Fire,
}

pub(crate) struct Proj {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    owner: usize,
    life: f32,
    kind: ProjKind,
}

pub(crate) struct Strike {
    x: f32,
    t: f32,
    owner: usize,
    struck: bool,
    vis: f32,
    pts: Vec<Vector2>,
}

// arena meteor: shows a warning, then falls diagonally and explodes
pub(crate) struct Meteor {
    tx: f32,
    warn: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    falling: bool,
}

// neon arcade: a horizontal laser beam that warns, then sweeps the whole screen
pub(crate) struct Laser {
    y: f32,
    t: f32,
    fired: bool,
    vis: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PickKind {
    Health,
    Power,
    Haste,
    Reset,
}

pub(crate) struct Pickup {
    x: f32,
    y: f32,
    kind: PickKind,
    age: f32,
}

impl PickKind {
    fn color(self) -> Color {
        match self {
            PickKind::Health => Color::new(70, 220, 90, 255),
            PickKind::Power => Color::new(235, 70, 60, 255),
            PickKind::Haste => Color::new(60, 200, 255, 255),
            PickKind::Reset => Color::new(255, 220, 70, 255),
        }
    }
    fn label(self) -> &'static str {
        match self {
            PickKind::Health => "+",
            PickKind::Power => "P",
            PickKind::Haste => ">",
            PickKind::Reset => "C",
        }
    }
}

pub(crate) struct Fx {
    shake: f32,   // screen shake in pixels
    hitstop: f32, // freeze-frame seconds
    punch: f32,   // quick camera zoom punch
    flash: f32,   // white screen flash 0..1
}

pub(crate) struct World {
    projs: Vec<Proj>,
    booms: Vec<Boom>,
    parts: Vec<Particle>,
    strikes: Vec<Strike>,
    meteors: Vec<Meteor>,
    lasers: Vec<Laser>,
    pickups: Vec<Pickup>,
    banner: Option<(String, Color, f32)>, // big ultimate / event text
    grav_scale: f32,                      // map events can make gravity lighter
    rng: Rng,
    fx: Fx,
}

impl World {
    fn new() -> Self {
        World {
            projs: Vec::new(),
            booms: Vec::new(),
            parts: Vec::new(),
            strikes: Vec::new(),
            meteors: Vec::new(),
            lasers: Vec::new(),
            pickups: Vec::new(),
            banner: None,
            grav_scale: 1.0,
            rng: Rng(2463534242),
            fx: Fx { shake: 0.0, hitstop: 0.0, punch: 0.0, flash: 0.0 },
        }
    }

    fn clear(&mut self) {
        self.projs.clear();
        self.booms.clear();
        self.parts.clear();
        self.strikes.clear();
        self.meteors.clear();
        self.lasers.clear();
        self.pickups.clear();
        self.banner = None;
        self.grav_scale = 1.0;
        self.fx = Fx { shake: 0.0, hitstop: 0.0, punch: 0.0, flash: 0.0 };
    }

    fn ring(&mut self, x: f32, y: f32, r: f32, color: Color) {
        self.booms.push(Boom { x, y, r, t: 0.0, color });
    }

    fn shake(&mut self, amount: f32) {
        self.fx.shake = self.fx.shake.max(amount);
    }

    // a burst of particles flying out in all directions
    #[allow(clippy::too_many_arguments)]
    fn burst(&mut self, x: f32, y: f32, n: usize, col: Color, speed: f32, life: f32, size: f32, shape: Shape, grav: f32) {
        if self.parts.len() > 1600 {
            return;
        }
        for _ in 0..n {
            let a = self.rng.range(0.0, 6.2832);
            let s = speed * self.rng.range(0.3, 1.0);
            let mut q = Particle::new(
                x,
                y,
                a.cos() * s,
                a.sin() * s,
                life * self.rng.range(0.6, 1.0),
                size * self.rng.range(0.6, 1.2),
                col,
                shape,
            );
            q.grav = grav;
            q.rot = self.rng.range(0.0, 90.0);
            self.parts.push(q);
        }
    }
}

pub(crate) fn update_particles(parts: &mut Vec<Particle>, dt: f32) {
    for q in parts.iter_mut() {
        q.vy += q.grav * dt;
        q.x += q.vx * dt;
        q.y += q.vy * dt;
        q.rot += q.spin * dt;
        q.life -= dt;
    }
    parts.retain(|q| q.life > 0.0);
}

pub(crate) fn update_booms(booms: &mut Vec<Boom>, dt: f32) {
    for b in booms.iter_mut() {
        b.t += dt;
    }
    booms.retain(|b| b.t < BOOM_TIME);
}


// =====================================================================
// input (humans read the keyboard, the CPU builds the same struct)
// =====================================================================

#[derive(Clone, Copy, Default)]
pub(crate) struct Keys;


#[derive(Clone, Copy, Default)]
pub(crate) struct Input {
    left: bool,
    right: bool,
    down: bool,
    jump: bool,
    a1: bool,
    a2: bool,
    melee: bool,
    block_down: bool,
    ult: bool,
}


// =====================================================================
// CPU personalities
// =====================================================================

// every number is a per-frame chance or a distance, so a profile is just a "feel"
pub(crate) struct Profile {
    dist: f32,   // preferred fighting distance
    flee: f32,   // chance to back off when the foe is too close
    melee: f32,  // knife swings when in reach
    abil: f32,   // chance to fire a ready ability
    block: f32,  // chance to guard when the foe attacks
    jump: f32,   // random hopping
    ult: f32,    // chance to cash in a full ultimate
    react: f32,  // 1.0 = acts every frame, lower = slow reactions
    smart: f32,  // how often it checks an ability makes sense before using it
    wobble: f32, // erratic strafing
    range: f32,  // how far away it will use abilities
}

pub(crate) const CPU_NAMES: [&str; 9] = [
    "BALANCED",
    "AGGRESSOR",
    "DEFENDER",
    "SNIPER",
    "TRICKSTER",
    "BERSERKER",
    "ROOKIE",
    "PRO",
    "RANDOM",
];

pub(crate) const CPU_BLURBS: [&str; 9] = [
    "a bit of everything",
    "closes in fast and loves the knife",
    "guards and parries, then counters",
    "keeps its distance and fires abilities",
    "erratic, hops around, unpredictable",
    "relentless, no defence, spams everything",
    "slow reactions, easy to beat",
    "sharp spacing, smart abilities, hard to hit",
    "a different personality each round",
];

pub(crate) fn cpu_profile(idx: usize) -> Profile {
    let p = |dist, flee, melee, abil, block, jump, ult, react, smart, wobble, range| Profile {
        dist,
        flee,
        melee,
        abil,
        block,
        jump,
        ult,
        react,
        smart,
        wobble,
        range,
    };
    match idx {
        1 => p(70.0, 0.05, 0.25, 0.05, 0.15, 0.006, 0.08, 1.0, 0.5, 0.1, 450.0), // AGGRESSOR
        2 => p(160.0, 0.5, 0.08, 0.025, 0.9, 0.003, 0.05, 1.0, 0.9, 0.05, 450.0), // DEFENDER
        3 => p(380.0, 0.9, 0.06, 0.07, 0.4, 0.01, 0.07, 1.0, 0.9, 0.3, 800.0),   // SNIPER
        4 => p(130.0, 0.4, 0.15, 0.06, 0.3, 0.02, 0.1, 1.0, 0.4, 0.8, 520.0),    // TRICKSTER
        5 => p(50.0, 0.0, 0.35, 0.08, 0.0, 0.008, 0.15, 1.0, 0.3, 0.2, 500.0),   // BERSERKER
        6 => p(130.0, 0.1, 0.05, 0.015, 0.1, 0.004, 0.02, 0.35, 0.2, 0.4, 420.0), // ROOKIE
        7 => p(100.0, 0.5, 0.2, 0.06, 0.8, 0.005, 0.1, 1.0, 1.0, 0.15, 560.0),   // PRO
        _ => p(110.0, 0.3, 0.12, 0.03, 0.5, 0.004, 0.05, 1.0, 0.7, 0.1, 520.0),  // BALANCED
    }
}

// would using this ability right now make sense?
pub(crate) fn ability_ok(ab: Ability, me: &Player, foe: &Player, dist: f32, foe_attacking: bool) -> bool {
    match ab {
        Ability::Mend => me.hp < me.max_hp - 25.0,
        Ability::Shield => foe_attacking || dist < 160.0,
        Ability::Pulse => dist < 190.0,
        Ability::Dash => dist > 90.0 && dist < 420.0,
        Ability::Magnet => dist > 220.0,
        Ability::Slam => dist < 300.0,
        Ability::Blink => dist > 200.0 || foe_attacking,
        Ability::Hop => foe.y + 40.0 < me.y || foe_attacking,
        Ability::Bomb | Ability::Frost | Ability::Fireball => dist > 130.0,
        Ability::Thunder => true,
    }
}

// the CPU brain: reads the same game state a player sees and builds an Input
pub(crate) fn bot_input(me: &Player, foe: &Player, rng: &mut Rng, prof: &Profile, t: f32, seed: f32) -> Input {
    let mut i = Input::default();
    let (mx, my) = me.center();
    let (fx, fy) = foe.center();
    let dx = fx - mx;
    let dy = fy - my;
    let dist = dx.abs();
    let foe_attacking = foe.melee_t > 0.0 || foe.dash_t > 0.0 || foe.slam;
    let react = rng.next() < prof.react; // slow profiles skip decisions some frames

    // ---- movement: hold the preferred distance ----
    let gap = dist - prof.dist;
    let mut want = if gap > 30.0 {
        dx.signum()
    } else if gap < -30.0 && rng.next() < prof.flee {
        -dx.signum()
    } else {
        0.0
    };
    // erratic profiles sometimes strafe the wrong way
    if prof.wobble > 0.0 && (t * 1.7 + seed).sin() > 1.0 - prof.wobble * 1.5 {
        want = -want;
    }
    if want > 0.0 {
        i.right = true;
    } else if want < 0.0 {
        i.left = true;
    }

    // ---- jumping: reach the foe, climb walls, double jump ----
    if me.on_ground {
        if dy < -80.0 && rng.next() < 0.06 {
            i.jump = true;
        } else if me.vx.abs() < 5.0 && want != 0.0 && rng.next() < 0.1 {
            i.jump = true;
        } else if rng.next() < prof.jump {
            i.jump = true;
        }
    } else if me.wall_dir != 0.0 && rng.next() < 0.1 {
        i.jump = true;
    } else if me.air_jumps > 0 && dy < -120.0 && me.vy > 0.0 && rng.next() < 0.05 {
        i.jump = true;
    }

    // ---- attacking ----
    if react {
        if dist < 95.0 && dy.abs() < 80.0 && rng.next() < prof.melee {
            i.melee = true;
        }
        let loadout = me.abilities;
        let in_range = dist < prof.range;
        let wants = |ab: Ability, rng: &mut Rng| -> bool {
            in_range && (rng.next() >= prof.smart || ability_ok(ab, me, foe, dist, foe_attacking))
        };
        if me.cd1 <= 0.0 && rng.next() < prof.abil && wants(loadout[0], rng) {
            i.a1 = true;
        }
        if me.cd2 <= 0.0 && rng.next() < prof.abil && wants(loadout[1], rng) {
            i.a2 = true;
        }
        if me.ult >= ULT_MAX && dist < 650.0 && rng.next() < prof.ult {
            i.ult = true;
        }
    }

    // ---- defending ----
    if foe_attacking && dist < 170.0 && rng.next() < prof.block * if react { 1.0 } else { 0.4 } {
        i.block_down = true;
    }
    i
}

// =====================================================================
// players and world
// =====================================================================

pub(crate) struct Player {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    on_ground: bool,
    rot: f32,
    charge: f32,
    crouch: f32,
    tap_timer: f32,
    facing: f32,
    eyes: Eyes,
    hp: f32,
    max_hp: f32,
    kind: Kind,
    abilities: [Ability; 2],
    trail_style: usize,
    trail_color: usize,
    trail_acc: f32,
    #[allow(dead_code)]
    keys: Keys,
    cd1: f32,
    cd2: f32,
    dash_t: f32,
    dash_hit: bool,
    slam: bool,
    slam_arm: bool,
    shield_t: f32,
    hurt_t: f32,
    melee_t: f32,
    melee_cd: f32,
    melee_hit: bool,
    frozen_t: f32,
    burn_t: f32,
    regen_t: f32,
    stun_t: f32,
    // movement tech
    wall_dir: f32,
    air_jumps: i32,
    // guard / parry
    block_held: bool,
    block_prev: bool,
    block_t: f32,
    block_cd: f32,
    parry_t: f32,
    parried: bool,
    blocked: bool,
    // ultimate and buffs
    ult: f32,
    over_t: f32,
    power_t: f32,
    haste_t: f32,
    dead_t: f32,
    invuln: bool, // the champion during a victory cutscene
}

impl Player {
    fn new(setup: &Setup, x: f32, facing: f32, keys: Keys) -> Self {
        let kind = KINDS[setup.kind];
        let mut p = Player {
            x,
            y: H as f32 - FLOOR_H - SIZE,
            vx: 0.0,
            vy: 0.0,
            on_ground: true,
            rot: 0.0,
            charge: 0.0,
            crouch: 0.0,
            tap_timer: 0.0,
            facing,
            eyes: Eyes::Forward,
            hp: kind.max_hp(),
            max_hp: kind.max_hp(),
            kind,
            abilities: [ABILITIES[setup.abil[0]], ABILITIES[setup.abil[1]]],
            trail_style: setup.trail,
            trail_color: setup.color,
            trail_acc: 0.0,
            keys,
            cd1: 0.0,
            cd2: 0.0,
            dash_t: 0.0,
            dash_hit: false,
            slam: false,
            slam_arm: false,
            shield_t: 0.0,
            hurt_t: 0.0,
            melee_t: 0.0,
            melee_cd: 0.0,
            melee_hit: false,
            frozen_t: 0.0,
            burn_t: 0.0,
            regen_t: 0.0,
            stun_t: 0.0,
            wall_dir: 0.0,
            air_jumps: kind.air_jumps(),
            block_held: false,
            block_prev: false,
            block_t: 0.0,
            block_cd: 0.0,
            parry_t: 0.0,
            parried: false,
            blocked: false,
            ult: 0.0,
            over_t: 0.0,
            power_t: 0.0,
            haste_t: 0.0,
            dead_t: 0.0,
            invuln: false,
        };
        p.apply_setup(setup);
        p
    }

    fn apply_setup(&mut self, s: &Setup) {
        self.kind = KINDS[s.kind];
        self.abilities = [ABILITIES[s.abil[0]], ABILITIES[s.abil[1]]];
        self.trail_style = s.trail;
        self.trail_color = s.color;
    }

    fn rect(&self) -> Rectangle {
        Rectangle::new(self.x, self.y, SIZE, SIZE)
    }

    fn center(&self) -> (f32, f32) {
        (self.x + SIZE / 2.0, self.y + SIZE / 2.0)
    }

    fn speed_mult(&self) -> f32 {
        let mut m = self.kind.speed();
        if self.haste_t > 0.0 {
            m *= 1.4;
        }
        if self.over_t > 0.0 {
            m *= 1.5;
        }
        if self.block_held {
            m *= 0.5;
        }
        m
    }

    // damage + knockback. Shield blocks everything, a well-timed block parries,
    // a held block takes a fraction of the hit and no knockback.
    fn hurt(&mut self, dmg: f32, kx: f32, ky: f32) {
        if self.dead_t > 0.0 || self.shield_t > 0.0 || self.invuln {
            return;
        }
        if self.parry_t > 0.0 {
            self.parry_t = 0.0;
            self.parried = true;
            return;
        }
        let mut dmg = dmg * self.kind.dmg_taken();
        let mut kx = kx * self.kind.kb_resist();
        let mut ky = ky * self.kind.kb_resist();
        if self.block_held {
            dmg *= 0.3;
            kx *= 0.2;
            ky = 0.0;
            self.blocked = true;
        }
        self.hp = (self.hp - dmg).max(0.0);
        self.vx = (self.vx + kx).clamp(-MAX_SPEED, MAX_SPEED);
        if ky != 0.0 {
            self.vy = ky.clamp(-MAX_SPEED, MAX_SPEED);
            self.on_ground = false;
        }
        self.hurt_t = 0.25;
    }
}

// everything you can stand on is fully solid: you collide from any direction
pub(crate) struct Solid {
    r: Rectangle,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Theme {
    Meadow,
    Skyline,
    Volcano,
    Frozen,
    Space,
    Sea,
    Sand,
    Neon,
}

pub(crate) const THEMES: [Theme; 8] = [
    Theme::Meadow,
    Theme::Skyline,
    Theme::Volcano,
    Theme::Frozen,
    Theme::Space,
    Theme::Sea,
    Theme::Sand,
    Theme::Neon,
];

impl Theme {
    fn name(self) -> &'static str {
        match self {
            Theme::Meadow => "SUNNY MEADOW",
            Theme::Skyline => "NIGHT SKYLINE",
            Theme::Volcano => "VOLCANO",
            Theme::Frozen => "FROZEN PEAKS",
            Theme::Space => "DEEP SPACE",
            Theme::Sea => "DEEP SEA",
            Theme::Sand => "SAND RUINS",
            Theme::Neon => "NEON ARCADE",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Theme::Meadow => "open hills, wind gusts push everyone",
            Theme::Skyline => "rooftops and towers, meteors fall",
            Theme::Volcano => "lava surges and meteors - stay off the floor!",
            Theme::Frozen => "icy ledges, blizzards shove you around",
            Theme::Space => "zero-g bursts and falling comets",
            Theme::Sea => "floaty water and shifting currents",
            Theme::Sand => "crumbling ruins, rocks fall from the sky",
            Theme::Neon => "sweeping laser beams - watch the warnings",
        }
    }
}

pub(crate) struct Map {
    solids: Vec<Solid>,
    hazard: Option<Rectangle>, // lava: damages while you touch it
    zones: Vec<Rectangle>,     // king of the hill spots
    spawns: [f32; 2],
}

pub(crate) fn make_map(theme: Theme) -> Map {
    let floor_y = H as f32 - FLOOR_H;
    let b = |x: f32, y: f32, w: f32, h: f32| Solid { r: Rectangle::new(x, y, w, h) };
    let z = |x: f32, y: f32, w: f32, h: f32| Rectangle::new(x, y, w, h);
    let floor = b(0.0, floor_y, W as f32, FLOOR_H);
    match theme {
        Theme::Meadow => Map {
            solids: vec![
                floor,
                b(590.0, floor_y - 70.0, 100.0, 70.0), // center block
                b(0.0, 300.0, 70.0, 30.0),             // wall ledges
                b(W as f32 - 70.0, 300.0, 70.0, 30.0),
                b(140.0, 520.0, 240.0, 20.0),
                b(900.0, 520.0, 240.0, 20.0),
                b(520.0, 420.0, 240.0, 20.0),
                b(200.0, 330.0, 200.0, 20.0),
                b(880.0, 330.0, 200.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(520.0, 340.0, 240.0, 80.0), z(140.0, 440.0, 240.0, 80.0), z(900.0, 440.0, 240.0, 80.0)],
            spawns: [120.0, W as f32 - 120.0 - SIZE],
        },
        Theme::Skyline => Map {
            solids: vec![
                floor,
                b(0.0, 560.0, 170.0, 120.0), // left and right buildings
                b(W as f32 - 170.0, 560.0, 170.0, 120.0),
                b(400.0, 500.0, 140.0, 180.0), // twin towers
                b(740.0, 500.0, 140.0, 180.0),
                b(560.0, 400.0, 160.0, 20.0), // floating billboards
                b(190.0, 400.0, 180.0, 20.0),
                b(910.0, 400.0, 180.0, 20.0),
                b(560.0, 270.0, 160.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(560.0, 320.0, 160.0, 80.0), z(400.0, 420.0, 140.0, 80.0), z(190.0, 320.0, 180.0, 80.0)],
            spawns: [210.0, W as f32 - 210.0 - SIZE],
        },
        Theme::Volcano => Map {
            solids: vec![
                floor,
                b(0.0, 440.0, 50.0, 240.0), // rock walls
                b(W as f32 - 50.0, 440.0, 50.0, 240.0),
                b(560.0, 520.0, 160.0, 20.0), // stepping stones over the lava
                b(120.0, 560.0, 200.0, 20.0),
                b(960.0, 560.0, 200.0, 20.0),
                b(330.0, 400.0, 160.0, 20.0),
                b(790.0, 400.0, 160.0, 20.0),
                b(560.0, 280.0, 160.0, 20.0),
            ],
            hazard: Some(Rectangle::new(470.0, floor_y - 10.0, 340.0, 14.0)),
            zones: vec![z(560.0, 440.0, 160.0, 80.0), z(330.0, 320.0, 160.0, 80.0), z(790.0, 320.0, 160.0, 80.0)],
            spawns: [120.0, W as f32 - 120.0 - SIZE],
        },
        Theme::Frozen => Map {
            solids: vec![
                floor,
                b(590.0, floor_y - 70.0, 100.0, 70.0), // center ice block
                b(300.0, 560.0, 120.0, 120.0),         // ice pillars
                b(860.0, 560.0, 120.0, 120.0),
                b(120.0, 470.0, 200.0, 20.0),
                b(960.0, 470.0, 200.0, 20.0),
                b(540.0, 400.0, 200.0, 20.0),
                b(300.0, 300.0, 160.0, 20.0),
                b(820.0, 300.0, 160.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(540.0, 320.0, 200.0, 80.0), z(120.0, 390.0, 200.0, 80.0), z(960.0, 390.0, 200.0, 80.0)],
            spawns: [100.0, W as f32 - 100.0 - SIZE],
        },
        Theme::Space => Map {
            solids: vec![
                floor,
                b(600.0, 600.0, 80.0, 80.0), // central station block
                b(100.0, 540.0, 180.0, 20.0),
                b(1000.0, 540.0, 180.0, 20.0),
                b(300.0, 430.0, 160.0, 20.0),
                b(820.0, 430.0, 160.0, 20.0),
                b(560.0, 340.0, 160.0, 20.0),
                b(120.0, 300.0, 140.0, 20.0),
                b(1020.0, 300.0, 140.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(560.0, 260.0, 160.0, 80.0), z(300.0, 350.0, 160.0, 80.0), z(820.0, 350.0, 160.0, 80.0)],
            spawns: [120.0, W as f32 - 120.0 - SIZE],
        },
        Theme::Sea => Map {
            solids: vec![
                floor,
                b(200.0, 590.0, 160.0, 90.0), // coral mounds
                b(920.0, 590.0, 160.0, 90.0),
                b(540.0, 560.0, 200.0, 120.0), // shipwreck hull
                b(580.0, 480.0, 120.0, 20.0),  // ship's deck
                b(80.0, 430.0, 180.0, 20.0),
                b(1020.0, 430.0, 180.0, 20.0),
                b(330.0, 360.0, 160.0, 20.0),
                b(790.0, 360.0, 160.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(580.0, 400.0, 120.0, 80.0), z(330.0, 280.0, 160.0, 80.0), z(790.0, 280.0, 160.0, 80.0)],
            spawns: [100.0, W as f32 - 100.0 - SIZE],
        },
        Theme::Sand => Map {
            solids: vec![
                floor,
                b(540.0, 620.0, 200.0, 60.0), // stepped pyramid
                b(580.0, 560.0, 120.0, 60.0),
                b(620.0, 500.0, 40.0, 60.0),
                b(180.0, 520.0, 40.0, 160.0), // temple pillars
                b(1060.0, 520.0, 40.0, 160.0),
                b(140.0, 500.0, 120.0, 20.0),
                b(1020.0, 500.0, 120.0, 20.0),
                b(330.0, 420.0, 200.0, 20.0),
                b(750.0, 420.0, 200.0, 20.0),
                b(540.0, 330.0, 200.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(540.0, 250.0, 200.0, 80.0), z(330.0, 340.0, 200.0, 80.0), z(750.0, 340.0, 200.0, 80.0)],
            spawns: [90.0, W as f32 - 90.0 - SIZE],
        },
        Theme::Neon => Map {
            solids: vec![
                floor,
                b(600.0, 600.0, 80.0, 80.0), // center cabinet
                b(60.0, 540.0, 200.0, 20.0),
                b(1020.0, 540.0, 200.0, 20.0),
                b(300.0, 450.0, 150.0, 20.0),
                b(830.0, 450.0, 150.0, 20.0),
                b(565.0, 360.0, 150.0, 20.0),
                b(60.0, 300.0, 160.0, 20.0),
                b(1060.0, 300.0, 160.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(565.0, 280.0, 150.0, 80.0), z(300.0, 370.0, 150.0, 80.0), z(830.0, 370.0, 150.0, 80.0)],
            spawns: [110.0, W as f32 - 110.0 - SIZE],
        },
    }
}

// =====================================================================
// per-round state
// =====================================================================

pub(crate) struct Round {
    time: f32,
    stocks: [i32; 2],
    hill: [f32; 2],
    zone_idx: usize,
    zone_t: f32,
    pickup_t: f32,
    meteor_t: f32,
    wind: f32,
    wind_warn: bool,
    zerog: bool,
    zerog_warn: bool,
    combo: [u32; 2],
    combo_t: [f32; 2],
    combo_show: [f32; 2],
}

impl Round {
    fn new() -> Self {
        Round {
            time: 0.0,
            stocks: [STOCKS, STOCKS],
            hill: [0.0, 0.0],
            zone_idx: 0,
            zone_t: HILL_MOVE,
            pickup_t: 6.0,
            meteor_t: 4.0,
            wind: 0.0,
            wind_warn: false,
            zerog: false,
            zerog_warn: false,
            combo: [0, 0],
            combo_t: [0.0, 0.0],
            combo_show: [0.0, 0.0],
        }
    }
}

// =====================================================================
// physics
// =====================================================================

// last line of defense: nobody ever leaves the arena
pub(crate) fn clamp_to_arena(p: &mut Player) {
    let max_x = W as f32 - SIZE;
    if p.x < 0.0 {
        p.x = 0.0;
        p.vx = p.vx.max(0.0);
    } else if p.x > max_x {
        p.x = max_x;
        p.vx = p.vx.min(0.0);
    }
    if p.y < 0.0 {
        p.y = 0.0;
        p.vy = p.vy.max(0.0);
    }
    let max_y = H as f32 - FLOOR_H - SIZE;
    if p.y > max_y {
        p.y = max_y;
        p.vy = 0.0;
        p.on_ground = true;
    }
}

// keep inside the screen walls and out of blocks (sideways)
pub(crate) fn push_out_x(p: &mut Player, solids: &[Solid]) {
    let max_x = W as f32 - SIZE;
    p.x = p.x.clamp(0.0, max_x);
    for s in solids {
        if !overlaps(&p.rect(), &s.r) {
            continue;
        }
        let left_x = s.r.x - SIZE; // stand to the left of the block
        let right_x = s.r.x + s.r.width; // stand to the right of it
        let from_left = if left_x < 0.0 {
            false // no room on the left side of the screen
        } else if right_x > max_x {
            true
        } else if p.vx > 0.0 {
            true
        } else if p.vx < 0.0 {
            false
        } else {
            p.x + SIZE / 2.0 < s.r.x + s.r.width / 2.0
        };
        p.x = if from_left { left_x } else { right_x };
        p.vx = 0.0;
    }
    p.x = p.x.clamp(0.0, max_x);
}

// squares are solid to each other: stand on heads, shove sideways
pub(crate) fn collide_players(pl: &mut [Player; 2], solids: &[Solid]) {
    let (a, b) = pl.split_at_mut(1);
    let (a, b) = (&mut a[0], &mut b[0]);
    if a.dead_t > 0.0 || b.dead_t > 0.0 || !overlaps(&a.rect(), &b.rect()) {
        return;
    }
    let ox = (a.x + SIZE).min(b.x + SIZE) - a.x.max(b.x);
    let oy = (a.y + SIZE).min(b.y + SIZE) - a.y.max(b.y);
    if oy < ox {
        // the top square rides on the bottom one; the bottom one is never
        // stopped, so jumps, hops and leaps still work with someone on your head
        let (up, low) = if a.y < b.y { (a, b) } else { (b, a) };
        up.y = low.y - SIZE;
        up.vy = low.vy;
        up.on_ground = true;
    } else {
        let push = ox / 2.0 + 0.5;
        if a.x + SIZE / 2.0 < b.x + SIZE / 2.0 {
            a.x -= push;
            b.x += push;
        } else {
            a.x += push;
            b.x -= push;
        }
        let avg = (a.vx + b.vx) / 2.0;
        a.vx = avg - (a.vx - avg) * 0.2;
        b.vx = avg - (b.vx - avg) * 0.2;
        push_out_x(a, solids);
        push_out_x(b, solids);
    }
}

pub(crate) fn step_once(p: &mut Player, solids: &[Solid], dt: f32, gs: f32) {
    p.x += p.vx * dt;
    push_out_x(p, solids);

    if p.dash_t <= 0.0 {
        p.vy += GRAVITY * p.kind.grav() * gs * dt;
    }
    let prev_bottom = p.y + SIZE;
    p.y += p.vy * dt;
    p.on_ground = false;
    for s in solids {
        if !overlaps(&p.rect(), &s.r) {
            continue;
        }
        if p.vy >= 0.0 && prev_bottom <= s.r.y + 1.0 {
            p.y = s.r.y - SIZE;
            p.vy = 0.0;
            p.on_ground = true;
        } else if p.vy < 0.0 {
            p.y = s.r.y + s.r.height; // bonk head
            p.vy = 0.0;
        }
    }
}

// which side (if any) is a wall within reach: -1 left, 1 right, 0 none
pub(crate) fn wall_side(p: &Player, solids: &[Solid]) -> f32 {
    if p.on_ground {
        return 0.0;
    }
    if p.x <= 1.0 {
        return -1.0;
    }
    if p.x >= W as f32 - SIZE - 1.0 {
        return 1.0;
    }
    for dir in [-1.0_f32, 1.0] {
        let probe = Rectangle::new(p.x + dir * 3.0, p.y + 6.0, SIZE, SIZE - 12.0);
        if solids.iter().any(|s| overlaps(&probe, &s.r)) {
            return dir;
        }
    }
    0.0
}

// move + collide; fast movers are split into small steps so nobody tunnels
pub(crate) fn step(p: &mut Player, solids: &[Solid], dt: f32, gs: f32) {
    let moves = (p.vx.abs().max(p.vy.abs()) * dt / 20.0).ceil().clamp(1.0, 8.0) as i32;
    let sub = dt / moves as f32;
    for _ in 0..moves {
        step_once(p, solids, sub, gs);
    }
    clamp_to_arena(p);
    p.wall_dir = wall_side(p, solids);
}

pub(crate) fn launch(p: &mut Player, power: f32) {
    p.vy = -(2.0 * GRAVITY * JUMP * power).sqrt();
    p.on_ground = false;
    p.charge = 0.0;
    p.crouch = 0.0;
}

// =====================================================================
// abilities
// =====================================================================

pub(crate) fn use_ability(ab: Ability, me: &mut Player, foe: &mut Player, my_idx: usize, solids: &[Solid], w: &mut World) {
    let (cx, cy) = me.center();
    match ab {
        Ability::Dash => {
            me.dash_t = DASH_TIME;
            me.dash_hit = false;
            w.burst(cx, cy, 10, Color::new(255, 200, 100, 255), 300.0, 0.3, 3.0, Shape::Streak, 0.0);
            w.shake(4.0);
        }
        Ability::Slam => {
            if me.on_ground {
                // leap up first, then crash down at the top of the leap
                me.vy = -(2.0 * GRAVITY * JUMP * 0.8).sqrt();
                me.on_ground = false;
                me.slam_arm = true;
                w.ring(cx, me.y + SIZE, 60.0, Color::new(255, 220, 120, 255));
                w.burst(cx, me.y + SIZE, 10, Color::new(200, 190, 170, 200), 160.0, 0.5, 8.0, Shape::Circle, -40.0);
            } else {
                me.slam = true;
            }
        }
        Ability::Bomb => {
            w.projs.push(Proj {
                x: cx,
                y: cy,
                vx: me.facing * 550.0 + me.vx * 0.5,
                vy: -300.0,
                owner: my_idx,
                life: 3.0,
                kind: ProjKind::Bomb,
            });
            w.burst(cx + me.facing * 30.0, cy, 6, Color::new(255, 190, 60, 255), 180.0, 0.25, 4.0, Shape::Circle, 0.0);
        }
        Ability::Hop => {
            me.vy = -(2.0 * GRAVITY * JUMP * 1.2).sqrt();
            me.on_ground = false;
            w.ring(cx, me.y + SIZE, 70.0, Color::WHITE);
            w.burst(cx, me.y + SIZE, 14, Color::new(230, 230, 230, 200), 220.0, 0.5, 7.0, Shape::Circle, 60.0);
        }
        Ability::Shield => {
            me.shield_t = SHIELD_TIME;
            w.ring(cx, cy, 90.0, Color::new(120, 220, 255, 255));
            w.burst(cx, cy, 16, Color::new(150, 230, 255, 255), 260.0, 0.4, 6.0, Shape::Star, 0.0);
        }
        Ability::Pulse => {
            let reach = 170.0;
            let rx = if me.facing > 0.0 { me.x + SIZE } else { me.x - reach };
            let zone = Rectangle::new(rx, me.y, reach, SIZE);
            let fx = if me.facing > 0.0 { me.x + SIZE + 40.0 } else { me.x - 40.0 };
            if overlaps(&zone, &foe.rect()) {
                foe.hurt(12.0, me.facing * 700.0, -250.0);
            }
            w.ring(fx, cy, 85.0, Color::new(255, 255, 200, 255));
            for k in 0..8 {
                let sp = 250.0 + k as f32 * 40.0;
                let q = Particle::new(cx + me.facing * 35.0, cy + (k as f32 - 3.5) * 8.0, me.facing * sp, 0.0, 0.3, 3.0, Color::WHITE, Shape::Streak);
                w.parts.push(q);
            }
            w.shake(6.0);
        }
        Ability::Blink => {
            let mut nx = me.x;
            for _ in 0..(BLINK_DIST / 10.0) as i32 {
                let tx = nx + me.facing * 10.0;
                let r = Rectangle::new(tx, me.y, SIZE, SIZE);
                if tx < 0.0 || tx > W as f32 - SIZE || solids.iter().any(|s| overlaps(&r, &s.r)) || overlaps(&r, &foe.rect()) {
                    break;
                }
                nx = tx;
            }
            let violet = Color::new(190, 120, 255, 255);
            // streak between start and end, flashes at both ends
            for k in 0..14 {
                let f = k as f32 / 13.0;
                let px = cx + (nx - me.x) * f;
                let q = Particle::new(px, cy + w.rng.range(-20.0, 20.0), 0.0, w.rng.range(-20.0, 20.0), 0.4, 5.0, violet, Shape::Star);
                w.parts.push(q);
            }
            w.ring(cx, cy, 70.0, violet);
            w.burst(cx, cy, 14, violet, 260.0, 0.4, 5.0, Shape::Square, 0.0);
            me.x = nx;
            w.ring(nx + SIZE / 2.0, cy, 70.0, Color::WHITE);
            w.burst(nx + SIZE / 2.0, cy, 14, Color::WHITE, 260.0, 0.4, 5.0, Shape::Star, 0.0);
            me.vy = me.vy.min(0.0);
        }
        Ability::Frost => {
            w.projs.push(Proj { x: cx, y: cy, vx: me.facing * 700.0, vy: 0.0, owner: my_idx, life: 1.2, kind: ProjKind::Frost });
            w.ring(cx + me.facing * 30.0, cy, 50.0, Color::new(150, 230, 255, 255));
        }
        Ability::Thunder => {
            let (fx, _) = foe.center();
            w.strikes.push(Strike { x: fx, t: STRIKE_DELAY, owner: my_idx, struck: false, vis: 0.0, pts: Vec::new() });
            w.burst(cx, cy, 8, Color::new(255, 240, 120, 255), 200.0, 0.4, 5.0, Shape::Star, 0.0);
        }
        Ability::Magnet => {
            let (fx, fy) = foe.center();
            let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
            if dist < 650.0 {
                let dir: f32 = if fx > cx { -1.0 } else { 1.0 };
                foe.vx = (dir * 1100.0).clamp(-MAX_SPEED, MAX_SPEED);
                foe.vy = -140.0;
                foe.on_ground = false;
                for k in 0..10 {
                    let f = k as f32 / 10.0;
                    let q = Particle::new(fx + (cx - fx) * f, fy + (cy - fy) * f, (cx - fx) * 1.5, (cy - fy) * 1.5, 0.3, 3.0, Color::new(255, 120, 200, 255), Shape::Streak);
                    w.parts.push(q);
                }
            }
            w.ring(cx, cy, 130.0, Color::new(255, 120, 200, 255));
            w.shake(5.0);
        }
        Ability::Mend => {
            me.regen_t = 2.0;
            w.ring(cx, cy, 80.0, Color::new(90, 255, 120, 255));
            w.burst(cx, cy, 14, Color::new(120, 255, 140, 255), 140.0, 0.8, 7.0, Shape::Star, -80.0);
        }
        Ability::Fireball => {
            w.projs.push(Proj { x: cx, y: cy, vx: me.facing * 650.0, vy: 0.0, owner: my_idx, life: 1.5, kind: ProjKind::Fire });
            w.burst(cx + me.facing * 30.0, cy, 8, Color::new(255, 150, 40, 255), 220.0, 0.3, 6.0, Shape::Circle, 0.0);
            w.shake(3.0);
        }
    }
}

// each character's ultimate: a big cinematic super move
pub(crate) fn use_ult(me: &mut Player, foe: &mut Player, my_idx: usize, w: &mut World) {
    let (cx, cy) = me.center();
    let col = me.kind.color();
    w.fx.hitstop = w.fx.hitstop.max(0.25);
    w.fx.flash = w.fx.flash.max(0.6);
    w.fx.punch = w.fx.punch.max(0.1);
    w.shake(16.0);
    w.ring(cx, cy, 300.0, Color::WHITE);
    w.ring(cx, cy, 200.0, col);
    w.burst(cx, cy, 30, col, 600.0, 0.7, 8.0, Shape::Star, 0.0);
    w.banner = Some((me.kind.ult_name().to_string(), col, 1.5));
    match me.kind {
        Kind::Dasher => {
            me.over_t = 6.0;
            me.cd1 = 0.0;
            me.cd2 = 0.0;
        }
        Kind::Bomber => {
            for k in 0..8 {
                w.projs.push(Proj {
                    x: 90.0 + k as f32 * 150.0,
                    y: -40.0 - k as f32 * 70.0,
                    vx: 0.0,
                    vy: 260.0,
                    owner: my_idx,
                    life: 5.0,
                    kind: ProjKind::Bomb,
                });
            }
        }
        Kind::Shielder => {
            me.shield_t = 3.0;
            let (fx, fy) = foe.center();
            let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
            if dist < 450.0 {
                let dir = if fx >= cx { 1.0 } else { -1.0 };
                foe.hurt(18.0, dir * 900.0, -300.0);
            }
            w.ring(cx, cy, 450.0, Color::new(150, 230, 255, 255));
        }
        Kind::Titan => {
            w.shake(30.0);
            for k in 0..14 {
                let x = 40.0 + k as f32 * 95.0;
                w.burst(x, H as f32 - FLOOR_H, 4, Color::new(120, 100, 80, 255), 380.0, 0.9, 12.0, Shape::Square, 900.0);
            }
            w.ring(cx, me.y + SIZE, 600.0, Color::new(255, 190, 80, 255));
            if foe.on_ground && foe.dead_t <= 0.0 {
                foe.hurt(25.0, 0.0, -650.0);
                foe.stun_t = 0.6;
            }
        }
        Kind::Ghost => {
            let (fx, fy) = foe.center();
            if foe.shield_t <= 0.0 && foe.dead_t <= 0.0 {
                foe.frozen_t = 2.0;
                foe.vx = 0.0;
                foe.hp = (foe.hp - 10.0).max(0.0);
                foe.hurt_t = 0.15;
            }
            w.ring(fx, fy, 260.0, Color::new(170, 235, 255, 255));
            w.burst(fx, fy, 30, Color::new(190, 240, 255, 255), 500.0, 0.8, 8.0, Shape::Star, 200.0);
        }
        Kind::Spark => {
            let (fx, _) = foe.center();
            for (k, off) in [-240.0_f32, -120.0, 0.0, 120.0, 240.0].iter().enumerate() {
                w.strikes.push(Strike {
                    x: (fx + off).clamp(40.0, W as f32 - 40.0),
                    t: 0.8 + k as f32 * 0.22,
                    owner: my_idx,
                    struck: false,
                    vis: 0.0,
                    pts: Vec::new(),
                });
            }
        }
    }
}

// runs AFTER collisions are resolved, so landing on the other square counts too
pub(crate) fn slam_land(me: &mut Player, foe: &mut Player, w: &mut World) {
    if !(me.slam && me.on_ground) {
        return;
    }
    me.slam = false;
    let (mx, _) = me.center();
    let (fx, _) = foe.center();
    let base = me.y + SIZE;
    w.ring(mx, base, 250.0, Color::new(255, 190, 80, 255));
    w.ring(mx, base, 150.0, Color::WHITE);
    w.burst(mx, base, 26, Color::new(150, 120, 90, 255), 520.0, 0.8, 12.0, Shape::Square, 900.0);
    w.burst(mx, base, 14, Color::new(210, 200, 185, 180), 300.0, 0.9, 10.0, Shape::Circle, -30.0);
    for k in 0..16 {
        let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
        let q = Particle::new(mx, base - 4.0, dir * (200.0 + k as f32 * 35.0), 0.0, 0.35, 3.0, Color::WHITE, Shape::Streak);
        w.parts.push(q);
    }
    w.shake(18.0);
    w.fx.punch = w.fx.punch.max(0.07);
    w.fx.flash = w.fx.flash.max(0.2);
    // hits if they're near and on the ground (or you landed right on them)
    let landed_on_foe = (foe.y - base).abs() < 2.0 && (fx - mx).abs() < SIZE;
    if (fx - mx).abs() < 250.0 && (landed_on_foe || foe.y + SIZE >= base - 40.0) {
        let dir = if (fx - mx).abs() < 1.0 { me.facing } else { (fx - mx).signum() };
        foe.hurt(20.0, dir * 500.0, -350.0);
    }
}

pub(crate) fn explode(players: &mut [Player; 2], owner: usize, cx: f32, cy: f32, w: &mut World) {
    w.ring(cx, cy, BLAST_R, Color::new(255, 150, 40, 255));
    w.ring(cx, cy, BLAST_R * 0.6, Color::new(255, 255, 220, 255));
    w.burst(cx, cy, 30, Color::new(255, 170, 40, 255), 520.0, 0.6, 9.0, Shape::Circle, 0.0);
    w.burst(cx, cy, 16, Color::new(255, 240, 160, 255), 420.0, 0.4, 6.0, Shape::Star, 0.0);
    w.burst(cx, cy, 12, Color::new(70, 60, 60, 200), 160.0, 1.0, 14.0, Shape::Circle, -60.0);
    w.burst(cx, cy, 14, Color::new(90, 70, 60, 255), 450.0, 0.8, 9.0, Shape::Square, 700.0);
    w.shake(12.0);
    w.fx.punch = w.fx.punch.max(0.04);
    w.fx.flash = w.fx.flash.max(0.15);
    let mult = if players[owner].kind == Kind::Bomber { 1.2 } else { 1.0 };
    let reach = BLAST_R + SIZE / 2.0;
    for j in 0..2 {
        let p = &mut players[j];
        let (px, py) = p.center();
        let (dx, dy) = (px - cx, py - cy);
        let d = (dx * dx + dy * dy).sqrt();
        if d > reach {
            continue;
        }
        let (nx, ny) = if d < 1.0 { (0.0, -1.0) } else { (dx / d, dy / d) };
        let f = 1.0 - (d / reach) * 0.5;
        if j == owner {
            // blast jump: knockback only, no self damage
            p.vx += nx * 500.0 * f;
            p.vy = ny * 600.0 * f - 100.0;
            p.on_ground = false;
        } else {
            p.hurt(25.0 * f * mult, nx * 600.0 * f, ny * 600.0 * f - 200.0);
        }
    }
}

// what happens when a projectile ends (hit something or ran out)
pub(crate) fn proj_impact(b: &Proj, hit_foe: bool, players: &mut [Player; 2], w: &mut World) {
    let foe = 1 - b.owner;
    match b.kind {
        ProjKind::Bomb => explode(players, b.owner, b.x, b.y, w),
        ProjKind::Frost => {
            let ice = Color::new(170, 235, 255, 255);
            if hit_foe {
                let p = &mut players[foe];
                p.hp = (p.hp - 5.0).max(0.0);
                p.frozen_t = FROZEN_TIME;
                p.vx = 0.0;
                p.hurt_t = 0.15;
            }
            w.ring(b.x, b.y, 80.0, ice);
            w.burst(b.x, b.y, 22, ice, 360.0, 0.6, 8.0, Shape::Star, 200.0);
            w.burst(b.x, b.y, 10, Color::WHITE, 260.0, 0.5, 8.0, Shape::Square, 400.0);
            w.shake(6.0);
        }
        ProjKind::Fire => {
            if hit_foe {
                let dir = if b.vx >= 0.0 { 1.0 } else { -1.0 };
                let p = &mut players[foe];
                if p.shield_t <= 0.0 {
                    p.hurt(14.0, dir * 300.0, -150.0);
                    p.burn_t = BURN_TIME;
                }
            }
            w.ring(b.x, b.y, 70.0, Color::new(255, 140, 40, 255));
            w.burst(b.x, b.y, 22, Color::new(255, 150, 40, 255), 340.0, 0.5, 9.0, Shape::Circle, -80.0);
            w.burst(b.x, b.y, 10, Color::new(255, 230, 120, 255), 300.0, 0.4, 5.0, Shape::Star, 0.0);
            w.shake(7.0);
        }
    }
}

pub(crate) fn update_projs(players: &mut [Player; 2], solids: &[Solid], w: &mut World, dt: f32) {
    let projs = std::mem::take(&mut w.projs);
    let mut keep = Vec::new();
    for mut b in projs {
        if let ProjKind::Bomb = b.kind {
            b.vy += BOMB_GRAVITY * dt;
        }
        b.x += b.vx * dt;
        b.y += b.vy * dt;
        b.life -= dt;

        // projectile trails
        match b.kind {
            ProjKind::Bomb => {
                if w.rng.next() < 0.5 {
                    let q = Particle::new(b.x, b.y, w.rng.range(-30.0, 30.0), w.rng.range(-40.0, -10.0), 0.4, 4.0, Color::new(255, 170, 60, 255), Shape::Circle);
                    w.parts.push(q);
                }
            }
            ProjKind::Fire => {
                let mut q = Particle::new(b.x, b.y + w.rng.range(-6.0, 6.0), -b.vx * 0.1, w.rng.range(-30.0, 10.0), 0.4, 9.0, Color::new(255, 130 + (w.rng.next() * 100.0) as u8, 30, 230), Shape::Circle);
                q.grav = -60.0;
                w.parts.push(q);
            }
            ProjKind::Frost => {
                if w.rng.next() < 0.7 {
                    let q = Particle::new(b.x, b.y + w.rng.range(-8.0, 8.0), -b.vx * 0.05, w.rng.range(-20.0, 20.0), 0.35, 5.0, Color::new(200, 245, 255, 255), Shape::Star);
                    w.parts.push(q);
                }
            }
        }

        let mut end = b.life <= 0.0 || b.x < 0.0 || b.x > W as f32 || b.y > H as f32 || b.y < -400.0;
        let br = Rectangle::new(b.x - 8.0, b.y - 8.0, 16.0, 16.0);
        if b.y > 0.0 && solids.iter().any(|s| overlaps(&br, &s.r)) {
            end = true;
        }
        let foe = 1 - b.owner;
        let mut hit = false;
        if players[foe].dead_t <= 0.0 && overlaps(&br, &players[foe].rect()) {
            if players[foe].shield_t > 0.0 {
                // reflected back at the thrower
                b.vx = -b.vx;
                b.vy = if let ProjKind::Bomb = b.kind { -b.vy.abs() } else { b.vy };
                b.owner = foe;
                b.life = 3.0;
                end = false;
                w.burst(b.x, b.y, 10, Color::new(150, 230, 255, 255), 300.0, 0.3, 5.0, Shape::Star, 0.0);
                w.ring(b.x, b.y, 40.0, Color::new(150, 230, 255, 255));
                w.shake(5.0);
            } else {
                hit = true;
                end = true;
            }
        }
        if end {
            proj_impact(&b, hit, players, w);
        } else {
            keep.push(b);
        }
    }
    w.projs = keep;
}

pub(crate) fn update_strikes(players: &mut [Player; 2], w: &mut World, dt: f32) {
    let ground = H as f32 - FLOOR_H;
    let mut strikes = std::mem::take(&mut w.strikes);
    for s in strikes.iter_mut() {
        if !s.struck {
            s.t -= dt;
            if s.t <= 0.0 {
                s.struck = true;
                s.vis = 0.35;
                let mut y = 0.0;
                let mut x = s.x;
                while y < ground {
                    s.pts.push(Vector2::new(x, y));
                    y += w.rng.range(30.0, 60.0);
                    x = s.x + w.rng.range(-28.0, 28.0);
                }
                s.pts.push(Vector2::new(s.x, ground));
                let column = Rectangle::new(s.x - 35.0, 0.0, 70.0, ground);
                for j in 0..2 {
                    if j != s.owner && overlaps(&column, &players[j].rect()) {
                        players[j].hurt(22.0, 0.0, -320.0);
                    }
                }
                w.fx.flash = w.fx.flash.max(0.55);
                w.shake(20.0);
                w.fx.punch = w.fx.punch.max(0.06);
                w.ring(s.x, ground, 140.0, Color::new(255, 245, 150, 255));
                w.burst(s.x, ground, 24, Color::new(255, 245, 150, 255), 460.0, 0.6, 8.0, Shape::Star, 400.0);
                w.burst(s.x, ground, 12, Color::new(120, 110, 100, 255), 300.0, 0.7, 8.0, Shape::Square, 800.0);
            }
        } else {
            s.vis -= dt;
        }
    }
    strikes.retain(|s| !(s.struck && s.vis <= 0.0));
    w.strikes = strikes;
}

// =====================================================================
// map events: wind, meteors, pickups
// =====================================================================

pub(crate) fn update_meteors(players: &mut [Player; 2], solids: &[Solid], w: &mut World, dt: f32) {
    let ground = H as f32 - FLOOR_H;
    let meteors = std::mem::take(&mut w.meteors);
    let mut keep = Vec::new();
    for mut m in meteors {
        if !m.falling {
            m.warn -= dt;
            if m.warn <= 0.0 {
                m.falling = true;
                m.x = m.tx - 300.0;
                m.y = -60.0;
                m.vx = 272.0;
                m.vy = 672.0;
            }
            keep.push(m);
            continue;
        }
        m.x += m.vx * dt;
        m.y += m.vy * dt;
        let mut q = Particle::new(m.x, m.y, w.rng.range(-40.0, 40.0), w.rng.range(-60.0, 0.0), 0.5, 10.0, Color::new(255, 140 + (w.rng.next() * 90.0) as u8, 30, 230), Shape::Circle);
        q.grav = -50.0;
        w.parts.push(q);
        let r = Rectangle::new(m.x - 14.0, m.y - 14.0, 28.0, 28.0);
        let hit = m.y >= ground || (m.y > 0.0 && solids.iter().any(|s| overlaps(&r, &s.r)));
        if hit {
            let (ix, iy) = (m.x, m.y.min(ground));
            w.ring(ix, iy, 120.0, Color::new(255, 150, 50, 255));
            w.ring(ix, iy, 70.0, Color::WHITE);
            w.burst(ix, iy, 26, Color::new(255, 160, 40, 255), 480.0, 0.6, 9.0, Shape::Circle, 100.0);
            w.burst(ix, iy, 12, Color::new(90, 70, 60, 255), 420.0, 0.8, 9.0, Shape::Square, 800.0);
            w.shake(14.0);
            w.fx.flash = w.fx.flash.max(0.15);
            for p in players.iter_mut() {
                let (px, py) = p.center();
                let d = ((px - ix).powi(2) + (py - iy).powi(2)).sqrt();
                if d < 130.0 {
                    let dir = if px >= ix { 1.0 } else { -1.0 };
                    p.hurt(15.0, dir * 450.0, -300.0);
                }
            }
        } else {
            keep.push(m);
        }
    }
    w.meteors = keep;
}

pub(crate) fn update_lasers(players: &mut [Player; 2], w: &mut World, dt: f32) {
    let lasers = std::mem::take(&mut w.lasers);
    let mut keep = Vec::new();
    for mut l in lasers {
        if !l.fired {
            l.t -= dt;
            if l.t <= 0.0 {
                l.fired = true;
                l.vis = 0.4;
                let beam = Rectangle::new(0.0, l.y - 22.0, W as f32, 44.0);
                for p in players.iter_mut() {
                    if overlaps(&beam, &p.rect()) {
                        p.hurt(18.0, 0.0, -300.0);
                    }
                }
                w.fx.flash = w.fx.flash.max(0.3);
                w.shake(14.0);
                for k in 0..22 {
                    let x = k as f32 * 60.0;
                    let q = Particle::new(x, l.y, w.rng.range(-80.0, 80.0), w.rng.range(-200.0, 200.0), 0.5, 4.0, Color::new(255, 80, 220, 255), Shape::Star);
                    w.parts.push(q);
                }
            }
        } else {
            l.vis -= dt;
        }
        if !(l.fired && l.vis <= 0.0) {
            keep.push(l);
        }
    }
    w.lasers = keep;
}

// gusting wind / blizzard / currents: (cycle length, warning starts, gust starts, drift speed)
pub(crate) fn wind_params(theme: Theme) -> Option<(f32, f32, f32, f32)> {
    match theme {
        Theme::Meadow => Some((14.0, 8.0, 10.0, 140.0)),
        Theme::Frozen => Some((11.0, 6.0, 8.0, 210.0)),
        Theme::Sea => Some((10.0, 5.0, 7.0, 170.0)),
        _ => None,
    }
}

pub(crate) fn update_events(theme: Theme, rd: &mut Round, players: &mut [Player; 2], solids: &[Solid], w: &mut World, dt: f32) {
    // default: normal gravity; events below may change it
    w.grav_scale = if theme == Theme::Sea { 0.65 } else { 1.0 };

    if let Some((cycle_len, warn_at, gust_at, speed)) = wind_params(theme) {
        let cyc = rd.time % cycle_len;
        let n = (rd.time / cycle_len) as i32;
        let dir: f32 = if n % 2 == 0 { 1.0 } else { -1.0 };
        rd.wind = if cyc >= gust_at { dir } else { 0.0 };
        rd.wind_warn = cyc >= warn_at && cyc < gust_at;
        if rd.wind != 0.0 {
            for p in players.iter_mut() {
                if p.dead_t <= 0.0 {
                    p.x += rd.wind * speed * dt;
                    push_out_x(p, solids);
                }
            }
            let streak = match theme {
                Theme::Sea => Color::new(200, 240, 255, 130),
                Theme::Frozen => Color::new(255, 255, 255, 190),
                _ => Color::new(255, 255, 255, 150),
            };
            for _ in 0..2 {
                let sx = if rd.wind > 0.0 { -10.0 } else { W as f32 + 10.0 };
                let y = w.rng.range(0.0, H as f32 - FLOOR_H);
                let vx = rd.wind * w.rng.range(700.0, 1000.0);
                let q = Particle::new(sx, y, vx, w.rng.range(-30.0, 30.0), 2.0, 3.0, streak, Shape::Streak);
                w.parts.push(q);
            }
        }
    }

    match theme {
        Theme::Skyline | Theme::Volcano | Theme::Sand | Theme::Space => {
            rd.meteor_t -= dt;
            if rd.meteor_t <= 0.0 {
                rd.meteor_t = match theme {
                    Theme::Skyline => w.rng.range(4.0, 6.5),
                    Theme::Volcano => w.rng.range(3.5, 5.5),
                    Theme::Sand => w.rng.range(3.0, 4.5),
                    _ => w.rng.range(5.5, 8.0),
                };
                let tx = w.rng.range(100.0, W as f32 - 100.0);
                w.meteors.push(Meteor { tx, warn: 1.0, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, falling: false });
            }
        }
        Theme::Neon => {
            rd.meteor_t -= dt;
            if rd.meteor_t <= 0.0 {
                rd.meteor_t = w.rng.range(3.5, 5.5);
                let y = w.rng.range(140.0, H as f32 - FLOOR_H - 50.0);
                w.lasers.push(Laser { y, t: 1.1, fired: false, vis: 0.0 });
            }
        }
        _ => {}
    }

    if theme == Theme::Space {
        // periodic zero-g: everything floats
        let cyc = rd.time % 16.0;
        rd.zerog = cyc >= 10.0;
        rd.zerog_warn = (8.0..10.0).contains(&cyc);
        if rd.zerog {
            w.grav_scale = 0.3;
        }
    }

    update_meteors(players, solids, w, dt);
    update_lasers(players, w, dt);
}

pub(crate) fn update_pickups(players: &mut [Player; 2], solids: &[Solid], rd: &mut Round, w: &mut World, dt: f32) {
    rd.pickup_t -= dt;
    if rd.pickup_t <= 0.0 && w.pickups.len() < 2 {
        // try a few random spots on top of blocks; only accept one that is
        // completely out in the open (not inside or touching any wall/block)
        for _ in 0..30 {
            let si = (w.rng.next() * solids.len() as f32) as usize % solids.len();
            let r = solids[si].r;
            if r.width < 70.0 {
                continue;
            }
            let x = r.x + w.rng.range(30.0, r.width - 30.0);
            let y = r.y - 32.0;
            let area = Rectangle::new(x - 28.0, y - 28.0, 56.0, 56.0 - 2.0); // sits just above the surface
            let blocked = solids.iter().any(|s| overlaps(&area, &s.r));
            let too_close = w.pickups.iter().any(|p| (p.x - x).abs() < 160.0);
            if blocked || too_close || x < 40.0 || x > W as f32 - 40.0 {
                continue;
            }
            rd.pickup_t = w.rng.range(8.0, 11.0);
            let kind = match (w.rng.next() * 4.0) as i32 {
                0 => PickKind::Health,
                1 => PickKind::Power,
                2 => PickKind::Haste,
                _ => PickKind::Reset,
            };
            w.ring(x, y, 40.0, kind.color());
            w.pickups.push(Pickup { x, y, kind, age: 0.0 });
            break;
        }
    }
    let pickups = std::mem::take(&mut w.pickups);
    let mut keep = Vec::new();
    for mut pk in pickups {
        pk.age += dt;
        let area = Rectangle::new(pk.x - 24.0, pk.y - 24.0, 48.0, 48.0);
        let mut taken = false;
        for p in players.iter_mut() {
            if p.dead_t <= 0.0 && !taken && overlaps(&area, &p.rect()) {
                taken = true;
                match pk.kind {
                    PickKind::Health => p.hp = (p.hp + 25.0).min(p.max_hp),
                    PickKind::Power => p.power_t = 8.0,
                    PickKind::Haste => p.haste_t = 8.0,
                    PickKind::Reset => {
                        p.cd1 = 0.0;
                        p.cd2 = 0.0;
                        p.ult = (p.ult + 25.0).min(ULT_MAX);
                    }
                }
            }
        }
        if taken {
            w.ring(pk.x, pk.y, 90.0, pk.kind.color());
            w.burst(pk.x, pk.y, 18, pk.kind.color(), 320.0, 0.6, 6.0, Shape::Star, 0.0);
        } else {
            keep.push(pk);
        }
    }
    w.pickups = keep;
}

// =====================================================================
// player update
// =====================================================================

pub(crate) fn update_player(inp: &Input, me: &mut Player, foe: &mut Player, my_idx: usize, solids: &[Solid], w: &mut World, dt: f32) {
    let cd_tick = if me.over_t > 0.0 { dt * 3.0 } else { dt };
    me.cd1 = (me.cd1 - cd_tick).max(0.0);
    me.cd2 = (me.cd2 - cd_tick).max(0.0);
    me.shield_t = (me.shield_t - dt).max(0.0);
    me.hurt_t = (me.hurt_t - dt).max(0.0);
    me.frozen_t = (me.frozen_t - dt).max(0.0);
    me.stun_t = (me.stun_t - dt).max(0.0);
    me.power_t = (me.power_t - dt).max(0.0);
    me.haste_t = (me.haste_t - dt).max(0.0);
    me.over_t = (me.over_t - dt).max(0.0);
    me.parry_t = (me.parry_t - dt).max(0.0);
    me.block_cd = (me.block_cd - dt).max(0.0);
    me.ult = (me.ult + 1.5 * dt).min(ULT_MAX); // slow passive charge
    let can_act = me.frozen_t <= 0.0 && me.stun_t <= 0.0;
    let (cx, cy) = me.center();

    if me.on_ground {
        me.air_jumps = me.kind.air_jumps();
    }

    // ---- guard: hold to block, tap right as a hit lands to parry ----
    let press = inp.block_down && !me.block_prev;
    me.block_prev = inp.block_down;
    if can_act && me.block_cd <= 0.0 && inp.block_down {
        if press {
            me.parry_t = PARRY_WINDOW;
        }
        me.block_held = true;
        me.block_t += dt;
        if me.block_t > BLOCK_MAX {
            me.block_held = false;
            me.block_cd = 1.2; // guard break: can't block for a moment
            me.block_t = 0.0;
        }
    } else {
        me.block_held = false;
        me.block_t = (me.block_t - dt * 2.0).max(0.0);
    }
    let acting = can_act && !me.block_held;

    // ---- burning, healing, buff and status particles ----
    if me.burn_t > 0.0 {
        me.burn_t -= dt;
        me.hp = (me.hp - 4.0 * dt).max(0.0);
        if w.rng.next() < 0.7 {
            let mut q = Particle::new(cx + w.rng.range(-22.0, 22.0), cy + w.rng.range(-10.0, 25.0), w.rng.range(-20.0, 20.0), -50.0, 0.5, 8.0, Color::new(255, 140 + (w.rng.next() * 100.0) as u8, 30, 230), Shape::Circle);
            q.grav = -80.0;
            w.parts.push(q);
        }
    }
    if me.regen_t > 0.0 {
        me.regen_t -= dt;
        me.hp = (me.hp + 9.0 * dt).min(me.max_hp);
        if w.rng.next() < 0.5 {
            let q = Particle::new(cx + w.rng.range(-25.0, 25.0), cy + 10.0, 0.0, -70.0, 0.7, 6.0, Color::new(110, 255, 140, 255), Shape::Star);
            w.parts.push(q);
        }
    }
    if me.frozen_t > 0.0 && w.rng.next() < 0.3 {
        let q = Particle::new(cx + w.rng.range(-25.0, 25.0), cy + w.rng.range(-25.0, 25.0), 0.0, 30.0, 0.5, 4.0, Color::new(200, 240, 255, 255), Shape::Star);
        w.parts.push(q);
    }
    if me.over_t > 0.0 && w.rng.next() < 0.6 {
        let mut q = Particle::new(cx, cy, 0.0, 0.0, 0.25, SIZE, Color::new(255, 150, 40, 90), Shape::Ghost);
        q.rot = me.rot;
        w.parts.push(q);
    }
    if me.power_t > 0.0 && w.rng.next() < 0.3 {
        let q = Particle::new(cx + w.rng.range(-25.0, 25.0), cy + 20.0, 0.0, -60.0, 0.5, 5.0, Color::new(255, 80, 60, 255), Shape::Star);
        w.parts.push(q);
    }

    // ---- wall slide ----
    let mut push = 0.0;
    if can_act {
        if inp.right {
            push += 1.0;
        }
        if inp.left {
            push -= 1.0;
        }
    }
    if push != 0.0 {
        me.facing = push;
    }
    if !me.on_ground && me.wall_dir != 0.0 && push == me.wall_dir && me.vy > 0.0 {
        me.vy = me.vy.min(WALL_SLIDE);
        if w.rng.next() < 0.4 {
            let q = Particle::new(cx + me.wall_dir * 28.0, cy + 20.0, 0.0, -30.0, 0.3, 4.0, Color::new(220, 220, 220, 200), Shape::Circle);
            w.parts.push(q);
        }
    }

    // ---- momentum: roll and slide ----
    let top = TOP_SPEED * me.speed_mult();
    let accel = top / 0.25;
    if me.dash_t > 0.0 {
        me.dash_t -= dt;
        me.vx = me.facing * DASH_SPEED;
        me.vy = 0.0;
    } else if push != 0.0 && me.vx * push < top {
        me.vx += push * accel * dt;
        if me.vx * push > top {
            me.vx = push * top;
        }
    } else {
        // no input, or knocked faster than top speed: slide down
        let slow = (ICE * dt).min(me.vx.abs());
        me.vx -= me.vx.signum() * slow;
    }
    me.rot += me.vx * dt / (SIZE / 2.0) * 57.3;
    if me.vx == 0.0 {
        let flat = (me.rot / 90.0).round() * 90.0;
        me.rot += (flat - me.rot) * (10.0 * dt).min(1.0);
    }

    // ---- crouch, charge, jump ----
    if acting && inp.down && me.on_ground {
        me.charge = (me.charge + dt).min(1.0);
        me.crouch = CROUCH;
    } else if me.tap_timer <= 0.0 {
        me.charge = 0.0;
        me.crouch = 0.0;
    }
    if acting && inp.jump {
        if me.on_ground && me.tap_timer <= 0.0 {
            if me.charge > 0.0 {
                let power = 1.0 + me.charge * (MAX_CHARGE - 1.0);
                launch(me, power);
            } else {
                me.tap_timer = TAP_CROUCH;
                me.crouch = CROUCH;
            }
        } else if !me.on_ground && me.wall_dir != 0.0 {
            // wall jump: kick off the wall, refunds your air jumps
            let kick = -me.wall_dir;
            me.vy = -(2.0 * GRAVITY * JUMP * 0.95).sqrt();
            me.vx = kick * WALL_KICK;
            me.facing = kick;
            me.air_jumps = me.kind.air_jumps();
            w.burst(cx - kick * 30.0, cy, 8, Color::new(230, 230, 230, 220), 200.0, 0.35, 5.0, Shape::Circle, 100.0);
        } else if !me.on_ground && me.air_jumps > 0 && me.tap_timer <= 0.0 {
            // double jump
            me.air_jumps -= 1;
            me.vy = -(2.0 * GRAVITY * JUMP * 0.85).sqrt();
            w.ring(cx, me.y + SIZE, 55.0, Color::WHITE);
            w.burst(cx, me.y + SIZE, 8, Color::new(230, 230, 230, 200), 180.0, 0.35, 5.0, Shape::Circle, 100.0);
        }
    }
    if me.tap_timer > 0.0 {
        me.tap_timer -= dt;
        if me.tap_timer <= 0.0 {
            launch(me, 1.0);
        }
    }

    // ---- loadout abilities and ultimate ----
    let loadout = me.abilities;
    if acting && inp.a1 && me.cd1 <= 0.0 {
        use_ability(loadout[0], me, foe, my_idx, solids, w);
        me.cd1 = loadout[0].cooldown();
    }
    if acting && inp.a2 && me.cd2 <= 0.0 {
        use_ability(loadout[1], me, foe, my_idx, solids, w);
        me.cd2 = loadout[1].cooldown();
    }
    if acting && inp.ult && me.ult >= ULT_MAX {
        me.ult = 0.0;
        use_ult(me, foe, my_idx, w);
    }
    if me.slam_arm && me.vy >= 0.0 {
        me.slam_arm = false;
        me.slam = true;
    }
    if me.slam {
        me.vy = me.vy.max(SLAM_SPEED);
        if w.rng.next() < 0.8 {
            let q = Particle::new(cx + w.rng.range(-20.0, 20.0), cy - 30.0, 0.0, -200.0, 0.25, 3.0, Color::WHITE, Shape::Streak);
            w.parts.push(q);
        }
    }

    // ---- melee: knife swing ----
    me.melee_cd = (me.melee_cd - dt).max(0.0);
    me.melee_t = (me.melee_t - dt).max(0.0);
    if acting && inp.melee && me.melee_cd <= 0.0 && me.melee_t <= 0.0 {
        me.melee_t = MELEE_TIME;
        me.melee_cd = MELEE_CD;
        me.melee_hit = false;
    }

    step(me, solids, dt, w.grav_scale);

    if me.melee_t > 0.0 && !me.melee_hit {
        let progress = 1.0 - me.melee_t / MELEE_TIME;
        if (0.2..0.85).contains(&progress) {
            let rx = if me.facing > 0.0 { me.x + SIZE } else { me.x - MELEE_REACH };
            let zone = Rectangle::new(rx, me.y - 10.0, MELEE_REACH, SIZE + 20.0);
            if overlaps(&zone, &foe.rect()) {
                let dmg = me.kind.melee_dmg() * if me.over_t > 0.0 { 1.8 } else { 1.0 };
                foe.hurt(dmg, me.facing * 350.0, -180.0);
                me.melee_hit = true;
                let (fx, fy) = foe.center();
                w.burst(fx, fy, 8, Color::new(220, 240, 255, 255), 320.0, 0.25, 3.0, Shape::Streak, 0.0);
            }
        }
    }

    // ---- dash results ----
    if me.dash_t > 0.0 && !me.dash_hit && overlaps(&me.rect(), &foe.rect()) {
        foe.hurt(15.0, me.facing * 600.0, -250.0);
        me.dash_hit = true;
        me.dash_t = 0.0; // dash ends on impact so you don't keep shoving them
        me.vx *= 0.3;
        let (fx, fy) = foe.center();
        w.ring(fx, fy, 90.0, Color::new(255, 200, 100, 255));
        w.shake(8.0);
    }

    // ---- eyes ----
    me.eyes = if me.crouch > 0.0 || me.hurt_t > 0.0 || me.frozen_t > 0.0 || me.stun_t > 0.0 {
        Eyes::Squint
    } else if !me.on_ground && me.vy < 0.0 {
        Eyes::UpWide
    } else if push > 0.0 {
        Eyes::Right
    } else if push < 0.0 {
        Eyes::Left
    } else {
        Eyes::Forward
    };
}

// =====================================================================
// trails
// =====================================================================

pub(crate) fn emit_trail(p: &mut Player, w: &mut World, t: f32, dt: f32) {
    let style = TRAIL_STYLES[p.trail_style];
    if style == Trail::Off || p.dead_t > 0.0 {
        return;
    }
    let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
    if speed < 40.0 {
        p.trail_acc = 0.0;
        return;
    }
    p.trail_acc += dt;
    let every = if style == Trail::Ghost { 0.04 } else { 0.02 };
    while p.trail_acc >= every {
        p.trail_acc -= every;
        if w.parts.len() > 1600 {
            return;
        }
        let (cx, cy) = p.center();
        let col = trail_color(p.trail_color, t, p.kind.color(), w.rng.range(0.0, 40.0));
        let r = &mut w.rng;
        let q = match style {
            Trail::Dots => {
                let size = r.range(5.0, 10.0);
                Particle::new(cx + r.range(-12.0, 12.0), cy + r.range(-12.0, 12.0), r.range(-25.0, 25.0), r.range(-25.0, 25.0), 0.5, size, col, Shape::Circle)
            }
            Trail::Ghost => {
                let mut q = Particle::new(cx, cy, 0.0, 0.0, 0.3, SIZE, with_alpha(col, 120), Shape::Ghost);
                q.rot = p.rot;
                q
            }
            Trail::Sparkle => {
                let size = r.range(4.0, 9.0);
                Particle::new(cx + r.range(-20.0, 20.0), cy + r.range(-20.0, 20.0), r.range(-30.0, 30.0), r.range(-60.0, -10.0), 0.7, size, col, Shape::Star)
            }
            _ => {
                let size = r.range(8.0, 14.0);
                let mut q = Particle::new(cx + r.range(-10.0, 10.0), cy + r.range(-5.0, 15.0), r.range(-15.0, 15.0), r.range(-40.0, -15.0), 0.9, size, with_alpha(col, 140), Shape::Circle);
                q.grow = true;
                q
            }
        };
        w.parts.push(q);
    }
}

// =====================================================================
// drawing: players
// =====================================================================

// pixel-style knife, drawn from rectangles. (px, py) is the handle end,
// theta is degrees clockwise from straight up, flip = 1 or -1 (facing).



// =====================================================================
// drawing: text and HUD
// =====================================================================

// readable on any background: colored text with a thick black outline




// =====================================================================
// drawing: scenery and themes
// =====================================================================


#[allow(clippy::too_many_arguments)]

// =====================================================================
// death animations: 10 + random (the loser picks how they go out)
// =====================================================================

pub(crate) const DEATH_NAMES: [&str; 11] = [
    "SHATTER",
    "EXPLODE",
    "DISINTEGRATE",
    "ASCEND",
    "LIGHTNING",
    "SPIN OFF",
    "CONFETTI POP",
    "FREEZE & CRACK",
    "BLACK HOLE",
    "GLITCH",
    "RANDOM",
];

pub(crate) const DEATH_BLURBS: [&str; 11] = [
    "bursts into spinning shards",
    "blows up in a fireball",
    "turns to dust and blows away",
    "a ghost floats up to the sky",
    "vaporized by a lightning bolt",
    "spins off into the distance",
    "pops like a balloon",
    "freezes solid, then cracks",
    "sucked into a black hole",
    "RGB glitch and static",
    "a surprise every time",
];

#[allow(clippy::too_many_arguments)]
pub(crate) fn put(w: &mut World, x: f32, y: f32, vx: f32, vy: f32, life: f32, size: f32, col: Color, shape: Shape, grav: f32, spin: f32) {
    if w.parts.len() > 2000 {
        return;
    }
    let mut q = Particle::new(x, y, vx, vy, life, size, col, shape);
    q.grav = grav;
    q.spin = spin;
    w.parts.push(q);
}

pub(crate) fn play_death(choice: usize, x: f32, y: f32, col: Color, w: &mut World) {
    let idx = if choice >= 10 { (w.rng.next() * 10.0) as usize % 10 } else { choice };
    w.ring(x, y, 220.0, Color::WHITE);
    w.fx.flash = w.fx.flash.max(0.3);
    w.shake(16.0);
    match idx {
        0 => {
            // SHATTER
            w.burst(x, y, 36, col, 600.0, 1.1, 14.0, Shape::Square, 500.0);
            w.burst(x, y, 20, Color::WHITE, 700.0, 0.6, 7.0, Shape::Star, 0.0);
            w.burst(x, y, 20, Color::WHITE, 800.0, 0.5, 3.0, Shape::Streak, 0.0);
            w.ring(x, y, 170.0, col);
        }
        1 => {
            // EXPLODE
            w.ring(x, y, 290.0, Color::new(255, 150, 40, 255));
            w.ring(x, y, 170.0, Color::new(255, 255, 200, 255));
            w.burst(x, y, 44, Color::new(255, 170, 40, 255), 650.0, 0.7, 10.0, Shape::Circle, 0.0);
            w.burst(x, y, 16, Color::new(255, 240, 160, 255), 500.0, 0.5, 6.0, Shape::Star, 0.0);
            w.burst(x, y, 18, col, 500.0, 0.9, 10.0, Shape::Square, 800.0);
            for _ in 0..14 {
                let (px, py) = (x + w.rng.range(-30.0, 30.0), y + w.rng.range(-30.0, 30.0));
                let (vx, vy) = (w.rng.range(-60.0, 60.0), w.rng.range(-120.0, -30.0));
                let size = w.rng.range(14.0, 24.0);
                let mut q = Particle::new(px, py, vx, vy, 1.2, size, Color::new(60, 55, 55, 170), Shape::Circle);
                q.grow = true;
                w.parts.push(q);
            }
            w.shake(28.0);
            w.fx.flash = w.fx.flash.max(0.6);
        }
        2 => {
            // DISINTEGRATE: dust blown off to the right
            for _ in 0..80 {
                let (px, py) = (x + w.rng.range(-30.0, 30.0), y + w.rng.range(-30.0, 30.0));
                let (vx, vy) = (w.rng.range(40.0, 170.0), w.rng.range(-90.0, -10.0));
                let (life, size, spin) = (w.rng.range(1.4, 2.4), w.rng.range(4.0, 9.0), w.rng.range(-200.0, 200.0));
                put(w, px, py, vx, vy, life, size, with_alpha(col, 230), Shape::Square, -15.0, spin);
            }
        }
        3 => {
            // ASCEND: a pale ghost with a halo drifts upward
            put(w, x, y, 0.0, -150.0, 1.6, SIZE, Color::new(235, 235, 255, 170), Shape::Ghost, 0.0, 0.0);
            for k in 0..12 {
                let a = k as f32 * 0.5236;
                put(w, x + a.cos() * 34.0, y - 48.0 + a.sin() * 10.0, 0.0, -150.0, 1.6, 6.0, Color::new(255, 240, 160, 255), Shape::Star, 0.0, 0.0);
            }
            w.burst(x, y, 20, Color::new(255, 240, 160, 255), 160.0, 1.4, 6.0, Shape::Star, -120.0);
        }
        4 => {
            // LIGHTNING: a bolt from the sky, ash and sparks
            let mut pts = Vec::new();
            let mut yy = 0.0;
            let mut xx = x;
            while yy < y {
                pts.push(Vector2::new(xx, yy));
                yy += w.rng.range(30.0, 60.0);
                xx = x + w.rng.range(-28.0, 28.0);
            }
            pts.push(Vector2::new(x, y));
            w.strikes.push(Strike { x, t: 0.0, owner: 0, struck: true, vis: 0.4, pts });
            put(w, x, y, 0.0, 0.0, 0.25, SIZE, Color::WHITE, Shape::Ghost, 0.0, 0.0);
            w.burst(x, y, 40, Color::new(50, 50, 50, 255), 400.0, 1.2, 7.0, Shape::Square, 600.0);
            w.burst(x, y, 24, Color::new(255, 245, 150, 255), 600.0, 0.5, 5.0, Shape::Star, 300.0);
            w.fx.flash = 1.0;
            w.shake(30.0);
        }
        5 => {
            // SPIN OFF: spins away diagonally into the distance
            put(w, x, y, 750.0, -650.0, 1.3, SIZE, col, Shape::Ghost, 200.0, 1100.0);
            for _ in 0..14 {
                let f = w.rng.range(0.3, 1.0);
                put(w, x, y, 750.0 * f, -650.0 * f, 0.9, 6.0, Color::new(255, 240, 160, 255), Shape::Star, 200.0, 0.0);
            }
            w.ring(x, y, 130.0, col);
        }
        6 => {
            // CONFETTI POP
            for _ in 0..70 {
                let a = w.rng.range(0.0, 6.2832);
                let sp = w.rng.range(150.0, 600.0);
                let (life, size, spin) = (w.rng.range(1.0, 1.8), w.rng.range(7.0, 13.0), w.rng.range(-400.0, 400.0));
                let c = hsv(w.rng.next() * 360.0);
                put(w, x, y, a.cos() * sp, a.sin() * sp - 200.0, life, size, c, Shape::Square, 500.0, spin);
            }
            w.burst(x, y, 14, Color::WHITE, 350.0, 0.4, 6.0, Shape::Star, 0.0);
            w.ring(x, y, 150.0, Color::new(255, 150, 220, 255));
        }
        7 => {
            // FREEZE & CRACK
            let ice = Color::new(170, 230, 255, 210);
            put(w, x, y, 0.0, 0.0, 0.7, SIZE, ice, Shape::Ghost, 0.0, 0.0);
            w.burst(x, y, 10, Color::WHITE, 30.0, 0.6, 10.0, Shape::Star, 0.0);
            w.burst(x, y, 40, Color::new(200, 240, 255, 255), 500.0, 1.0, 10.0, Shape::Square, 900.0);
            w.ring(x, y, 190.0, ice);
        }
        8 => {
            // BLACK HOLE: everything is pulled into the middle
            for k in 0..48 {
                let a = k as f32 * 0.1309;
                let r = w.rng.range(150.0, 260.0);
                let sp = r / 0.5;
                let c = if k % 2 == 0 { Color::new(150, 60, 255, 255) } else { Color::new(30, 0, 50, 255) };
                let size = w.rng.range(5.0, 10.0);
                put(w, x + a.cos() * r, y + a.sin() * r, -a.cos() * sp, -a.sin() * sp, 0.5, size, c, Shape::Circle, 0.0, 0.0);
            }
            put(w, x, y, 0.0, 0.0, 0.7, 60.0, Color::new(10, 0, 20, 255), Shape::Circle, 0.0, 0.0);
            w.ring(x, y, 280.0, Color::new(150, 60, 255, 255));
        }
        _ => {
            // GLITCH: RGB split, static and scan streaks
            for (dx, c, vx) in [
                (-14.0, Color::new(255, 40, 40, 170), -260.0),
                (0.0, Color::new(40, 255, 60, 150), 0.0),
                (14.0, Color::new(60, 80, 255, 170), 260.0),
            ] {
                put(w, x + dx, y, vx, 0.0, 0.55, SIZE, c, Shape::Ghost, 0.0, 0.0);
            }
            for k in 0..16 {
                let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                let (px, py, sp) = (x + w.rng.range(-80.0, 80.0), y + w.rng.range(-50.0, 50.0), w.rng.range(500.0, 1100.0));
                let c = if k % 3 == 0 { Color::new(0, 255, 255, 255) } else { Color::new(255, 0, 200, 255) };
                put(w, px, py, dir * sp, 0.0, 0.3, 3.0, c, Shape::Streak, 0.0, 0.0);
            }
            for _ in 0..30 {
                let (vx, vy, life) = (w.rng.range(-300.0, 300.0), w.rng.range(-300.0, 300.0), w.rng.range(0.3, 0.5));
                let c = hsv((w.rng.next() * 3.0).floor() * 120.0);
                put(w, x, y, vx, vy, life, 8.0, c, Shape::Square, 0.0, 0.0);
            }
        }
    }
}

// =====================================================================
// victory cutscenes: 10 + random (the winner picks how they celebrate)
// =====================================================================

pub(crate) const VICTORY_NAMES: [&str; 11] = [
    "KNIFE STAB FINALE",
    "VICTORY DANCE",
    "FIREWORKS",
    "LIGHTNING ASCENSION",
    "PHOENIX RISE",
    "DOMINATION STOMP",
    "SPOTLIGHT BOW",
    "BLINK STORM",
    "GIANT SLASH",
    "METEOR SHOWER",
    "RANDOM",
];

pub(crate) const VICTORY_BLURBS: [&str; 11] = [
    "knife into the ground, then ALL abilities fire",
    "spins, hops and showers confetti",
    "fireworks burst across the sky",
    "rises into the storm as lightning crashes",
    "flames and fiery wings",
    "three earth-shaking stomps",
    "spotlight, golden sparkles and a bow",
    "teleports all over in a storm of slashes",
    "a screen-wide knife slash cuts the world",
    "meteors rain down around the champion",
    "a surprise every time",
];

// how far the camera zooms in, and how long each cutscene lasts
pub(crate) const VIC_ZOOM: [f32; 10] = [1.35, 1.4, 1.0, 1.4, 1.5, 1.5, 1.7, 1.0, 1.15, 1.0];
pub(crate) const VIC_DUR: [f32; 10] = [4.8, 3.4, 3.6, 3.6, 3.4, 3.6, 3.4, 3.0, 3.2, 3.6];

pub(crate) struct Vic {
    idx: usize,
    who: usize,
    t: f32,
    dur: f32,
    cycle: i32,
}

pub(crate) fn at(prev: f32, now: f32, mark: f32) -> bool {
    prev < mark && now >= mark
}

pub(crate) fn victory_start(choice: usize, who: usize, players: &mut [Player; 2], map: &Map, w: &mut World) -> Vic {
    let idx = if choice >= 10 { (w.rng.next() * 10.0) as usize % 10 } else { choice };
    let ground = H as f32 - FLOOR_H;
    let p = &mut players[who];
    p.x = map.spawns[who];
    p.y = ground - SIZE;
    p.vx = 0.0;
    p.vy = 0.0;
    p.rot = 0.0;
    p.on_ground = true;
    p.facing = if who == 0 { 1.0 } else { -1.0 };
    p.invuln = true;
    p.shield_t = 0.0;
    p.frozen_t = 0.0;
    p.stun_t = 0.0;
    p.burn_t = 0.0;
    p.hurt_t = 0.0;
    p.dash_t = 0.0;
    p.slam = false;
    p.slam_arm = false;
    p.melee_t = 0.0;
    p.block_held = false;
    p.over_t = 0.0;
    p.crouch = 0.0;
    w.projs.clear();
    w.strikes.clear();
    w.meteors.clear();
    w.lasers.clear();
    w.pickups.clear();
    w.grav_scale = 1.0;
    w.fx.flash = 0.8;
    let (cx, cy) = p.center();
    w.ring(cx, cy, 260.0, Color::WHITE);
    w.burst(cx, cy, 20, p.kind.color(), 400.0, 0.6, 7.0, Shape::Star, 0.0);
    Vic { idx, who, t: 0.0, dur: VIC_DUR[idx], cycle: 0 }
}

// put the champion back on the ground when a cutscene ends
pub(crate) fn victory_finish(who: usize, players: &mut [Player; 2], map: &Map) {
    let p = &mut players[who];
    p.x = map.spawns[who];
    p.y = H as f32 - FLOOR_H - SIZE;
    p.vx = 0.0;
    p.vy = 0.0;
    p.rot = 0.0;
    p.on_ground = true;
}

// advance one frame of a cutscene; returns true when it is finished
pub(crate) fn victory_update(v: &mut Vic, players: &mut [Player; 2], map: &Map, w: &mut World, dt: f32) -> bool {
    let prev = v.t;
    v.t += dt;
    let t = v.t;
    let ground = H as f32 - FLOOR_H;
    let floor_y = ground - SIZE;
    let who = v.who;
    {
        let (a, b) = players.split_at_mut(1);
        let (win, lose) = if who == 0 { (&mut a[0], &mut b[0]) } else { (&mut b[0], &mut a[0]) };
        let (cx, cy) = win.center();
        let gold = Color::new(255, 220, 90, 255);
        let orange = Color::new(255, 150, 40, 255);
        match v.idx {
            0 => {
                // KNIFE STAB FINALE
                step(win, &map.solids, dt, 1.0);
                let kx = cx + 130.0;
                if at(prev, t, 0.9) {
                    w.ring(kx, ground, 320.0, Color::WHITE);
                    w.ring(kx, ground, 210.0, orange);
                    w.burst(kx, ground, 32, Color::new(110, 90, 70, 255), 650.0, 0.9, 12.0, Shape::Square, 900.0);
                    w.burst(kx, ground, 20, Color::WHITE, 600.0, 0.6, 7.0, Shape::Star, 0.0);
                    w.shake(32.0);
                    w.fx.flash = 0.7;
                    w.fx.punch = 0.08;
                }
                if at(prev, t, 1.8) {
                    // every ability at once
                    let (sx, sy) = (win.x, win.y);
                    for (k, ab) in ABILITIES.iter().enumerate() {
                        use_ability(*ab, win, lose, who, &map.solids, w);
                        w.ring(cx, cy, 80.0 + k as f32 * 30.0, hsv(k as f32 * 30.0));
                    }
                    win.x = sx;
                    win.y = sy;
                    win.vx = 0.0;
                    win.vy = 0.0;
                    win.dash_t = 0.0;
                    win.slam = false;
                    win.slam_arm = false;
                    win.shield_t = 0.0;
                    win.regen_t = 0.0;
                    w.banner = Some(("ALL ABILITIES!".to_string(), gold, 1.8));
                    w.shake(36.0);
                    w.fx.flash = 1.0;
                    w.burst(cx, cy, 60, Color::WHITE, 900.0, 1.0, 8.0, Shape::Star, 0.0);
                }
                if t > 1.8 && w.rng.next() < 0.5 {
                    let (px, py) = (cx + w.rng.range(-120.0, 120.0), cy + w.rng.range(-100.0, 60.0));
                    put(w, px, py, 0.0, -80.0, 0.7, 6.0, hsv(t * 200.0), Shape::Star, 0.0, 0.0);
                }
            }
            1 => {
                // VICTORY DANCE
                win.rot += 540.0 * dt;
                if win.on_ground && (t * 2.0) as i32 != (prev * 2.0) as i32 {
                    win.vy = -(2.0 * GRAVITY * JUMP * 0.5).sqrt();
                    win.on_ground = false;
                    w.burst(cx, cy + 30.0, 8, gold, 250.0, 0.5, 6.0, Shape::Star, 0.0);
                }
                for _ in 0..3 {
                    let (px, vx, vy) = (w.rng.range(0.0, W as f32), w.rng.range(-40.0, 40.0), w.rng.range(150.0, 300.0));
                    let (size, spin) = (w.rng.range(8.0, 14.0), w.rng.range(-300.0, 300.0));
                    let c = hsv(w.rng.next() * 360.0);
                    put(w, px, -10.0, vx, vy, 2.4, size, c, Shape::Square, 150.0, spin);
                }
                step(win, &map.solids, dt, 1.0);
            }
            2 => {
                // FIREWORKS
                if at(prev, t, 0.3) {
                    win.vy = -(2.0 * GRAVITY * JUMP * 0.7).sqrt();
                    win.on_ground = false;
                }
                if (t * 4.0) as i32 != (prev * 4.0) as i32 {
                    let (fx, fy) = (w.rng.range(150.0, W as f32 - 150.0), w.rng.range(70.0, 320.0));
                    let c = hsv(w.rng.next() * 360.0);
                    w.ring(fx, fy, 120.0, c);
                    w.burst(fx, fy, 45, c, 420.0, 1.1, 6.0, Shape::Star, 150.0);
                    w.burst(fx, fy, 20, Color::WHITE, 300.0, 0.8, 3.0, Shape::Streak, 100.0);
                    w.shake(6.0);
                }
                step(win, &map.solids, dt, 1.0);
            }
            3 => {
                // LIGHTNING ASCENSION
                win.vy = 0.0;
                win.on_ground = false;
                win.rot += 90.0 * dt;
                if win.y > 120.0 {
                    win.y -= 110.0 * dt;
                }
                if (t * 2.5) as i32 != (prev * 2.5) as i32 {
                    let mut sx = w.rng.range(80.0, W as f32 - 80.0);
                    if (sx - cx).abs() < 110.0 {
                        sx += if sx >= cx { 160.0 } else { -160.0 };
                    }
                    let sx = sx.clamp(40.0, W as f32 - 40.0);
                    w.strikes.push(Strike { x: sx, t: 0.45, owner: who, struck: false, vis: 0.0, pts: Vec::new() });
                }
                let (px, py) = (cx + w.rng.range(-40.0, 40.0), cy + w.rng.range(0.0, 60.0));
                put(w, px, py, 0.0, 60.0, 0.6, 5.0, gold, Shape::Star, 0.0, 0.0);
                if at(prev, t, v.dur - 0.5) {
                    w.fx.flash = 1.0;
                    w.ring(cx, cy, 420.0, Color::WHITE);
                    w.shake(30.0);
                }
            }
            4 => {
                // PHOENIX RISE
                win.y = floor_y - 90.0 * ease(t / 0.9);
                win.vy = 0.0;
                win.on_ground = false;
                for _ in 0..4 {
                    let (px, py) = (cx + w.rng.range(-50.0, 50.0), cy + w.rng.range(-20.0, 40.0));
                    let (vy, size) = (-w.rng.range(80.0, 220.0), w.rng.range(6.0, 12.0));
                    let c = Color::new(255, 110 + (w.rng.next() * 120.0) as u8, 30, 230);
                    put(w, px, py, 0.0, vy, 0.8, size, c, Shape::Circle, -100.0, 0.0);
                }
                if at(prev, t, 0.9) {
                    w.ring(cx, ground, 340.0, orange);
                    for k in 0..11 {
                        let fx = (cx + (k as f32 - 5.0) * 70.0).clamp(20.0, W as f32 - 20.0);
                        w.burst(fx, ground, 6, orange, 260.0, 0.9, 10.0, Shape::Circle, -420.0);
                    }
                    w.shake(20.0);
                    w.fx.flash = 0.5;
                }
            }
            5 => {
                // DOMINATION STOMP: three huge stomps
                let tt = (t - 0.2).max(0.0);
                let cyc = ((tt / 0.8) as i32).min(3);
                if t >= 0.2 && cyc < 3 {
                    let phase = (tt % 0.8) / 0.8;
                    win.y = floor_y - 170.0 * (std::f32::consts::PI * phase).sin().max(0.0).powf(0.7);
                    win.on_ground = false;
                } else {
                    win.y = floor_y;
                    win.on_ground = true;
                }
                win.vy = 0.0;
                if cyc != v.cycle {
                    v.cycle = cyc;
                    if cyc >= 1 {
                        let k = cyc as f32;
                        w.ring(cx, ground, 200.0 + 70.0 * k, orange);
                        w.ring(cx, ground, 130.0, Color::WHITE);
                        w.burst(cx, ground, 14 + cyc as usize * 4, Color::new(120, 95, 70, 255), 500.0, 0.8, 11.0, Shape::Square, 900.0);
                        for j in 0..14 {
                            let dir = if j % 2 == 0 { 1.0 } else { -1.0 };
                            let sp = w.rng.range(250.0, 900.0);
                            put(w, cx, ground - 4.0, dir * sp, 0.0, 0.4, 3.0, Color::WHITE, Shape::Streak, 0.0, 0.0);
                        }
                        w.shake(14.0 + 7.0 * k);
                        w.fx.flash = 0.3;
                        w.fx.punch = 0.07;
                    }
                }
            }
            6 => {
                // SPOTLIGHT BOW
                if at(prev, t, 0.3) {
                    win.vy = -(2.0 * GRAVITY * JUMP * 0.45).sqrt();
                    win.on_ground = false;
                }
                win.rot = if t > 1.0 && t < 1.9 { 28.0 * (std::f32::consts::PI * (t - 1.0) / 0.9).sin() * win.facing } else { 0.0 };
                if w.rng.next() < 0.6 {
                    let (px, vx, vy) = (cx + w.rng.range(-110.0, 110.0), w.rng.range(-20.0, 20.0), w.rng.range(120.0, 220.0));
                    put(w, px, cy - 260.0, vx, vy, 1.6, 7.0, gold, Shape::Star, 0.0, 0.0);
                }
                step(win, &map.solids, dt, 1.0);
            }
            7 => {
                // BLINK STORM
                let n = (t / 0.3) as i32;
                let pn = (prev / 0.3) as i32;
                if n != pn && (1..=8).contains(&n) {
                    let violet = Color::new(190, 120, 255, 255);
                    put(w, cx, cy, 0.0, 0.0, 0.5, SIZE, with_alpha(violet, 160), Shape::Ghost, 0.0, 0.0);
                    w.ring(cx, cy, 90.0, violet);
                    if n == 8 {
                        win.x = map.spawns[who];
                        win.y = floor_y;
                    } else {
                        for _ in 0..10 {
                            let si = (w.rng.next() * map.solids.len() as f32) as usize % map.solids.len();
                            let r = map.solids[si].r;
                            if r.width >= SIZE + 20.0 && r.y - SIZE >= 0.0 {
                                win.x = r.x + w.rng.range(10.0, r.width - SIZE - 10.0);
                                win.y = r.y - SIZE;
                                break;
                            }
                        }
                    }
                    win.vx = 0.0;
                    win.vy = 0.0;
                    win.facing = if n % 2 == 0 { 1.0 } else { -1.0 };
                    let (nx, ny) = win.center();
                    w.ring(nx, ny, 100.0, Color::WHITE);
                    w.ring(nx, ny, 150.0, violet);
                    w.burst(nx, ny, 12, Color::WHITE, 380.0, 0.4, 5.0, Shape::Star, 0.0);
                    w.burst(nx, ny, 8, Color::new(220, 240, 255, 255), 700.0, 0.25, 3.0, Shape::Streak, 0.0);
                    w.shake(8.0);
                }
                step(win, &map.solids, dt, 1.0);
            }
            8 => {
                // GIANT SLASH
                step(win, &map.solids, dt, 1.0);
                if at(prev, t, 0.6) {
                    w.shake(10.0);
                }
                if at(prev, t, 0.95) {
                    w.fx.flash = 1.0;
                    w.shake(38.0);
                    w.fx.punch = 0.08;
                    for k in 0..40 {
                        let f = k as f32 / 39.0;
                        let (px, py) = (f * W as f32, 40.0 + f * (H as f32 - 80.0));
                        let (vx, vy) = (w.rng.range(-80.0, 80.0), w.rng.range(-200.0, -40.0));
                        put(w, px, py, vx, vy, 0.8, 6.0, Color::new(180, 240, 255, 255), Shape::Star, 300.0, 0.0);
                    }
                }
            }
            _ => {
                // METEOR SHOWER
                step(win, &map.solids, dt, 1.0);
                if t > 0.3 && t < 2.6 && (t / 0.17) as i32 != (prev / 0.17) as i32 {
                    let tx = w.rng.range(40.0, W as f32 - 40.0);
                    w.meteors.push(Meteor { tx, warn: 0.03, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, falling: false });
                }
                if at(prev, t, v.dur - 0.4) {
                    w.ring(cx, cy, 420.0, orange);
                    w.ring(cx, cy, 280.0, Color::WHITE);
                    w.fx.flash = 0.9;
                    w.shake(30.0);
                }
            }
        }
    }
    update_particles(&mut w.parts, dt);
    update_booms(&mut w.booms, dt);
    update_strikes(players, w, dt);
    update_projs(players, &map.solids, w, dt);
    update_meteors(players, &map.solids, w, dt);
    v.t >= v.dur
}

// extra world-space visuals for each cutscene (drawn over the players)


// =====================================================================
// main
// =====================================================================

pub(crate) const ROWS: usize = 7;
pub(crate) const ROW_NAMES: [&str; ROWS] = ["CHARACTER", "ABILITY 1", "ABILITY 2", "TRAIL", "TRAIL COLOR", "DEATH ANIM", "VICTORY"];

pub(crate) fn respawn(p: &mut Player, setup: &Setup, x: f32, facing: f32, keys: Keys, w: &mut World) {
    let ult = p.ult;
    *p = Player::new(setup, x, facing, keys);
    p.ult = ult * 0.5;
    p.shield_t = 1.5; // spawn protection
    w.ring(x + SIZE / 2.0, p.y + SIZE / 2.0, 120.0, Color::WHITE);
    w.burst(x + SIZE / 2.0, p.y + SIZE / 2.0, 16, p.kind.color(), 300.0, 0.5, 6.0, Shape::Star, 0.0);
}

pub(crate) fn lava_rise(theme: Theme, time: f32) -> f32 {
    if theme == Theme::Volcano {
        90.0 * ease((time * 0.5).sin().max(0.0))
    } else {
        0.0
    }
}

mod app;
mod config;
mod session;
mod ui;
mod view;

pub fn run() {
    let cfg = config::load();
    app::run(cfg);
}

#[cfg(test)]
mod sim_tests;

