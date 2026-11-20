//! Training lifecycle and fixed virtual clock, independent of Python and rendering.
use crate::game::{normalize, Controls, Game, Phase, MAX_FUEL, STEP};

#[derive(Clone, Debug)]
pub struct Config {
    pub reference_hz: u32,
    pub frame_skip: u32,
    pub max_episode_steps: u32,
    pub allow_slow_motion: bool,
    pub reward_scale: f64,
    pub warmup_ticks: u32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            reference_hz: 120,
            frame_skip: 4,
            max_episode_steps: 1500,
            allow_slow_motion: false,
            reward_scale: 0.001,
            warmup_ticks: 120,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !(30..=240).contains(&self.reference_hz) {
            return Err("reference_hz must be between 30 and 240".into());
        }
        if !(1..=32).contains(&self.frame_skip) {
            return Err("frame_skip must be between 1 and 32".into());
        }
        if !(1..=1_000_000).contains(&self.max_episode_steps) {
            return Err("max_episode_steps must be between 1 and 1000000".into());
        }
        if self.warmup_ticks > 600 {
            return Err("warmup_ticks must be between 0 and 600".into());
        }
        if !self.reward_scale.is_finite() || self.reward_scale <= 0. {
            return Err("reward_scale must be finite and positive".into());
        }
        Ok(())
    }
    pub fn action_count(&self) -> usize {
        if self.allow_slow_motion {
            48
        } else {
            24
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Keep,
    Release,
    Deploy,
    Retract,
}
#[derive(Clone, Copy, Debug)]
pub struct Action {
    pub controls: Controls,
    pub command: Command,
}
impl Action {
    pub fn decode(value: usize, allow_slow: bool) -> Result<Self, String> {
        let count = if allow_slow { 48 } else { 24 };
        if value >= count {
            return Err(format!("action must be an integer in [0, {})", count));
        }
        let base = value % 24;
        Ok(Self {
            controls: Controls {
                thrust: base % 2 == 1,
                turn: match (base / 2) % 3 {
                    0 => 0.,
                    1 => -1.,
                    _ => 1.,
                },
                slow: value >= 24,
            },
            command: match base / 6 {
                0 => Command::Keep,
                1 => Command::Release,
                2 => Command::Deploy,
                _ => Command::Retract,
            },
        })
    }
    pub fn names(allow_slow: bool) -> Vec<String> {
        (0..if allow_slow { 48 } else { 24 })
            .map(|i| {
                let a = Self::decode(i, allow_slow).unwrap();
                format!(
                    "{} / {} / {}{}",
                    if a.controls.thrust { "thrust" } else { "coast" },
                    if a.controls.turn < 0. {
                        "left"
                    } else if a.controls.turn > 0. {
                        "right"
                    } else {
                        "straight"
                    },
                    match a.command {
                        Command::Keep => "keep",
                        Command::Release => "release",
                        Command::Deploy => "deploy legs",
                        Command::Retract => "retract legs",
                    },
                    if a.controls.slow { " / slow time" } else { "" }
                )
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct Info {
    pub score: u32,
    pub score_delta: u32,
    pub is_success: bool,
    pub phase: &'static str,
    pub outcome: String,
    pub fuel: f32,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub angle: f32,
    pub angular_velocity: f32,
    pub episode_steps: u32,
    pub simulation_ticks: u64,
    pub virtual_time: f64,
    pub physics_time: f64,
    pub episode_return: f64,
    pub time_scale: f32,
    pub landing_multiplier: u32,
}
pub struct Transition {
    pub observation: Vec<f32>,
    pub reward: f64,
    pub terminated: bool,
    pub truncated: bool,
    pub info: Info,
}
pub struct Environment {
    pub config: Config,
    game: Option<Game>,
    episode_steps: u32,
    simulation_ticks: u64,
    physics_time: f64,
    episode_return: f64,
    needs_reset: bool,
}
impl Environment {
    pub fn new(config: Config) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            game: None,
            episode_steps: 0,
            simulation_ticks: 0,
            physics_time: 0.,
            episode_return: 0.,
            needs_reset: true,
        })
    }
    pub fn reset(&mut self, initial_fuel: f32) -> Result<(Vec<f32>, Info), String> {
        if !initial_fuel.is_finite() || !(0.0..=MAX_FUEL).contains(&initial_fuel) {
            return Err(format!(
                "initial_fuel must be finite and between 0 and {MAX_FUEL}"
            ));
        }
        let mut g = Game::new(0);
        for _ in 0..self.config.warmup_ticks {
            g.update(1. / self.config.reference_hz as f32, Controls::default());
        }
        g.rocket.fuel = initial_fuel;
        g.events.clear();
        g.notices.clear();
        self.game = Some(g);
        self.episode_steps = 0;
        self.simulation_ticks = 0;
        self.physics_time = 0.;
        self.episode_return = 0.;
        self.needs_reset = false;
        Ok((self.observation(), self.info(0, false)))
    }
    pub fn step(&mut self, value: usize) -> Result<Transition, String> {
        if self.needs_reset {
            return Err("Call reset() before step(), and after an episode ends".into());
        }
        let a = Action::decode(value, self.config.allow_slow_motion)?;
        let g = self.game.as_mut().unwrap();
        let before = g.score;
        match a.command {
            Command::Keep => {}
            Command::Release => g.release_if_attached(),
            Command::Deploy => g.set_landing_legs(true),
            Command::Retract => g.set_landing_legs(false),
        }
        for _ in 0..self.config.frame_skip {
            g.update(1. / self.config.reference_hz as f32, a.controls);
            self.simulation_ticks += 1;
            self.physics_time += (STEP * g.time_scale) as f64;
            g.events.clear();
            g.notices.clear();
            if g.finished() {
                break;
            }
        }
        self.episode_steps += 1;
        let delta = g.score.saturating_sub(before);
        let terminated = g.finished();
        let truncated = !terminated && self.episode_steps >= self.config.max_episode_steps;
        self.needs_reset = terminated || truncated;
        let reward = delta as f64 * self.config.reward_scale;
        self.episode_return += reward;
        let observation = self.observation();
        if observation.iter().any(|x| !x.is_finite()) {
            self.needs_reset = true;
            return Err("Non-finite physics state: reset the episode".into());
        }
        Ok(Transition {
            observation,
            reward,
            terminated,
            truncated,
            info: self.info(delta, truncated),
        })
    }
    pub fn observation_names() -> Vec<String> {
        let mut names = Game::training_observation_names();
        names.extend(
            [
                "episode_progress",
                "virtual_time",
                "physics_time",
                "reference_hz",
                "frame_skip",
            ]
            .into_iter()
            .map(String::from),
        );
        names
    }
    fn observation(&self) -> Vec<f32> {
        let mut out = self.game.as_ref().unwrap().training_observation();
        out.extend([
            self.episode_steps as f32 / self.config.max_episode_steps as f32,
            normalize(
                self.simulation_ticks as f32 / self.config.reference_hz as f32,
                60.,
            ),
            normalize(self.physics_time as f32, 60.),
            self.config.reference_hz as f32 / 240.,
            self.config.frame_skip as f32 / 32.,
        ]);
        out
    }
    fn info(&self, score_delta: u32, truncated: bool) -> Info {
        let g = self.game.as_ref().unwrap();
        Info {
            score: g.score,
            score_delta,
            is_success: g.phase == Phase::Landed,
            phase: match g.phase {
                Phase::Ready => "ready",
                Phase::Attached => "attached",
                Phase::Flying => "flying",
                Phase::Crashed => "crashed",
                Phase::Landed => "landed",
            },
            outcome: if truncated {
                "time_limit".into()
            } else if g.reason.is_empty() {
                "running".into()
            } else {
                g.reason.to_lowercase().replace(' ', "_")
            },
            fuel: g.rocket.fuel,
            position: g.rocket.pos.to_array(),
            velocity: g.rocket.vel.to_array(),
            angle: g.rocket.angle,
            angular_velocity: g.rocket.omega,
            episode_steps: self.episode_steps,
            simulation_ticks: self.simulation_ticks,
            virtual_time: self.simulation_ticks as f64 / self.config.reference_hz as f64,
            physics_time: self.physics_time,
            episode_return: self.episode_return,
            time_scale: g.time_scale,
            landing_multiplier: g.landing_multiplier,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gym_reset_is_clean_and_observations_match_schema() {
        let mut e = Environment::new(Config::default()).unwrap();
        let (a, i) = e.reset(MAX_FUEL).unwrap();
        assert_eq!(a.len(), Environment::observation_names().len());
        assert!(a.iter().all(|v| v.is_finite() && (-1.0..=1.0).contains(v)));
        assert_eq!(i.score, 0);
        e.step(1).unwrap();
        let (b, i) = e.reset(MAX_FUEL).unwrap();
        assert_eq!(a, b);
        assert_eq!(i.episode_steps, 0);
    }
    #[test]
    fn repeated_release_does_not_toggle_legs() {
        let mut e = Environment::new(Config {
            frame_skip: 1,
            ..Config::default()
        })
        .unwrap();
        e.reset(MAX_FUEL).unwrap();
        e.step(6).unwrap();
        e.step(12).unwrap();
        e.step(6).unwrap();
        assert!(e.game.as_ref().unwrap().rocket.legs);
        e.step(12).unwrap();
        assert!(e.game.as_ref().unwrap().rocket.legs);
    }
    #[test]
    fn horizon_is_truncation_and_requires_explicit_reset() {
        let mut e = Environment::new(Config {
            max_episode_steps: 2,
            ..Config::default()
        })
        .unwrap();
        assert!(e.step(0).is_err());
        e.reset(MAX_FUEL).unwrap();
        assert!(!e.step(0).unwrap().truncated);
        let t = e.step(0).unwrap();
        assert!(t.truncated && !t.terminated);
        assert!(e.step(0).is_err());
    }
    #[test]
    fn episodes_are_independent() {
        let mut a = Environment::new(Config::default()).unwrap();
        let mut b = Environment::new(Config::default()).unwrap();
        a.reset(MAX_FUEL).unwrap();
        b.reset(MAX_FUEL).unwrap();
        for _ in 0..10 {
            assert_eq!(
                a.step(1).unwrap().observation,
                b.step(1).unwrap().observation
            );
        }
    }
    #[test]
    fn rejects_invalid_actions_without_advancing() {
        let mut e = Environment::new(Config::default()).unwrap();
        e.reset(MAX_FUEL).unwrap();
        assert!(e.step(24).is_err());
        assert_eq!(e.episode_steps, 0);
        assert!(Environment::new(Config {
            frame_skip: 0,
            ..Config::default()
        })
        .is_err());
    }
}
