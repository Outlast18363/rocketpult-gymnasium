"""Single-instance Gymnasium API; use Gymnasium's standard vector wrappers."""
from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any

import gymnasium as gym
import numpy as np
from gymnasium import spaces

from . import _native


def _integer(value: Any, name: str, low: int, high: int) -> int:
    if isinstance(value, (bool, np.bool_)) or not isinstance(value, (int, np.integer)):
        raise ValueError(f"{name} must be an integer between {low} and {high}")
    result = int(value)
    if not low <= result <= high:
        raise ValueError(f"{name} must be between {low} and {high}")
    return result


def _fuel_fraction(value: Any) -> float:
    try:
        result = float(value)
    except (TypeError, ValueError, OverflowError) as exc:
        raise ValueError("fuel_fraction must be a finite number between 0 and 1") from exc
    if not math.isfinite(result) or not 0 <= result <= 1:
        raise ValueError("fuel_fraction must be a finite number between 0 and 1")
    return result


class RocketpultEnv(gym.Env[np.ndarray, int]):
    """Classic Rocketpult with bounded state observations and score-delta rewards.

    No browser, window, audio, display server, or GPU is needed. A native world
    belongs to its creating thread. For parallel execution, construct separate
    environments in ``gymnasium.vector.AsyncVectorEnv`` worker processes.

    Action = thrust + 2 * turn_code + 6 * command_code + 24 * slow.
    turn_code: straight=0, left=1, right=2.
    command_code: keep=0, release=1, deploy=2, retract=3.
    Use :meth:`encode_action` rather than remembering these numbers.
    """

    metadata = {"render_modes": ["ansi"], "render_fps": 30}

    def __init__(
        self,
        *,
        reference_hz: int = 120,
        frame_skip: int = 4,
        max_episode_steps: int = 1500,
        allow_slow_motion: bool = False,
        reward_scale: float = 0.001,
        initial_fuel_range: Sequence[float] = (1.0, 1.0),
        warmup_ticks: int = 120,
        render_mode: str | None = None,
    ) -> None:
        super().__init__()
        self.reference_hz = _integer(reference_hz, "reference_hz", 30, 240)
        self.frame_skip = _integer(frame_skip, "frame_skip", 1, 32)
        self.max_episode_steps = _integer(max_episode_steps, "max_episode_steps", 1, 1_000_000)
        self.warmup_ticks = _integer(warmup_ticks, "warmup_ticks", 0, 600)
        if not isinstance(allow_slow_motion, (bool, np.bool_)):
            raise ValueError("allow_slow_motion must be a boolean")
        self.allow_slow_motion = bool(allow_slow_motion)
        try:
            self.reward_scale = float(reward_scale)
        except (TypeError, ValueError, OverflowError) as exc:
            raise ValueError("reward_scale must be finite and positive") from exc
        if not math.isfinite(self.reward_scale) or self.reward_scale <= 0:
            raise ValueError("reward_scale must be finite and positive")
        if render_mode is not None and render_mode not in self.metadata["render_modes"]:
            raise ValueError("render_mode must be None or 'ansi'")
        try:
            if len(initial_fuel_range) != 2:
                raise ValueError
            low, high = (_fuel_fraction(x) for x in initial_fuel_range)
        except (TypeError, ValueError) as exc:
            raise ValueError("initial_fuel_range must contain two fractions with 0 <= low <= high <= 1") from exc
        if low > high:
            raise ValueError("initial_fuel_range must have low <= high")
        self.initial_fuel_range = (low, high)
        self.render_mode = render_mode
        self.metadata = {**type(self).metadata, "render_fps": max(1, round(self.reference_hz / self.frame_skip))}
        self._engine: _native.NativeEnvironment | None = _native.NativeEnvironment(
            reference_hz=self.reference_hz,
            frame_skip=self.frame_skip,
            max_episode_steps=self.max_episode_steps,
            allow_slow_motion=self.allow_slow_motion,
            reward_scale=self.reward_scale,
            warmup_ticks=self.warmup_ticks,
        )
        self.observation_names = tuple(self._engine.observation_names)
        self.action_names = tuple(self._engine.action_names)
        self.action_space = spaces.Discrete(len(self.action_names))
        self.observation_space = spaces.Box(-1.0, 1.0, shape=(len(self.observation_names),), dtype=np.float32)
        self._needs_reset = True
        self._last_info: dict[str, Any] | None = None

    def encode_action(
        self, *, thrust: bool = False, turn: int = 0, command: str = "keep", slow: bool = False
    ) -> int:
        """Create a valid discrete action using readable controls.

        Release, deploy, and retract are idempotent: holding the same action
        across decisions never toggles the landing legs accidentally.
        """
        if not isinstance(thrust, (bool, np.bool_)) or not isinstance(slow, (bool, np.bool_)):
            raise ValueError("thrust and slow must be booleans")
        turn = _integer(turn, "turn", -1, 1)
        commands = {"keep": 0, "release": 1, "deploy": 2, "retract": 3}
        if command not in commands:
            raise ValueError("command must be 'keep', 'release', 'deploy', or 'retract'")
        if slow and not self.allow_slow_motion:
            raise ValueError("Set allow_slow_motion=True to use slow-time actions")
        turn_code = {0: 0, -1: 1, 1: 2}[turn]
        return int(thrust) + 2 * turn_code + 6 * commands[command] + 24 * int(slow)

    def reset(
        self, *, seed: int | None = None, options: dict[str, Any] | None = None
    ) -> tuple[np.ndarray, dict[str, Any]]:
        if self._engine is None:
            raise RuntimeError("Environment is closed. Create a new instance.")
        super().reset(seed=seed)
        if options is None:
            options = {}
        if not isinstance(options, Mapping):
            raise ValueError("options must be a mapping or None")
        unknown = set(options) - {"fuel_fraction"}
        if unknown:
            raise ValueError(f"Unknown reset options: {sorted(unknown)}")
        if "fuel_fraction" in options:
            fraction = _fuel_fraction(options["fuel_fraction"])
        else:
            low, high = self.initial_fuel_range
            fraction = low if low == high else float(self.np_random.uniform(low, high))
        observation, info = self._engine.reset(initial_fuel=fraction * _native.MAX_FUEL)
        self._needs_reset = False
        self._last_info = info
        return np.asarray(observation, dtype=np.float32), info

    def step(self, action: int) -> tuple[np.ndarray, float, bool, bool, dict[str, Any]]:
        if self._engine is None:
            raise RuntimeError("Environment is closed. Create a new instance.")
        if self._needs_reset:
            raise gym.error.ResetNeeded("Call reset() before step() and after an episode ends.")
        if not self.action_space.contains(action):
            raise gym.error.InvalidAction(f"Expected {self.action_space}, received {action!r}")
        observation, reward, terminated, truncated, info = self._engine.step(int(action))
        self._needs_reset = terminated or truncated
        self._last_info = info
        return np.asarray(observation, dtype=np.float32), reward, terminated, truncated, info

    def render(self) -> str | None:
        if self.render_mode != "ansi":
            return None
        if self._last_info is None:
            return "Rocketpult: call reset() to start an episode."
        i = self._last_info
        return (
            f"Rocketpult | phase={i['phase']} | score={i['score']} | fuel={i['fuel']:.0f}\n"
            f"position=({i['position'][0]:.2f}, {i['position'][1]:.2f}) "
            f"velocity=({i['velocity'][0]:.2f}, {i['velocity'][1]:.2f})\n"
            f"step={i['episode_steps']}/{self.max_episode_steps} | outcome={i['outcome']}"
        )

    def close(self) -> None:
        self._engine = None
        self._needs_reset = True
