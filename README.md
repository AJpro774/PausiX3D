# PausiX3D

A 3D remake of [Pausi](https://github.com/AJpro774/Pausi), the fast cube-vs-cube duel. Same characters, abilities, maps, and match rules. Every shape keeps the original 2D silhouette and palette, thickened into a lit low-poly diorama and seen from a slight angle. Gameplay stays on the 2D plane. The HUD, menus, and banners are the original flat overlay. There are no art files to download.

Built with Rust and [Bevy](https://bevyengine.org/) 0.20.

## Run

Install Rust with [rustup](https://rustup.rs/) (the MSVC toolchain on Windows). Then:

```bash
cargo run --release
```

The first release build compiles Bevy and takes a while. After that, launches are quick. The window is 1280×720, the original resolution.

```bash
# straight into a match
cargo run --release -- --players cpu --map volcano --start

# or drop a pausi.cfg next to the binary (see pausi.cfg.example)
```

## Controls

| | Player 1 | Player 2 |
|---|---|---|
| Move | `A` / `D` | `Left` / `Right` |
| Jump (again in the air to double jump; on a wall to wall jump) | `W` | `Up` |
| Hold to charge a bigger jump | `S` | `Down` |
| Ability 1 / Ability 2 | `F` / `G` | `,` / `.` |
| Knife | `E` | `/` |
| Block (tap as a hit lands to parry) | `Q` | `Right Shift` |
| Ultimate, when the meter is full | `V` | `Right Ctrl` |

| Key | What it does |
|---|---|
| `W`/`S` or `Up`/`Down` | Move between rows on your panel |
| `A`/`D` or `Left`/`Right` | Change the selected value |
| `F` / `L` | Ready up |
| `M` | Starting map |
| `N` | Rounds: 1, 3, 5, 10, 15 |
| `B` | Game mode |
| `C` | Human vs human, human vs CPU, CPU vs CPU |
| `1` / `2` | CPU personality for that player |
| `Space` / `Enter` | Next round, or skip a victory cutscene |
| `R` | Back to the menu |
| `Esc` | Quit |

## What carried over

- **6 characters** with the original passives and ultimates: Dasher, Bomber, Shielder, Titan, Ghost, Spark.
- **12 abilities** in two loadout slots: Dash, Slam, Bomb, Hop, Shield, Pulse, Blink, Frost, Thunder, Magnet, Mend, Fireball.
- **Combat**: block, parry, knife, stagger via hit-stop, combo counter, ultimate meter, cinematic supers, hit-stop, camera shake, punch zoom, and the slow-motion K.O. zoom.
- **8 maps** with their events: Sunny Meadow, Night Skyline, Volcano, Frozen Peaks, Deep Space, Deep Sea, Sand Ruins, Neon Arcade. Wind, meteors, lava, zero-g, and lasers are the same rules as the 2D game.
- **4 modes**: Classic, Timed, Stock, King of the Hill. Series length is 1, 3, 5, 10, or 15 rounds.
- **10 death animations** and **10 victory cutscenes**, plus Random.
- **5 trail styles** and **9 trail colors**.
- **CPU** with 8 personalities, Random, and a CPU vs CPU spectator mode.
- Movement is the original simulation: charged jumps, wall slides, wall jumps, double jumps, and the same cooldowns and damage numbers. Gameplay still runs in the 1280×720, y-down space and is drawn in 3D on top.

## Command line and `pausi.cfg`

Flags override the file. Names are case-insensitive and ignore spaces (`king of the hill`, `koth`, and `hill` are the same mode).

```bash
cargo run --release -- \
  --mode hill \
  --map neon \
  --rounds 5 \
  --players cpu \
  --p1 spark --p1-a1 thunder --p1-a2 dash --p1-trail sparkle --p1-color yellow \
  --p1-death lightning --p1-victory meteor \
  --p2 titan --p2-cpu pro \
  --start
```

| Flag | File key | Meaning |
|---|---|---|
| `--mode` | `mode` | `classic` `timed` `stock` `hill` |
| `--map` | `map` | `meadow` `skyline` `volcano` `frozen` `space` `sea` `sand` `neon` |
| `--rounds` | `rounds` | `1` `3` `5` `10` `15` |
| `--players` | `players` | `pvp` `cpu` `cpu-vs-cpu` |
| `--p1` `--p2` | `p1` `p2` | character name |
| `--p1-a1` `--p1-a2` `--p2-a1` `--p2-a2` | `p1.a1` … | ability name |
| `--p1-trail` `--p2-trail` | `p1.trail` | `none` `dots` `ghost` `sparkle` `smoke` |
| `--p1-color` `--p2-color` | `p1.color` | `body` `white` `red` `yellow` `green` `cyan` `purple` `pink` `rainbow` |
| `--p1-death` `--p2-death` | `p1.death` | death animation, or `random` |
| `--p1-victory` `--p2-victory` | `p1.victory` | victory cutscene, or `random` |
| `--p1-cpu` `--p2-cpu` | `p1.cpu` | personality, or `random` |
| `--start` | `start = yes` | skip the menu |
| `--config <file>` | | settings file (default is `pausi.cfg` when it exists) |

`--demo-victory fireworks` jumps into that cutscene. `--screenshot-after 2 --screenshot shot.png` saves a frame and exits.

## Benchmark

```bash
cargo run --release -- --benchmark 60
```

This starts a CPU vs CPU match on **Volcano** (lava, meteors, bomber carpet-bombs, spark lightning, smoke and sparkle trails, both ultimates already full), with **vsync off**, and leaves the window updating even if it loses focus. It is unattended: no key presses, and the process exits on its own.

Shader compile is not part of the score. The clock waits until frames have settled (at least a second of steady frames, or 15 seconds, whichever comes first), then records the next N seconds. It prints the report and writes the same text to `benchmark.txt` in the working directory:

- average FPS
- 1% low FPS (average FPS of the slowest 1% of frames)
- minimum FPS
- frame time average, min, max, and 99th percentile, in milliseconds

Leave the window visible. A minimized window is often throttled by the OS and will not reflect how the game actually runs. A GPU and a window are required — the benchmark measures rendered frames, it does not use a software headless backend.

```bash
cargo run --release -- --benchmark 60
type benchmark.txt   # Windows
cat benchmark.txt    # macOS / Linux
```

## Tests

```bash
cargo test --release
```

The tests cover gravity, launch, shield / parry / block, map generation, a short CPU vs CPU match, and config parsing. They do not open a window.

## Differences from the 2D game

- Everything on screen is a mesh (cubes, spheres, cones, cylinders, toruses) with simple lighting. One directional light casts shadows. Particles, trails, and transparent effects do not, so a full ultimate still stays cheap.
- At most 1400 particles are drawn per frame. The simulation still generates them the same way and drops new bursts past 1600, matching the original cap.
- Camera shake is visual. It does not advance the gameplay random-number stream the way the original draw call did.
- Victory cutscenes run the original choreography (particles, strikes, meteors, ability dumps). The knife, phoenix wings, spotlight column, and giant slash are 3D stand-ins for the 2D overlays.
- There is no gamepad and no audio. The original was keyboard-only and silent too.
