mod audio;
use _native::game;
mod render;
mod storage;

use game::{Controls, Game, Phase};
use macroquad::experimental::coroutines::start_coroutine;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "ROCKETPULT Classic — Rust recreation".into(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        window_resizable: true,
        sample_count: 1,
        platform: miniquad::conf::Platform {
            webgl_version: miniquad::conf::WebGLVersion::WebGL2,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    #[cfg(target_os = "macos")]
    let native_refresh_hz = configure_color_space();
    #[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
    let native_refresh_hz = 60.;
    let saved_best = storage::load();
    let mut game = Game::new(saved_best);
    let mut view = render::View::new();
    // Web browsers unlock audio on a key press. Draw the title before waiting
    // for that gesture, so a fresh tab never sits on a black screen.
    let mut sounds: Option<audio::Sounds> = None;
    let sound_loader = start_coroutine(audio::Sounds::new());
    let mut muted = false;
    #[cfg(not(target_arch = "wasm32"))]
    let mut accumulator = 0.0;
    let mut paused = false;
    let mut help = false;
    let mut reset_hold = 0.0;
    let mut fullscreen = false;
    let mut stored_best = saved_best;

    loop {
        let dt = get_frame_time().min(0.1);
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) {
            paused = !paused;
        }
        if is_key_pressed(KeyCode::H) {
            help = !help;
        }
        if is_key_pressed(KeyCode::M) {
            muted = !muted;
            if let Some(sounds) = &mut sounds {
                sounds.toggle();
            }
        }
        if is_key_pressed(KeyCode::F) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if is_key_pressed(KeyCode::R) {
            game.restart();
            view.reset();
            paused = false;
            help = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                accumulator = 0.0;
            }
        }
        if is_key_down(KeyCode::Backspace) {
            reset_hold += dt;
            if reset_hold >= 5.0 {
                game = Game::new(0);
                stored_best = 0;
                storage::save(0);
                view.reset();
                reset_hold = 0.0;
            }
        } else {
            reset_hold = 0.0;
        }
        let controls = Controls {
            thrust: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
            turn: (is_key_down(KeyCode::Right) || is_key_down(KeyCode::D)) as i32 as f32
                - (is_key_down(KeyCode::Left) || is_key_down(KeyCode::A)) as i32 as f32,
            slow: is_key_down(KeyCode::Z),
        };
        if sounds.is_none() {
            if let Some(mut loaded) = sound_loader.retrieve() {
                loaded.muted = muted;
                sounds = Some(loaded);
            }
        }
        if !paused && !help {
            if is_key_pressed(KeyCode::Space) {
                let was_finished = game.finished();
                game.space();
                if was_finished && game.phase == Phase::Ready {
                    view.reset();
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        accumulator = 0.0;
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                game.update(dt, controls);
                view.reference_frames = 1.;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let frame_dt = 1. / native_refresh_hz;
                accumulator += dt;
                while accumulator >= frame_dt {
                    game.update(frame_dt, controls);
                    accumulator -= frame_dt;
                }
                view.reference_frames = dt * native_refresh_hz;
            }
            view.turn = controls.turn;
            view.update(&mut game, dt);
            if let Some(sounds) = &mut sounds {
                sounds.update(&game, dt);
            }
            game.events.clear();
        } else {
            if let Some(sounds) = &mut sounds {
                sounds.quiet();
            }
        }
        if game.best != stored_best {
            stored_best = game.best;
            storage::save(stored_best);
        }
        view.draw(&game, paused, help, muted, reset_hold);
        next_frame().await;
    }
}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn configure_color_space() -> f32 {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    // Match the browser's sRGB canvas rather than the display's wider gamut.
    unsafe {
        let view = miniquad::window::apple_view();
        let window: *mut Object = msg_send![view, window];
        let space: *mut Object = msg_send![class!(NSColorSpace), sRGBColorSpace];
        let _: () = msg_send![window,setColorSpace:space];
        let screen: *mut Object = msg_send![window, screen];
        let fps: isize = msg_send![screen, maximumFramesPerSecond];
        (fps as f32).clamp(30., 240.)
    }
}
