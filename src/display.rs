use embedded_graphics::{
    mono_font::{
        ascii::{FONT_10X20, FONT_6X10},
        MonoTextStyle,
    },
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

use crate::{
    ble::{HostView, HostsView},
    model::{AppState, Screen, Target},
};

const WIDTH: u32 = 240;
const BACKGROUND: Rgb565 = Rgb565::new(1, 3, 5);
const PANEL: Rgb565 = Rgb565::new(2, 7, 10);
const CYAN: Rgb565 = Rgb565::new(0, 58, 27);
const AMBER: Rgb565 = Rgb565::new(31, 43, 0);
const MUTED: Rgb565 = Rgb565::new(10, 22, 18);

pub fn draw<D>(
    display: &mut D,
    app: &AppState,
    hosts: HostsView,
    passkey: u32,
    feedback: Option<&str>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    display.clear(BACKGROUND)?;
    Rectangle::new(Point::zero(), Size::new(WIDTH, 24))
        .into_styled(PrimitiveStyle::with_fill(PANEL))
        .draw(display)?;

    let small = MonoTextStyle::new(&FONT_6X10, MUTED);
    let heading = MonoTextStyle::new(&FONT_6X10, CYAN);
    let large = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let accent = MonoTextStyle::new(&FONT_6X10, AMBER);

    let title = match app.screen {
        Screen::Send => "SEND",
        Screen::SelectShortcut => "SELECT SHORTCUT",
        Screen::SelectDevice => "SELECT DEVICE",
    };
    Text::new(title, Point::new(6, 16), heading).draw(display)?;
    let connection_text = format!("BT {}/2", hosts.connected_count());
    Text::with_alignment(
        &connection_text,
        Point::new(234, 16),
        heading,
        Alignment::Right,
    )
    .draw(display)?;

    match app.screen {
        Screen::Send => {
            let shortcut = app.shortcut();
            Text::new(shortcut.label, Point::new(8, 50), large).draw(display)?;
            Text::new(shortcut.combination, Point::new(8, 68), small).draw(display)?;
            draw_target(display, app.target, hosts, 88, heading, small)?;

            let status = feedback.unwrap_or_else(|| {
                if hosts.target_ready(app.target) {
                    "A: SEND"
                } else {
                    "TARGET OFFLINE"
                }
            });
            let status_style = if feedback.is_some() || hosts.target_ready(app.target) {
                accent
            } else {
                MonoTextStyle::new(&FONT_6X10, Rgb565::RED)
            };
            Text::new(status, Point::new(8, 110), status_style).draw(display)?;
        }
        Screen::SelectShortcut => {
            let shortcut = app.shortcut();
            Text::new(shortcut.label, Point::new(8, 57), large).draw(display)?;
            Text::new(shortcut.combination, Point::new(8, 80), small).draw(display)?;
            Text::new("A: NEXT SHORTCUT", Point::new(8, 110), accent).draw(display)?;
        }
        Screen::SelectDevice => {
            draw_target(display, app.target, hosts, 55, large, small)?;
            Text::new("A: NEXT DEVICE", Point::new(8, 110), accent).draw(display)?;
        }
    }

    let footer = format!("B: NEXT SCREEN   PIN {passkey:06}");
    Text::new(&footer, Point::new(6, 130), small).draw(display)?;
    Ok(())
}

fn draw_target<D>(
    display: &mut D,
    target: Target,
    hosts: HostsView,
    y: i32,
    title_style: MonoTextStyle<'_, Rgb565>,
    detail_style: MonoTextStyle<'_, Rgb565>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    match target {
        Target::All => {
            Text::new("ALL DEVICES", Point::new(8, y), title_style).draw(display)?;
            let detail = format!(
                "{} ready",
                hosts.slots.iter().filter(|host| host.ready).count()
            );
            Text::new(&detail, Point::new(8, y + 16), detail_style).draw(display)?;
        }
        Target::Device1 => draw_host(
            display,
            "DEVICE 1",
            hosts.slots[0],
            y,
            title_style,
            detail_style,
        )?,
        Target::Device2 => draw_host(
            display,
            "DEVICE 2",
            hosts.slots[1],
            y,
            title_style,
            detail_style,
        )?,
    }
    Ok(())
}

fn draw_host<D>(
    display: &mut D,
    label: &str,
    host: HostView,
    y: i32,
    title_style: MonoTextStyle<'_, Rgb565>,
    detail_style: MonoTextStyle<'_, Rgb565>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    Text::new(label, Point::new(8, y), title_style).draw(display)?;
    let identity = host
        .identity
        .map(|bytes| format!("{:02X}:{:02X}", bytes[4], bytes[5]))
        .unwrap_or_else(|| "UNPAIRED".to_owned());
    let status = if host.ready {
        "READY"
    } else if host.connected {
        "CONNECTING"
    } else {
        "OFFLINE"
    };
    let detail = format!("{identity}  {status}");
    Text::new(&detail, Point::new(8, y + 16), detail_style).draw(display)?;
    Ok(())
}
