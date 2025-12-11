use std::env::args;
use std::io::{stdin, stdout, BufRead, Read, Write};
use std::thread;

use serde::{Deserialize, Serialize};
use tudelft_arm_qemu_runner::Runner;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    dx: i8,
    dy: i8,
    steps: u8,
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
        const FRAME_CAPACITY: usize = 16;
        let stdin = stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match parse_instruction(&line) {
                Some(msg) => {
                    let mut buf = [0u8; FRAME_CAPACITY];
                    match postcard::to_slice_cobs(&msg, &mut buf) {
                        Ok(encoded) => write_stream.write_all(encoded)?,
                        Err(err) => eprintln!("Failed to encode message: {err}"),
                    }
                }
                None => eprintln!("Unknown command. Use: move <dx> <dy> <steps>"),
            }
        }

        Ok(())
    });

    reader.join().expect("Reader thread panicked")?;
    writer.join().expect("Writer thread panicked")?;
    Ok(())
}

fn parse_instruction(line: &str) -> Option<Message> {
    let mut parts = line.split_whitespace();
    let cmd = parts.next()?;
    if !cmd.eq_ignore_ascii_case("move") {
        return None;
    }

    let dx = parts.next()?.parse().ok()?;
    let dy = parts.next()?.parse().ok()?;
    let steps = parts.next()?.parse().ok()?;

    Some(Message { dx, dy, steps })
}
