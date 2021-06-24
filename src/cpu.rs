use std::fs::File;
use std::io::Read;

use rand;
use rand::Rng;

use crate::fonts;

// 4 => r,g,b,a
// 64 * 32 => resolution of chip8
const VIDEO_BUFFER_SIZE: usize = 4 * 64 * 32;
const ROM_MEMORY_START: u16 = 0x200;

pub struct CPU
{
    registers: [u8; 16],
    memory: [u8; 4096],
    index: u16,
    program_counter: u16,
    stack: [u16; 16],
    stack_pointer: u8,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [u8; 16],
    video: [u8; VIDEO_BUFFER_SIZE],
}

impl CPU
{
    pub fn new() -> Self
    {
        let mut c = CPU
        {
            registers: [0; 16],
            memory: [0; 4096],
            index: 0,
            program_counter: ROM_MEMORY_START,
            stack: [0; 16],
            stack_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [0; 16],
            video: [0; VIDEO_BUFFER_SIZE],
        };

        // Load fonts
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

    fn mem_read_u16(&mut self) -> u16
    {
        let most_sig_byte = (self.memory[self.program_counter as usize] as u16) << 8;
        let least_sig_byte = self.memory[self.program_counter as usize + 1] as u16;

        self.program_counter += 2;

        most_sig_byte | least_sig_byte
    }

    fn fetch_and_execute(&mut self)
    {
        let opcode = self.mem_read_u16();

        let nib1 = (opcode & 0xF000) << 12;
        let nib2 = (opcode & 0x0F00) << 8;
        let nib3 = (opcode & 0x00F0) << 4;
        let nib4 = opcode & 0x000F;

        match (nib1, nib2, nib3, nib4)
        {
            (0, _, 0xE, 0) => self.video = [0; VIDEO_BUFFER_SIZE],

            (0, _, 0xE, 0xE) =>
            {
                self.stack_pointer -= 1;
                self.program_counter = self.stack[self.stack_pointer as usize];
            },

            (1, _, _, _) =>
            {
                let nnn = opcode & 0x0FFF;
                self.program_counter = nnn;
            },

            (2, _, _, _) =>
            {
                let nnn = opcode & 0x0FFF;
                self.stack[self.stack_pointer as usize] = self.program_counter;
                self.stack_pointer += 1;
                self.program_counter = nnn;
            },

            (3, x, _, _) =>
            {
                let kk = (opcode & 0x00FF) as u8;
                if self.registers[x as usize] == kk
                {
                    self.program_counter += 2;
                }
            },

            (4, x, _, _) =>
            {
                let kk = (opcode & 0x00FF) as u8;
                if self.registers[x as usize] != kk
                {
                    self.program_counter += 2;
                }
            },

            (5, x, y, _) =>
            {
                if self.registers[x as usize] == self.registers[y as usize]
                {
                    self.program_counter += 2;
                }
            },

            (6, x, _, _) =>
            {
                let kk = (opcode & 0x00FF) as u8;
                self.registers[x as usize] = kk;
            },

            (7, x, _, _) =>
            {
                let kk = (opcode & 0x00FF) as u8;
                self.registers[x as usize] += kk;
            },

            (8, x, y, i) =>
            {
                match i
                {
                    0 => self.registers[x as usize] = self.registers[y as usize],

                    1 => self.registers[x as usize] |= self.registers[y as usize],

                    2 => self.registers[x as usize] &= self.registers[y as usize],

                    3 => self.registers[x as usize] ^= self.registers[y as usize],

                    4 =>
                    {
                        let sum = self.registers[x as usize] as u16 + self.registers[y as usize] as u16;

                        if sum > 0xFF
                        {
                            self.registers[0xF] = 1;
                        }
                        else
                        {
                            self.registers[0xF] = 0;
                        }
                    },

                    5 =>
                    {
                        if self.registers[x as usize] > self.registers[y as usize]
                        {
                            self.registers[0xF] = 1;
                        }
                        else
                        {
                            self.registers[0xF] = 0;
                        }
                        self.registers[x as usize] -= self.registers[y as usize];
                    },

                    6 =>
                    {
                        self.registers[0xF] = self.registers[x as usize] & 0x1;
                        self.registers[x as usize] >>= 1;
                    },

                    7 =>
                    {
                        if self.registers[x as usize] < self.registers[y as usize]
                        {
                            self.registers[0xF] = 1;
                        }
                        else
                        {
                            self.registers[0xF] = 0;
                        }
                        self.registers[x as usize] = self.registers[y as usize] - self.registers[x as usize];
                    },

                    0xE =>
                    {
                        self.registers[0xF] = (self.registers[x as usize] & 0x80) >> 7;
                        self.registers[x as usize] <<= 1;
                    }

                    _ => panic!("Error Could Not Interpret Instruction {:x}", opcode),
                }
            },

            (9, x, y, _) =>
            {
                if self.registers[x as usize] != self.registers[y as usize]
                {
                    self.program_counter += 2;
                }
            },

            (0xA, _, _, _) => self.index = opcode & 0x0FFF,

            (0xB, _, _, _) => self.program_counter = (opcode & 0x0FFF) + self.registers[0] as u16,

            (0xC, x, _, _) =>
            {
                let mut rng = rand::thread_rng();
                let ran_byte = rng.gen::<u8>();

                let kk = (opcode & 0x00FF) as u8;

                self.registers[x as usize] = ran_byte & kk;
            },

            (0xD, x, y, n) =>
            {

            }

            _ => panic!("Error Could Not Interpret Instruction {:x}", opcode)
        }
    }
}
