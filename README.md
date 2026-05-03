# orbitty

Terminal idle screensaver - slowly spinning planets with procedural surface textures.

![gif](demo.gif)

## Install

```bash
cargo install --path .
```

This installs the binary to `~/.cargo/bin/`. Make sure it is in your `PATH`:

For bash/zsh:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Add that line to your `~/.bashrc` or `~/.zshrc` to make it permanent.

For fish:

```fish
fish_add_path $HOME/.cargo/bin
```

After that, `orbitty` works from anywhere.

Requires a terminal with true color support (`COLORTERM=truecolor` or `COLORTERM=24bit`).

## Usage

```bash
orbitty
```

| Flag | Default | Description |
|------|---------|-------------|
| `--fps <N>` | 30 | Target frame rate (1-240) |
| `--zoom <N>` | 0.43 | Initial zoom (0.3-4.0) |
| `--speed <N>` | 4 | Rotation speed multiplier |
| `--planet <NAME>` | earth | Starting planet or hex seed |

## Controls

| Key | Action |
|-----|--------|
| `1` - `8` | Switch planet |
| `+` / `-` | Zoom in / out |
| `[` / `]` | Rotation speed |
| `r` | Random planet |
| `s` | Enter seed manually |
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
