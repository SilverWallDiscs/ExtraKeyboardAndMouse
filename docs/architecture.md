# Architecture

## Core principle

Keep the host's ordinary input path unchanged unless the user explicitly enables a multiplayer session. The application should capture per-device events, route them to player slots, and eventually expose each slot as a separate virtual HID device.

## Data flow

### Local player

Physical keyboard/mouse
-> Windows Raw Input (device-specific source)
-> device-to-player mapping
-> virtual HID output
-> game

### Remote player

Remote keyboard/mouse
-> client-side Raw Input
-> authenticated encrypted network session
-> host validates session and player assignment
-> virtual HID output
-> game

No screen, microphone, or audio data is part of the protocol.

## Components

- platform/windows: Raw Input registration, device enumeration, device notifications, and later virtual HID integration.
- core: player slots, device identity, event normalization, key/button state, and session state.
- network: host/client transport, authentication, packet validation, rate limits, and disconnect handling.
- cursor: independent pointer state and appearance configuration.
- config: player assignments, colors, shortcut, and game profiles.

## Input safety and correctness

- Never inject remote input into the host's ordinary physical input path.
- Require explicit host approval before a remote client can send events.
- Encrypt and authenticate every remote session.
- Reject malformed, oversized, stale, or out-of-order packets.
- Clear all held keys and mouse buttons when a client disconnects or times out.
- Provide a host-local emergency stop shortcut; default proposal: Ctrl + Shift + Pause.
- Prevent a remote client from changing host configuration or controlling the host desktop outside an active session.

## Windows implementation notes

Raw Input can identify which physical device generated an event, but it does not by itself create independent input streams for arbitrary games. A virtual HID device layer is likely required for robust game-facing separation. Driver signing, installation, Windows version support, and anti-cheat compatibility must be evaluated before choosing the final driver approach.

## Open design questions

- Which virtual HID implementation can be maintained and distributed safely?
- Should mouse movement be represented as relative or absolute input for each target game?
- How should cursor focus and clipping behave when multiple pointers are visible?
- Which games support multiple keyboard/mouse players without game-specific mods?
