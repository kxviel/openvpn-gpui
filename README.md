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

1. Click **Add profile**.
2. Choose your `.ovpn` or `.conf` file and enter a profile name.
3. Optionally enter a username and password, then click **Import profile**. To add or replace a login later, open **Profile settings** from the connection card or the profile list.
4. Click **Connect**. Profiles without a saved login continue to use your desktop's authentication prompt.
5. Click **Disconnect** to end the connection. You can also cancel an attempt while it is connecting.

Open **Profiles** to choose a saved connection. Each profile has its own settings screen with username and password fields. Settings remain viewable while connected; disconnect before changing them. Removing a profile requires confirmation and deletes that profile's NetworkManager connection and the app's saved files; it does not delete the original download.

**Disconnect and closing the app stop its VPN connections**, including a pending connection attempt. The app waits for NetworkManager to confirm deactivation, which releases the VPN tunnel's routes and DNS configuration while keeping the underlying Wi-Fi or Ethernet connection active. Saved profiles and logins are retained for reconnecting. Other VPNs outside this app's profile library are not affected.

The window stays open with an error if normal exit cleanup fails. Window close, **Ctrl+Q**, SIGINT and SIGTERM share this cleanup flow. Forced termination such as SIGKILL or a system crash cannot run application cleanup; NetworkManager may retain a tunnel in that case.

The duration counter measures the time the current app instance has observed the tunnel connected. Addresses come from the selected VPN connection.

Keyboard shortcuts: **Ctrl+O** imports a profile, **Ctrl+P** opens saved profiles, **Esc** goes back, and **Ctrl+Q** disconnects and closes the app. Form controls support keyboard navigation and standard text editing.

## Requirements

- Linux with a Wayland or X11 desktop and a working Vulkan driver.
- NetworkManager, `nmcli`, OpenVPN, and the NetworkManager OpenVPN plugin.
- A desktop NetworkManager secret agent, such as KDE Plasma's network applet or `nm-applet`, if you want password prompts instead of a saved profile login.
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

Profile metadata, copied configurations, and optional saved passwords are stored under `$XDG_DATA_HOME/openvpn-gpui`, normally `~/.local/share/openvpn-gpui`. Directories are private to your user, sensitive files have mode `0600`, and metadata is saved atomically. Passwords are kept out of profile JSON and process arguments; `nmcli` reads them through its password-file interface when connecting. External certificates and keys are copied and their references rewritten, so moving the original files does not break a saved profile. Inline certificates use unique config names to avoid collisions in NetworkManager's import storage.

NetworkManager owns the tunnel, routes, DNS, and authentication. The app does not change a profile into a full-tunnel VPN or claim all internet traffic is protected: routing follows the imported configuration and NetworkManager's handling of it. Its connection entries are named `OpenVPN · <profile name>`, have autoconnect disabled, and are identified by their exact NetworkManager UUID.

Saved passwords are plaintext files protected by user-only filesystem permissions, not an encrypted keyring.

Only configurations supported by NetworkManager's OpenVPN importer are supported. Configurations requiring executable hooks, plugins, or nested config files are rejected with an explanation. Browser SSO is not implemented by this app; setups that require OpenVPN 3's browser authentication need additional backend work. An actual DFKI login cannot be verified without your DFKI configuration and authentication.

If a saved configuration contains credentials, the copied files retain those credentials with private permissions. NetworkManager may also extract inline certificates to its own user certificate storage. Removing an app profile does not erase independently managed NetworkManager certificate files.

## Source structure

The flat modules, explicit constants, crate-scoped types, and helper functions follow the conventions in your IcyMap project.

```text
src/
├── main.rs       # window, theme, embedded assets, single-instance lock
├── app.rs        # app state, events, file picker, connection actions
├── ui.rs         # connection screen, profile list, settings and import forms
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

The optional integration test creates two disposable NetworkManager profiles, verifies credentials and selection persistence, starts a connection to a loopback discard port, then tests cancellation, shutdown during activation, and worker-drop cleanup. It checks the existing Wi-Fi/Ethernet connection's routes and DNS are unchanged, then deletes the test profiles. It requires `openssl` and may produce desktop network notifications. It does not connect to DFKI or any external VPN server; teardown after an authenticated VPN session requires a real configuration and login.

```bash
cargo test --locked networkmanager_import_cancel_and_remove -- --ignored
```

For visual development, debug builds support starting on a screen without automating desktop input:

```bash
OPENVPN_GPUI_SCREEN=import cargo run --locked
OPENVPN_GPUI_SCREEN=profiles cargo run --locked
OPENVPN_GPUI_SCREEN=settings cargo run --locked
```
