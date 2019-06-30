# Chip-8_Emulator
A simple Chip-8 emulator written in Rust

## Building Project
`cargo build --release`

## Usage
`./chip-8_emu <path_to_ch8_program>`

The Chip-8 uses a 16-key hexadecimal keypad. This emulator maps those keys to the leftmost part of a standard US keyboard.

Keys 5-9 change the colors of the display.
This emulator currently does not support sound because I could not find a crate for it.
