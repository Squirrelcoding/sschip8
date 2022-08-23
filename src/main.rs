mod lib;

use std::{io::prelude::*, path::Path};
use std::env;

fn main() {
    let mut bytes: Vec<u8> = Vec::new();
    let args: Vec<_> = env::args().collect();

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .open(Path::new(
            &args[1],
        ))
        .unwrap();
    file.read_to_end(&mut bytes).unwrap();

    let mut cpu = lib::cpu::CPU::new_with_memory(&bytes);

    cpu.run();
}
