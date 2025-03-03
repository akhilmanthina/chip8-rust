use std::error::Error;
use std::env;
use std::fs;
use std::time::{Duration, Instant};

use minifb::{Key, Window, WindowOptions};
use core::Core;

const WIDTH: usize = 640;
const HEIGHT: usize = 320;

const FPS: usize = 60;
const CPS: usize = 660;

const TIMER_FREQUENCY: u64 = 60;
const TIMER_PERIOD: Duration = Duration::from_nanos((1_000_000_000) / TIMER_FREQUENCY);

fn get_program(args: &[String]) -> Result<Vec<u8>, Box<dyn Error>> {
    if args.len() < 2 {
        return Err("Not enough arguments".into());
    }
    let file_path = format!("../roms/{}", &args[1]);
    Ok(fs::read(file_path)?)
}

fn write_to_buffer(display: &[bool], buffer: &mut Vec<u32>) {
    for (i, pixel) in buffer.iter_mut().enumerate() {
        let (x, y) = (i % 640, i / 640);
        let original_pixel = display[(64 * (y/10)) + (x/10)];

        *pixel = match original_pixel {
            true => 0xFFFFFFFF,
            false => 0x00000000
        }
    }
}

fn keymap(key: &Key) -> Option<u8> {
    let translated = match key {
        Key::Key1 => 0x1,
        Key::Key2 => 0x2,
        Key::Key3 => 0x3,
        Key::Key4 => 0xC,
        Key::Q => 0x4,
        Key::W => 0x5,
        Key::E => 0x6,
        Key::R => 0xD,
        Key::A => 0x7,
        Key::S => 0x8,
        Key::D => 0x9,
        Key::F => 0xE,
        Key::Z => 0xA,
        Key::X => 0x0,
        Key::C => 0xB,
        Key::V => 0xF,
        _ => return None
    };
    Some(translated)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let program = get_program(&args)?;
    let legacy_mode = args.iter().any(|arg| arg == "--legacy");

    let mut core = Core::new(&program, legacy_mode);
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Chip8 emulator - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )?;

    let mut prev_time = Instant::now();
    
    window.set_target_fps(FPS);
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let cycles_per_frame = CPS / FPS;
        let all_keys_pressed = window.get_keys_pressed(minifb::KeyRepeat::Yes);
        let keys: Vec<u8> = all_keys_pressed
            .iter()
            .filter_map(|x| keymap(x))
            .collect();

        for _ in 0..cycles_per_frame {
            let now = Instant::now();
            if now - prev_time >= TIMER_PERIOD {
                core.decrement_timers();
                prev_time = now;
            }
            core.cycle(&keys);
        }
        write_to_buffer(&core.display, &mut buffer);
        window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;
    }
    
    Ok(())
}

