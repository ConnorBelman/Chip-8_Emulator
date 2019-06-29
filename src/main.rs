extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate rand;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL };
use rand::Rng;
use graphics::*;
use ambisonic::{rodio, AmbisonicBuilder};
use std::{sync, thread, time};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::env;
use std::num::Wrapping;

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const FONTSET: [u8; 80] =  [
    0xF0, 0x90, 0x90, 0x90, 0xF0,     // 0
    0x20, 0x60, 0x20, 0x20, 0x70,     // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0,     // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0,     // 3
    0x90, 0x90, 0xF0, 0x10, 0x10,     // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0,     // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0,     // 6
    0xF0, 0x10, 0x20, 0x40, 0x40,     // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0,     // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0,     // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90,     // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0,     // B
    0xF0, 0x80, 0x80, 0x80, 0xF0,     // C
    0xE0, 0x90, 0x90, 0x90, 0xE0,     // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0,     // E
    0xF0, 0x80, 0xF0, 0x80, 0x80      // F
];

struct Cpu {
	opcode: u16,            // Current Opcode
	v: [u8; 16],            // General Purpose Registers
	i: u16,                 // Index Register
	st: Arc<Mutex<u8>>,     // Sound Timer
	dt: Arc<Mutex<u8>>,     // Delay Timer
	pc: u16,                // Program Counter
	sp: u8,                 // Stack Pointer
    stack: [u16; 16],       // Stack
	memory: [u8; 4096],     // 4096 bytes of memory

    screen: [[u8; 64]; 32], // Screen Data
    foreground: [f32; 4],   // Foreground color
    background: [f32; 4],   // Background color

    key: [u8; 16],          // Current Key Pressed
}

impl Cpu {
    fn new() -> Cpu {
        Cpu {
            opcode: 0,
            v: [0; 16],
            i: 0x200,
            st: Arc::new(Mutex::new(0)),
            dt: Arc::new(Mutex::new(0)),
            pc: 0x200,
            sp: 0,
            stack: [0u16; 16],
            memory: [0; 4096],
            screen: [[0u8; 64]; 32],
            foreground: WHITE,
            background: BLACK,
            key: [0; 16],
        }
    }

    fn load_fontset(&mut self) {
        for i in 0..80 {
            self.memory[i] = FONTSET[i];
        }
    }

    fn load_program(&mut self, buffer: &[u8; 3584]) {
        for i in 0..3584 {
            self.memory[i + 0x200] = buffer[i];
        }
    }

    fn update_timers(&mut self, tx: Arc<Mutex<Sender<bool>>>) {
        let st = Arc::clone(&self.st);
        let dt = Arc::clone(&self.dt);
        thread::spawn(move || {
            let mut sound_timer = st.lock().unwrap();
            let mut delay_timer = dt.lock().unwrap();
            if *sound_timer > 0 {
                *sound_timer -= 1;
            }
            if *delay_timer > 0 {
                *delay_timer -= 1;
            }
            thread::sleep(Duration::from_millis(17));
            let sender = tx.lock().unwrap();
            sender.send(true);
        });
    }

    fn key_press<E: GenericEvent>(&mut self, e: &E) {
        if let Some(Button::Keyboard(key)) = e.press_args() {
            match key {
                Key::D1 => self.key[0x1] = 1,
                Key::D2 => self.key[0x2] = 1,
                Key::D3 => self.key[0x3] = 1,
                Key::D4 => self.key[0xC] = 1,
                Key::Q  => self.key[0x4] = 1,
                Key::W  => self.key[0x5] = 1,
                Key::E  => self.key[0x6] = 1,
                Key::R  => self.key[0xD] = 1,
                Key::A  => self.key[0x7] = 1,
                Key::S  => self.key[0x8] = 1,
                Key::D  => self.key[0x9] = 1,
                Key::F  => self.key[0xE] = 1,
                Key::Z  => self.key[0xA] = 1,
                Key::X  => self.key[0x0] = 1,
                Key::C  => self.key[0xB] = 1,
                Key::V  => self.key[0xF] = 1,
                _ => ()
            }
        }
        if let Some(Button::Keyboard(key)) = e.release_args() {
            match key {
                Key::D1 => self.key[0x1] = 0,
                Key::D2 => self.key[0x2] = 0,
                Key::D3 => self.key[0x3] = 0,
                Key::D4 => self.key[0xC] = 0,
                Key::Q  => self.key[0x4] = 0,
                Key::W  => self.key[0x5] = 0,
                Key::E  => self.key[0x6] = 0,
                Key::R  => self.key[0xD] = 0,
                Key::A  => self.key[0x7] = 0,
                Key::S  => self.key[0x8] = 0,
                Key::D  => self.key[0x9] = 0,
                Key::F  => self.key[0xE] = 0,
                Key::Z  => self.key[0xA] = 0,
                Key::X  => self.key[0x0] = 0,
                Key::C  => self.key[0xB] = 0,
                Key::V  => self.key[0xF] = 0,
                _ => ()
            }
        }
    }

