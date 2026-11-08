# Architecture

## Core principle

Keep the host's ordinary input path unchanged unless the user explicitly enables a multiplayer session. The application captures per-device events, routes them to player slots, and eventually exposes each slot as a separate virtual HID device.

## Implemented foundation

The Windows prototype uses Raw Input with generic desktop controls:

- Usage page `0x01`, usage `0x02`: mouse.
- Usage page `0x01`, usage `0x06`: keyboard.
- `RIDEV_INPUTSINK`: receive input while the hidden window is not foreground.
- `RIDEV_DEVNOTIFY`: receive device-arrival and device-removal notifications.
- No `RIDEV_NOLEGACY` flag: the ordinary Windows input path remains enabled.

The app uses `GetRawInputDeviceList` and `GetRawInputDeviceInfoW` to enumerate devices and identify their device-interface paths. It reads `WM_INPUT` via `GetRawInputData` and logs the source device alongside basic keyboard or mouse movement information.

This stage observes input only. It does not block, remap, or inject it.

## Data flow

### Local player (target architecture)

Physical keyboard/mouse
-> Windows Raw Input (device-specific source)
-> device-to-player mapping
-> virtual HID output
-> game

### Remote player (target architecture)

Remote keyboard/mouse
-> client-side Raw Input
-> authenticated encrypted network session
-> host validates session and player assignment
-> virtual HID output
-> game

No screen, microphone, or audio data is part of the protocol.

## Components

- `platform/windows/raw_input.rs`: Raw Input registration, event capture, device enumeration, and hot-plug notifications.
- `core`: future player slots, device identity, event normalization, key/button state, and session state.
- `network`: future host/client transport, authentication, packet validation, rate limits, and disconnect handling.
- `cursor`: future independent pointer state and appearance configuration.
- `config`: future player assignments, colors, shortcut, and game profiles.

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

The current prototype deliberately avoids `RIDEV_NOLEGACY` so that registering for Raw Input does not suppress normal keyboard and mouse messages.

## Milestones

1. Passive per-device Raw Input capture. **Prototype implemented.**
2. Device-to-player assignment and normalized event model.
3. Virtual HID output and a safe enable/disable lifecycle.
4. Independent cursor rendering and pointer appearance settings.
5. Authenticated input-only networking with multiple remote clients.
6. Game compatibility profiles and validation.

## Open design questions

- Which virtual HID implementation can be maintained and distributed safely?
- Should mouse movement be represented as relative or absolute input for each target game?
- How should cursor focus and clipping behave when multiple pointers are visible?
- Which games support multiple keyboard/mouse players without game-specific mods?
