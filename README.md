# Wireless Shortcut Button

Rust firmware that turns an M5StickC Plus2 into a two-host Bluetooth LE shortcut keyboard for macOS.

## Controls

Button B cycles through three screens:

1. **Send**: Button A sends the selected shortcut to the selected target.
2. **Select Shortcut**: Button A selects the next shortcut.
3. **Select Device**: Button A selects All, Device 1, or Device 2.

The initial shortcuts are:

- **Lock Screen**: `Ctrl+Cmd+Q`
- **Dictate**: `Shift+Ctrl+Alt+Cmd+D`

The selected target is retained after restart. The shortcut resets to Lock Screen and the display returns to Send. An offline individual target never falls back to another device.

## Pairing

1. Power on the M5StickC Plus2.
2. Open **System Settings > Bluetooth** on the Mac.
3. Connect to **Shortcut Button**.
4. Enter the six-digit PIN shown on the M5StickC display.
5. Repeat on a second Mac if required. The button continues advertising until two hosts are connected.

Hosts are assigned Device 1 and Device 2 in pairing order. The display includes the final two bytes of each stable Bluetooth identity address to distinguish them. Bonds and device assignments survive restarts.

The Dictate action requires `Shift+Ctrl+Alt+Cmd+D` to be configured as the Dictation shortcut on each target Mac.

## Prerequisites

Install the Espressif Rust toolchain and flashing utility:

```sh
cargo install espup
espup install
source "$HOME/export-esp.sh"
cargo install ldproxy
cargo install espflash
```

The project targets the original ESP32 in the M5StickC Plus2 and pins ESP-IDF `v5.3.3`.

## Build And Flash

Connect the M5StickC Plus2 over USB-C, then run:

```sh
cargo check --release
cargo run --release
```

`cargo run` builds, flashes, and opens the serial monitor. If more than one serial device is attached, pass the M5StickC port directly to `espflash`:

```sh
espflash flash --monitor --port /dev/cu.usbserial-XXXX \
  target/xtensa-esp32-espidf/release/wireless-button
```

To erase all bonds and saved selections:

```sh
espflash erase-flash --port /dev/cu.usbserial-XXXX
```

Then flash the firmware again.

## Configure Shortcuts

Edit the `SHORTCUTS` array in `src/model.rs`. Each shortcut has a display label, a combination label, a HID modifier mask, and a HID keycode. Rebuild and reflash after changing it.

HID keycodes represent physical keyboard positions. Their character depends on the active keyboard layout on each host.

## Verification

The hardware smoke test should confirm:

- The display and Bluetooth remain active while running from the internal battery.
- Button B cycles through all three screens and wraps to Send.
- Button A changes selections without sending from either selection screen.
- Both shortcuts send once per press and do not repeat when held.
- Device 1 and Device 2 receive targeted shortcuts independently.
- All sends to both connected hosts.
- An offline individual target sends nothing.
- Both Macs reconnect without another PIN after a power cycle.
