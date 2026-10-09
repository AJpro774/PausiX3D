//! Command-line options and an optional `pausi.cfg` read at startup.
//! CLI flags override the file. `--benchmark` forces a heavy CPU match.

use super::*;
use std::env;
use std::fs;
use std::process;

#[derive(Clone, Default)]
pub(crate) struct PCfg {
    pub kind: Option<usize>,
    pub a1: Option<usize>,
    pub a2: Option<usize>,
    pub trail: Option<usize>,
    pub color: Option<usize>,
    pub death: Option<usize>,
    pub victory: Option<usize>,
    pub cpu: Option<usize>,
}

#[derive(Clone, Default)]
pub(crate) struct Cfg {
    pub mode: Option<usize>,
    pub map: Option<usize>,
    pub rounds: Option<usize>,
    pub players: Option<usize>,
    pub p: [PCfg; 2],
    pub start: bool,
    pub benchmark_secs: Option<u32>,
    pub screenshot_after: Option<f32>,
    pub screenshot_path: Option<String>,
    pub demo_victory: Option<usize>,
}

pub(crate) fn load() -> Cfg {
    let args: Vec<String> = env::args().skip(1).collect();
    match from_args(&args) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("pausix3d: {e}");
            eprintln!("Run with --help for the option list.");
            process::exit(2);
        }
    }
}

pub(crate) fn from_args(args: &[String]) -> Result<Cfg, String> {
    let mut file: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--config" {
            file = Some(need(args, &mut i, "--config")?);
        } else if let Some(v) = args[i].strip_prefix("--config=") {
            file = Some(v.to_string());
        }
        i += 1;
    }

    let mut cfg = Cfg::default();
    let path = file.clone().unwrap_or_else(|| "pausi.cfg".to_string());
    let explicit = file.is_some();
    if explicit || std::path::Path::new(&path).is_file() {
        let text = fs::read_to_string(&path).map_err(|e| format!("could not read {path}: {e}"))?;
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .map(|(a, b)| (a.trim(), b.trim()))
                .ok_or_else(|| format!("{path}:{}: expected key = value", n + 1))?;
            apply_kv(&mut cfg, k, v).map_err(|e| format!("{path}:{}: {e}", n + 1))?;
        }
    }

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--help" || a == "-h" {
            print_help();
            process::exit(0);
        }
        if a == "--config" {
            i += 2;
            continue;
        }
        if a.starts_with("--config=") {
            i += 1;
            continue;
        }
        if a == "--start" {
            cfg.start = true;
            i += 1;
            continue;
        }
        let (key, inline) = if let Some((k, v)) = a.split_once('=') {
            (k, Some(v.to_string()))
        } else {
            (a, None)
        };
        let val = match inline {
            Some(v) => v,
            None => need(args, &mut i, key)?,
        };
        match key {
            "--mode" => cfg.mode = Some(parse_mode(&val)?),
            "--map" => cfg.map = Some(parse_map(&val)?),
            "--rounds" => cfg.rounds = Some(parse_rounds(&val)?),
            "--players" => cfg.players = Some(parse_players(&val)?),
            "--p1" => cfg.p[0].kind = Some(parse_kind(&val)?),
            "--p2" => cfg.p[1].kind = Some(parse_kind(&val)?),
            "--p1-a1" => cfg.p[0].a1 = Some(parse_ability(&val)?),
            "--p1-a2" => cfg.p[0].a2 = Some(parse_ability(&val)?),
            "--p2-a1" => cfg.p[1].a1 = Some(parse_ability(&val)?),
            "--p2-a2" => cfg.p[1].a2 = Some(parse_ability(&val)?),
            "--p1-trail" => cfg.p[0].trail = Some(parse_trail(&val)?),
            "--p2-trail" => cfg.p[1].trail = Some(parse_trail(&val)?),
            "--p1-color" => cfg.p[0].color = Some(parse_color(&val)?),
            "--p2-color" => cfg.p[1].color = Some(parse_color(&val)?),
            "--p1-death" => cfg.p[0].death = Some(parse_named(&val, &DEATH_NAMES, &DEATH_ALIASES)?),
            "--p2-death" => cfg.p[1].death = Some(parse_named(&val, &DEATH_NAMES, &DEATH_ALIASES)?),
            "--p1-victory" => cfg.p[0].victory = Some(parse_named(&val, &VICTORY_NAMES, &VIC_ALIASES)?),
            "--p2-victory" => cfg.p[1].victory = Some(parse_named(&val, &VICTORY_NAMES, &VIC_ALIASES)?),
            "--p1-cpu" => cfg.p[0].cpu = Some(parse_named(&val, &CPU_NAMES, &[])?),
            "--p2-cpu" => cfg.p[1].cpu = Some(parse_named(&val, &CPU_NAMES, &[])?),
            "--benchmark" => {
                let n: u32 = val.parse().map_err(|_| format!("--benchmark needs a number of seconds, got {val}"))?;
                if n == 0 {
                    return Err("--benchmark seconds must be at least 1".into());
                }
                cfg.benchmark_secs = Some(n);
            }
            "--screenshot-after" => {
                let t: f32 = val.parse().map_err(|_| format!("--screenshot-after needs seconds, got {val}"))?;
                cfg.screenshot_after = Some(t.max(0.0));
            }
            "--screenshot" => cfg.screenshot_path = Some(val),
            "--demo-victory" => cfg.demo_victory = Some(parse_named(&val, &VICTORY_NAMES, &VIC_ALIASES)?),
            other => return Err(format!("unknown option {other}")),
        }
        i += 1;
    }
    if cfg.screenshot_path.is_some() && cfg.screenshot_after.is_none() {
        cfg.screenshot_after = Some(1.5);
    }
    Ok(cfg)
}

