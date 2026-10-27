# Discord PS5 Homebrew Client

A Discord client for jailbroken PS5 consoles, built with Rust. Runs as a homebrew application (`.pkg`) accessible from the PS5 Media tab.

## Features

- **Discord Gateway**: WebSocket connection for real-time events (messages, voice state updates, presence)
- **Discord REST API**: HTTP client for API requests (channels, messages, users, guilds)
- **Voice Chat**: WebRTC-based voice with Opus audio codec
- **PS5 Native UI**: ImGui-based interface rendered via OpenGL/EGL
- **Cross-platform**: Builds for both PS5 (aarch64-unknown-freebsd) and Linux (x86_64-unknown-linux-gnu)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        discord-app                            │
├──────────────┬──────────────┬──────────────┬────────────────┤
│ discord-     │ discord-     │ discord-     │ ps5-platform   │
│ gateway      │ rest         │ voice        │                │
├──────────────┼──────────────┼──────────────┼────────────────┤
│                     discord-common                       │
│              (shared types, errors, config)              │
├─────────────────────────────────────────────────────────────┤
│                        ui (imgui)                          │
└─────────────────────────────────────────────────────────────┘
```

### Crates

| Crate | Description |
|-------|-------------|
| `discord-gateway` | WebSocket gateway connection, event handling |
| `discord-rest` | REST API client with rate limiting |
| `discord-voice` | WebRTC voice connection, Opus encoding/decoding |
| `ps5-platform` | PS5-specific platform abstraction (EGL, GBM, input) |
| `ui` | ImGui-based UI components and rendering |
| `app` | Main application binary, ties everything together |
| `common` | Shared types, errors, configuration |

## Building

### Prerequisites

- Rust stable toolchain (1.80+)
- For PS5 target: `aarch64-unknown-freebsd` target (`rustup target add aarch64-unknown-freebsd`)
- For PS5: Homebrew PS5SDK installed at `/opt/ps5sdk` or `C:/PS5SDK`

### Native Linux Build (Testing)

```bash
cargo build --release --workspace --target x86_64-unknown-linux-gnu
```

### PS5 Homebrew Build

```bash
# Add the FreeBSD target
rustup target add aarch64-unknown-freebsd

# Configure cargo for PS5 target (see .cargo/config.toml)
# Ensure PS5SDK is installed and clang/ar are in PATH

cargo build --release --workspace --target aarch64-unknown-freebsd --bin discord-app

# Package as .pkg
python3 packaging/build_pkg.py \
  target/aarch64-unknown-freebsd/release/discord-app \
  -o discord-ps5.pkg \
  --title-id DISCORDPS5 \
  --title "Discord PS5" \
  --version 0.1.0 \
  --app-ver 01.00 \
  --content-id DISCORDPS5_001 \
  --ps5sdk /opt/ps5sdk \
  --work-dir build/pkg
```

### GitHub Actions CI

Builds run automatically on push to `main` and on tags. Artifacts:
- `discord-ps5-homebrew-binary` - Raw ELF binary
- `discord-ps5-homebrew-pkg` - Installable `.pkg` for PS5

## Installation on PS5

1. Transfer `discord-ps5.pkg` to your PS5 (USB, network, etc.)
2. Install via **Debug Settings > Game > Package Installer** or homebrew pkg installer
3. Launch **Discord PS5** from the **Media** tab

## Configuration

Configuration file location:
- PS5: `/data/discord-ps5/config.toml`
- Linux: `~/.config/discord-ps5/config.toml`

```toml
# config.toml
[discord]
token = "your_bot_token_here"  # User token (not recommended) or bot token

[voice]
enabled = true
input_device = "default"
output_device = "default"

[ui]
scale = 1.0
theme = "dark"
```

## Development

### Project Structure

```
DiscordPS5/
├── .github/workflows/       # CI/CD pipelines
├── .cargo/config.toml       # Cargo target configuration
├── Cargo.toml               # Workspace manifest
├── packaging/               # .pkg build scripts
├── crates/
│   ├── common/              # Shared types, errors
│   ├── discord-gateway/     # Gateway WebSocket
│   ├── discord-rest/        # REST API client
│   ├── discord-voice/       # Voice WebRTC
│   ├── ps5-platform/        # PS5 platform layer
│   ├── ui/                  # ImGui UI
│   └── app/                 # Main binary
└── README.md
```

### Adding Dependencies

Add to `[workspace.dependencies]` in root `Cargo.toml`, then reference with `{ workspace = true }` in crate `Cargo.toml`.

### Code Style

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
```

## Technical Details

### TLS & Cross-Compilation

Uses **rustls** with **aws-lc-rs** (pure-Rust crypto) instead of OpenSSL to avoid cross-compilation issues with `ring`/`aws-lc-sys` on FreeBSD targets.

### Voice Architecture

```
Discord Voice WebSocket (Signaling)
        │
        ▼
   WebRTC PeerConnection
        │
        ├──► Opus Encoder ──► SRTP ──► Network
        │
        └──◄ Network ◄── SRTP ◄── Opus Decoder ◄── Jitter Buffer
```

### PS5 Package Structure

```
discord-ps5.pkg
├── sce_sys/
│   ├── param.sfo        # Metadata (CATEGORY=media for Media tab)
│   └── icon0.png        # 512x512 app icon
└── eboot.bin            # Signed ELF executable
```

## Requirements

- **PS5**: Jailbroken console (firmware ≤ 11.00 recommended), Homebrew Enabler
- **Discord**: Bot token with `Gateway Intents` enabled (Guilds, Guild Messages, Guild Voice States, Message Content)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Disclaimer

This is an unofficial third-party client. Use at your own risk. Discord's Terms of Service may prohibit third-party clients. This project is for educational/research purposes on legally owned hardware.