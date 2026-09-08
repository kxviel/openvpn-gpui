# OpenVPN

A small native Linux VPN app built with [GPUI](https://gpui.rs/). Import an OpenVPN configuration, save it under a profile name, and connect. Multiple profiles are supported.
This project is a GPUI application and uses NetworkManager as its VPN backend.

Code Inspiration was taken from: https://github.com/fonzi/openvpn-gui-rust

This project was generated with OpenAI's ```GPT-6 Astra xhigh```. The generated code was reviewed by me.

## Run

```bash
cargo run --locked
```

To build an optimized binary and add launchers to the desktop and application menu:

```bash
./scripts/install.sh
```

The installer runs as your normal user. It installs `~/.local/bin/openvpn-gpui`, an icon, and `OpenVPN.desktop` on your desktop. It does not install system packages or modify network settings.

## Use

1. Click **Import configuration** or the **+** button.
2. Choose the `.ovpn` or `.conf` supplied by DFKI, enter a profile name, and click **Save profile**.
3. Click **Connect**. Complete any authentication prompt shown by your desktop.
4. Click **Disconnect** to end the connection. You can also cancel an attempt while it is connecting.

Click the profile card to choose a saved profile, add another, or remove the selected profile. Profile changes are disabled while its tunnel is active. Removing a profile requires a second click and deletes that profile's NetworkManager connection and the app's saved files; it does not delete the original download.

Closing the app leaves active or pending connections managed by NetworkManager. Reopen the app to see the selected profile's current state and disconnect it. The duration counter measures the time the current app instance has observed the tunnel connected; it restarts when you reopen the app. Addresses come from the selected VPN connection, not a hard-coded network interface.

Keyboard shortcuts: **Ctrl+O** imports a profile, **Ctrl+P** opens saved profiles, **Esc** returns to the connection screen, and **Ctrl+Q** closes the app. Form controls support keyboard navigation and standard text editing.

## Requirements

- Linux with a Wayland or X11 desktop and a working Vulkan driver.
- NetworkManager, `nmcli`, OpenVPN, and the NetworkManager OpenVPN plugin.
- A desktop NetworkManager secret agent, such as KDE Plasma's network applet or `nm-applet`, for password prompts. The app does not collect passwords or send them on command lines.
- An XDG desktop portal and a matching desktop backend for the file picker.
- A current Rust toolchain and GPUI's Linux build dependencies.

On Arch/CachyOS, the relevant packages include:

```bash
sudo pacman -S --needed rust pkgconf clang cmake libxkbcommon libxkbcommon-x11 \
  fontconfig libxcb libx11 wayland vulkan-headers vulkan-icd-loader \
  networkmanager networkmanager-openvpn xdg-desktop-portal
```

Keep your desktop's portal backend and Vulkan GPU driver installed. This machine already had the required VPN packages and successfully ran the app on both Wayland and X11.

## Profiles and compatibility

Profile metadata and copied configurations are stored under `$XDG_DATA_HOME/openvpn-gpui`, normally `~/.local/share/openvpn-gpui`. Directories are private to your user, files have mode `0600`, and metadata is saved atomically. External certificates and keys are copied and their references rewritten, so moving the original files does not break a saved profile. Inline certificates use unique config names to avoid collisions in NetworkManager's import storage.

NetworkManager owns the tunnel, routes, DNS, and authentication. The app does not change a profile into a full-tunnel VPN or claim all internet traffic is protected: routing follows the imported configuration and NetworkManager's handling of it. Its connection entries are named `OpenVPN · <profile name>`, have autoconnect disabled, and are identified by their exact NetworkManager UUID.

Only configurations supported by NetworkManager's OpenVPN importer are supported. Configurations requiring executable hooks, plugins, or nested config files are rejected with an explanation. Browser SSO is not implemented by this app; setups that require OpenVPN 3's browser authentication need additional backend work. An actual DFKI login cannot be verified without your DFKI configuration and authentication.

If a saved configuration contains credentials, the copied files retain those credentials with private permissions. NetworkManager may also extract inline certificates to its own user certificate storage. Removing an app profile does not erase independently managed NetworkManager certificate files.

## Source structure

The flat modules, explicit constants, crate-scoped types, and helper functions follow the conventions in your IcyMap project.

```text
src/
├── main.rs       # window, theme, embedded assets, single-instance lock
├── app.rs        # app state, events, file picker, connection actions
├── ui.rs         # connection screen, profile list, import form
├── backend.rs    # bounded NetworkManager operations and status monitoring
└── profile.rs    # profile metadata, private storage, config bundling
assets/          # embedded UI icons and generated Linux app icons
scripts/         # user-local installation
```

## Checks

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The optional integration test creates two disposable NetworkManager profiles, verifies import and selection persistence, starts a connection to a loopback discard port, cancels it, verifies no tunnel remains active, then deletes the test profiles. It requires `openssl` and may produce desktop network notifications. It does not connect to DFKI or any external VPN server.

```bash
cargo test --locked networkmanager_import_cancel_and_remove -- --ignored
```

For visual development, debug builds support starting on a screen without automating desktop input:

```bash
OPENVPN_GPUI_SCREEN=import cargo run --locked
OPENVPN_GPUI_SCREEN=profiles cargo run --locked
```
