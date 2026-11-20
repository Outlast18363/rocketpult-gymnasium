//! Box2D rigid bodies built from the original exported collision polygons.
//! Positions here use original Construct coordinates × 0.02, with +Y downward.
use crate::game::{pads, Controls, Rocket};
use box2d_rs::{
    b2_body::*, b2_fixture::*, b2_joint::*, b2_math::*, b2_world::*, b2rs_common::UserDataType,
    joints::b2_revolute_joint::*, shapes::b2_polygon_shape::*,
};
use glam::{vec2, Vec2};
use std::{cell::RefCell, rc::Rc};

#[derive(Default, Clone)]
pub struct Data;
impl UserDataType for Data {
    type Fixture = usize;
    type Body = usize;
    type Joint = ();
}
pub struct Physics {
    world: B2worldPtr<Data>,
    rocket: BodyPtr<Data>,
    body: BodyPtr<Data>,
    arm: BodyPtr<Data>,
    release_joint: Option<B2jointPtr<Data>>,
    rocket_fixtures: Vec<FixturePtr<Data>>,
    platforms: Vec<BodyPtr<Data>>,
    legs: bool,
}

pub struct State {
    pub rocket: Rocket,
    pub body_pos: Vec2,
    pub body_angle: f32,
    pub body_velocity: Vec2,
    pub body_omega: f32,
    pub arm_pivot: Vec2,
    pub arm_angle: f32,
    pub arm_omega: f32,
    pub arm_velocity: Vec2,
    pub contact_pad: Option<usize>,
}
fn bvec(x: f32, y: f32) -> B2vec2 {
    B2vec2::new(x, y)
}
fn source(p: Vec2) -> B2vec2 {
    bvec((p.x + 180.) * 0.04, (796. - 2. * p.y) * 0.02)
}
fn rust(p: B2vec2) -> Vec2 {
    vec2(p.x / 0.04 - 180., (15.92 - p.y) / 0.04)
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics {
    pub fn new() -> Self {
        let world = B2world::new(bvec(0., 8.));
        let ground = make_body(&world, 0, bvec(0., 1080. * 0.02), 0., false);
        fixture(
            &ground,
            &[
                [-640., -348.],
                [624., -348.],
                [640., -332.],
                [640., 348.],
                [-640., 348.],
            ],
            1,
            0.01,
        );
        let body = make_body(&world, 100, bvec(320. * 0.02, 572. * 0.02), 0., true);
        fixture(
            &body,
            &[
                [0., -128.],
                [240., 112.],
                [224., 128.],
                [-224., 128.],
                [-240., 112.],
            ],
            4,
            0.,
        );
        let arm = make_body(
            &world,
            101,
            bvec(320. * 0.02, 460. * 0.02),
            std::f32::consts::FRAC_PI_2,
            true,
        );
        fixture(
            &arm,
            &[
                [0., -16.],
                [176., -16.],
                [192., 0.],
                [176., 16.],
                [0., 16.],
                [-16., 0.],
            ],
            4,
            0.,
        );
        let rocket = make_body(&world, 102, bvec(320. * 0.02, 636. * 0.02), 0., true);
        let rocket_fixtures = rocket_shapes(&rocket, false);
        let mut hinge = B2revoluteJointDef::default();
        hinge.initialize(body.clone(), arm.clone(), bvec(320. * 0.02, 460. * 0.02));
        world
            .borrow_mut()
            .create_joint(&B2JointDefEnum::RevoluteJoint(hinge));
        let mut lock = B2revoluteJointDef::default();
        lock.initialize(rocket.clone(), arm.clone(), bvec(320. * 0.02, 636. * 0.02));
        lock.enable_limit = true;
        let release_joint = Some(
            world
                .borrow_mut()
                .create_joint(&B2JointDefEnum::RevoluteJoint(lock)),
        );
        let mut platforms = Vec::new();
        for (i, pad) in pads().iter().enumerate().skip(1) {
            let b = make_body(
                &world,
                i,
                source(vec2(pad.x + pad.width / 2., pad.y - 16.)),
                0.,
                false,
            );
            let w = pad.width;
            fixture(
                &b,
                &[
                    [-w, -32.],
                    [w, -32.],
                    [w, 16.],
                    [w - 16., 32.],
                    [-w + 16., 32.],
                    [-w, 16.],
                ],
                1,
                0.01,
            );
            platforms.push(b);
        }
        Self {
            world,
            rocket,
            body,
            arm,
            release_joint,
            rocket_fixtures,
            platforms,
            legs: false,
        }
    }
    pub fn release(&mut self) {
        if let Some(j) = self.release_joint.take() {
            self.world.borrow_mut().destroy_joint(j);
        }
    }
    pub fn set_legs(&mut self, legs: bool) {
        if self.legs == legs {
            return;
        }
        for f in self.rocket_fixtures.drain(..) {
            B2body::destroy_fixture(self.rocket.clone(), f);
        }
        self.rocket_fixtures = rocket_shapes(&self.rocket, legs);
        self.legs = legs;
    }
    #[cfg(test)]
    pub fn set_rocket(&mut self, r: &Rocket) {
        self.release();
        self.set_legs(r.legs);
        let mut b = self.rocket.borrow_mut();
        b.set_transform(source(r.pos), r.angle - std::f32::consts::FRAC_PI_2);
        b.set_linear_velocity(bvec(r.vel.x * 0.04, -r.vel.y * 0.04));
        b.set_angular_velocity(r.omega);
    }
    pub fn step(
        &mut self,
        dt: f32,
        scale: f32,
        c: Controls,
        engine: bool,
        enabled: bool,
        time: f32,
    ) {
        if enabled {
            let angle = self.rocket.borrow().get_angle();
            let mut b = self.rocket.borrow_mut();
            // The original uses 4× thrust while attached, inverse-timescale
            // compensation in free flight, and no damping in air.
            if engine {
                let f = if self.release_joint.is_some() {
                    128.
                } else {
                    32. / scale
                };
                let point = b.get_world_point(bvec(-64. * 0.02, 0.));
                b.apply_force(bvec(angle.cos() * f, angle.sin() * f), point, true);
            }
            if self.release_joint.is_none() {
                b.apply_torque(c.turn * 128f32.to_radians(), true);
            }
            let water = rust(b.get_position()).y < 32.;
            b.set_linear_damping(if water { 1. } else { 0. });
            b.set_angular_damping(if water { 1. } else { 0. });
        }
        let all_pads = pads();
        for (i, b) in self.platforms.iter().enumerate() {
            let p = all_pads[i + 1];
            let angle = (time * 100. + (p.x + p.width / 2. + 180.) * 2.)
                .to_radians()
                .sin()
                .to_radians();
            let pos = b.borrow().get_position();
            b.borrow_mut().set_transform(pos, angle);
        }
        self.world.borrow_mut().step(dt, 8, 3);
        let (mut p, a) = {
            let b = self.body.borrow();
            (b.get_position(), b.get_angle())
        };
        if p.x < 6.4 {
            p.x = 6.4;
            self.body.borrow_mut().set_transform(p, a);
        }
    }
    pub fn state(&self, fuel: f32) -> State {
        let b = self.rocket.borrow();
        let v = b.get_linear_velocity();
        let mut contact_pad = None;
        for edge in b.get_contact_list().iter() {
            let e = edge.borrow();
            if e.contact
                .upgrade()
                .is_some_and(|c| c.borrow().get_base().is_touching())
            {
                let id = e
                    .other
                    .upgrade()
                    .and_then(|o| o.borrow().get_user_data())
                    .unwrap_or(100);
                if id < 100 {
                    contact_pad = Some(id);
                    break;
                }
            }
        }
        let body = self.body.borrow();
        let arm = self.arm.borrow();
        let body_v = body.get_linear_velocity();
        let arm_v = arm.get_linear_velocity();
        State {
            rocket: Rocket {
                pos: rust(b.get_position()),
                vel: vec2(v.x / 0.04, -v.y / 0.04),
                angle: b.get_angle() + std::f32::consts::FRAC_PI_2,
                omega: b.get_angular_velocity(),
                fuel,
                legs: self.legs,
            },
            body_pos: rust(body.get_position()),
            body_angle: body.get_angle(),
            body_velocity: vec2(body_v.x / 0.04, -body_v.y / 0.04),
            body_omega: body.get_angular_velocity(),
            arm_pivot: rust(arm.get_position()),
            arm_angle: -arm.get_angle(),
            arm_omega: -arm.get_angular_velocity(),
            arm_velocity: vec2(arm_v.x / 0.04, -arm_v.y / 0.04),
            contact_pad,
        }
    }
    pub fn freeze_rocket(&mut self) {
        B2body::set_enabled(self.rocket.clone(), false);
    }
    #[cfg(test)]
    pub fn mass(&self) -> f32 {
        self.rocket.borrow().get_mass()
    }
}
fn make_body(
    world: &B2worldPtr<Data>,
    id: usize,
    pos: B2vec2,
    angle: f32,
    dynamic: bool,
) -> BodyPtr<Data> {
    B2world::create_body(
        world.clone(),
        &B2bodyDef {
            position: pos,
            angle,
            user_data: Some(id),
            body_type: if dynamic {
                B2bodyType::B2DynamicBody
            } else {
                B2bodyType::B2StaticBody
            },
            ..Default::default()
        },
    )
}
fn fixture(
    body: &BodyPtr<Data>,
    points: &[[f32; 2]],
    category: u16,
    damping: f32,
) -> FixturePtr<Data> {
    body.borrow_mut().set_angular_damping(damping);
    let mut shape = B2polygonShape::default();
    shape.set(
        &points
            .iter()
            .map(|p| bvec(p[0] * 0.02, p[1] * 0.02))
            .collect::<Vec<_>>(),
    );
    B2body::create_fixture(
        body.clone(),
        &B2fixtureDef {
            shape: Some(Rc::new(RefCell::new(shape))),
            density: 1.,
            friction: 0.5,
            restitution: 0.2,
            filter: B2filter {
                category_bits: category,
                mask_bits: if category == 2 {
                    !4
                } else if category == 4 {
                    !2
                } else {
                    0xffff
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )
}
fn rocket_shapes(body: &BodyPtr<Data>, legs: bool) -> Vec<FixturePtr<Data>> {
    let mut out = vec![fixture(
        body,
        &[[-64., -12.], [48., -12.], [48., 12.], [-64., 12.]],
        2,
        0.,
    )];
    if legs {
        out.push(fixture(
            body,
            &[[-48., -12.], [-80., -48.], [48., -12.]],
            2,
            0.,
        ));
        out.push(fixture(
            body,
            &[[-48., 12.], [48., 12.], [-80., 48.]],
            2,
            0.,
        ));
    }
    out
}
