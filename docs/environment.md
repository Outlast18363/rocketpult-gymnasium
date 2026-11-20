# Environment reference

[← Back to the README](../README.md)

## Actions

The default action space is `Discrete(24)`: thrust on/off × straight/left/right ×
keep/release/deploy/retract. Use the helper to avoid memorizing action numbers:

```python
from rocketpult_gymnasium import RocketpultEnv

env = RocketpultEnv()
obs, info = env.reset()
swing = env.encode_action(thrust=True)
release = env.encode_action(thrust=True, turn=-1, command="release")
deploy = env.encode_action(command="deploy")
obs, reward, terminated, truncated, info = env.step(swing)
```

| Field | Values |
| --- | --- |
| `thrust` | `False`, `True` |
| `turn` | `-1` left, `0` straight, `1` right |
| `command` | `"keep"`, `"release"`, `"deploy"`, `"retract"` |
| `slow` | Disabled by default; enable with `allow_slow_motion=True` |

Release only acts while attached; it cannot toggle legs. Deploy/retract set the
desired state while flying and are safe to repeat. Commands happen once before
the repeated physics ticks. Thrust and steering are held for all repeated ticks.
`env.action_names` lists every action.

For numeric encoders: `action = thrust + 2*turn_code + 6*command_code + 24*slow`,
where turn codes are straight=0, left=1, right=2, and command codes follow the
table's order. Slow motion enables `Discrete(48)` and keeps the game's fuel cost.

## Observations and reward

Observations are fresh `float32` arrays of shape `(474,)` in `Box(-1, 1)`:

| Group | Features |
| --- | --- |
| Rocket and game state (42) | Position, velocity, angle as sine/cosine, angular velocity, fuel, legs, phase, catapult base/arm state, timer, scoring counters, score and flight records |
| Platforms (61 × 7) | Relative position, width, angle sine/cosine, multiplier, whether already hit; platform 0 is the launch platform |
| Episode state (5) | Progress toward the step limit, virtual time, physics time, reference rate, frame skip |

`env.observation_names` gives the exact index-to-feature mapping. Positions,
velocities, counters and times use a smooth bounded transform:
`z = value / scale; observation = z / (1 + abs(z))`. Position/velocity scales
are 1000 game units, angular velocity 10, width 100, score 10000, counters 10,
and time 60 seconds. Fuel, flags, sine/cosine and fractions use their natural
bounds. This is a state observation environment; image observations and a Python
graphical renderer are not part of this first version. `render_mode="ansi"`
returns a short text status.

Reward is **the change in the original game score × `reward_scale`** (default
0.001). There is no added survival reward or new landing bonus. Landing retains
the game's score multiplier. `info` includes unscaled `score`, `score_delta`,
`is_success`, `phase`, `outcome`, fuel, raw position/velocity/angle, episode steps,
simulation ticks, clocks, cumulative return, time scale and landing multiplier.
Positions use the Rust game's coordinates: right/up are positive.

## Timing, reset, and configuration

```python
env = RocketpultEnv(
    reference_hz=120,
    frame_skip=4,
    max_episode_steps=1500,
    allow_slow_motion=False,
    reward_scale=0.001,
    initial_fuel_range=(1.0, 1.0),
    warmup_ticks=120,
)
```

The playable recreation advances Box2D by 1/60 second per display tick, while
fuel and game timers use elapsed display time. This environment preserves that
behavior with a **fixed virtual display clock**, default 120 Hz. Each decision
repeats four classic ticks: 1/30 second of virtual display time and 1/15 second of
Box2D time before slow motion. It never waits for a real display or wall clock.
`info["virtual_time"]` and `info["physics_time"]` expose both clocks. Changing
`reference_hz` changes the classic timing profile; keep it fixed across training
and evaluation. Changing `frame_skip` changes control frequency.

Reset creates a new physics world, clears scores and episode state, and settles
the idle catapult for `warmup_ticks` before the first observation. Warmup consumes
no episode steps or fuel. No saved high scores are read or written.

The default layout and initial fuel are fixed, so different seeds intentionally
give the same initial state. To randomize fuel reproducibly, use
`initial_fuel_range=(0.7, 1.0)`. `reset(seed=42)` seeds this sampling; subsequent
`reset()` calls continue the random sequence. Override a single episode with
`reset(options={"fuel_fraction": 0.8})`. The terrain and physics contain no added
randomness. Identical seeds and action sequences reproduce trajectories on the
same build/platform; cross-platform floating-point identity is not guaranteed.

The default horizon is 1500 decisions (50 seconds of virtual display time).
Limits: reference rate 30–240, frame skip 1–32, horizon 1–1000000, warmup 0–600.

## Standard process parallelism

No custom worker implementation is needed. The same single-instance class works
with Gymnasium's `SyncVectorEnv` and process-based `AsyncVectorEnv`:

```python
from functools import partial
from gymnasium.vector import AsyncVectorEnv
from rocketpult_gymnasium import RocketpultEnv

if __name__ == "__main__":
    envs = AsyncVectorEnv(
        [partial(RocketpultEnv, max_episode_steps=500) for _ in range(4)],
        context="spawn",
    )
    try:
        observations, infos = envs.reset(seed=42)
        observations, rewards, terminated, truncated, infos = envs.step(
            envs.action_space.sample()
        )
    finally:
        envs.close()
```

Run process examples from a Python file with the `__main__` guard. Each process
constructs its own Rust physics world. A native environment belongs to the
thread that created it; do not move a live instance between threads or pickle a
live world. Pass factories to process wrappers. No Rust native worker pool or
custom vector API is included. Vector wrappers handle batched outputs and their
own autoreset behavior; their `info` format differs from a single environment.
