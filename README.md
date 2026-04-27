# orbitty

Terminal idle screensaver - slowly spinning planets on actual orbital mechanics.
![gif](demo.gif)

## Requirements

- Rust toolchain (`rustup` / `cargo`)
- A terminal with true color support (`COLORTERM=truecolor` or `COLORTERM=24bit`)

## Install

```bash
cargo install --path .
```

After that, the `orbitty` command is available in your shell. Cargo installs binaries to `~/.cargo/bin/` - make sure it is in your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Add that line to your `~/.bashrc` or `~/.zshrc` to make it permanent.

## Usage

```bash
orbitty
```

Optional flags:

| Flag | Default | Description |
|------|---------|-------------|
| `--fps <N>` | 30 | Target frame rate (1-240) |

## Controls

| Key | Action |
|-----|--------|
| `1` - `8` | Switch planet |
| `+` / `-` | Zoom in / out |
| `[` / `]` | Rotation speed |
| `r` | Reset rotation |
| `h` | Toggle help overlay |
| `q` | Quit |

## Planets

| Key | Planet |
|-----|--------|
| `1` | Mercury |
| `2` | Venus |
| `3` | Earth |
| `4` | Mars |
| `5` | Jupiter |
| `6` | Saturn |
| `7` | Uranus |
| `8` | Neptune |

## License

MIT - see [LICENSE](LICENSE).
