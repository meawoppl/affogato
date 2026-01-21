# Affogato Development Guidelines

## Git Workflow

- Always develop on branches, never commit directly to main
- Create PRs for all changes
- CI must pass before merging
- Use `gh pr checks <PR#> --watch` to wait for CI results

## Rust Development

- Run `cargo fmt` before committing
- Run `cargo clippy -- -D warnings` to check for issues
- Use `cargo add` to add dependencies

## Project Structure

- `cli/` - Rust CLI tool
- `components/ice40/` - ESP-IDF component for FPGA loading
- `docker/` - Build container with FPGA toolchain
- `examples/` - Demo projects (colorwheel, web-led)
- `fpga/` - Reusable Verilog modules
- `docs/` - Documentation
