# Development

Install [Rust](https://rustup.rs/) (includes Cargo),
[NVM](https://github.com/nvm-sh/nvm#install--update-script), and
[Tauri’s system dependencies](https://v2.tauri.app/start/prerequisites/).

```sh
# Install Node and pnpm
nvm install 24
npm install --global pnpm@11.19.0

# Start the runtime and GUI from the repo root
./scripts/serve.sh dev
```

Run `./scripts/serve.sh` for the release build.

## Documentation

Preview with:

```sh
uv run --locked tools/docs/build.py --serve 8769
```
