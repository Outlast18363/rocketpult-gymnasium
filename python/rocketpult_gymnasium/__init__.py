"""Gymnasium environments backed by the native Rust Rocketpult simulation."""
from gymnasium.envs.registration import register, registry

from .env import RocketpultEnv

__version__ = "0.1.0"
__all__ = ["RocketpultEnv"]

if "Rocketpult-v0" not in registry:
    register(id="Rocketpult-v0", entry_point="rocketpult_gymnasium.env:RocketpultEnv")
