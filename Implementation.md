# OpenVPN — Implementation Notes

## Scope

A focused GPUI desktop app for importing named OpenVPN profiles and connecting to a workplace network. No accounts, subscriptions, traffic graphs, public-IP services, or COSMIC dependencies.

## State and rendering

`app.rs` owns the selected screen, imported profiles, file-picker state, and observed VPN state. `ui.rs` renders those values and dispatches explicit actions. It does not run network commands during rendering. A background worker handles NetworkManager operations; the UI receives updates through a channel.

The connection states are idle, connecting, connected, disconnecting, and unavailable. Only NetworkManager's `activated` state produces the connected screen. Monitoring failures produce an unavailable state rather than a false connected indicator.

## Profiles

Each profile has an app UUID and a separate NetworkManager UUID. A profile name is display text, never a process identifier. Imported files are bundled into a private per-profile directory. Metadata writes are atomic. A process lock prevents concurrent app instances from racing saves.

The importer retains OpenVPN directives supported by NetworkManager, preserves inline blocks, and copies referenced certificate/key files. Unsupported script hooks and nested configs are rejected. The unique config basename prevents NetworkManager's inline certificate extraction from colliding across profiles.

## Connection lifecycle

Commands use argument arrays, fixed locale, bounded execution time, and private output handles. Authentication is delegated to the desktop's NetworkManager secret agent. Cancellation first interrupts the pending `nmcli` command, then explicitly deactivates that exact connection UUID, because killing `nmcli` alone does not cancel NetworkManager activation.

NetworkManager continues to own connections when the window closes. The app refreshes the selected profile after reopening. The displayed duration is observed uptime for the current app instance.

## Verification

Unit tests cover file bundling and permissions, unsupported configs, profile-name validation, persistence/corruption handling, exact UUID parsing, and connection-state parsing. An opt-in local integration test covers two-profile import, selection persistence, process startup/cancellation, and removal. Visual checks cover the native Linux screens. Real DFKI authentication remains a user configuration check.
