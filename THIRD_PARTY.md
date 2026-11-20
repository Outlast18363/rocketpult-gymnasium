# Third-party software

- [PyO3](https://github.com/PyO3/pyo3), MIT or Apache-2.0, for the optional Python binding.
- [glam](https://github.com/bitshifter/glam-rs), MIT or Apache-2.0, for vectors shared by headless and graphical builds.
- [Gymnasium](https://github.com/Farama-Foundation/Gymnasium), MIT, and [NumPy](https://numpy.org), BSD-3-Clause, are Python runtime dependencies.

- [Macroquad](https://github.com/not-fl3/macroquad), MIT or Apache-2.0.
- [Miniquad](https://github.com/not-fl3/miniquad), MIT or Apache-2.0. `web/gl.js` comes directly from the `miniquad` 0.4.8 crate. The upstream MIT text is included in [web/licenses/miniquad-LICENSE-MIT](web/licenses/miniquad-LICENSE-MIT).
- [quad-snd](https://github.com/not-fl3/quad-snd), MIT or Apache-2.0. `web/audio.js` comes directly from the `quad-snd` 0.2.8 crate. See [web/licenses/quad-snd-NOTICE.md](web/licenses/quad-snd-NOTICE.md).
- [box2d-rs](https://github.com/HumMan/box2d-rs), MIT. Version 0.0.4 is vendored in `vendor/box2d-rs`, with its license included. Its profiling clock is disabled on WebAssembly because `std::time::Instant` is unavailable there; solver code is unchanged. Compiler-era warnings are suppressed within that third-party crate.

Rust dependency versions and transitive dependencies are recorded in `Cargo.lock`. Their licenses remain in effect independently of the project source license.
