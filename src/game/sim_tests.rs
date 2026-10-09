use super::config::from_args;
use super::session::{FrameInput, Session};
use super::*;

fn fighter(kind: usize) -> Player {
    Player::new(&Setup::new(kind), 200.0, 1.0, Keys)
}

#[test]
fn gravity_pulls_down() {
    let mut p = fighter(0);
    p.on_ground = false;
    p.y = 200.0;
    p.vy = 0.0;
    step(&mut p, &[], 1.0 / 60.0, 1.0);
    assert!(p.vy > 0.0, "y-down sim: gravity increases vy, got {}", p.vy);
    assert!(p.y > 200.0);
}

#[test]
fn launch_sends_the_cube_up() {
    let mut p = fighter(0);
    launch(&mut p, 1.0);
    assert!(p.vy < 0.0);
    assert!(!p.on_ground);
}

#[test]
fn shielder_takes_less_and_titan_resists_knockback() {
    let mut shield = fighter(2);
    shield.hurt(20.0, 100.0, 0.0);
    assert!((shield.hp - (shield.max_hp - 17.0)).abs() < 0.01);

    let mut titan = fighter(3);
    titan.hurt(0.0, 100.0, 0.0);
    assert!((titan.vx - 55.0).abs() < 0.01);

    let mut dash = fighter(0);
    dash.hurt(0.0, 100.0, 0.0);
    assert!((dash.vx - 100.0).abs() < 0.01);
}

#[test]
fn shield_parry_and_block() {
    let mut p = fighter(0);
    p.shield_t = 1.0;
    p.hurt(40.0, 200.0, -100.0);
    assert_eq!(p.hp, p.max_hp);
    assert_eq!(p.vx, 0.0);

    let mut p = fighter(0);
    p.parry_t = 0.1;
    p.hurt(40.0, 200.0, 0.0);
    assert!(p.parried);
    assert_eq!(p.hp, p.max_hp);

    let mut p = fighter(0);
    p.block_held = true;
    p.hurt(10.0, 100.0, -50.0);
    assert!((p.hp - (p.max_hp - 3.0)).abs() < 0.01);
    assert!(p.blocked);
    assert_eq!(p.vy, 0.0);
}

#[test]
fn every_theme_has_a_floor_and_spawns() {
    for theme in THEMES {
        let map = make_map(theme);
        assert!(map.solids.len() >= 5, "{}", theme.name());
        assert!(map.spawns[0] < map.spawns[1]);
        assert!(!map.zones.is_empty());
    }
    assert!(make_map(Theme::Volcano).hazard.is_some());
}

#[test]
fn cpu_match_survives_a_few_seconds() {
    let mut s = Session::new();
    s.cpu_mode = 2;
    s.cpu_prof = [5, 5];
    s.theme_idx = 2;
    s.map = make_map(THEMES[2]);
    s.begin_match();
    let input = FrameInput::default();
    for _ in 0..180 {
        s.tick(&input, 1.0 / 60.0);
        assert!(s.players[0].hp.is_finite() && s.players[1].hp.is_finite());
        assert!(s.world.parts.len() <= 4000);
    }
}

#[test]
fn benchmark_flag_and_config_names() {
    let args = ["--benchmark".into(), "30".into(), "--map".into(), "neon".into()];
    let cfg = from_args(&args).unwrap();
    assert_eq!(cfg.benchmark_secs, Some(30));
    assert_eq!(cfg.map, Some(7));

    let args = ["--p1".into(), "ghost".into(), "--p1-a1".into(), "blink".into(), "--mode".into(), "hill".into(), "--rounds".into(), "5".into(), "--players".into(), "cpu-vs-cpu".into()];
    let cfg = from_args(&args).unwrap();
    assert_eq!(cfg.p[0].kind, Some(4));
    assert_eq!(cfg.p[0].a1, Some(6));
    assert_eq!(cfg.mode, Some(3));
    assert_eq!(cfg.rounds, Some(2));
    assert_eq!(cfg.players, Some(2));
}
