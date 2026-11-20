# README media

[← Back to the project](../../README.md)

| File | Source |
| --- | --- |
| `launch.gif` | A three-second excerpt from the launch recording, showing catapult momentum and release. |
| `flight.gif` | A different three-second excerpt from the same recording, showing free flight and altitude bonuses. |
| `effects.gif` | A three-second excerpt from the collision recording, showing smoke and score effects. |
| `gameplay.gif` | A short recording of the existing Rust/WebAssembly front end, showing a catapult launch, free flight and score bonuses. |
| `smoke-and-scoring.gif` | A second Rust/WebAssembly recording showing smoke, spin bonuses and a high-speed collision. |
| `rocket.png` | The rocket sprite at `(769, 321, 32, 32)` in the original sprite atlas, rotated upright and enlarged with nearest-neighbor sampling. |

The game recordings use ordinary fixed key presses. They do not run an AI model,
train a policy or claim a benchmark result. They were captured from the existing
playable Rust build, which shares the Gymnasium copy's game and rendering source.
The Python environment currently returns numerical state observations and text
rendering; the GIFs are a view of the optional graphical game.

Frames were captured at approximately 10 fps, cropped to the game viewport to
remove browser margins, and encoded locally with FFmpeg using one encoder thread.
The three header GIFs are each 320 × 180, loop independently, and appear side by
side at 260 pixels wide in the README. The larger gameplay GIF is 720 × 405;
the detail GIF is 480 × 270. Files live in this repository so they can be embedded
with relative paths on GitHub.

Original game and artwork: **parse**, with additional art credited to **Quills**.
See [asset provenance](../../assets/reference/PROVENANCE.md) and
[rendering notes](../../RENDERING.md).
