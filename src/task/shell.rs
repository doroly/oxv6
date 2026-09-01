//! Interactive shell task built on the line-buffered console driver.

use crate::drivers::console;
use crate::{print, println};
use core::hint::spin_loop;

const INPUT_CAPACITY: usize = 128;

/// Runs the kernel's interactive shell.
///
/// The console driver performs canonical line editing and only makes input
/// available after Enter, so this task only needs to consume complete lines.
pub(crate) fn shell_task() -> ! {
    let mut line = [0; INPUT_CAPACITY];

    print!("$ ");
    loop {
        let count = console::read(&mut line);
        if count == 0 {
            spin_loop();
            continue;
        }

        let command_len = line[..count]
            .iter()
            .position(|&ch| ch == b'\n')
            .unwrap_or_else(|| count);

        if command_len != 0 {
            print!("Running command: '");
            console::write(&line[..command_len]);
            println!("', unsupported!");
        }
        print!("$ ");
    }
}
