use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use clap::{arg, command, value_parser};
use interpreter::{C8, HEIGHT, WIDTH};

use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};

fn keys_to_key_codes(keys: &[Key]) -> Vec<usize> {
    keys.iter()
        .filter_map(|key| match key {
            Key::Key1 => Some(0x1),
            Key::Key2 => Some(0x2),
            Key::Key3 => Some(0x3),
            Key::Key4 => Some(0xc),

            Key::Q => Some(0x4),
            Key::W => Some(0x5),
            Key::E => Some(0x6),
            Key::R => Some(0xd),

            Key::A => Some(0x7),
            Key::S => Some(0x8),
            Key::D => Some(0x9),
            Key::F => Some(0xe),

            Key::Z => Some(0xa),
            Key::X => Some(0x0),
            Key::C => Some(0xb),
            Key::V => Some(0xf),
            _ => None,
        })
        .collect()
}

fn main() -> Result<()> {
    env_logger::init();

    let matches = command!()
        .arg(arg!(<FILE> "Chip-8 program to execute.").value_parser(value_parser!(PathBuf)))
        .get_matches();

    let file: &PathBuf = matches.get_one("FILE").expect("FILE is required");

    let mut c8 = C8::new();
    c8.load_program(file)?;
    for _ in 0..10 {
        c8.tick();
    }

    let mut window = Window::new(
        "C8",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: true,
            scale: minifb::Scale::FitScreen,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .context("Unable to create window")?;

    window.limit_update_rate(Some(Duration::from_secs(1) / 60));

    let mut buf = [0; WIDTH * HEIGHT];
    while window.is_open() && !window.is_key_down(Key::Escape) {
        keys_to_key_codes(&window.get_keys_pressed(KeyRepeat::No))
            .iter()
            .for_each(|k| c8.key_pressed(*k, true));

        keys_to_key_codes(&window.get_keys_released())
            .iter()
            .for_each(|k| c8.key_pressed(*k, false));

        for _ in 0..10 {
            c8.tick();
        }

        c8.render(&mut buf);

        window
            .update_with_buffer(&buf, WIDTH, HEIGHT)
            .context("Failed to update display.")?
    }

    Ok(())
}
