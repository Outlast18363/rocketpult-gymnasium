//! Classic event-sheet rules, with a 60 Hz reference clock and Box2D physics.
use crate::physics::Physics;
use glam::{vec2, Vec2};
use std::f32::consts::{PI, TAU};
pub const STEP: f32 = 1. / 60.;
pub const MAX_FUEL: f32 = 30_000.;
pub const PIVOT: Vec2 = Vec2::new(-20., 168.);
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    pub thrust: bool,
    pub turn: f32,
    pub slow: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Ready,
    Attached,
    Flying,
    Crashed,
    Landed,
}
#[derive(Clone, Copy, Debug)]
pub struct Pad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub multiplier: u32,
}
pub fn pads() -> Vec<Pad> {
    let mut out = vec![Pad {
        x: -500.,
        y: 32.,
        width: 640.,
        multiplier: 1,
    }];
    for sector in 0..12 {
        for (i, (width, multiplier)) in [(256., 2), (128., 4), (64., 8), (128., 4), (256., 2)]
            .iter()
            .enumerate()
        {
            let center = 140. + sector as f32 * 5000. + (i + 1) as f32 * 5000. / 6.;
            out.push(Pad {
                x: center - width / 2.,
                y: 16.,
                width: *width,
                multiplier: *multiplier,
            });
        }
    }
    out
}
#[derive(Clone, Copy, Debug)]
pub enum Event {
    Launch,
    Crash,
    Splash,
    Land,
    Bonus,
}
#[derive(Clone, Debug)]
pub struct Notice {
    pub id: u64,
    pub label: String,
    pub value: String,
    pub age: f32,
    pub kind: usize,
}
#[derive(Clone, Debug)]
pub struct Rocket {
    pub pos: Vec2,
    pub vel: Vec2,
    pub angle: f32,
    pub omega: f32,
    pub fuel: f32,
    pub legs: bool,
}
impl Rocket {
    pub fn up(&self) -> Vec2 {
        vec2(self.angle.sin(), self.angle.cos())
    }
    pub fn right(&self) -> Vec2 {
        vec2(self.angle.cos(), -self.angle.sin())
    }
    pub fn point(&self, p: Vec2) -> Vec2 {
        self.pos + self.right() * p.x + self.up() * p.y
    }
    pub fn extent_y(&self) -> f32 {
        let c = self.angle.cos();
        32. * self.angle.sin().abs() + if c >= 0. { 40. * c } else { -24. * c }
    }
}
pub struct Game {
    pub rocket: Rocket,
    pub phase: Phase,
    pub pads: Vec<Pad>,
    pub arm_angle: f32,
    pub arm_omega: f32,
    pub arm_pivot: Vec2,
    pub body_pos: Vec2,
    pub body_angle: f32,
    pub score: u32,
    pub best: u32,
    pub max_distance: f32,
    pub max_altitude: f32,
    pub flight_time: f32,
    pub elapsed: f32,
    pub end_time: f32,
    pub reason: String,
    pub notices: Vec<Notice>,
    pub events: Vec<Event>,
    pub slow: bool,
    pub active_thrust: bool,
    pub time_scale: f32,
    pub landing_multiplier: u32,
    pub landing_award: u32,
    physics: Physics,
    spin: f32,
    direction: i32,
    distance_awards: u32,
    altitude_awards: u32,
    speed_awards: u32,
    top_spin: u32,
    hit_pads: Vec<bool>,
    empty_awarded: bool,
    creative: bool,
    stable_time: f32,
    next_notice: u64,
}
impl Game {
    pub fn new(best: u32) -> Self {
        let physics = Physics::new();
        let state = physics.state(MAX_FUEL);
        let pads = pads();
        let n = pads.len();
        Self {
            rocket: state.rocket,
            phase: Phase::Ready,
            pads,
            arm_angle: state.arm_angle,
            arm_omega: 0.,
            arm_pivot: PIVOT,
            body_pos: state.body_pos,
            body_angle: 0.,
            score: 0,
            best,
            max_distance: 0.,
            max_altitude: 0.,
            flight_time: 0.,
            elapsed: 0.,
            end_time: 0.,
            reason: String::new(),
            notices: Vec::new(),
            events: Vec::new(),
            slow: false,
            active_thrust: false,
            time_scale: 1.,
            landing_multiplier: 1,
            landing_award: 0,
            physics,
            spin: 0.,
            direction: 0,
            distance_awards: 0,
            altitude_awards: 0,
            speed_awards: 0,
            top_spin: 0,
            hit_pads: vec![false; n],
            empty_awarded: false,
            creative: false,
            stable_time: 0.,
            next_notice: 0,
        }
    }
    pub fn restart(&mut self) {
        *self = Self::new(self.best);
    }
    pub fn finished(&self) -> bool {
        matches!(self.phase, Phase::Crashed | Phase::Landed)
    }
    /// Idempotent agent command: releasing again in free flight does nothing.
    pub fn release_if_attached(&mut self) {
        if matches!(self.phase, Phase::Ready | Phase::Attached) {
            self.space();
        }
    }
    /// Agent commands set a desired leg state rather than repeatedly toggling it.
    pub fn set_landing_legs(&mut self, deployed: bool) {
        if self.phase == Phase::Flying {
            self.rocket.legs = deployed;
            self.physics.set_legs(deployed);
        }
    }
    /// A bounded state vector for the CPU-only Gymnasium adapter.
    pub fn training_observation(&self) -> Vec<f32> {
        let s = self.physics.state(self.rocket.fuel);
        let r = &self.rocket;
        let b = |v: bool| if v { 1. } else { 0. };
        let mut out = vec![
            normalize(r.pos.x, 1000.),
            normalize(r.pos.y, 1000.),
            normalize(r.vel.x, 1000.),
            normalize(r.vel.y, 1000.),
            r.angle.sin(),
            r.angle.cos(),
            normalize(r.omega, 10.),
            r.fuel / MAX_FUEL,
            b(r.legs),
            b(self.phase == Phase::Ready),
            b(self.phase == Phase::Attached),
            b(self.phase == Phase::Flying),
            b(self.phase == Phase::Crashed),
            b(self.phase == Phase::Landed),
            normalize(self.body_pos.x - r.pos.x, 1000.),
            normalize(self.body_pos.y - r.pos.y, 1000.),
            normalize(s.body_velocity.x, 1000.),
            normalize(s.body_velocity.y, 1000.),
            self.body_angle.sin(),
            self.body_angle.cos(),
            normalize(s.body_omega, 10.),
            normalize(self.arm_pivot.x - r.pos.x, 1000.),
            normalize(self.arm_pivot.y - r.pos.y, 1000.),
            normalize(s.arm_velocity.x, 1000.),
            normalize(s.arm_velocity.y, 1000.),
            self.arm_angle.sin(),
            self.arm_angle.cos(),
            normalize(self.arm_omega, 10.),
            self.time_scale,
            normalize(self.elapsed, 60.),
            self.spin / TAU,
            self.direction as f32,
            normalize(self.distance_awards as f32, 10.),
            normalize(self.altitude_awards as f32, 10.),
            normalize(self.speed_awards as f32, 10.),
            normalize(self.top_spin as f32, 10.),
            b(self.empty_awarded),
            b(self.creative),
            (self.stable_time / 0.1).min(1.),
            normalize(self.score as f32, 10000.),
            normalize(self.max_distance, 1000.),
            normalize(self.max_altitude, 1000.),
        ];
        for (i, p) in self.pads.iter().enumerate() {
            let center = p.x + p.width / 2.;
            let angle = if i == 0 {
                0.
            } else {
                (self.elapsed * 100. + (center + 180.) * 2.)
                    .to_radians()
                    .sin()
                    .to_radians()
            };
            out.extend([
                normalize(center - r.pos.x, 1000.),
                normalize(p.y - r.pos.y, 1000.),
                normalize(p.width, 100.),
                angle.sin(),
                angle.cos(),
                p.multiplier as f32 / 8.,
                b(self.hit_pads[i]),
            ]);
        }
        out
    }
    pub fn training_observation_names() -> Vec<String> {
        let mut names = [
            "rocket_x",
            "rocket_y",
            "rocket_vx",
            "rocket_vy",
            "rocket_sin",
            "rocket_cos",
            "rocket_omega",
            "fuel_fraction",
            "legs_deployed",
            "phase_ready",
            "phase_attached",
            "phase_flying",
            "phase_crashed",
            "phase_landed",
            "base_relative_x",
            "base_relative_y",
            "base_vx",
            "base_vy",
            "base_sin",
            "base_cos",
            "base_omega",
            "arm_relative_x",
            "arm_relative_y",
            "arm_vx",
            "arm_vy",
            "arm_sin",
            "arm_cos",
            "arm_omega",
            "time_scale",
            "world_time",
            "spin_progress",
            "rotation_direction",
            "distance_awards",
            "altitude_awards",
            "speed_awards",
            "top_spin_awards",
            "empty_awarded",
            "creative_awarded",
            "landing_stability",
            "score",
            "max_distance",
            "max_altitude",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
        for i in 0..pads().len() {
            for field in [
                "relative_x",
                "relative_y",
                "width",
                "sin",
                "cos",
                "multiplier",
                "already_hit",
            ] {
                names.push(format!("platform_{i}_{field}"));
            }
        }
        names
    }
    pub fn space(&mut self) {
        match self.phase {
            Phase::Ready | Phase::Attached => {
                self.physics.release();
                self.phase = Phase::Flying;
                self.events.push(Event::Launch);
            }
            Phase::Flying => {
                self.rocket.legs = !self.rocket.legs;
                self.physics.set_legs(self.rocket.legs);
            }
            _ if self.end_time > 2. => self.restart(),
            _ => {}
        }
    }
    pub fn award(&mut self, label: &str, points: u32, _kind: usize) {
        self.score = self.score.saturating_add(points);
        self.best = self.best.max(self.score);
        self.notice(label, format!("+{}", points), points);
    }
    fn notice(&mut self, label: &str, value: String, points: u32) {
        self.next_notice += 1;
        self.notices.push(Notice {
            id: self.next_notice,
            label: label.into(),
            value,
            age: 0.,
            kind: if points >= 1000 {
                2
            } else if points >= 100 {
                1
            } else {
                0
            },
        });
        self.events.push(Event::Bonus);
    }
    pub fn update(&mut self, real_dt: f32, c: Controls) {
        let target = if c.slow { 1. / 16. } else { 1. };
        self.time_scale += (target - self.time_scale) / 16.;
        self.slow = c.slow;
        let dt = real_dt * self.time_scale;
        self.elapsed += dt;
        for n in &mut self.notices {
            n.age += dt;
        }
        self.notices.retain(|n| n.age < 5.);
        self.active_thrust = c.thrust && self.rocket.fuel > 0. && !self.finished();
        if self.phase == Phase::Ready && (c.thrust || c.turn != 0. || c.slow) {
            self.phase = Phase::Attached;
        }
        if self.finished() {
            self.end_time += dt;
            self.active_thrust = false;
            self.physics.step(
                STEP * self.time_scale,
                self.time_scale,
                Controls::default(),
                false,
                false,
                self.elapsed,
            );
            let state = self.physics.state(self.rocket.fuel);
            self.rocket = state.rocket;
            self.body_pos = state.body_pos;
            self.body_angle = state.body_angle;
            self.arm_pivot = state.arm_pivot;
            self.arm_angle = state.arm_angle;
            return;
        }
        if self.active_thrust {
            self.rocket.fuel = (self.rocket.fuel - real_dt * 1000.).max(0.);
        }
        if c.slow {
            self.rocket.fuel = (self.rocket.fuel - real_dt * 1000.).max(0.);
        }
        let old_vel = self.rocket.vel;
        let old_omega = self.rocket.omega;
        let physical_controls = Controls {
            turn: c.turn
                / if self.rocket.fuel > 0. {
                    self.time_scale
                } else {
                    1.
                },
            ..c
        };
        self.physics.step(
            STEP * self.time_scale,
            self.time_scale,
            physical_controls,
            self.active_thrust,
            true,
            self.elapsed,
        );
        let s = self.physics.state(self.rocket.fuel);
        self.rocket = s.rocket;
        self.arm_pivot = s.arm_pivot;
        self.arm_angle = s.arm_angle;
        self.arm_omega = s.arm_omega;
        self.body_pos = s.body_pos;
        self.body_angle = s.body_angle;
        if self.phase == Phase::Ready {
            return;
        }
        if self.phase == Phase::Flying {
            self.flight_time += dt;
        }
        let dir = if self.rocket.omega > 0.01 {
            1
        } else if self.rocket.omega < -0.01 {
            -1
        } else {
            0
        };
        if dir != 0 && dir != self.direction {
            self.spin = 0.;
            if self.direction != 0
                && self.phase == Phase::Flying
                && self.rocket.pos.y > self.rocket.extent_y()
            {
                self.award("Wobble", 10, 0);
            }
            self.direction = dir;
        }
        self.spin += self.rocket.omega * dt;
        if self.spin.abs() >= TAU {
            self.spin = 0.;
            self.award("Spin", 50, 0);
        }
        let top = (self.arm_omega.to_degrees().abs() / 768.) as u32;
        if top > self.top_spin {
            self.top_spin = top;
            self.award("Top Spin", 500, 1);
        }
        if !self.creative && self.body_pos.y < 64. {
            self.creative = true;
            self.award("Creative", 750, 1);
        }
        let distance = ((self.rocket.pos.x - 140.) / 5.).max(0.);
        let altitude = ((self.rocket.pos.y - self.rocket.extent_y()) / 5.).max(0.);
        self.max_distance = self.max_distance.max(distance);
        self.max_altitude = self.max_altitude.max(altitude);
        let d = (distance / 500.) as u32;
        if d > self.distance_awards {
            self.distance_awards = d;
            self.award("Distance", 500, 1);
        }
        let a = (altitude / 250.) as u32;
        if a > self.altitude_awards {
            self.altitude_awards = a;
            self.award("Altitude", 125, 1);
        }
        let speed = self.rocket.vel.length() / 5.;
        let fast = (speed / 500.) as u32;
        if fast > self.speed_awards {
            self.speed_awards = fast;
            self.award("Fast", 5000, 2);
        }
        if let Some(i) = s.contact_pad {
            let p = self.pads[i];
            if i > 0 && !self.hit_pads[i] {
                self.hit_pads[i] = true;
                self.award("Hit", p.multiplier * 1000, 2);
            }
            let impact = (self.rocket.vel - old_vel).length();
            if impact > 256. {
                self.crash("Too Fast", false);
                return;
            }
            if self.phase == Phase::Flying
                && (self.rocket.omega - old_omega).abs().to_degrees() > 64.
                && wrap_angle(self.rocket.angle).abs() > 45f32.to_radians()
            {
                self.crash("Tipped Over", false);
                return;
            }
            let pad_angle = if i == 0 {
                0.
            } else {
                (self.elapsed * 100. + (p.x + p.width / 2. + 180.) * 2.)
                    .to_radians()
                    .sin()
                    .to_radians()
            };
            if self.phase == Phase::Flying
                && self.rocket.legs
                && wrap_angle(self.rocket.angle - pad_angle).abs() < 0.012
                && self.rocket.vel.length() < 1.
                && self.rocket.omega.abs() < 0.1
            {
                self.stable_time += dt;
                if self.stable_time > 0.1 {
                    self.phase = Phase::Landed;
                    self.reason = "Landed".into();
                    self.active_thrust = false;
                    self.landing_multiplier = p.multiplier;
                    self.score = self.score.saturating_mul(p.multiplier);
                    self.landing_award = self.score;
                    self.best = self.best.max(self.score);
                    let label = match p.multiplier {
                        2 => "Nice",
                        4 => "Cool",
                        8 => "Great",
                        _ => "Landed",
                    };
                    self.notice(label, format!("*{}", p.multiplier), 1000);
                    self.events.push(Event::Land);
                    self.physics.freeze_rocket();
                }
            } else {
                self.stable_time = 0.;
            }
        } else {
            self.stable_time = 0.;
        }
        if self.rocket.pos.y <= 0. {
            self.crash("Submerged", true);
        } else if self.rocket.pos.x < -230. || self.rocket.pos.y < -142. {
            self.crash("Out Of Bounds", false);
        }
        if self.rocket.fuel <= 0. && !self.empty_awarded {
            self.empty_awarded = true;
            self.reason = "Out Of Fuel".into();
            self.award("Empty", 1000, 2);
            if self.phase == Phase::Attached {
                self.phase = Phase::Crashed;
            }
        }
    }
    fn crash(&mut self, reason: &str, water: bool) {
        self.phase = Phase::Crashed;
        self.reason = reason.into();
        self.active_thrust = false;
        self.best = self.best.max(self.score);
        self.events
            .push(if water { Event::Splash } else { Event::Crash });
        if !water {
            self.physics.freeze_rocket();
        }
    }
    #[cfg(test)]
    fn test_flight(&mut self, pos: Vec2, vel: Vec2, legs: bool) {
        self.phase = Phase::Flying;
        self.rocket.pos = pos;
        self.rocket.vel = vel;
        self.rocket.angle = 0.;
        self.rocket.omega = 0.;
        self.rocket.legs = legs;
        self.physics.set_rocket(&self.rocket);
    }
}
pub fn wrap_angle(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

/// Softsign preserves ordering and bounds an unbounded quantity without clipping.
pub fn normalize(value: f32, scale: f32) -> f32 {
    let x = value / scale;
    x / (1. + x.abs())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(g: &mut Game, seconds: f32, c: Controls) {
        for _ in 0..(seconds / STEP).round() as usize {
            g.update(STEP, c);
            g.events.clear();
        }
    }
    #[test]
    fn original_flying_mass() {
        let g = Game::new(0);
        assert!((g.physics.mass() - 1.0752).abs() < 0.0001);
    }
    #[test]
    fn gravity_matches_export() {
        let mut g = Game::new(0);
        g.test_flight(vec2(500., 2000.), Vec2::ZERO, false);
        run(&mut g, 1., Controls::default());
        assert!((g.rocket.vel.y + 200.).abs() < 0.1);
    }
    #[test]
    fn free_thrust_matches_original_force_and_mass() {
        let mut g = Game::new(0);
        g.test_flight(vec2(500., 2000.), Vec2::ZERO, false);
        run(
            &mut g,
            1.,
            Controls {
                thrust: true,
                ..Controls::default()
            },
        );
        let expected = (32. / 1.0752 - 8.) * 25.;
        assert!((g.rocket.vel.y - expected).abs() < 0.1);
    }
    #[test]
    fn classic_refresh_timing_keeps_fuel_on_real_time() {
        let mut normal = Game::new(0);
        let mut fast = Game::new(0);
        normal.test_flight(vec2(500., 4000.), Vec2::ZERO, false);
        fast.test_flight(vec2(500., 4000.), Vec2::ZERO, false);
        let c = Controls {
            thrust: true,
            ..Controls::default()
        };
        for _ in 0..60 {
            normal.update(1. / 60., c);
        }
        for _ in 0..120 {
            fast.update(1. / 120., c);
        }
        assert!((normal.rocket.fuel - fast.rocket.fuel).abs() < 1.);
        assert!((fast.rocket.vel.y - 2. * normal.rocket.vel.y).abs() < 0.2);
    }
    #[test]
    fn steering_preserves_momentum_without_damping() {
        let mut g = Game::new(0);
        g.test_flight(vec2(500., 2000.), Vec2::ZERO, false);
        run(
            &mut g,
            0.5,
            Controls {
                turn: 1.,
                ..Controls::default()
            },
        );
        let omega = g.rocket.omega;
        run(&mut g, 0.5, Controls::default());
        assert!((g.rocket.omega - omega).abs() < 0.0001);
        assert_eq!(g.rocket.fuel, MAX_FUEL);
    }
    #[test]
    fn thrust_and_slow_motion_use_real_time_fuel() {
        let mut g = Game::new(0);
        g.test_flight(vec2(500., 4000.), Vec2::ZERO, false);
        run(
            &mut g,
            1.,
            Controls {
                thrust: true,
                slow: true,
                ..Controls::default()
            },
        );
        assert!((g.rocket.fuel - 28000.).abs() < 1.);
        assert!(g.time_scale < 0.09);
    }
    #[test]
    fn empty_tank_still_allows_steering() {
        let mut g = Game::new(0);
        g.test_flight(vec2(500., 2000.), Vec2::ZERO, false);
        g.rocket.fuel = 0.;
        run(
            &mut g,
            0.2,
            Controls {
                thrust: true,
                turn: 1.,
                ..Controls::default()
            },
        );
        assert!(!g.active_thrust);
        assert!(g.rocket.omega > 0.1);
    }
    #[test]
    fn release_keeps_rigid_body_velocity() {
        let mut g = Game::new(0);
        run(
            &mut g,
            0.6,
            Controls {
                thrust: true,
                ..Controls::default()
            },
        );
        let v = g.rocket.vel;
        let omega = g.rocket.omega;
        g.space();
        assert_eq!(g.phase, Phase::Flying);
        assert_eq!(g.rocket.vel, v);
        assert_eq!(g.rocket.omega, omega);
    }
    #[test]
    fn safe_target_landing_multiplies_hit_score() {
        let mut g = Game::new(0);
        let p = g.pads[1];
        g.test_flight(vec2(p.x + p.width / 2., 70.), vec2(0., -8.), true);
        run(&mut g, 5., Controls::default());
        assert_eq!(g.phase, Phase::Landed);
        assert!(g.score >= 4000);
        assert_eq!(g.landing_multiplier, 2);
    }
    #[test]
    fn no_legs_does_not_receive_landing_multiplier() {
        let mut g = Game::new(0);
        let p = g.pads[1];
        g.test_flight(vec2(p.x + p.width / 2., 70.), Vec2::ZERO, false);
        run(&mut g, 2., Controls::default());
        assert_ne!(g.phase, Phase::Landed);
        assert_eq!(g.score, 2000);
    }
    #[test]
    fn high_speed_impact_is_caught() {
        let mut g = Game::new(0);
        let p = g.pads[1];
        g.test_flight(vec2(p.x + p.width / 2., 200.), vec2(0., -2000.), true);
        run(&mut g, 0.3, Controls::default());
        assert_eq!(g.phase, Phase::Crashed);
        assert_eq!(g.reason, "Too Fast");
    }
    #[test]
    fn restart_keeps_best() {
        let mut g = Game::new(0);
        g.award("Hit", 4000, 2);
        g.restart();
        assert_eq!(g.best, 4000);
        assert_eq!(g.score, 0);
        assert_eq!(g.phase, Phase::Ready);
    }
}
