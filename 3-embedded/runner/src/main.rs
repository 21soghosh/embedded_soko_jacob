use std::env::args;
use std::io::{stdout, Read, Write};

use tudelft_arm_qemu_runner::Runner;

fn main() -> color_eyre::Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let binary = args().nth(1).unwrap();
    let mut runner: Runner = Runner::new(&binary, false)?;

    // receive up to 4 bytes
    let mut buf = [0u8; 4];
    loop {
        let num_received = runner.stream.read(&mut buf)?;
        // get the portion we actually received
        let received = &buf[0..num_received];
        // print it (without newline)
        print!("{}", String::from_utf8_lossy(received));
        // flush to show what's printed without having to print a newline
        stdout().lock().flush().unwrap();

        // send back the bytes to the Stellaris board
        runner.stream.write_all(received)?;
    }
}
