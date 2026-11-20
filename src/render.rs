//! Sprite rectangles, layout, fonts, camera and compositing follow the public
//! Construct export. See RENDERING.md for the source-to-Rust coordinate mapping.
use crate::game::*;
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, TAU};

const W: f32 = 960.0;
const H: f32 = 540.0;
const INK: Color = Color::new(0.145, 0.19, 0.21, 1.0);

#[derive(Clone, Copy)]
struct Frame {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    hx: f32,
    hy: f32,
}
impl Frame {
    const fn new(x: f32, y: f32, w: f32, h: f32, hx: f32, hy: f32) -> Self {
        Self { x, y, w, h, hx, hy }
    }
    fn rect(self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}
const SKY: Frame = Frame::new(1., 1., 480., 270., 0., 0.);
const LAND: Frame = Frame::new(483., 1., 320., 174., 0.5, 0.5);
const BODY: Frame = Frame::new(903., 75., 120., 64., 0.5, 0.5);
const ARM: Frame = Frame::new(961., 433., 52., 8., 4. / 52., 0.5);
const ROCKET: Frame = Frame::new(769., 321., 32., 32., 0.625, 0.5);
const LEGS: Frame = Frame::new(961., 449., 32., 32., 0.625, 0.5);
const LEG_A: Frame = Frame::new(681., 321., 32., 32., 0.625, 0.5);
const LEG_B: Frame = Frame::new(715., 321., 32., 32., 0.625, 0.5);
const FLAMES: [Frame; 2] = [
    Frame::new(993., 209., 24., 8., 1., 0.5),
    Frame::new(871., 177., 24., 8., 1., 0.5),
];
const JETS: [Frame; 2] = [
    Frame::new(993., 225., 24., 8., 0.5, 0.5),
    Frame::new(993., 241., 24., 8., 0.5, 0.5),
];
const BRONZE: Frame = Frame::new(769., 481., 128., 16., 0.5, 0.5);
const SILVER: Frame = Frame::new(805., 161., 64., 16., 0.5, 0.5);
const GOLD: Frame = Frame::new(897., 449., 32., 16., 0.5, 0.5);
const BUOY: Frame = Frame::new(871., 321., 48., 51., 0.5, 43. / 51.);
const WATER: Frame = Frame::new(1021., 69., 1., 1., 0., 0.);
const TELEMETRY: Frame = Frame::new(903., 1., 112., 72., 0.5, 0.5);
const SMALL_BONUS: Frame = Frame::new(897., 161., 64., 16., 0.5, 0.5);
const MEDIUM_BONUS: Frame = Frame::new(769., 193., 128., 32., 0.5, 0.5);
const LARGE_BONUS: Frame = Frame::new(805., 1., 96., 96., 0.5, 0.5);

#[derive(Clone, Copy)]
enum Font {
    LargeDark,
    LargeWhite,
    SmallWhite,
    SmallDark,
    LargeRed,
}
impl Font {
    fn spec(self) -> (f32, f32, f32, f32) {
        match self {
            Font::LargeDark => (1., 417., 8., 16.),
            Font::LargeWhite => (1., 449., 8., 16.),
            Font::LargeRed => (1., 481., 8., 16.),
            Font::SmallWhite => (513., 385., 4., 8.),
            Font::SmallDark => (513., 401., 4., 8.),
        }
    }
}

struct Particle {
    pos: Vec2,
    axis: Vec2,
    speed: f32,
    gravity_speed: f32,
    acceleration: f32,
    gravity: f32,
    age: f32,
    life: f32,
    size: f32,
    grow: f32,
    kind: u8,
}
struct Popup {
    label: String,
    value: String,
    kind: usize,
    pos: Vec2,
    angle: f32,
    age: f32,
    scale: f32,
    opacity: f32,
}

pub struct View {
    target: RenderTarget,
    atlas: Texture2D,
    clouds: Texture2D,
    cloud_material: Material,
    present_material: Material,
    camera: Vec2,
    scroll_x: f32,
    zoom: f32,
    telemetry: Vec2,
    particles: Vec<Particle>,
    popups: Vec<Popup>,
    last_notice: u64,
    trail_timer: f32,
    shake: f32,
    time: f32,
    seed: u32,
    legs_before: bool,
    legs_changed: f32,
    displayed_score: f32,
    helper_step: u8,
    pub turn: f32,
    pub reference_frames: f32,
}

impl View {
    pub fn new() -> Self {
        let target = render_target_ex(
            W as u32,
            H as u32,
            RenderTargetParams {
                sample_count: 1,
                ..Default::default()
            },
        );
        target.texture.set_filter(FilterMode::Nearest);
        let atlas = Texture2D::from_file_with_format(
            include_bytes!("../assets/reference/shared-0-sheet0.png"),
            Some(ImageFormat::Png),
        );
        atlas.set_filter(FilterMode::Nearest);
        let clouds = Texture2D::from_file_with_format(
            include_bytes!("../assets/reference/cloudtiles-sheet0.png"),
            Some(ImageFormat::Png),
        );
        clouds.set_filter(FilterMode::Nearest);
        let cloud_material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: CLOUD_FRAGMENT,
            },
            MaterialParams {
                pipeline_params: miniquad::PipelineParams {
                    color_blend: Some(miniquad::BlendState::new(
                        miniquad::Equation::Add,
                        miniquad::BlendFactor::One,
                        miniquad::BlendFactor::OneMinusValue(miniquad::BlendValue::SourceColor),
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("cloud screen-blend shader");
        let present_material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: PRESENT_FRAGMENT,
            },
            MaterialParams::default(),
        )
        .expect("opaque presentation shader");
        Self {
            target,
            atlas,
            clouds,
            cloud_material,
            present_material,
            camera: vec2(300., 128.),
            scroll_x: 960.,
            zoom: 1.,
            telemetry: vec2(800., 294.),
            particles: Vec::new(),
            popups: Vec::new(),
            last_notice: 0,
            trail_timer: 0.,
            shake: 0.,
            time: 0.,
            seed: 7281,
            legs_before: false,
            legs_changed: -1.,
            displayed_score: 0.,
            helper_step: 0,
            turn: 0.,
            reference_frames: 1.,
        }
    }
    pub fn reset(&mut self) {
        self.camera = vec2(300., 128.);
        self.scroll_x = 960.;
        self.zoom = 1.;
        self.telemetry = vec2(800., 294.);
        self.particles.clear();
        self.popups.clear();
        self.last_notice = 0;
        self.shake = 0.;
        self.legs_before = false;
        self.legs_changed = -1.;
        self.displayed_score = 0.;
        self.helper_step = 0;
    }
    fn random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 8) as f32 / 16_777_216.
    }
    fn emit(&mut self, pos: Vec2, vel: Vec2, kind: u8, size: f32, life: f32, grow: f32) {
        self.particles.push(Particle {
            pos,
            axis: vel.normalize_or_zero(),
            speed: vel.length(),
            gravity_speed: 0.,
            acceleration: if kind == 7 || (kind == 4 && life == 4.) {
                -64.
            } else if life == 4. {
                -128.
            } else {
                0.
            },
            gravity: if kind == 7 {
                -512.
            } else if life == 4. {
                256.
            } else {
                0.
            },
            age: 0.,
            life,
            size,
            grow,
            kind,
        });
    }
    pub fn update(&mut self, g: &mut Game, dt: f32) {
        let sim_dt = dt * g.time_scale;
        self.time += sim_dt;
        // Original event-sheet interpolation per classic display tick.
        let gain = 1. - 0.875f32.powf(self.reference_frames);
        let rocket_y = 796. - 2. * g.rocket.pos.y;
        let target_zoom = (540. / (540. - rocket_y.min(1080.)).abs()).clamp(0.25, 1.);
        self.zoom += (target_zoom - self.zoom) * gain;
        self.scroll_x += (2. * (g.rocket.pos.x + 180.) - self.scroll_x) * gain;
        let cx = self.scroll_x.max(960. / self.zoom);
        let cy = (rocket_y + 270. / self.zoom).min(1080. - 540. / self.zoom);
        self.camera = vec2(cx / 2. - 180., (796. - cy) / 2.);
        let rocket_screen = self.screen(g.rocket.pos);
        let tx = if rocket_screen.x < 640. { 800. } else { 480. };
        self.telemetry.x += (tx - self.telemetry.x) * gain;
        self.telemetry.y =
            (self.telemetry.y + (rocket_screen.y - self.telemetry.y) * gain).clamp(104., 294.);
        self.displayed_score += (g.score as f32 - self.displayed_score) * gain;
        let last_notice = self.last_notice;
        for n in g.notices.iter().filter(|n| n.id > last_notice) {
            // Original popup sizes, random placement bounds and ±5° rotation.
            let frame = match n.kind {
                0 => SMALL_BONUS,
                1 => MEDIUM_BONUS,
                _ => LARGE_BONUS,
            };
            let width = frame.w * 2.;
            let height = frame.h * 2.;
            let x = 32. + width / 2. + self.random() * (320. - width - 32.);
            let y = 160. + height / 2. + self.random() * (206. - height);
            let angle = (self.random() * 10. - 5.).to_radians();
            self.popups.push(Popup {
                label: n.label.clone(),
                value: n.value.clone(),
                kind: n.kind,
                pos: vec2(x, y),
                angle,
                age: 0.,
                scale: 3.,
                opacity: 1.,
            });
        }
        self.last_notice = g.notices.last().map_or(self.last_notice, |n| n.id);
        let obscures_rocket =
            rocket_screen.x + 32. * self.zoom >= 0. && rocket_screen.x - 32. * self.zoom <= 320.;
        for p in &mut self.popups {
            p.age += sim_dt;
            let wait = [1., 2., 4.][p.kind];
            p.scale += (if p.age < wait { 2. } else { 0. } - p.scale)
                * (1. - 0.5f32.powf(self.reference_frames));
            p.opacity += (if obscures_rocket { 0.25 } else { 1. } - p.opacity) * gain;
        }
        self.popups.retain(|p| p.age < [1., 2., 4.][p.kind] + 0.125);
        self.shake = (self.shake - dt * 16.).max(0.);
        if g.rocket.legs != self.legs_before {
            self.legs_before = g.rocket.legs;
            self.legs_changed = self.time;
        }
        if g.active_thrust && self.helper_step == 0 {
            self.helper_step = 1;
        }
        if g.slow && self.helper_step == 1 {
            self.helper_step = 2;
        }
        if g.phase == Phase::Flying && self.helper_step < 3 {
            self.helper_step = 3;
        }
        if g.phase == Phase::Flying && g.rocket.omega.abs() < 1.12 && self.helper_step == 3 {
            self.helper_step = 4;
        }
        if g.phase == Phase::Flying && g.rocket.vel.length() < 90. && self.helper_step == 4 {
            self.helper_step = 5;
        }
        if g.rocket.legs && self.helper_step == 5 {
            self.helper_step = 6;
        }
        if g.active_thrust || (g.rocket.fuel <= 0. && !g.finished()) {
            self.trail_timer += sim_dt;
            // Construct sets the smoke emission rate from the square root of speed.
            let interval = 1. / (g.rocket.vel.length() * 2.).sqrt().max(0.001);
            while self.trail_timer >= interval {
                self.trail_timer -= interval;
                let kind = if g.active_thrust { 0 } else { 4 };
                let vel = -g.rocket.up() * 0.000001;
                let size = 16. + self.random() * 32.;
                self.emit(g.rocket.point(vec2(0., -32.)), vel, kind, size, 16., -4.);
            }
        } else {
            self.trail_timer = 0.;
        }
        for event in &g.events {
            match event {
                Event::Crash => {
                    self.shake = 16.;
                    // Four original particle emitters: charcoal, orange, yellow, white.
                    for (kind, count, base_size, grow) in [
                        (4, 128, 128., -64.),
                        (5, 128, 64., -32.),
                        (6, 96, 64., -64.),
                        (0, 64, 32., -64.),
                    ] {
                        for _ in 0..count {
                            let angle = self.random() * TAU;
                            let speed = 64. + self.random() * 128.;
                            let size = base_size * (0.5 + self.random());
                            self.emit(
                                g.rocket.pos,
                                vec2(angle.cos(), angle.sin()) * speed,
                                kind,
                                size,
                                4.,
                                grow,
                            );
                        }
                    }
                }
                Event::Splash => {
                    self.shake = 4.;
                    for _ in 0..32 {
                        let angle = (-g.rocket.vel.y).atan2(g.rocket.vel.x)
                            + (self.random() - 0.5) * 45f32.to_radians();
                        let vel = vec2(angle.cos(), angle.sin()) * (192. + self.random() * 128.);
                        let size = 16. + self.random() * 32.;
                        self.emit(vec2(g.rocket.pos.x, 0.), vel, 7, size, 8., -16.);
                    }
                }
                _ => {}
            }
        }
        let mut particle_seed = self.seed;
        for p in &mut self.particles {
            p.age += sim_dt;
            p.pos += p.axis * p.speed * sim_dt + vec2(0., p.gravity_speed * sim_dt);
            p.size += p.grow * sim_dt;
            p.speed += p.acceleration * sim_dt;
            p.gravity_speed += p.gravity * sim_dt;
            if p.life == 16. {
                particle_seed = particle_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = (particle_seed >> 8) as f32 / 16_777_216. - 0.5;
                p.speed += noise * 64. * sim_dt;
            }
        }
        self.seed = particle_seed;
        self.particles
            .retain(|p| p.age < p.life && p.size > 0.5 && p.pos.y > -600.);
        if self.particles.len() > 1200 {
            self.particles.drain(0..self.particles.len() - 1200);
        }
    }
    fn screen(&self, p: Vec2) -> Vec2 {
        vec2(
            W * 0.5 + (p.x - self.camera.x) * self.zoom,
            H * 0.5 - (p.y - self.camera.y) * self.zoom,
        )
    }
    fn sprite(&self, frame: Frame, pos: Vec2, size: Vec2, angle: f32, alpha: f32) {
        draw_texture_ex(
            &self.atlas,
            pos.x - frame.hx * size.x,
            pos.y - frame.hy * size.y,
            Color::new(1., 1., 1., alpha),
            DrawTextureParams {
                source: Some(frame.rect()),
                dest_size: Some(size),
                rotation: angle,
                pivot: Some(pos),
                ..Default::default()
            },
        );
    }
    fn world_sprite(&self, frame: Frame, pos: Vec2, size: Vec2, angle: f32, alpha: f32) {
        self.sprite(frame, self.screen(pos), size * self.zoom, angle, alpha);
    }
    fn label(&self, s: &str, pos: Vec2, font: Font, scale: f32, align: f32, alpha: f32) {
        let (sx, sy, cw, ch) = font.spec();
        let lines: Vec<_> = s.split('\n').collect();
        for (row, line) in lines.iter().enumerate() {
            let left = pos.x - line.len() as f32 * cw * scale * align;
            for (i, c) in line.chars().enumerate() {
                let code = c as u32;
                if !(32..=126).contains(&code) {
                    continue;
                }
                let frame = Frame::new(sx + (code - 32) as f32 * cw, sy, cw, ch, 0., 0.);
                self.sprite(
                    frame,
                    vec2(
                        left + i as f32 * cw * scale,
                        pos.y + row as f32 * ch * scale,
                    ),
                    vec2(cw, ch) * scale,
                    0.,
                    alpha,
                );
            }
        }
    }
    fn draw_world(&self, g: &Game) {
        self.sprite(SKY, Vec2::ZERO, vec2(W, H), 0., 1.);
        let sea = self.screen(vec2(0., 0.)).y;
        // The water is drawn twice. The foreground pass at 50% also tints land,
        // rocket debris and the lower half of floating platforms.
        if sea < H {
            self.sprite(WATER, vec2(0., sea), vec2(W, H - sea), 0., 1.);
        }
        self.world_sprite(LAND, vec2(-180., -142.), vec2(640., 348.), 0., 1.);
        for p in g.pads.iter().skip(1) {
            let center = vec2(p.x + p.width / 2., 0.);
            let screen = self.screen(center);
            if screen.x + p.width * self.zoom / 2. < -30.
                || screen.x - p.width * self.zoom / 2. > W + 30.
            {
                continue;
            }
            let frame = match p.multiplier {
                2 => BRONZE,
                4 => SILVER,
                _ => GOLD,
            };
            let angle = (self.time * 100. + (center.x + 180.) * 2.)
                .to_radians()
                .sin()
                .to_radians();
            self.world_sprite(frame, center, vec2(frame.w, frame.h) * 2., angle, 1.);
        }
        for i in 1..14 {
            let x = 140. + i as f32 * 5000.;
            let screen = self.screen(vec2(x, 0.));
            if screen.x < -100. || screen.x > W + 100. {
                continue;
            }
            self.world_sprite(BUOY, vec2(x, 0.), vec2(96., 102.), 0., 1.);
            self.label(
                &format!("{}m", i * 1000),
                self.screen(vec2(x, 48.)),
                Font::LargeRed,
                2. * self.zoom,
                0.5,
                1.,
            );
        }
        self.world_sprite(BODY, g.body_pos, vec2(240., 128.), g.body_angle, 1.);
        self.world_sprite(ARM, g.arm_pivot, vec2(104., 16.), -g.arm_angle, 1.);
        for p in &self.particles {
            let sx = match p.kind {
                0 => 483.,
                4 => 935.,
                5 => 615.,
                6 => 549.,
                7 => 805.,
                _ => 483.,
            };
            let frame = Frame::new(sx, 257., 64., 64., 0.5, 0.5);
            self.world_sprite(frame, p.pos, Vec2::splat(p.size), 0., 1.);
        }
        if !(g.phase == Phase::Crashed && matches!(g.reason.as_str(), "Too Fast" | "Tipped Over")) {
            self.draw_rocket(g);
        }
        if sea < H {
            self.sprite(WATER, vec2(0., sea), vec2(W, H - sea), 0., 0.5);
        }
        let camera_original_y = 796. - 2. * self.camera.y;
        let opacity = 0.0625 * (-camera_original_y).max(0.) / (1080. * 8.);
        if opacity > 0.0001 {
            // Screen blend, exactly as the CloudTiles layer effect specifies.
            gl_use_material(&self.cloud_material);
            let size = 2048. * self.zoom;
            let offset = self.screen(vec2(-180., 398.));
            let ox = offset.x.rem_euclid(size) - size;
            let oy = offset.y.rem_euclid(size) - size;
            for x in 0..3 {
                for y in 0..3 {
                    draw_texture_ex(
                        &self.clouds,
                        ox + x as f32 * size,
                        oy + y as f32 * size,
                        Color::new(1., 1., 1., opacity),
                        DrawTextureParams {
                            dest_size: Some(Vec2::splat(size)),
                            ..Default::default()
                        },
                    );
                }
            }
            gl_use_default_material();
        }
    }
    fn draw_rocket(&self, g: &Game) {
        let angle = g.rocket.angle - FRAC_PI_2;
        let age = self.time - self.legs_changed;
        let frame = if age < 1. / 30. && self.legs_changed >= 0. {
            if (age * 60.) as i32 == 0 {
                LEG_A
            } else {
                LEG_B
            }
        } else if g.rocket.legs {
            LEGS
        } else {
            ROCKET
        };
        if g.active_thrust {
            self.world_sprite(
                FLAMES[(self.time * 30.) as usize % 2],
                g.rocket.point(vec2(0., -32.)),
                vec2(48., 16.),
                angle,
                1.,
            );
        }
        if self.turn != 0. && !g.finished() && g.rocket.fuel > 0. {
            let local = vec2(if self.turn > 0. { -22. } else { 22. }, 16.);
            self.world_sprite(
                JETS[(self.time * 30.) as usize % 2],
                g.rocket.point(local),
                vec2(48., 16.),
                angle
                    + if self.turn > 0. {
                        FRAC_PI_2
                    } else {
                        -FRAC_PI_2
                    },
                1.,
            );
        }
        self.world_sprite(frame, g.rocket.pos, vec2(64., 64.), angle, 1.);
    }
    #[allow(clippy::too_many_arguments)]
    fn popup_label(
        &self,
        s: &str,
        p: &Popup,
        offset: Vec2,
        font: Font,
        scale: f32,
        align: f32,
        alpha: f32,
    ) {
        let (sx, sy, cw, ch) = font.spec();
        let rot = Mat2::from_angle(p.angle);
        for (i, c) in s.chars().enumerate() {
            let code = c as u32;
            if !(32..=126).contains(&code) {
                continue;
            }
            let local = offset + vec2((i as f32 - s.len() as f32 * align) * cw * scale, 0.);
            self.sprite(
                Frame::new(sx + (code - 32) as f32 * cw, sy, cw, ch, 0., 0.),
                p.pos + rot * local,
                vec2(cw, ch) * scale,
                p.angle,
                alpha,
            );
        }
    }
    fn telemetry(&self, g: &Game) {
        self.sprite(TELEMETRY, self.telemetry, vec2(224., 144.), 0., 1.);
        for (offset, value) in [
            (-56., ((g.rocket.pos.x - 140.) / 5.).max(0.).floor() as i32),
            (-40., ((g.rocket.pos.y - 24.) / 5.).max(0.).floor() as i32),
            (
                -24.,
                (g.rocket.angle - FRAC_PI_2)
                    .to_degrees()
                    .rem_euclid(360.)
                    .floor() as i32,
            ),
            (8., (g.rocket.vel.length() / 5.).floor() as i32),
            (24., g.rocket.omega.to_degrees().floor() as i32),
            (56., g.rocket.fuel.ceil() as i32),
        ] {
            self.label(
                &value.to_string(),
                self.telemetry + vec2(104., offset - 8.),
                Font::SmallWhite,
                2.,
                1.,
                1.,
            );
        }
        if g.rocket.fuel < 7500. && (self.time * 4.) as i32 % 2 == 0 {
            self.sprite(
                Frame::new(1009., 417., 9., 8., 4. / 9., 0.5),
                self.telemetry + vec2(-64., 56.),
                vec2(18., 16.),
                0.,
                1.,
            );
        }
    }
    fn hud(&self, g: &Game) {
        if g.phase == Phase::Ready {
            self.label("ROCKETPULT", vec2(480., 148.), Font::LargeDark, 4., 0.5, 1.);
            self.label(
                "lf.itch.io // 1.0.1",
                vec2(480., 220.),
                Font::SmallDark,
                2.,
                0.5,
                1.,
            );
        } else {
            self.label(
                &format!("{:08}", self.displayed_score.ceil() as u32),
                vec2(32., 32.),
                Font::LargeDark,
                4.,
                0.,
                1.,
            );
            let best = if g.score > 0 && g.score >= g.best {
                "NEW BEST".to_string()
            } else {
                format!("BEST {:08}", g.best)
            };
            self.label(&best, vec2(32., 96.), Font::LargeDark, 2., 0., 1.);
            if g.reason.is_empty() && !g.finished() {
                self.telemetry(g);
            }
        }
        for p in &self.popups {
            let frame = match p.kind {
                0 => SMALL_BONUS,
                1 => MEDIUM_BONUS,
                _ => LARGE_BONUS,
            };
            self.sprite(
                frame,
                p.pos,
                vec2(frame.w, frame.h) * p.scale,
                p.angle,
                p.opacity,
            );
            let wait = [1., 2., 4.][p.kind];
            let alpha = (p.age / 0.125).min(1.) * ((wait - p.age) / 0.125).clamp(0., 1.);
            match p.kind {
                0 => {
                    self.popup_label(
                        &p.label,
                        p,
                        vec2(-56., -8.),
                        Font::SmallWhite,
                        2.,
                        0.,
                        alpha,
                    );
                    self.popup_label(&p.value, p, vec2(56., -8.), Font::SmallWhite, 2., 1., alpha);
                }
                1 => {
                    self.popup_label(
                        &p.label,
                        p,
                        vec2(-120., -16.),
                        Font::LargeWhite,
                        2.,
                        0.,
                        alpha,
                    );
                    self.popup_label(
                        &p.value,
                        p,
                        vec2(120., -16.),
                        Font::LargeWhite,
                        2.,
                        1.,
                        alpha,
                    );
                }
                _ => {
                    self.popup_label(&p.label, p, vec2(0., -64.), Font::LargeDark, 4., 0.5, alpha);
                    self.popup_label(&p.value, p, Vec2::ZERO, Font::LargeDark, 4., 0.5, alpha);
                }
            }
        }
        if !g.reason.is_empty() {
            self.label(&g.reason, vec2(480., 148.), Font::LargeDark, 4., 0.5, 1.);
            if g.phase == Phase::Landed {
                self.label(
                    &format!("X{} // {}", g.landing_multiplier, g.score),
                    vec2(480., 220.),
                    Font::LargeDark,
                    2.,
                    0.5,
                    1.,
                );
            }
        }
        if !g.finished() || g.end_time > 2. {
            let hint = if g.finished() {
                "Press SPACE to restart"
            } else {
                match self.helper_step {
                    0 => "Hold UP ARROW to thrust",
                    1 => "Hold Z to slow time\n(This drains your fuel!)",
                    2 => "Press SPACE to launch",
                    3 => "Hold LEFT/RIGHT ARROWS to rotate",
                    4 => "Thrust in the Opposite Direction to Slow Down",
                    5 => "Press SPACE to deploy landing legs",
                    _ => "Land on a platform to multiply your score",
                }
            };
            let rows = hint.lines().count() as f32;
            let font = if self.screen(Vec2::ZERO).y > 469. {
                Font::SmallDark
            } else {
                Font::SmallWhite
            };
            self.label(hint, vec2(480., 469. - rows * 8.), font, 2., 0.5, 1.);
        }
    }
    pub fn draw(&self, g: &Game, paused: bool, help: bool, _muted: bool, reset_hold: f32) {
        let shake = vec2((self.time * 89.).sin(), (self.time * 117.).cos()) * self.shake;
        set_camera(&Camera2D {
            render_target: Some(self.target.clone()),
            ..Camera2D::from_display_rect(Rect::new(-shake.x, -shake.y, W, H))
        });
        clear_background(BLACK);
        self.draw_world(g);
        set_camera(&Camera2D {
            render_target: Some(self.target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0., 0., W, H))
        });
        self.hud(g);
        if paused || help {
            draw_rectangle(0., 0., W, H, Color::new(0.08, 0.12, 0.14, 0.85));
            if help {
                self.label(
                    "FLIGHT MANUAL",
                    vec2(480., 70.),
                    Font::LargeWhite,
                    3.,
                    0.5,
                    1.,
                );
                for (i, line) in [
                    "UP / W       Thrust forward",
                    "LEFT / A     Rotate counterclockwise",
                    "RIGHT / D    Rotate clockwise",
                    "SPACE        Release / toggle landing legs",
                    "Z            Slow time (uses fuel)",
                    "R            Restart flight",
                    "P / ESC      Pause / resume",
                    "M / F        Sound / fullscreen",
                ]
                .iter()
                .enumerate()
                {
                    self.label(
                        line,
                        vec2(160., 150. + i as f32 * 28.),
                        Font::SmallWhite,
                        2.,
                        0.,
                        1.,
                    );
                }
                self.label(
                    "H  Return to game",
                    vec2(480., 470.),
                    Font::SmallWhite,
                    2.,
                    0.5,
                    1.,
                );
            } else {
                self.label("PAUSED", vec2(480., 200.), Font::LargeWhite, 4., 0.5, 1.);
                self.label(
                    "P / ESC  Resume     H  Controls",
                    vec2(480., 300.),
                    Font::SmallWhite,
                    2.,
                    0.5,
                    1.,
                );
            }
        }
        if reset_hold > 0. {
            self.label(
                "Hold BACKSPACE to reset everything",
                vec2(32., 492.),
                Font::SmallWhite,
                2.,
                0.,
                1.,
            );
            draw_rectangle(32., 514., 272. * reset_hold / 5., 4., INK);
        }
        set_default_camera();
        clear_background(BLACK);
        let scale = (screen_width() / W).min(screen_height() / H);
        let size = vec2(W, H) * scale;
        // The original canvas is opaque. Ignore the intermediate render target's
        // accumulated alpha when presenting it, preserving half-opacity water RGB.
        gl_use_material(&self.present_material);
        draw_texture_ex(
            &self.target.texture,
            (screen_width() - size.x) / 2.,
            (screen_height() - size.y) / 2.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                flip_y: true,
                ..Default::default()
            },
        );
        gl_use_default_material();
    }
}

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main(){gl_Position=Projection*Model*vec4(position,1.0);uv=texcoord;color=color0/255.0;}
"#;
const CLOUD_FRAGMENT: &str = r#"#version 100
precision lowp float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main(){vec4 pixel=texture2D(Texture,uv);gl_FragColor=vec4(pixel.rgb*color.a,1.0);}
"#;
const PRESENT_FRAGMENT: &str = r#"#version 100
precision lowp float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main(){gl_FragColor=vec4(texture2D(Texture,uv).rgb,1.0);}
"#;