fn apply_kv(cfg: &mut Cfg, key: &str, val: &str) -> Result<(), String> {
    match key {
        "mode" => cfg.mode = Some(parse_mode(val)?),
        "map" => cfg.map = Some(parse_map(val)?),
        "rounds" => cfg.rounds = Some(parse_rounds(val)?),
        "players" => cfg.players = Some(parse_players(val)?),
        "start" => cfg.start = truthy(val)?,
        "p1" => cfg.p[0].kind = Some(parse_kind(val)?),
        "p2" => cfg.p[1].kind = Some(parse_kind(val)?),
        "p1.a1" => cfg.p[0].a1 = Some(parse_ability(val)?),
        "p1.a2" => cfg.p[0].a2 = Some(parse_ability(val)?),
        "p2.a1" => cfg.p[1].a1 = Some(parse_ability(val)?),
        "p2.a2" => cfg.p[1].a2 = Some(parse_ability(val)?),
        "p1.trail" => cfg.p[0].trail = Some(parse_trail(val)?),
        "p2.trail" => cfg.p[1].trail = Some(parse_trail(val)?),
        "p1.color" => cfg.p[0].color = Some(parse_color(val)?),
        "p2.color" => cfg.p[1].color = Some(parse_color(val)?),
        "p1.death" => cfg.p[0].death = Some(parse_named(val, &DEATH_NAMES, &DEATH_ALIASES)?),
        "p2.death" => cfg.p[1].death = Some(parse_named(val, &DEATH_NAMES, &DEATH_ALIASES)?),
        "p1.victory" => cfg.p[0].victory = Some(parse_named(val, &VICTORY_NAMES, &VIC_ALIASES)?),
        "p2.victory" => cfg.p[1].victory = Some(parse_named(val, &VICTORY_NAMES, &VIC_ALIASES)?),
        "p1.cpu" => cfg.p[0].cpu = Some(parse_named(val, &CPU_NAMES, &[])?),
        "p2.cpu" => cfg.p[1].cpu = Some(parse_named(val, &CPU_NAMES, &[])?),
        other => Err(format!("unknown setting {other}"))?,
    }
    Ok(())
}

fn need(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i).cloned().ok_or_else(|| format!("{flag} needs a value"))
}

fn truthy(v: &str) -> Result<bool, String> {
    match norm(v).as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("expected yes or no, got {v}")),
    }
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

fn parse_named(val: &str, names: &[&str], aliases: &[(&str, usize)]) -> Result<usize, String> {
    let n = norm(val);
    if let Ok(i) = val.trim().parse::<usize>() {
        if i < names.len() {
            return Ok(i);
        }
    }
    for (a, i) in aliases {
        if n == *a {
            return Ok(*i);
        }
    }
    for (i, name) in names.iter().enumerate() {
        if norm(name) == n {
            return Ok(i);
        }
    }
    Err(format!("unknown name {val}"))
}

fn parse_kind(val: &str) -> Result<usize, String> {
    let names: Vec<&str> = KINDS.iter().map(|k| k.name()).collect();
    parse_named(val, &names, &[])
}

