# ROCKETPULT Classic — Rust recreation

This document describes the optional playable front end included in this
repository. Enable the `play` feature for graphical builds; see [README.md](README.md).

A playable, independently written recreation of [ROCKETPULT Classic by parse](https://parsenoire.itch.io/rocketpult). Built with Rust and Macroquad. The same Rust simulation and renderer run as a native desktop game and as WebAssembly in a browser.

## Play

Run the graphical game from source:

```sh
cargo run --release --features play --bin rocketpult
```

On macOS, `Play.command` builds and starts the game. To create a local app bundle,
run `./scripts/package-mac.sh`, then open **Rocketpult.app**.

Browser build:

```sh
./scripts/build-web.sh
python3 scripts/serve.py
```

Open <http://127.0.0.1:8086>. Click the game if the keyboard is not focused.

## Controls

| Key | Action |
| --- | --- |
| Up / W | Thrust; build momentum while attached to the catapult |
| Left / A, Right / D | Rotate the rocket |
| Space | Release from catapult; toggle landing legs in flight; retry after a flight |
| Z (hold) | Slow time, at a real-time fuel cost |
| R | Restart the current flight |
| H | Open/close the flight manual |
| P / Escape | Pause/resume |
| M | Mute/unmute |
| F | Toggle fullscreen |
| Backspace (hold 5 seconds) | Clear the record and restart |

Thrust to swing the arm. Release while the rocket points up and to the right. Countersteer to stop spinning. Point the engine downward and thrust to brake the fall, then land upright with the legs deployed. The bronze X2 targets are the widest and easiest landings.

## Implemented mechanics

- Box2D catapult body and arm, original hinge/attachment joints, collision polygons and inherited launch velocity.
- Gravity, thrust, rotation with momentum, limited fuel, and slow motion.
- Box2D collision detection, impact analysis, friction and restitution from the original export.
- Upright, low-speed landings with deployed legs; original bronze, silver and gold targets with X2/X4/X8 multipliers and the exported sector spacing.
- Original distance, altitude, spin, wobble, top-spin, hit and empty-tank bonuses. Landing multiplies earned points by the target multiplier.
- Original sprite atlas and bitmap fonts, original camera zoom/follow formulas, moving telemetry, two-pass water compositing, smoke, fire and splash effects.
- Synthesized sound effects, telemetry, pause, flight manual, and saved high scores.
- Original per-display-frame physics timing, including its high-refresh behavior; fuel and timers follow real time. On macOS, the screen refresh rate drives equivalent classic physics ticks.

The rendering and physics were revised from a visual approximation to use the original exported artwork, drawing rules and dynamic configuration. See [RENDERING.md](RENDERING.md) for the source mapping and remaining differences. The Rust Box2D solver is a different version from the export, random sequences are independent, some UI transitions differ, and audio is synthesized. The original music is not included.

## Build and verify

Install Rust from <https://rustup.rs>. Native builds require the platform's standard C development tools.

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release --features play --bin rocketpult
rustup target add wasm32-unknown-unknown
./scripts/build-web.sh
```

## Source map

- `src/game.rs`: game state, classic scoring/status rules, and simulation tests.
- `src/physics.rs`: Box2D world, original collision shapes, joints and dynamic configuration.
- `src/render.rs`: original sprites, particles, camera, bitmap text, telemetry, popups and menus.
- `src/audio.rs`: generated audio and playback.
- `src/main.rs`: window setup, input, fixed-step loop, persistence, and rendering.
- `src/storage.rs`: native/browser high-score persistence.
- `web/`: browser entry point and vendored support scripts; the WebAssembly build is generated locally.

The desktop record is stored in the user's application-data folder (`~/Library/Application Support/RocketpultRust/best-score.txt` on macOS). The web record is stored in this site's browser local storage. They are separate records.

## Credit

Original game concept, design, and artwork: **parse**, [ROCKETPULT Classic](https://parsenoire.itch.io/rocketpult), with additional art credited to Quills. Asset provenance is recorded in `assets/reference/PROVENANCE.md`. This is an unofficial recreation and is not affiliated with the original developer.

Third-party Rust dependencies retain their original licenses. The browser support files are vendored directly from the matching Miniquad 0.4.8 and quad-snd 0.2.8 crates. See `THIRD_PARTY.md`.
