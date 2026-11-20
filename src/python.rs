use crate::{
    environment::{Action, Config, Environment, Info},
    game::MAX_FUEL,
};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyDict,
};

type StepResult = (Vec<f32>, f64, bool, bool, Py<PyDict>);

/// A Box2D world stays on the creating thread. Standard process-based vector
/// wrappers construct one of these inside each worker; no Rust threads are used.
#[pyclass(unsendable, module = "rocketpult_gymnasium._native")]
struct NativeEnvironment {
    environment: Environment,
}

fn info_dict(py: Python<'_>, info: &Info) -> PyResult<Py<PyDict>> {
    let d = PyDict::new(py);
    d.set_item("score", info.score)?;
    d.set_item("score_delta", info.score_delta)?;
    d.set_item("is_success", info.is_success)?;
    d.set_item("phase", info.phase)?;
    d.set_item("outcome", &info.outcome)?;
    d.set_item("fuel", info.fuel)?;
    d.set_item("position", info.position)?;
    d.set_item("velocity", info.velocity)?;
    d.set_item("angle", info.angle)?;
    d.set_item("angular_velocity", info.angular_velocity)?;
    d.set_item("episode_steps", info.episode_steps)?;
    d.set_item("simulation_ticks", info.simulation_ticks)?;
    d.set_item("virtual_time", info.virtual_time)?;
    d.set_item("physics_time", info.physics_time)?;
    d.set_item("episode_return", info.episode_return)?;
    d.set_item("time_scale", info.time_scale)?;
    d.set_item("landing_multiplier", info.landing_multiplier)?;
    Ok(d.unbind())
}

#[pymethods]
impl NativeEnvironment {
    #[new]
    #[pyo3(signature=(reference_hz=120,frame_skip=4,max_episode_steps=1500,allow_slow_motion=false,reward_scale=0.001,warmup_ticks=120))]
    fn new(
        reference_hz: u32,
        frame_skip: u32,
        max_episode_steps: u32,
        allow_slow_motion: bool,
        reward_scale: f64,
        warmup_ticks: u32,
    ) -> PyResult<Self> {
        let config = Config {
            reference_hz,
            frame_skip,
            max_episode_steps,
            allow_slow_motion,
            reward_scale,
            warmup_ticks,
        };
        Ok(Self {
            environment: Environment::new(config).map_err(PyValueError::new_err)?,
        })
    }
    #[pyo3(signature=(initial_fuel=30000.0))]
    fn reset(&mut self, py: Python<'_>, initial_fuel: f32) -> PyResult<(Vec<f32>, Py<PyDict>)> {
        let (observation, info) = self
            .environment
            .reset(initial_fuel)
            .map_err(PyValueError::new_err)?;
        Ok((observation, info_dict(py, &info)?))
    }
    fn step(&mut self, py: Python<'_>, action: usize) -> PyResult<StepResult> {
        let t = self
            .environment
            .step(action)
            .map_err(PyRuntimeError::new_err)?;
        Ok((
            t.observation,
            t.reward,
            t.terminated,
            t.truncated,
            info_dict(py, &t.info)?,
        ))
    }
    #[getter]
    fn action_names(&self) -> Vec<String> {
        Action::names(self.environment.config.allow_slow_motion)
    }
    #[getter]
    fn observation_names(&self) -> Vec<String> {
        Environment::observation_names()
    }
}

#[pymodule(gil_used = true)]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeEnvironment>()?;
    m.add("MAX_FUEL", MAX_FUEL)?;
    m.add("OBSERVATION_VERSION", 1)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
