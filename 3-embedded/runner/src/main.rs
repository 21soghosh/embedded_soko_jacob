use std::env::args;
use std::io::{stdin, stdout, BufRead, Read, Write};
use std::net::Shutdown;
use std::thread;

use serde::{Deserialize, Serialize};
use tudelft_arm_qemu_runner::Runner;

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    Move { dx: i8, dy: i8 },
    MoveTo { x: u8, y: u8 },
    SetDisplayMode(DisplayMode),
    Reset,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
enum DisplayMode {
    Trail,
    Steps,
}

#[derive(Serialize, Deserialize, Debug)]
struct Envelope {
    msg: Message,
    checksum: u8,
}

enum Command {
    Send(Message),
    Help,
    Exit,
}

fn main() -> color_eyre::Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let binary = args().nth(1).unwrap();
    let runner: Runner = Runner::new(&binary, false)?;

    let mut read_stream = runner.stream.try_clone()?;
    let mut write_stream = runner.stream.try_clone()?;

    let reader = thread::spawn(move || -> color_eyre::Result<()> {
        let mut buf = [0u8; 64];
        loop {
            let num_received = read_stream.read(&mut buf)?;
            if num_received == 0 {
                break;
            }
            let received = &buf[0..num_received];
            print!("{}", String::from_utf8_lossy(received));
            stdout().lock().flush().unwrap();
        }
        Ok(())
    });

    let writer = thread::spawn(move || -> color_eyre::Result<()> {
        const FRAME_CAPACITY: usize = 32;
        let stdin = stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match parse_instruction(&line) {
                Command::Send(msg) => {
                    let mut buf = [0u8; FRAME_CAPACITY];
                    match Envelope::new(msg) {
                        Ok(envelope) => match postcard::to_slice_cobs(&envelope, &mut buf) {
                            Ok(encoded) => write_stream.write_all(encoded)?,
                            Err(err) => eprintln!("Failed to encode message: {err}"),
                        },
                        Err(err) => eprintln!("Failed to prepare message: {err}"),
                    }
                }
                Command::Help => print_help(),
                Command::Exit => {
                    println!("Exiting runner...");
                    let _ = write_stream.shutdown(Shutdown::Both);
                    break;
                }
            }
        }

        Ok(())
    });

    reader.join().expect("Reader thread panicked")?;
    writer.join().expect("Writer thread panicked")?;
    Ok(())
}

impl Envelope {
    fn new(msg: Message) -> color_eyre::Result<Self> {
        let checksum = checksum_for(&msg)?;
        Ok(Self { msg, checksum })
    }
}

fn checksum_for(msg: &Message) -> color_eyre::Result<u8> {
    let mut buf = [0u8; 32];
    let encoded = postcard::to_slice(msg, &mut buf)?;
    Ok(encoded.iter().fold(0u8, |acc, b| acc.wrapping_add(*b)))
}

fn parse_instruction(line: &str) -> Command {
    let mut parts = line.split_whitespace();
    let Some(cmd) = parts.next() else {
        return Command::Help;
    };

    if cmd.eq_ignore_ascii_case("help") {
        return Command::Help;
    }

    if cmd.eq_ignore_ascii_case("exit") || cmd.eq_ignore_ascii_case("quit") {
        return Command::Exit;
    }

    if cmd.eq_ignore_ascii_case("move") {
        let dx = match parts.next().and_then(|p| p.parse().ok()) {
            Some(v) => v,
            None => return Command::Help,
        };
        let dy = match parts.next().and_then(|p| p.parse().ok()) {
            Some(v) => v,
            None => return Command::Help,
        };
        return Command::Send(Message::Move { dx, dy });
    }

    if cmd.eq_ignore_ascii_case("move_to") || cmd.eq_ignore_ascii_case("moveto") {
        let x = match parts.next().and_then(|p| p.parse().ok()) {
            Some(v) => v,
            None => return Command::Help,
        };
        let y = match parts.next().and_then(|p| p.parse().ok()) {
            Some(v) => v,
            None => return Command::Help,
        };

        return Command::Send(Message::MoveTo { x, y });
    }

    if cmd.eq_ignore_ascii_case("mode") {
        let Some(mode_str) = parts.next() else {
            return Command::Help;
        };
        let mode = if mode_str.eq_ignore_ascii_case("trail") {
            DisplayMode::Trail
        } else if mode_str.eq_ignore_ascii_case("steps") {
            DisplayMode::Steps
        } else {
            return Command::Help;
        };

        return Command::Send(Message::SetDisplayMode(mode));
    }

    if cmd.eq_ignore_ascii_case("reset") {
        return Command::Send(Message::Reset);
    }

    Command::Help
}

fn print_help() {
    println!("Commands:");
    println!("  move <dx> <dy>          - relative move, stepwise");
    println!("  move_to <x> <y>         - move to absolute pixel");
    println!("  mode trail|steps        - switch display mode");
    println!("  reset                   - reset position, steps, and trail");
    println!("  help                    - show this help");
    println!("  exit|quit               - stop the runner");
}
