"""Optional real optimizer/checkpoint integration; skipped without training extras."""
import numpy as np
import pytest

from rocketpult_gymnasium import RocketpultEnv

@pytest.mark.training
def test_ppo_learns_saves_reloads_and_predicts(tmp_path):
    sb3 = pytest.importorskip("stable_baselines3")
    from stable_baselines3.common.env_checker import check_env

    with RocketpultEnv(max_episode_steps=50) as env:
        check_env(env)
        model = sb3.PPO("MlpPolicy", env, n_steps=32, batch_size=16, n_epochs=1,
                        seed=42, device="cpu")
        before = [p.detach().clone() for p in model.policy.parameters()]
        model.learn(total_timesteps=32)
        assert any(not old.equal(new) for old, new in zip(before, model.policy.parameters()))
        checkpoint = tmp_path / "ppo"
        model.save(checkpoint)
        restored = sb3.PPO.load(checkpoint, env=env, device="cpu")
        observation, _ = env.reset(seed=999)
        action, _ = model.predict(observation, deterministic=True)
        reloaded_action, _ = restored.predict(observation, deterministic=True)
        np.testing.assert_array_equal(action, reloaded_action)
        assert env.observation_space.contains(env.step(reloaded_action)[0])
