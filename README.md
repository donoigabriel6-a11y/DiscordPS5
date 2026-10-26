# Discord PS5 Homebrew Client

A Discord client for jailbroken PS5 with voice calling, screen sharing, and full Discord feature support. Built with Rust, ImGui, and the homebrew PS5SDK.

## Features

- **Text Chat** - Full channel/message support with threads, mentions, reactions
- **Voice Calling** - Join voice channels, mute/deafen, push-to-talk
- **Screen Share** - Share your PS5 screen to Discord
- **DualSense Support** - Native controller navigation
- **Media Tab Integration** - Appears in PS5's Media tab when installed as .pkg
- **Modern UI** - ImGui-based Discord-styled interface

## Building

### Prerequisites

- Rust 1.75+ with `aarch64-unknown-freebsd` target
- PS5SDK (homebrew) - [ps5dev/PS5SDK](https://github.com/ps5dev/PS5SDK)
- For Linux builds: `clang`, `lld`, `cmake`, `ninja`, `libssl-dev`, `pkg-config`, `libxcb1-dev`

### Cross-compile for PS5

```bash
# Install target
rustup target add aarch64-unknown-freebsd

# Set up PS5SDK
export PS5_SDK_PATH=/path/to/ps5sdk
export PATH=$PS5_SDK_PATH/bin:$PATH

# Build
cargo build --release --target aarch64-unknown-freebsd --bin discord-app
```

### Build PS5 Package (.pkg)

The package will appear in the PS5's **Media tab** when installed.

```bash
# Linux/WSL
./build_pkg.sh

# Windows
build_pkg.bat
```

Or manually:
```bash
python3 packaging/build_pkg.py \
  target/aarch64-unknown-freebsd/release/discord-app \
  -o discord-ps5.pkg \
  --title-id DISCORDPS5 \
  --title "Discord PS5" \
  --version 0.1.0 \
  --app-ver 01.00 \
  --content-id DISCORDPS5_001 \
  --category media \
  --ps5sdk /path/to/ps5sdk
```

### Install on PS5

1. Copy `discord-ps5.pkg` to a USB drive (FAT32/exFAT)
2. On PS5: Settings → System → Package Installer → Install from USB
3. Select `discord-ps5.pkg` and install
4. Find **Discord PS5** in the **Media tab**

## CI/CD

GitHub Actions automatically:
- Cross-compiles for PS5 (`aarch64-unknown-freebsd`)
- Builds native Linux binary for testing
- Creates `.pkg` package on every push
- Publishes artifacts and releases

## Project Structure

```
Discord-PS5/
├── .github/workflows/build.yml   # CI/CD pipeline
├── packaging/
│   └── build_pkg.py              # PS5 package builder
├── build_pkg.sh                  # Linux build script
├── build_pkg.bat                 # Windows build script
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── app/                      # Main application entry
│   ├── common/                   # Shared Discord types
│   ├── discord-gateway/          # WebSocket gateway client
│   ├── discord-rest/             # REST API client
│   ├── discord-voice/            # Voice/WebRTC implementation
│   ├── discord-ui/               # ImGui UI components
│   └── ps5-platform/             # PS5 platform abstraction
└── .cargo/config.toml            # Cross-compilation config
```

## Architecture

- **discord-gateway** - Maintains WebSocket connection to Discord, handles events
- **discord-rest** - HTTP API for messages, channels, users, guilds
- **discord-voice** - WebRTC/Opus voice connection with ICE/STUN
- **ps5-platform** - Graphics (DRM/KMS/GBM/EGL), Audio, Input (DualSense)
- **discord-ui** - ImGui windows, components, theming
- **app** - Main loop, state management, integration

## Configuration

Create `config.toml` in the working directory:

```toml
[gateway]
token = "YOUR_BOT_TOKEN"
intents = 3276799  # All privileged intents

[rest]
token = "YOUR_BOT_TOKEN"
api_url = "https://discord.com/api/v10"

[voice]
bitrate = 64000
fec = true

[graphics]
width = 1920
height = 1080
vsync = true

[audio]
sample_rate = 48000
channels = 2
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo check --workspace` and `cargo test --workspace`
5. Submit a PR

## License

MIT License - see LICENSE file for details.

## Disclaimer

This is a homebrew application for **jailbroken PS5 consoles only**. Use at your own risk. Not affiliated with Discord Inc. or Sony Interactive Entertainment.

## Credits

- [ps5dev/PS5SDK](https://github.com/ps5dev/PS5SDK) - Homebrew PS5 SDK
- [Discord API](https://discord.com/developers/docs) - Discord API documentation
- [imgui-rs](https://github.com/imgui-rs/imgui-rs) - Rust ImGui bindings
- [webrtc-rs](https://github.com/webrtc-rs/webrtc) - WebRTC implementation