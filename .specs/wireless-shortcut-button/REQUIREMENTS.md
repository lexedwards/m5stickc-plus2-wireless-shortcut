# Wireless Shortcut Button Requirements

The M5StickC Plus2 will act as a portable Bluetooth keyboard that sends a selected macOS keyboard shortcut to one or two paired devices.

## Problem Statement

A user wants to trigger frequently used keyboard shortcuts without returning to a computer's physical keyboard. The user may work with two devices and needs to choose both the shortcut and its destination without accidentally sending a shortcut while navigating the controls.

## Outcome

The user can pair the button with up to two macOS devices, select a configured shortcut and target on the built-in display, and send that shortcut with one physical button press.

## Requirements

### Requirement 1

**User Story**: As a user, I want the button to pair as a Bluetooth keyboard, so that macOS can receive shortcuts without additional desktop software.

#### Acceptance Criteria

1. **Given** an unpaired device, **when** the user opens Bluetooth settings on a Mac, **then** the button is discoverable as a keyboard.
2. **Given** first-time pairing, **when** macOS requests a passkey, **then** the current six-digit passkey is visible on the button's display.
3. **Given** a completed pairing, **when** the button or Mac reconnects later, **then** the existing bond is reused without requesting a new passkey.
4. **Given** fewer than two active host connections, **when** another eligible host searches for the button, **then** the button remains available for connection.
5. **Given** two paired hosts are available, **when** both connect, **then** the button maintains both connections concurrently.

### Requirement 2

**User Story**: As a user, I want to move between clearly separated screens, so that navigation cannot accidentally send a shortcut.

#### Acceptance Criteria

1. **Given** the button has started, **when** its interface becomes ready, **then** the Send screen is displayed.
2. **Given** any screen, **when** Button B is pressed once, **then** the next screen is shown in the order Send, Select Shortcut, Select Device, and back to Send.
3. **Given** either selection screen is active, **when** Button A is pressed, **then** only the displayed selection changes and no shortcut is sent.
4. **Given** any screen is displayed, **when** the user views it, **then** its purpose and current selection are distinguishable without consulting external documentation.

### Requirement 3

**User Story**: As a user, I want to choose from configured shortcuts, so that one button can perform multiple actions.

#### Acceptance Criteria

1. **Given** the Select Shortcut screen, **when** Button A is pressed once, **then** the next configured shortcut is selected and displayed.
2. **Given** the last configured shortcut is selected, **when** Button A is pressed, **then** selection wraps to the first shortcut.
3. **Given** the button starts, **when** the shortcut selection is initialized, **then** Lock Screen is selected.
4. **Given** the initial firmware configuration, **when** the user cycles the shortcut list, **then** it contains `Lock Screen: Ctrl+Cmd+Q` and `Dictate: Shift+Ctrl+Alt+Cmd+D`.

### Requirement 4

**User Story**: As a user, I want to choose an individual device or all devices, so that a shortcut reaches only the intended destination.

#### Acceptance Criteria

1. **Given** the Select Device screen, **when** Button A is pressed once, **then** selection advances through All, Device 1, and Device 2 before wrapping.
2. **Given** a host is paired for the first time, **when** an unassigned device slot is available, **then** it is assigned the next slot in pairing order.
3. **Given** a previously assigned host reconnects, **when** it appears in the device list, **then** it retains its Device 1 or Device 2 assignment.
4. **Given** an individual device is displayed, **when** the user views the selection, **then** the display includes its slot and a stable address suffix that distinguishes it from the other device.
5. **Given** the user changes the selected target, **when** the button is restarted, **then** the last selected target is restored.
6. **Given** a restored individual target is offline, **when** the interface starts, **then** that target remains selected and is visibly marked offline.

### Requirement 5

**User Story**: As a user, I want Button A to send the selected shortcut from the Send screen, so that shortcut execution is deliberate and predictable.

#### Acceptance Criteria

1. **Given** the Send screen and an online individual target, **when** Button A is pressed once, **then** the selected shortcut is sent once to only that target.
2. **Given** the Send screen and All is selected, **when** Button A is pressed once, **then** the selected shortcut is sent once to every connected and ready host.
3. **Given** an individual offline target is selected, **when** Button A is pressed, **then** no shortcut is sent and the display indicates that the target is offline.
4. **Given** All is selected but no host is ready, **when** Button A is pressed, **then** no shortcut is sent and the display indicates that no destination is available.
5. **Given** Button A remains held, **when** the initial press has been handled, **then** the shortcut is not repeated until Button A is released and pressed again.
6. **Given** a shortcut is successfully sent, **when** transmission completes, **then** the display gives brief confirmation while preserving the current shortcut and target selections.

### Requirement 6

**User Story**: As a user, I want visible Bluetooth and target status, so that I know whether pressing Send can have an effect.

#### Acceptance Criteria

1. **Given** the button is waiting for a host, **when** the display is viewed, **then** it indicates that the button is available for pairing or connection.
2. **Given** one or two hosts are connected, **when** connection status changes, **then** the display reflects the current connected-host count.
3. **Given** an individual device target, **when** its readiness changes, **then** its online or offline state is reflected on the display.

### Requirement 7

**User Story**: As a user, I want the button to work as a portable standalone device, so that it can be used away from its programming computer.

#### Acceptance Criteria

1. **Given** the M5StickC Plus2 has sufficient battery charge, **when** it is powered without USB attached, **then** the shortcut and Bluetooth functions remain available.
2. **Given** normal button use, **when** a button is pressed once, **then** mechanical bounce does not cause multiple navigation or send actions.

## Decisions and Constraints

- The firmware must be written in Rust for the M5StickC Plus2.
- macOS is the required host platform for the initial release.
- The device acts as a standard Bluetooth Low Energy keyboard and requires no companion desktop application.
- The device supports two simultaneous host connections and three target choices: All, Device 1, and Device 2.
- Device slots are assigned by pairing order and remain stable across reconnections and restarts.
- Friendly host names are not assumed to be available; device slots and stable address suffixes identify hosts.
- Shortcuts are configured in the firmware source and require rebuilding and reflashing to change.
- The selected device target is retained across restarts. The selected shortcut resets to Lock Screen, and the interface resets to the Send screen.
- An offline individual target never falls back to All or another device.

## Out of Scope

- Editing shortcuts through the device interface, USB serial, Bluetooth, or a companion application.
- Automatically retrieving or assigning friendly host names such as "Work Mac".
- Supporting more than two simultaneous host connections.
- Formal compatibility guarantees for Windows, Linux, iOS, or Android.
- Per-device shortcut lists or different shortcuts sent to each host in one action.
- On-device pairing deletion, device-slot reassignment, or bond-management screens.
- Battery level reporting, charging indicators, or advanced power-management settings.

## Further Notes

- Keyboard combinations are interpreted according to each host's active keyboard layout and shortcut configuration.
- The Dictate shortcut assumes the corresponding macOS shortcut has been configured on each target Mac.
