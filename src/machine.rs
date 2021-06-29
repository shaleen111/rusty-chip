use std::fs::File;
use std::io::Read;
use std::time::{Instant, Duration};

use rand;
use rand::Rng;

use crate::fonts;

pub const VIDEO_WIDTH: usize = 64;
pub const VIDEO_HEIGHT: usize = 32;
const VIDEO_BUFFER_SIZE: usize = VIDEO_WIDTH * VIDEO_HEIGHT;

const ROM_MEMORY_START: u16 = 0x200;

const NUM_KEYS: u8 = 16;

pub struct Chip8
{
    registers: [u8; 16],
    memory: [u8; 4096],

    program_counter: u16,
    index: u16,

    stack: [u16; 16],
    stack_pointer: u8,

    time_since_timer_decrement: Instant,
    pub delay_timer: u8,
    pub sound_timer: u8,

    pub keypad: [bool; NUM_KEYS as usize],
    pub video: [bool; VIDEO_BUFFER_SIZE],
    pub redraw: bool,
}

impl Chip8
{
    pub fn new() -> Self
    {
        let mut c = Chip8
        {
            registers: [0; 16],
            memory: [0; 4096],

            program_counter: ROM_MEMORY_START,
            index: 0,

            stack: [0; 16],
            stack_pointer: 0,

            time_since_timer_decrement: Instant::now(),
            delay_timer: 0,
            sound_timer: 0,

            keypad: [false; 16],
            video: [false; VIDEO_BUFFER_SIZE],
            redraw: true,
        };

        for i in 0..fonts::FONTS.len()
        {
            c.memory[fonts::FONT_MEMORY_START as usize + i]  = fonts::FONTS[i];
        }

        c
    }

    pub fn load(&mut self, path: &str)
    {
        let mut file = File::open(path).expect("Error Opening File");
        let file_metadata = file.metadata().expect("Error Reading Metadata");
        // Can't use array since the size of array needs to be a known constant
        let mut buffer = vec![0; file_metadata.len() as usize];

        file.read(&mut buffer).expect("Buffer Overflow");

        for i in 0..buffer.len()
        {
            self.memory[ROM_MEMORY_START as usize + i] = buffer[i];
        }
    }

    pub fn cycle(&mut self)
    {
        self.fetch_and_execute();

        if self.time_since_timer_decrement.elapsed() > Duration::from_millis(20)
        {
            self.time_since_timer_decrement = Instant::now();

            if self.delay_timer > 0
            {
                self.delay_timer -= 1;
            }

            if self.sound_timer > 0
            {
                self.sound_timer -= 1;
            }
        }
    }

    fn mem_read_u16(&mut self) -> u16
    {
        let most_sig_byte = (self.memory[self.program_counter as usize] as u16) << 8;
        let least_sig_byte = self.memory[self.program_counter as usize + 1] as u16;

        self.program_counter += 2;

        most_sig_byte | least_sig_byte
    }

    fn check_keypad(&self) -> Option<u8>
    {
        for i in 0..NUM_KEYS
        {
            if self.keypad[i as usize]
            {
                return Some(i);
            }
        }
        None
    }

    fn opcode_not_found(opcode: u16)
    {
        panic!("Error Could Not Interpret Opcode: {:x}", opcode);
    }

    // To make the matching easier we can think of opcodes in general being made up of 3 parts:
    // FIRST NIBBLE - (OPTIONAL) ARGS / ADDITIONAL IDENTIFIER - ADDITIONAL IDENTIFIER
    // Eg - 00E0, 1nnn, 8xy7, Fx15
    fn fetch_and_execute(&mut self)
    {
        let opcode = self.mem_read_u16();

        let first = ((opcode & 0xF000) >> 12) as u8;

        match first
        {
            0x0 =>
            {
               let identifier = opcode & 0x000F;
               match identifier
               {
                    0x0 => self.video = [false; VIDEO_BUFFER_SIZE],

                    0xE =>
                    {
                        self.stack_pointer -= 1;
                        self.program_counter = self.stack[self.stack_pointer as usize];
                    }

                    _ => Chip8::opcode_not_found(opcode),
               }
            },

            0x1 =>
            {
                let nnn = opcode & 0x0FFF;
                self.program_counter = nnn;
            },

            0x2 =>
            {
                let nnn = opcode & 0x0FFF;
                self.stack[self.stack_pointer as usize] = self.program_counter;
                self.stack_pointer += 1;
                self.program_counter = nnn;
            },

            0x3 =>
            {
                let kk = (opcode & 0x00FF) as u8;
                let x = ((opcode & 0x0F00) >> 8) as usize;

                if self.registers[x] == kk
                {
                    self.program_counter += 2;
                }
            },

            0x4 =>
            {
                let kk = (opcode & 0x00FF) as u8;
                let x = ((opcode & 0x0F00) >> 8) as usize;

                if self.registers[x] != kk
                {
                    self.program_counter += 2;
                }
            },

            0x5 =>
            {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 4) as usize;

                if self.registers[x] == self.registers[y]
                {
                    self.program_counter += 2;
                }
            },

            0x6 =>
            {
                let kk = (opcode & 0x00FF) as u8;
                let x = ((opcode & 0x0F00) >> 8) as usize;

                self.registers[x] = kk;
            },

            0x7 =>
            {
                let kk = (opcode & 0x00FF) as u8;
                let x = ((opcode & 0x0F00) >> 8) as usize;

                self.registers[x] = self.registers[x].wrapping_add(kk);
            },

            0x8 =>
            {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 4) as usize;
                let identifier = opcode & 0x000F;

                match identifier
                {
                    0x0 => self.registers[x] = self.registers[y],

                    0x1 => self.registers[x] |= self.registers[y],

                    0x2 => self.registers[x] &= self.registers[y],

                    0x3 => self.registers[x] ^= self.registers[y],

                    0x4 =>
                    {
                        let (sum, overflow) = self.registers[x].overflowing_add(self.registers[y]);

                        if overflow
                        {
                            self.registers[0xF] = 1;
                        }
                        else
                        {
                            self.registers[0xF] = 0;
                        }

                        self.registers[x] = sum;
                    },

                    0x5 =>
                    {
                        let (diff, overflow) = self.registers[x].overflowing_sub(self.registers[y]);

                        if overflow
                        {
                            self.registers[0xF] = 0;
                        }
                        else
                        {
                            self.registers[0xF] = 1;
                        }

                        self.registers[x] = diff;
                    },

                    0x6 =>
                    {
                        self.registers[0xF] = self.registers[x] & 0x1;
                        self.registers[x] >>= 1;
                    },

                    0x7 =>
                    {
                        let (diff, overflow) = self.registers[y].overflowing_sub(self.registers[x]);

                        if overflow
                        {
                            self.registers[0xF] = 0;
                        }
                        else
                        {
                            self.registers[0xF] = 1;
                        }

                        self.registers[x] = diff;
                    },

                    0xE =>
                    {
                        self.registers[0xF] = (self.registers[x] & 0x80) >> 7;
                        self.registers[y] <<= 1;
                    }

                    _ => Chip8::opcode_not_found(opcode),
                }
            },

