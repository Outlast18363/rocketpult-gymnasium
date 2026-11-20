# Contributing

Thanks for helping improve Rocketpult Gymnasium.

## Local setup

Follow the [quick start](README.md#quick-start), then install the test dependencies:

```sh
python -m pip install -e '.[test]'
```

The Rust simulation, Python interface and optional playable game live in this
repository. No sibling project or prebuilt game is required.

## Before opening a pull request

Keep changes focused and describe the resulting behavior. For simulation or API
changes, include a small reproducible action sequence and the relevant
configuration and seed.

Useful environment checks, without loading or running any AI model:

```sh
cargo fmt --check
cargo test --lib
python -m pytest tests/test_environment.py -q
```

Changes to Rust code require rebuilding the Python extension with
`python -m pip install -e .`. Documentation-only changes need link and rendering
checks. The optional model integration test is excluded from the default suite;
run it only when explicitly intended. See the [development guide](docs/development.md).

## Bug reports

Include your operating system, Python and Rust versions, environment settings,
seed, action sequence, and expected versus actual output. A short script that
uses only `reset` and `step` is especially useful.

## Game assets and attribution

Preserve the [original artwork credits](assets/reference/PROVENANCE.md),
[media provenance](docs/media/README.md), and [third-party notices](THIRD_PARTY.md).
The project's MIT license covers its code, not ownership of the original game
artwork or third-party components.
