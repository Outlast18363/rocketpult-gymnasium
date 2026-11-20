"""Exercise the public API against the real Rust simulation (no mocked physics)."""
from functools import partial

import gymnasium as gym
import numpy as np
import pytest
from gymnasium.utils.env_checker import check_env, data_equivalence
from gymnasium.vector import AsyncVectorEnv, AutoresetMode, SyncVectorEnv

from rocketpult_gymnasium import RocketpultEnv


@pytest.mark.parametrize("slow", [False, True])
def test_registered_environment_passes_gymnasium_checker(slow):
    with gym.make("Rocketpult-v0", allow_slow_motion=slow) as env:
        check_env(env.unwrapped)
        assert env.action_space.n == (48 if slow else 24)
        obs, _ = env.reset(seed=42)
        assert obs.shape == (474,)
        assert obs.dtype == np.float32
        assert len(set(env.unwrapped.observation_names)) == 474


def test_seeded_trajectories_and_owned_observations():
    settings = dict(initial_fuel_range=(0.7, 1.0))
    with RocketpultEnv(**settings) as a, RocketpultEnv(**settings) as b:
        first, ia = a.reset(seed=123)
        other, ib = b.reset(seed=123)
        np.testing.assert_array_equal(first, other)
        assert ia == ib
        saved = first.copy()
        # Swing, then release and coast: this trace earns bonuses and crashes.
        for action in [1] * 30 + [7] + [0] * 200:
            ta, tb = a.step(action), b.step(action)
            assert data_equivalence(ta, tb, exact=True)
            assert a.observation_space.contains(ta[0])
            assert not np.shares_memory(first, ta[0])
            if ta[2] or ta[3]:
                break
        np.testing.assert_array_equal(first, saved)
        reset, info = a.reset(seed=123)
        np.testing.assert_array_equal(first, reset)
        assert info["score"] == 0
        assert info["episode_steps"] == 0
        # A subsequent unseeded reset continues the seeded random sequence.
        _, next_info = a.reset()
        assert next_info["fuel"] != info["fuel"]
        _, different = a.reset(seed=456)
        assert different["fuel"] != info["fuel"]
        _, override = a.reset(options={"fuel_fraction": 0.5})
        assert override["fuel"] == 15000.0


def test_horizon_is_truncated_and_reset_is_required():
    with RocketpultEnv(max_episode_steps=2) as e:
        with pytest.raises(gym.error.ResetNeeded):
            e.step(0)
        e.reset()
        assert e.step(0)[2:4] == (False, False)
        _, reward, terminated, truncated, info = e.step(0)
        assert not terminated and truncated
        assert info["outcome"] == "time_limit"
        assert info["episode_steps"] == 2
        with pytest.raises(gym.error.ResetNeeded):
            e.step(0)
        e.reset()
        assert e.step(0)[2:4] == (False, False)


def test_natural_termination_keeps_terminal_observation_and_score_reward():
    with RocketpultEnv(reward_scale=0.01) as e:
        obs, info = e.reset()
        total = 0.0
        previous_score = 0
        for t in range(300):
            action = 1 if t < 30 else (7 if t == 30 else 0)
            obs, reward, terminated, truncated, info = e.step(action)
            assert reward == pytest.approx((info["score"] - previous_score) * 0.01)
            assert info["score_delta"] == info["score"] - previous_score
            previous_score = info["score"]
            total += reward
            if terminated or truncated:
                break
        assert terminated and not truncated
        assert info["phase"] == "crashed"
        assert not info["is_success"]
        assert info["outcome"] != "time_limit"
        assert info["score"] > 0  # This trace exercises actual score events.
        assert total == pytest.approx(info["score"] * 0.01)
        assert info["episode_return"] == pytest.approx(total)
        assert e.observation_space.contains(obs)
        assert obs[e.observation_names.index("phase_crashed")] == 1
        with pytest.raises(gym.error.ResetNeeded):
            e.step(0)


def test_fixed_clock_fuel_and_optional_slow_motion():
    with RocketpultEnv() as e:
        e.reset()
        _, _, _, _, i = e.step(e.encode_action(thrust=True))
        assert i["simulation_ticks"] == 4
        assert i["virtual_time"] == pytest.approx(4 / 120)
        assert i["physics_time"] == pytest.approx(4 / 60)
        assert i["fuel"] == pytest.approx(30000 - 1000 * 4 / 120, abs=0.02)
        with pytest.raises(ValueError, match="allow_slow_motion"):
            e.encode_action(slow=True)
    with RocketpultEnv(allow_slow_motion=True) as e:
        e.reset()
        action = e.encode_action(thrust=True, slow=True)
        assert action == 25 and e.action_space.contains(action)
        _, _, _, _, i = e.step(action)
        assert i["fuel"] == pytest.approx(30000 - 2000 * 4 / 120, abs=0.02)
        assert i["physics_time"] < 4 / 60
        assert i["virtual_time"] == pytest.approx(4 / 120)


