//! Small synthesised effects are embedded at runtime as WAV data.
use crate::game::{Event, Game};
use macroquad::audio::*;

pub struct Sounds {
    engine: Sound,
    launch: Sound,
    crash: Sound,
    splash: Sound,
    land: Sound,
    bonus: Sound,
    engine_playing: bool,
    pub muted: bool,
    cooldown: f32,
}

impl Sounds {
    pub async fn new() -> Self {
        Self {
            engine: load_sound_from_bytes(&synth(1.0, 0)).await.unwrap(),
            launch: load_sound_from_bytes(&synth(0.2, 1)).await.unwrap(),
            crash: load_sound_from_bytes(&synth(0.7, 2)).await.unwrap(),
            splash: load_sound_from_bytes(&synth(0.5, 3)).await.unwrap(),
            land: load_sound_from_bytes(&synth(0.65, 4)).await.unwrap(),
            bonus: load_sound_from_bytes(&synth(0.12, 5)).await.unwrap(),
            engine_playing: false,
            muted: false,
            cooldown: 0.0,
        }
    }
    pub fn toggle(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            self.quiet();
        }
    }
    pub fn quiet(&mut self) {
        if self.engine_playing {
            stop_sound(&self.engine);
            self.engine_playing = false;
        }
    }
    pub fn update(&mut self, game: &Game, dt: f32) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        if self.muted {
            return;
        }
        if game.active_thrust && !game.finished() {
            if !self.engine_playing {
                play_sound(
                    &self.engine,
                    PlaySoundParams {
                        looped: true,
                        volume: 0.22,
                    },
                );
                self.engine_playing = true;
            }
        } else {
            self.quiet();
        }
        for event in &game.events {
            let sound = match event {
                Event::Launch => &self.launch,
                Event::Crash => &self.crash,
                Event::Splash => &self.splash,
                Event::Land => &self.land,
                Event::Bonus if self.cooldown == 0.0 => {
                    self.cooldown = 0.12;
                    &self.bonus
                }
                _ => continue,
            };
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: 0.35,
                },
            );
        }
    }
}

fn synth(seconds: f32, kind: u8) -> Vec<u8> {
    let rate = 22_050u32;
    let count = (seconds * rate as f32) as u32;
    let bytes = count * 2;
    let mut out = Vec::with_capacity((bytes + 44) as usize);
    out.extend(b"RIFF");
    out.extend((bytes + 36).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(bytes.to_le_bytes());
    let mut seed = 18491u32;
    let mut smooth_noise = 0.0;
    for i in 0..count {
        let t = i as f32 / rate as f32;
        let p = t / seconds;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = ((seed >> 16) as f32 / 32768.0) - 1.0;
        smooth_noise += (noise - smooth_noise) * 0.18;
        let tone = |freq: f32| (std::f32::consts::TAU * freq * t).sin();
        let sample = match kind {
            0 => smooth_noise * 0.75 + tone(65.0) * 0.1,
            1 => (tone(220.0 + 900.0 * p) * 0.2 + noise * 0.15) * (1.0 - p),
            2 => (smooth_noise * 1.4 + tone(55.0 - 25.0 * p) * 0.3) * (1.0 - p).powi(2),
            3 => smooth_noise * (1.0 - p) * 1.5,
            4 => {
                let note = [523.25, 659.25, 783.99, 1046.5][((p * 4.0) as usize).min(3)];
                tone(note) * 0.24 * (1.0 - (p * 4.0).fract())
            }
            _ => tone(880.0 + p * 440.0) * 0.18 * (1.0 - p),
        };
        out.extend(((sample.clamp(-1.0, 1.0) * 28000.0) as i16).to_le_bytes());
    }
    out
}
