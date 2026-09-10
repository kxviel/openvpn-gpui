# OpenVPN — Implementation Notes

## Scope

A focused GPUI desktop app for importing named OpenVPN profiles and connecting to a workplace network. No accounts, subscriptions, traffic graphs, public-IP services, or COSMIC dependencies.

## State and rendering

`app.rs` owns the selected screen, imported profiles, credential inputs, file-picker state, and observed VPN state. `ui.rs` renders those values and dispatches explicit actions. It does not run network commands during rendering. A background worker handles NetworkManager operations; the UI receives updates through a channel.

The connection states are idle, connecting, connected, disconnecting, and unavailable. Only NetworkManager's `activated` state produces the connected screen. Monitoring failures produce an unavailable state rather than a false connected indicator.

## Profiles

Each profile has an app UUID and a separate NetworkManager UUID. A profile name is display text, never a process identifier. Imported files are bundled into a private per-profile directory. Metadata writes are atomic. A process lock prevents concurrent app instances from racing saves.

The importer retains OpenVPN directives supported by NetworkManager, preserves inline blocks, and copies referenced certificate/key files. Unsupported script hooks and nested configs are rejected. The unique config basename prevents NetworkManager's inline certificate extraction from colliding across profiles.

## Connection lifecycle

Commands use argument arrays, fixed locale, bounded execution time, and private output handles. Profiles can delegate authentication to the desktop's NetworkManager secret agent or use a saved per-profile password file with mode `0600`. The password-file path—not its contents—is passed to `nmcli`, keeping passwords out of process arguments. Cancellation first interrupts the pending `nmcli` command, then explicitly deactivates that exact connection UUID, because killing `nmcli` alone does not cancel NetworkManager activation.

Window close, Ctrl+Q, SIGINT and SIGTERM request asynchronous shutdown. Pending activation is cancelled, all active UUIDs in the app's profile library are deactivated, and the worker confirms their absence before allowing the process to exit. Failure keeps the window open for retry. The backend's Drop handler cancels, requests cleanup and joins the worker as a fallback for application-level exits.

The shared disconnect path uses [NetworkManager connection deactivation](https://networkmanager.pages.freedesktop.org/NetworkManager/NetworkManager/nmcli.html), allowing NetworkManager to withdraw the VPN's routes and DNS contributions. It preserves the physical connection, saved VPN profiles, and unrelated tunnels. It never flushes the routing table, rewrites resolv.conf, or restarts Wi-Fi. Forced kills cannot run cleanup.

## Verification

Unit tests cover file bundling and permissions, credentials, name validation, persistence/corruption handling, UUID/state parsing, and disconnect races/failures. An opt-in local integration test covers two-profile import, credential persistence, cancellation, asynchronous shutdown, fallback worker-drop cleanup, base-connection routes/DNS preservation, and removal. Visual checks cover native Linux screens. The loopback test never establishes an authenticated tunnel; full DFKI route/DNS teardown remains a real-configuration check.
