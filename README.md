# ExtraKeyboardAndMouse

A Windows-first input bridge for local and remote multiplayer.

ExtraKeyboardAndMouse is being built to let multiple physical or remote keyboard/mouse pairs act as separate players on one Windows PC. Remote sessions are input-only: the project does not capture or stream screen, microphone, or audio.

## Current status

**Milestone 1: passive, per-device Raw Input monitor.**

The Windows prototype now:

- Enumerates Raw Input mouse and keyboard devices.
- Registers for keyboard and mouse Raw Input.
- Receives input even while its invisible window is in the background.
- Logs the originating device path for each keyboard and mouse event.
- Detects device arrivals and removals.
- Leaves the ordinary Windows input path enabled. It does not suppress legacy input or inject input.

This is a diagnostic foundation, not yet a player splitter. It does **not** create independent game inputs, extra cursors, virtual devices, or remote sessions yet.

## Run the prototype

Requirements: Windows 10/11 and the stable Rust toolchain.

In PowerShell:

```powershell
cargo run
```

To see individual events:

```powershell
$env:RUST_LOG = "debug"
cargo run
```

The app prints the current number of Raw Input keyboards and mice. Connect a second keyboard or mouse and watch for the hot-plug log. Press keys or move each mouse; the debug output includes the source device path so you can check whether Windows reports each device separately.

Stop the monitor with Ctrl+C in its console.

## Planned features

- Keep normal Windows input untouched when only one keyboard/mouse pair is in use.
- Assign each physical keyboard and mouse to an independent player slot.
- Expose player input as separate virtual HID devices for game compatibility.
- Display independently styled cursors for mouse players.
- Receive keyboard/mouse input from multiple remote clients without screen or audio streaming.
- Provide a configurable host emergency shortcut (default proposal: Ctrl + Shift + Pause).
- Support multiple remote participants, subject to the target game's player/input support.

## Important compatibility note

Windows Raw Input identifies which device generated an event, but it does not create independent input streams for arbitrary games. Delivering separate game-facing inputs will require a carefully selected virtual HID driver or another supported device-virtualization mechanism. A second cursor drawn on screen does not make games accept a second mouse.

Game compatibility will vary. Some games accept multiple gamepads but only one keyboard/mouse pair; no user-mode application can guarantee universal two-keyboard/two-mouse support.

## Technology direction

- **Rust** for the core, device management, networking, and shared protocol.
- **Windows Raw Input** for identifying physical input devices.
- **Virtual HID** integration for exposing independent player devices to Windows.
- **QUIC** (or another authenticated, encrypted transport) for remote input events.
- A lightweight desktop UI after the input architecture is validated.

## Safety

Remote input is powerful: a connected participant can control devices on the host. Remote sessions must require explicit host approval, use authenticated encrypted connections, support immediate disconnect, clear held keys/buttons after disconnect, and never accept unauthenticated input packets. The emergency shortcut must be handled locally by the host and remain available while remote input is active.

## Roadmap

1. **Passive device capture:** enumerate devices and read per-device Raw Input events. (Implemented prototype)
2. **Player routing:** assign physical devices to player slots and normalize keyboard/mouse events.
3. **Virtual device output:** expose independent virtual keyboard/mouse devices.
4. **Independent cursors:** render and customize per-player pointers.
5. **Remote sessions:** host/client pairing, encrypted input-only transport, multiple clients.
6. **Game compatibility:** test matrix and per-game profiles.
