"""Dry run by default. Pass a positive --timesteps value to explicitly train."""
import argparse
from pathlib import Path

from rocketpult_gymnasium import RocketpultEnv


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timesteps", type=int, default=0,
                        help="0 checks one transition without training (default)")
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--output", type=Path, default=Path("artifacts/ppo_rocketpult"))
    args = parser.parse_args()
    if args.timesteps < 0:
        parser.error("timesteps must be nonnegative")
    if args.timesteps == 0:
        with RocketpultEnv() as env:
            observation, info = env.reset(seed=args.seed)
            observation, reward, terminated, truncated, info = env.step(1)
            print(f"Dry run OK: observation={observation.shape} "
                  f"reward={reward} terminated={terminated} truncated={truncated}")
        return
    try:
        from stable_baselines3 import PPO
        from stable_baselines3.common.monitor import Monitor
        import torch
    except ImportError:
        parser.error("Install training dependencies: pip install 'stable-baselines3==2.7.1'")
    torch.set_num_threads(1)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with Monitor(RocketpultEnv()) as env:
        model = PPO("MlpPolicy", env, seed=args.seed, device="cpu", verbose=1,
                    n_steps=256, batch_size=64)
        model.learn(total_timesteps=args.timesteps)
        model.save(str(args.output))
        print(f"Saved policy: {args.output}.zip")


if __name__ == "__main__":
    main()
