# ExtraKeyboardAndMouse

A Windows-first input bridge for local and remote multiplayer.

ExtraKeyboardAndMouse aims to let multiple physical or remote keyboard/mouse pairs act as separate players on one Windows PC. It does **not** stream video or audio; remote clients send input events only.

## Project status

**Early scaffold — not yet a usable input splitter.**

The first milestone is a safe Windows Raw Input prototype that identifies physical keyboards and mice independently. Later milestones will add per-player virtual devices, independent cursor rendering, and authenticated remote input sessions.

## Planned features

- Leave normal Windows input untouched when only one keyboard/mouse pair is in use.
- Detect multiple physical keyboards and mice independently.
- Assign devices to player slots.
- Display separately styled cursors for mouse players.
- Receive keyboard/mouse input from multiple remote clients without screen or audio streaming.
- Provide a configurable emergency toggle shortcut (default: Ctrl + Shift + Pause).
- Support multiple remote participants, subject to the target game's player/input support.

## Important compatibility note

Windows and most games normally merge physical keyboard input into one logical keyboard and physical mouse input into one logical mouse. Capturing per-device events is only the first step. Delivering independent input to games requires a signed, properly designed virtual HID driver or another supported device-virtualization mechanism. A second cursor drawn on the desktop does not make games accept a second mouse.

Game compatibility will vary. Some games accept multiple gamepads but only one keyboard/mouse pair; no user-mode application can guarantee universal two-mouse/two-keyboard support.

## Technology direction

- **Rust** for the core, device management, networking, and shared protocol.
- **Windows Raw Input** for identifying physical input devices.
- **Virtual HID** integration for exposing independent player devices to Windows.
- **QUIC** (or another authenticated, encrypted transport) for remote input events.
- A lightweight desktop UI can be selected after the core architecture is validated.

## Build

The initial scaffold targets Windows:

    cargo build
    cargo run

The current executable only reports the project status. Device capture and injection are not implemented yet.

## Safety

Remote input is powerful: a connected participant can control devices on the host. Remote sessions must require explicit host approval, use authenticated encrypted connections, support immediate disconnect, and never accept unauthenticated input packets. The emergency shortcut must be handled locally by the host and remain available while remote input is active.

## Roadmap

1. **Foundation:** Rust workspace, Windows-only target, architecture and safety requirements.
2. **Physical device discovery:** enumerate keyboards/mice and identify Raw Input source handles.
3. **Local input separation:** map physical devices to player slots and test event isolation.
4. **Virtual device output:** expose independent virtual keyboard/mouse devices.
5. **Independent cursors:** render and customize per-player pointers.
6. **Remote sessions:** host/client pairing, encrypted input-only transport, multiple clients.
7. **Game compatibility:** test matrix and per-game profiles.
