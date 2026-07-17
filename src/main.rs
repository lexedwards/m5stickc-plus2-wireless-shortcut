use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, RgbColor},
};
use esp_idf_svc::hal::{
    delay::{Delay, FreeRtos},
    gpio::{AnyIOPin, PinDriver},
    peripherals::Peripherals,
    spi::{config::Config as SpiConfig, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
    units::Hertz,
};
use mipidsi::{
    interface::SpiInterface,
    options::{ColorInversion, ColorOrder, Orientation, Rotation},
};

use crate::{
    ble::{BleKeyboard, SendResult},
    model::{AppState, DebouncedButton, PrimaryAction},
    storage::Storage,
};

mod ble;
mod display;
mod model;
mod storage;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;

    // GPIO4 must remain high for battery-powered operation on the Plus2.
    let mut power_hold = PinDriver::output(peripherals.pins.gpio4)?;
    power_hold.set_high()?;

    let mut backlight = PinDriver::output(peripherals.pins.gpio27)?;
    backlight.set_high()?;
    log::info!("power hold and display backlight enabled");

    let mut storage = Storage::open()?;
    let mut app = AppState::new(storage.load_target()?);
    let stored_hosts = storage.load_hosts()?;

    let button_a = PinDriver::input(peripherals.pins.gpio37)?;
    let button_b = PinDriver::input(peripherals.pins.gpio39)?;

    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio13,
        peripherals.pins.gpio15,
        None::<AnyIOPin>,
        &SpiDriverConfig::new(),
    )?;
    let spi = SpiDeviceDriver::new(
        spi_driver,
        Some(peripherals.pins.gpio5),
        &SpiConfig::new()
            .baudrate(Hertz(20_000_000))
            .data_mode(embedded_hal::spi::MODE_0),
    )?;
    let dc = PinDriver::output(peripherals.pins.gpio14)?;
    let reset = PinDriver::output(peripherals.pins.gpio12)?;
    let mut display_buffer = [0u8; 512];
    let interface = SpiInterface::new(spi, dc, &mut display_buffer);
    let mut delay = Delay::new_default();
    let mut screen = mipidsi::Builder::new(mipidsi::models::ST7789, interface)
        .display_size(135, 240)
        .display_offset(52, 40)
        .invert_colors(ColorInversion::Inverted)
        .color_order(ColorOrder::Bgr)
        .orientation(Orientation::new().rotate(Rotation::Deg90))
        .reset_pin(reset)
        .init(&mut delay)
        .map_err(|error| anyhow::anyhow!("display initialization failed: {error:?}"))?;
    log::info!("display initialized");

    screen
        .clear(Rgb565::RED)
        .map_err(|error| anyhow::anyhow!("display test fill failed: {error:?}"))?;
    FreeRtos::delay_ms(500);

    let passkey = unsafe { esp_idf_svc::sys::esp_random() } % 1_000_000;
    display::draw(&mut screen, &app, Default::default(), passkey, None)
        .map_err(|error| anyhow::anyhow!("initial screen render failed: {error:?}"))?;
    log::info!("initial screen rendered");

    let keyboard = BleKeyboard::init(passkey, stored_hosts)?;
    log::info!("BLE keyboard initialized");

    let mut debounced_a = DebouncedButton::new();
    let mut debounced_b = DebouncedButton::new();
    let mut rendered_revision = u32::MAX;
    let mut persisted_hosts = stored_hosts;
    let mut dirty = true;
    let mut feedback: Option<(&'static str, u8)> = None;

    loop {
        FreeRtos::delay_ms(10);

        if debounced_b.update(button_b.is_low()) {
            app.next_screen();
            feedback = None;
            dirty = true;
        }

        if debounced_a.update(button_a.is_low()) {
            let old_target = app.target;
            match app.primary_action() {
                PrimaryAction::SelectionChanged => {
                    if app.target != old_target {
                        storage.save_target(app.target)?;
                    }
                    feedback = None;
                }
                PrimaryAction::Send { target, shortcut } => {
                    feedback = Some(match keyboard.send(target, shortcut) {
                        SendResult::Sent(1) => ("SENT TO 1 DEVICE", 75),
                        SendResult::Sent(_) => ("SENT TO 2 DEVICES", 75),
                        SendResult::NoDestination => ("TARGET OFFLINE", 75),
                    });
                }
            }
            dirty = true;
        }

        if let Some((_, remaining)) = feedback.as_mut() {
            if *remaining > 0 {
                *remaining -= 1;
            } else {
                feedback = None;
                dirty = true;
            }
        }

        let hosts = keyboard.hosts_view();
        let current_identities = keyboard.host_identities();
        if current_identities != persisted_hosts {
            storage.save_hosts(current_identities)?;
            persisted_hosts = current_identities;
        }
        if hosts.revision != rendered_revision {
            dirty = true;
        }

        if dirty {
            display::draw(
                &mut screen,
                &app,
                hosts,
                passkey,
                feedback.map(|(message, _)| message),
            )
            .map_err(|error| anyhow::anyhow!("screen render failed: {error:?}"))?;
            rendered_revision = hosts.revision;
            dirty = false;
        }
    }
}