    fn wait_for_key_press<E: GenericEvent>(&mut self, e: &E, x: usize) {
        if let Some(Button::Keyboard(key)) = e.press_args() {
            match key {
                Key::D1 => {
                    self.v[x] = 0x1;
                    self.pc += 2;
                },
                Key::D2 => {
                    self.v[x] = 0x2;
                    self.pc += 2;
                },
                Key::D3 => {
                    self.v[x] = 0x3;
                    self.pc += 2;
                },
                Key::D4 => {
                    self.v[x] = 0xC;
                    self.pc += 2;
                },
                Key::Q  => {
                    self.v[x] = 0x4;
                    self.pc += 2;
                },
                Key::W  => {
                    self.v[x] = 0x5;
                    self.pc += 2;
                },
                Key::E  => {
                    self.v[x] = 0x6;
                    self.pc += 2;
                },
                Key::R  => {
                    self.v[x] = 0xD;
                    self.pc += 2;
                },
                Key::A  => {
                    self.v[x] = 0x7;
                    self.pc += 2;
                },
                Key::S  => {
                    self.v[x] = 0x8;
                    self.pc += 2;
                },
                Key::D  => {
                    self.v[x] = 0x9;
                    self.pc += 2;
                },
                Key::F  => {
                    self.v[x] = 0xE;
                    self.pc += 2;
                },
                Key::Z  => {
                    self.v[x] = 0xA;
                    self.pc += 2;
                },
                Key::X  => {
                    self.v[x] = 0x0;
                    self.pc += 2;
                },
                Key::C  => {
                    self.v[x] = 0xB;
                    self.pc += 2;
                },
                Key::V  => {
                    self.v[x] = 0xF;
                    self.pc += 2;
                },
                _ => ()
            }
        }
    }

    pub fn draw<G: Graphics>(&self, c: &Context, g: &mut G) {
        let square = rectangle::square(0.0, 0.0, 10.0);
        for yz in 0..32 {
            for xz in 0..64 {
                if self.screen[yz][xz] == 1 {
                    let transform = c.transform.trans(10.0 * (xz as f64), 10.0 * (yz as f64));
                    rectangle(self.foreground, square, transform, g);
                } else {
                    let transform = c.transform.trans(10.0 * (xz as f64), 10.0 * (yz as f64));
                    rectangle(self.background, square, transform, g);
                }
            }
        }
    }

    fn fetch_opcode(&mut self) {
        self.opcode = (self.memory[self.pc as usize] as u16) << 8 | (self.memory[self.pc as usize + 1] as u16);
    }