fn parse_ability(val: &str) -> Result<usize, String> {
    let names: Vec<&str> = ABILITIES.iter().map(|a| a.name()).collect();
    parse_named(val, &names, &[])
}

fn parse_trail(val: &str) -> Result<usize, String> {
    let names: Vec<&str> = TRAIL_STYLES.iter().map(|t| t.name()).collect();
    parse_named(val, &names, &[("off", 0), ("none", 0)])
}

fn parse_color(val: &str) -> Result<usize, String> {
    parse_named(val, &TRAIL_COLOR_NAMES, &[("rainbow", 8)])
}

fn parse_mode(val: &str) -> Result<usize, String> {
    let names: Vec<&str> = MODES.iter().map(|m| m.name()).collect();
    parse_named(val, &names, &[("koth", 3), ("kingofthehill", 3), ("hill", 3)])
}

fn parse_map(val: &str) -> Result<usize, String> {
    let names: Vec<&str> = THEMES.iter().map(|t| t.name()).collect();
    parse_named(
        val,
        &names,
        &[
            ("meadow", 0),
            ("sunny", 0),
            ("skyline", 1),
            ("night", 1),
            ("frozen", 3),
            ("peaks", 3),
            ("space", 4),
            ("sea", 5),
            ("sand", 6),
            ("ruins", 6),
            ("neon", 7),
            ("arcade", 7),
        ],
    )
}

fn parse_rounds(val: &str) -> Result<usize, String> {
    let n: u32 = val.trim().parse().map_err(|_| format!("rounds must be 1, 3, 5, 10, or 15 (got {val})"))?;
    super::session::ROUND_OPTIONS
        .iter()
        .position(|r| *r == n)
        .ok_or_else(|| format!("rounds must be 1, 3, 5, 10, or 15 (got {val})"))
}

fn parse_players(val: &str) -> Result<usize, String> {
    match norm(val).as_str() {
        "0" | "pvp" | "human" | "humans" | "humanvshuman" => Ok(0),
        "1" | "cpu" | "vscpu" | "humanvscpu" => Ok(1),
        "2" | "spectate" | "spectator" | "cpuvscpu" | "cpus" => Ok(2),
        _ => Err(format!("players must be pvp, cpu, or cpu-vs-cpu (got {val})")),
    }
}

const DEATH_ALIASES: &[(&str, usize)] = &[
    ("spin", 5),
    ("spinoff", 5),
    ("confetti", 6),
    ("freeze", 7),
    ("freezecrack", 7),
    ("blackhole", 8),
    ("hole", 8),
];

const VIC_ALIASES: &[(&str, usize)] = &[
    ("knife", 0),
    ("stab", 0),
    ("finale", 0),
    ("dance", 1),
    ("lightning", 3),
    ("ascension", 3),
    ("phoenix", 4),
    ("stomp", 5),
    ("domination", 5),
    ("spotlight", 6),
    ("bow", 6),
    ("blink", 7),
    ("blinkstorm", 7),
    ("slash", 8),
    ("giantslash", 8),
    ("meteor", 9),
    ("meteors", 9),
    ("meteorshower", 9),
];

fn print_help() {
    println!(
        "\
PausiX3D — a 3D remake of Pausi

Usage:
  pausix3d [options]

Match setup (also accepted in pausi.cfg as `key = value`):
  --mode classic|timed|stock|hill
  --map meadow|skyline|volcano|frozen|space|sea|sand|neon
  --rounds 1|3|5|10|15
  --players pvp|cpu|cpu-vs-cpu
  --p1 / --p2 <character>          dasher bomber shielder titan ghost spark
  --p1-a1 / --p1-a2 / --p2-a1 / --p2-a2 <ability>
  --p1-trail / --p2-trail          none dots ghost sparkle smoke
  --p1-color / --p2-color          body white red yellow green cyan purple pink rainbow
  --p1-death / --p2-death          shatter … glitch, or random
  --p1-victory / --p2-victory      knife finale … meteor shower, or random
  --p1-cpu / --p2-cpu              balanced … pro, or random
  --start                          skip the menu and begin the match
  --config <file>                  read settings (default: pausi.cfg if it exists)

Benchmark (vsync off, exits on its own):
  --benchmark <seconds>            CPU vs CPU on Volcano with heavy effects

Screenshots:
  --screenshot-after <seconds> --screenshot <path.png>
  --demo-victory <name>            jump straight into a victory cutscene
"
    );
}
