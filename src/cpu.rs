use std::fs::File;
use std::io::Read;

use rand;
use rand::Rng;

use crate::fonts;

const VIDEO_WIDTH: usize = 64;
const VIDEO_HEIGHT: usize = 32;
const VIDEO_BUFFER_SIZE: usize = VIDEO_WIDTH * VIDEO_HEIGHT;

const ROM_MEMORY_START: u16 = 0x200;

const NUM_KEYS: u8 = 16;

pub struct CHIP_8
{
    registers: [u8; 16],
    memory: [u8; 4096],
    index: u16,
    program_counter: u16,
    stack: [u16; 16],
    stack_pointer: u8,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [bool; NUM_KEYS as usize],
    video: [bool; VIDEO_BUFFER_SIZE],
    redraw: bool,
}

impl CHIP_8
{
    pub fn new() -> Self
    {
        let mut c = CHIP_8
        {
            registers: [0; 16],
            memory: [0; 4096],
            index: 0,
            program_counter: ROM_MEMORY_START,
            stack: [0; 16],
            stack_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [false; 16],
            video: [false; VIDEO_BUFFER_SIZE],
            redraw: false,
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

    fn fetch_and_execute(&mut self)
    {
        let opcode = self.mem_read_u16();

        let nib_1 = ((opcode & 0xF000) << 12) as u8;
        let nib_2 = ((opcode & 0x0F00) << 8) as u8;
        let nib_3 = ((opcode & 0x00F0) << 4) as u8;
        let nib_4 = (opcode & 0x000F) as u8;

        match (nib_1, nib_2, nib_3, nib_4)
        {
            (0, _, 0xE, 0) => self.video = [false; VIDEO_BUFFER_SIZE],

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
                let x = self.registers[x as usize] % VIDEO_WIDTH as u8;
                let y = self.registers[y as usize] % VIDEO_HEIGHT as u8;

                for i in 0..n
                {
                    let row_of_sprite = self.memory[(self.index + i as u16) as usize];
                    for j in 0..8
                    {
                        let pixel = (row_of_sprite & (0x80 >> j)) == 1;

                        let video_pixel = &mut self.video[(y + i) as usize * VIDEO_WIDTH + (x + j) as usize];

                        if pixel && *video_pixel
                        {
                            self.registers[0xF] = 1;
                            *video_pixel ^= true;
                        }

                    }
                }
            },

            (0xE, x, i, j) =>
            {
                match (i, j)
                {
                    (9, 0xE) =>
                    {
                        if self.keypad[self.registers[x as usize] as usize]
                        {
                            self.program_counter += 2;
                        }
                    },

                    (0xA, 1) =>
                    {
                        if !self.keypad[self.registers[x as usize] as usize]
                        {
                            self.program_counter += 2;
                        }
                    },

                    _ => panic!("Error Could Not Interpret Instruction {:x}", opcode),
                }
            },

            (0xF, x, i, j) =>
            {
                match (i, j)
                {
                    (0, 7) => self.registers[x as usize] = self.delay_timer,

                    (0, 0xA) =>
                    {
                        match self.check_keypad()
                        {
                            Some(key) => self.registers[x as usize] = key,
                            None => self.program_counter -= 2,
                        }
                    },

                    (1, 5) => self.delay_timer = self.registers[x as usize],

                    (1, 8) => self.sound_timer = self.registers[x as usize],

                    (1, 0xE) => self.index += self.registers[x as usize] as u16,

                    (2, 9) => self.index = fonts::FONT_MEMORY_START + (5 * self.registers[x as usize] as u16),

                    _ => panic!("Error Could Not Interpret Instrution {:x}", opcode),
                }
            }

            _ => panic!("Error Could Not Interpret Instruction {:x}", opcode),
        }
    }
}
