$ErrorActionPreference = 'Stop'
cargo test --workspace --manifest-path native/Cargo.toml
python -m unittest discover -s ai_engine/tests -v
