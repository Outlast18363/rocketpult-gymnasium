"""Run complete episodes using a random policy; no training dependency required."""
import argparse

from rocketpult_gymnasium import RocketpultEnv


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--episodes", type=int, default=3)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()
    with RocketpultEnv(render_mode="ansi") as env:
        env.action_space.seed(args.seed)
        for episode in range(args.episodes):
            observation, info = env.reset(seed=args.seed + episode)
            episode_return = 0.0
            while True:
                observation, reward, terminated, truncated, info = env.step(env.action_space.sample())
                episode_return += reward
                if terminated or truncated:
                    print(f"episode={episode + 1} return={episode_return:.3f} "
                          f"score={info['score']} steps={info['episode_steps']} "
                          f"outcome={info['outcome']}")
                    break


if __name__ == "__main__":
    main()
