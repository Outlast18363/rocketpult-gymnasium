//! CPU-only Rocketpult core. Graphics and audio are optional (`play` feature).
pub mod environment;
pub mod game;
pub mod physics;

#[cfg(feature = "python")]
mod python;