    fn emulate_cycle<E: GenericEvent>(&mut self, e: &E) {
        match (self.opcode & 0xF000) >> 12 {
            // 00E?
            0x0 =>
                match self.opcode {
                    // 0000: Does nothing
                    0x0000 => self.pc += 2,
                    // 00E0: Clears the display
                    0x00E0 => {
                        self.screen = [[0u8; 64]; 32];
                        self.pc += 2;
                    },
                    // 00EE: Return from a subroutine
                    0x00EE => {
                        //println!("00EE Before, sp: {:x}, pc: {:x}", self.sp, self.pc);
                        self.sp -= 1;
                        self.pc = self.stack[self.sp as usize] + 2;
                        //println!("00EE After,  sp: {:x}, pc: {:x}", self.sp, self.pc);
                    },
                    _ => self.pc += 2
                },
            // 1nnn: Jumps to location nnn in memory
            0x1 => self.pc = (self.opcode & 0x0FFF) as u16,
            // 2nnn: Calls subroutine at nnn
            0x2 => {
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = self.opcode & 0x0FFF;
            },
            // 3xkk: Skips the next instruction if vx == kk
            0x3 => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let kk = self.opcode & 0x00FF;
                if (self.v[x]) == kk as u8 {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            },
            // 4xkk: Skips the next instruction if vx != kk
            0x4 => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let kk = self.opcode & 0x00FF;
                if (self.v[x]) != kk as u8 {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            },
            // 5xy0: Skips the next instruction if vx == vy
            0x5 =>
                match self.opcode & 0x000F {
                    0x0 => {
                        let x = ((self.opcode & 0x0F00) >> 8) as usize;
                        let y = ((self.opcode & 0x00F0) >> 4) as usize;
                        self.pc += if self.v[x] == self.v[y] { 4 } else { 2 };
                    },
                    _ => self.pc += 2
                },
            // 6xkk: Loads the value kk into vx
            0x6 => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let kk = (self.opcode & 0x00FF) as u8;
                self.v[x] = kk;
                self.pc += 2;
            },
            // 7xkk: Adds kk to the value in vx and stores the result in vx
            0x7 => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let kk = (self.opcode & 0x00FF) as u8;
                self.v[x] = (Wrapping(self.v[x]) + Wrapping(kk)).0;
                self.pc += 2;
            },
            // 8xy?
            0x8 => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let y = ((self.opcode & 0x00F0) >> 4) as usize;
                match self.opcode & 0x000F {
                    // 8xy0: Stores the value in vy into vx
                    0x0 => self.v[x] = self.v[y],
                    // 8xy1: Performs bitwise OR on vx and xy, stores the result in vx
                    0x1 => self.v[x] = self.v[x] | self.v[y],
                    // 8xy2: Performs bitwise AND on vx and vy, stores the result in vx
                    0x2 => self.v[x] = self.v[x] & self.v[y],
                    // 8xy3: Performs bitwise XOR on vx and vy, stores the result in vx
                    0x3 => self.v[x] = self.v[x] ^ self.v[y],
                    // 8xy4: Adds vx and xy, stores the result in vx. Sets vF if the result is greater than 255
                    0x4 => {
                        self.v[0xF] = if self.v[x] as u16 + self.v[y] as u16 > 255 { 1 } else { 0 };
                        self.v[x] = (Wrapping(self.v[x]) + Wrapping(self.v[y])).0;
                    },
                    // 8xy5: Subtracts vy from vx, stores the result in vx. Sets vF is vx > vy
                    0x5 => {
                        self.v[0xF] = if self.v[x] > self.v[y] { 1 } else { 0 };
                        self.v[x] = (Wrapping(self.v[x]) - Wrapping(self.v[y])).0;
                    },
                    // 8xy6: Sets vF if the least significant bit of vx is 1, then shifts vx right by one
                    0x6 => {
                        self.v[0xF] = if (self.v[x] & 1) == 1 { 1 } else { 0 };
                        self.v[x] = self.v[x] >> 1;
                    },
                    // 8xy7: Subtracts vx from vy, stores the result in vx. Sets vF is vy > vx
                    0x7 => {
                        self.v[0xF] = if self.v[y] > self.v[x] { 1 } else { 0 };
                        self.v[x] = (Wrapping(self.v[y]) - Wrapping(self.v[x])).0;
                    },
                    // 8xyE: Sets vF if the most significant bit of vx is 1, then shifts vx left by one
                    0xE => {
                        self.v[0xF] = if (self.v[x] & 128) == 128 { 1 } else { 0 };
                        self.v[x] = self.v[x] << 1;
                    },
                    _ => ()
                }
                self.pc += 2;
            },
            // 9xy0: Skips the next instruction if vx == vy
            0x9 => match self.opcode & 0x000F {
                0x0 => {
                    let x = ((self.opcode & 0x0F00) >> 8) as usize;
                    let y = ((self.opcode & 0x00F0) >> 4) as usize;
                    self.pc += if self.v[x] != self.v[y] { 4 } else { 2 };
                },
                _ => self.pc += 2
            },
            // Annn: Sets the value of register i to nnn
            0xA => {
                self.i = (self.opcode & 0x0FFF) as u16;
                self.pc += 2;
            },
            // Bnnn: Jumps to location v0 + nnn in memory
            0xB => self.pc = ((self.opcode & 0x0FFF) as u16) + (self.v[0x0] as u16),
            // Cxkk: Performs logical AND on kk and a random number from 0-255, stores the result in vx
            0xC => {
                let mut rng = rand::thread_rng();
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let kk = (self.opcode & 0x00FF) as u8;
                let rand = rng.gen_range(0,256) as u8;
                self.v[x] = kk & rand;
                self.pc += 2;
            },
            // Dxyn: Draws a sprite to the screen
            0xD => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let y = ((self.opcode & 0x00F0) >> 4) as usize;
                let n = (self.opcode & 0x000F) as usize;
                let mut pixel: u8;
                self.v[0xF] = 0;
                for height in 0..n {
                    pixel = self.memory[self.i as usize + height];
                    for width in 0..8 {
                        if pixel & (0x80 >> width) != 0 {
                            if self.screen[self.v[y] as usize + height][self.v[x] as usize + width] == 1 {
                                self.v[0xF] = 1;
                            }
                            self.screen[self.v[y] as usize + height][self.v[x] as usize + width] ^= 1;
                        }
                    }
                }
                self.pc += 2;
            }
            // Ex??:
            0xE => match self.opcode & 0x00FF {
                // Ex9E: Skip next instruction if key with the value of vx is pressed
                0x9E => {
                    let x = ((self.opcode & 0x0F00) >> 8) as usize;
                    self.pc += if self.key[self.v[x] as usize] == 1 { 4 } else { 2 };
                },
                // ExA1: Skip next instruction if key with the value of vx is not pressed
                0xA1 => {
                    let x = ((self.opcode & 0x0F00) >> 8) as usize;
                    self.pc += if self.key[self.v[x] as usize] == 1 { 2 } else { 4 }
                },
                _ => ()
            },
            0xF => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                match self.opcode & 0x00FF {
                    // Fx07: Loads the value of dt into vx
                    0x07 => {
                        let dt = self.dt.clone();
                        let mut delay_timer = dt.lock().unwrap();
                        self.v[x] = *delay_timer;
                        self.pc += 2;
                    },
                    // Fx0A: Wait for key press, stores value of key in vx
                    0x0A => self.wait_for_key_press(e, x),
                    // Fx15: Loads the value of vx into dt
                    0x15 => {
                        let dt = self.dt.clone();
                        let mut delay_timer = dt.lock().unwrap();
                        *delay_timer = self.v[x];
                        self.pc += 2;
                    },
                    // Fx18: Loads the value of vx into st
                    0x18 => {
                        let st = self.st.clone();
                        let mut sound_timer = st.lock().unwrap();
                        *sound_timer = self.v[x];
                        self.pc += 2;
                    },
                    // Fx1E: Adds i and vx, stores the result in i
                    0x1E => {
                        self.i = self.i + (self.v[x] as u16);
                        self.pc += 2;
                    },
                    // Fx29: Sets i = location of sprite for digit vx
                    0x29 => {
                        let x = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.i = self.v[x] as u16 * 5;
                        self.pc += 2;
                    },
                    // Fx33: Stores BCD representation of vx in memory
                    0x33 => {
                        self.memory[self.i as usize] = self.v[x] / 100;
                        self.memory[(self.i + 1) as usize] = (self.v[x] / 10) % 10;
                        self.memory[(self.i + 2) as usize] = (self.v[x] % 100) % 10;
                        self.pc += 2;
                    },
                    // Fx55: Stores registers v0 through vx in memory starting at location i
                    0x55 => {
                        for x in 0 ..= x {
                            self.memory[self.i as usize + x] = self.v[x];
                        }
                        self.i = self.i + (x as u16) + 1;
                        self.pc += 2;
                    },
                    // Fx65: Reads registers v0 through vx from memory starting at location i
                    0x65 => {
                        for x in 0 ..= x {
                            self.v[x] = self.memory[self.i as usize + x];
                        }
                        self.i = self.i + (x as u16) + 1;
                        self.pc += 2;
                    },
                    _ => self.pc += 2
                };
            },
            _ => self.pc += 2
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let file = &args[1];
    let mut f = File::open(file)?;

    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    let mut window: Window = WindowSettings::new(
            "Chip-8 Emulator",
            [640, 320]
        )
        .graphics_api(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut buffer = [0; 3584];
    f.read(&mut buffer)?;

    let mut cpu = Cpu::new();
    cpu.load_fontset();
    cpu.load_program(&buffer);

    let mut events = Events::new(EventSettings::new());
    let mut gl = GlGraphics::new(opengl);
    // Setup for data channel between
    let (tx, rx) = mpsc::channel();
    let sender = Arc::new(Mutex::new(tx));
    cpu.update_timers(sender.clone(), );
    while let Some(e) = events.next(&mut window) {
        cpu.fetch_opcode();
        cpu.emulate_cycle(&e);
        if let Some(args) = e.render_args() {
            gl.draw(args.viewport(), |c, g| {
                cpu.draw(&c, g);
            });
        }
        if let Some(Button::Keyboard(key)) = e.press_args() {
            cpu.key_press(&e);
        }
        if let Some(Button::Keyboard(key)) = e.release_args() {
            cpu.key_press(&e);
        }
        match rx.try_recv() {
            Ok(_) => cpu.update_timers(sender.clone()),
            Err(_) => ()
        };
        thread::sleep(time::Duration::from_millis(2));
    }
    Ok(())
}