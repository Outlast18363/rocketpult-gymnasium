"""Use Gymnasium's existing process workers; no custom Rust worker pool."""
import argparse
from functools import partial

from gymnasium.vector import AsyncVectorEnv

from rocketpult_gymnasium import RocketpultEnv


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--num-envs", type=int, default=2)
    parser.add_argument("--steps", type=int, default=100)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()
    if args.num_envs < 1 or args.steps < 1:
        parser.error("num-envs and steps must be positive")
    factories = [partial(RocketpultEnv, max_episode_steps=100) for _ in range(args.num_envs)]
    envs = AsyncVectorEnv(factories, context="spawn")
    try:
        envs.action_space.seed(args.seed)
        observations, infos = envs.reset(seed=args.seed)
        completed = 0
        for _ in range(args.steps):
            observations, rewards, terminated, truncated, infos = envs.step(envs.action_space.sample())
            completed += int((terminated | truncated).sum())
        print(f"workers={args.num_envs} batch_shape={observations.shape} "
              f"decisions={args.steps} completed_episodes={completed}")
    finally:
        envs.close()


if __name__ == "__main__":
    main()
