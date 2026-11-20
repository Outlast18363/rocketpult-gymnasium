// The upstream profiling clock uses std::time::Instant, which is unavailable
// on wasm32-unknown-unknown. Profiling timings never affect the physics solver.
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[derive(Default)]
pub struct B2timer {
    #[cfg(not(target_arch = "wasm32"))]
    start: NativeStart,
}
#[cfg(not(target_arch = "wasm32"))]
struct NativeStart(Instant);
#[cfg(not(target_arch = "wasm32"))]
impl Default for NativeStart {fn default()->Self{Self(Instant::now())}}
impl B2timer {
    pub fn reset(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        { self.start.0 = Instant::now(); }
    }
    pub fn get_milliseconds(&self)->f32 {
        #[cfg(not(target_arch = "wasm32"))]
        { self.start.0.elapsed().as_secs_f32()*1000. }
        #[cfg(target_arch = "wasm32")]
        { 0. }
    }
    pub fn precise_time_ns(&self)->u64 {
        #[cfg(not(target_arch = "wasm32"))]
        { self.start.0.elapsed().as_nanos() as u64 }
        #[cfg(target_arch = "wasm32")]
        { 0 }
    }
}