def test_repeated_commands_set_leg_state_without_toggling():
    with RocketpultEnv(frame_skip=1) as e:
        e.reset()
        legs = e.observation_names.index("legs_deployed")
        e.step(e.encode_action(command="release"))
        assert e.step(e.encode_action(command="deploy"))[0][legs] == 1
        assert e.step(e.encode_action(command="deploy"))[0][legs] == 1
        assert e.step(e.encode_action(command="release"))[0][legs] == 1
        assert e.step(e.encode_action(command="retract"))[0][legs] == 0


def test_invalid_input_does_not_advance_or_destroy_the_episode():
    with RocketpultEnv() as e:
        e.reset()
        for bad in [-1, 24, 1.5, [1], "1"]:
            with pytest.raises(gym.error.InvalidAction):
                e.step(bad)
        for options in [{"unknown": 1}, {"fuel_fraction": float("nan")}, {"fuel_fraction": -0.1}, []]:
            with pytest.raises(ValueError):
                e.reset(options=options)
        assert e.step(0)[4]["episode_steps"] == 1
    for options in [dict(frame_skip=0), dict(frame_skip=True), dict(reward_scale=float("inf")),
                    dict(initial_fuel_range=(0.9, 0.1)), dict(render_mode="rgb_array")]:
        with pytest.raises(ValueError):
            RocketpultEnv(**options)


def test_text_render_and_close_lifecycle():
    e = RocketpultEnv(render_mode="ansi")
    assert "reset" in e.render()
    e.reset()
    assert "score=0" in e.render()
    e.close()
    e.close()
    with pytest.raises(RuntimeError, match="closed"):
        e.reset()
    with pytest.raises(RuntimeError, match="closed"):
        e.step(0)


def test_many_random_episodes_remain_finite_and_bounded():
    with RocketpultEnv(allow_slow_motion=True, max_episode_steps=150) as e:
        e.action_space.seed(789)
        for episode in range(25):
            obs, _ = e.reset(seed=episode)
            assert e.observation_space.contains(obs)
            for _ in range(e.max_episode_steps):
                obs, reward, terminated, truncated, info = e.step(e.action_space.sample())
                assert e.observation_space.contains(obs)
                assert np.isfinite(reward)
                if terminated or truncated:
                    break
            else:
                pytest.fail("Episode exceeded its configured horizon")


@pytest.mark.parametrize("asynchronous", [False, True], ids=["sync", "spawn-processes"])
def test_standard_vector_wrappers_match_single_instances_and_autoreset(asynchronous):
    # Factories, not live Rust instances, cross the process boundary.
    options = dict(max_episode_steps=3, initial_fuel_range=(0.7, 1.0))
    factories = [partial(RocketpultEnv, **options) for _ in range(2)]
    kwargs = dict(autoreset_mode=AutoresetMode.NEXT_STEP)
    if asynchronous:
        vector = AsyncVectorEnv(factories, context="spawn", **kwargs)
    else:
        vector = SyncVectorEnv(factories, **kwargs)
    singles = [RocketpultEnv(**options) for _ in range(2)]
    try:
        obs, infos = vector.reset(seed=100)
        assert obs.shape == (2, 474)
        assert infos["fuel"][0] != infos["fuel"][1]
        for index, single in enumerate(singles):
            expected, _ = single.reset(seed=100 + index)
            np.testing.assert_array_equal(obs[index], expected)
        for _ in range(3):
            obs, rewards, terminated, truncated, infos = vector.step(np.array([1, 0]))
            for index, single in enumerate(singles):
                expected, reward, term, trunc, info = single.step([1, 0][index])
                np.testing.assert_array_equal(obs[index], expected)
                assert rewards[index] == reward
                assert terminated[index] == term
                assert truncated[index] == trunc
                assert infos["score"][index] == info["score"]
        assert truncated.all() and not terminated.any()
        obs, rewards, terminated, truncated, infos = vector.step(np.array([0, 0]))
        assert not terminated.any() and not truncated.any()
        np.testing.assert_array_equal(rewards, 0)
        np.testing.assert_array_equal(infos["episode_steps"], 0)
        assert vector.observation_space.contains(obs)
    finally:
        vector.close()
        for single in singles:
            single.close()