            0x9 =>
            {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 4) as usize;

                if self.registers[x] != self.registers[y]
                {
                    self.program_counter += 2;
                }
            },

            0xA => self.index = opcode & 0x0FFF,

            0xB => self.program_counter = (opcode & 0x0FFF) + self.registers[0] as u16,

            0xC =>
            {
                let mut rng = rand::thread_rng();
                let ran_byte = rng.gen::<u8>();

                let kk = (opcode & 0x00FF) as u8;
                let x = ((opcode & 0x0F00) >> 8) as usize;

                self.registers[x] = ran_byte & kk;
            },

            0xD =>
            {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let y = ((opcode & 0x00F0) >> 4) as usize;
                let n = (opcode & 0x000F) as usize;

                let x = self.registers[x];
                let y = self.registers[y];

                for i in 0..n
                {
                    let row_of_sprite = self.memory[self.index as usize + i];
                    for j in 0..8
                    {
                        let pixel = (row_of_sprite & (0x80 >> j)) != 0;

                        let video_pixel = &mut self.video[((y as usize + i) * VIDEO_WIDTH + (x as usize + j)) % (VIDEO_WIDTH * VIDEO_HEIGHT)];

                        if pixel
                        {
                            if *video_pixel
                            {
                                self.registers[0xF] = 1;
                            }
                            *video_pixel ^= true;
                            self.redraw = true;
                        }

                    }
                }
            },

            0xE =>
            {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let identifier = opcode & 0x00FF;

                match identifier
                {
                    0x9E =>
                    {
                        if self.keypad[self.registers[x] as usize]
                        {
                            self.program_counter += 2;
                        }
                    },

                    0xA1 =>
                    {
                        if !self.keypad[self.registers[x] as usize]
                        {
                            self.program_counter += 2;
                        }
                    },

                    _ => Chip8::opcode_not_found(opcode),
                }
            },

            0xF =>
            {
                let x = ((opcode & 0x0F00) >> 8) as usize;
                let identifier = opcode & 0x00FF;

                match identifier
                {
                    0x07 => self.registers[x] = self.delay_timer,

                    0x0A =>
                    {
                        match self.check_keypad()
                        {
                            Some(key) => self.registers[x] = key,
                            None => self.program_counter -= 2,
                        }
                    },

                    0x15 => self.delay_timer = self.registers[x],

                    0x18 => self.sound_timer = self.registers[x],

                    0x1E => self.index = self.index.wrapping_add(self.registers[x] as u16),

                    0x29 => self.index = fonts::FONT_MEMORY_START + (5 * self.registers[x] as u16),

                    0x33 =>
                    {
                        let mut value = self.registers[x];

                        self.memory[self.index as usize + 2] = value % 10;
                        value /= 10;

                        self.memory[self.index as usize + 1] = value % 10;
                        value /= 10;

                        self.memory[self.index as usize] = value % 10;
                    }

                    0x55 => self.memory[self.index as usize ..= self.index as usize + x].copy_from_slice(&self.registers[0 ..= x]),

                    0x65 => self.registers[0 ..= x].copy_from_slice(&self.memory[self.index as usize ..= self.index as usize + x]),

                    _ => Chip8::opcode_not_found(opcode),
                }
            }

            _ => Chip8::opcode_not_found(opcode),
        }
    }
}
