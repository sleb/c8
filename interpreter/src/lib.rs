mod error;
mod timer;

use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use error::Result;
use log::debug;
use rand::random;
use timer::Timer;

/// Default mem size
const DEFAULT_MEM_SIZE: usize = 4096;

/// Font start addr
const FONT_START: usize = 0x50;

/// Typical program start address
const PROGRAM_START: usize = 0x200;

/// Display width
pub const WIDTH: usize = 64;

/// Display height
pub const HEIGHT: usize = 32;

/// RGB black
const BLACK: u32 = 0x00_00_00;

/// RGB green
const GREEN: u32 = 0x00_ff_00;

/// Font
const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct C8 {
    memory: [u8; DEFAULT_MEM_SIZE],
    pc: usize,
    i: usize,
    reg: [u8; 16],
    display: [[bool; HEIGHT]; WIDTH],
    stack: VecDeque<usize>,
    delay: Timer,
    sound: Timer,
    input: [bool; 16],
}

trait AsU16 {
    fn as_u16(&self) -> u16;
}

impl AsU16 for (u8, u8, u8) {
    fn as_u16(&self) -> u16 {
        (self.0 as u16) << 8 | (self.1 as u16) << 4 | self.2 as u16
    }
}

impl AsU16 for (u8, u8) {
    fn as_u16(&self) -> u16 {
        (0u8, self.0, self.1).as_u16()
    }
}

type Instruction = (u8, u8, u8, u8);

trait AsInstruction {
    fn as_instruction(&self) -> Instruction;
}

impl AsInstruction for (u8, u8) {
    fn as_instruction(&self) -> Instruction {
        (self.0 >> 4, self.0 & 0x0f, self.1 >> 4, self.1 & 0x0f)
    }
}

impl Default for C8 {
    fn default() -> Self {
        let mut c8 = C8 {
            memory: [0; DEFAULT_MEM_SIZE],
            pc: PROGRAM_START,
            i: 0,
            reg: [0; 16],
            display: [[false; HEIGHT]; WIDTH],
            stack: VecDeque::with_capacity(1024),
            delay: Timer::zero(),
            sound: Timer::zero(),
            input: [false; 16],
        };

        c8.memory[FONT_START..(FONT_START + FONT.len())].copy_from_slice(&FONT);

        c8
    }
}

impl C8 {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load_program(&mut self, path: &Path) -> Result<()> {
        let f = File::open(path)?;
        let mut b = BufReader::new(f);
        let mut buf = Vec::with_capacity(DEFAULT_MEM_SIZE);

        b.read_to_end(&mut buf)?;
        self.memory[PROGRAM_START..(PROGRAM_START + buf.len())].copy_from_slice(&buf);

        Ok(())
    }

    pub fn tick(&mut self) {
        self.delay.update();
        self.sound.update();

        let instruction = self.fetch();
        self.execute(instruction);
    }

    pub fn render(&mut self, frame: &mut [u32]) {
        for (i, pixel) in frame.iter_mut().enumerate() {
            let c = i % WIDTH;
            let r = i / WIDTH;

            *pixel = if self.display[c][r] { GREEN } else { BLACK };
        }
    }

    pub fn key_pressed(&mut self, key: usize, pressed: bool) {
        debug!("key {key:x} => {pressed}");
        self.input[key] = pressed;
    }

    fn execute(&mut self, instruction: Instruction) {
        match instruction {
            (0x0, 0x0, 0xe, 0x0) => self.clear_screen(),
            (0x0, 0x0, 0xe, 0xe) => self.ret(),
            (0x1, a, b, c) => self.jump((a, b, c).as_u16() as usize),
            (0x2, a, b, c) => self.sub((a, b, c).as_u16() as usize),
            (0x3, x, a, b) => self.skip_if(self.reg[x as usize] as u16 == (a, b).as_u16()),
            (0x4, x, a, b) => self.skip_if(self.reg[x as usize] as u16 != (a, b).as_u16()),
            (0x5, x, y, _) => self.skip_if(self.reg[x as usize] == self.reg[y as usize]),
            (0x6, x, a, b) => self.set_reg(x as usize, (a, b).as_u16()),
            (0x7, x, a, b) => self.add_to_reg(x as usize, (a, b).as_u16()),
            (0x8, x, y, 0) => self.assign(x as usize, y as usize),
            (0x8, x, y, 1) => self.or(x as usize, y as usize),
            (0x8, x, y, 2) => self.and(x as usize, y as usize),
            (0x8, x, y, 3) => self.xor(x as usize, y as usize),
            (0x8, x, y, 4) => self.plus(x as usize, y as usize),
            (0x8, x, y, 5) => self.minus(x as usize, y as usize),
            (0x8, x, _, 6) => self.shr(x as usize),
            (0x8, x, y, 7) => self.diff(x as usize, y as usize),
            (0x8, x, _, 0xe) => self.shl(x as usize),
            (0x9, x, y, _) => self.skip_if(self.reg[x as usize] != self.reg[y as usize]),
            (0xa, a, b, c) => self.set_index((a, b, c).as_u16() as usize),
            (0xc, x, a, b) => self.and_rand(x as usize, (a, b).as_u16()),
            (0xd, x, y, n) => self.draw(x as usize, y as usize, n),
            (0xe, x, 0x9, 0xe) => self.skip_if(self.input[self.reg[x as usize] as usize]),
            (0xe, x, 0xa, 0x1) => self.skip_if(!self.input[self.reg[x as usize] as usize]),
            (0xf, x, 0x0, 0xa) => self.get_key(x as usize),
            (0xf, x, 0x3, 0x3) => self.bcd(x as usize),
            (0xf, x, 0x5, 0x5) => self.dump(x as usize),
            (0xf, x, 0x6, 0x5) => self.load(x as usize),
            (0xf, x, 0x2, 0x9) => self.char(x as usize),
            (0xf, x, 0x1, 0x5) => self.delay(x as usize),
            (0xf, x, 0x1, 0x8) => self.sound(x as usize),
            (0xf, x, 0x0, 0x7) => self.get_delay(x as usize),
            (0xf, x, 0x1, 0xe) => self.add_to_index(x as usize),
            _ => panic!(
                "Unknown instruction {:X}{:X}{:X}{:X}",
                instruction.0, instruction.1, instruction.2, instruction.3
            ),
        }
    }

    fn fetch(&mut self) -> Instruction {
        let instruction = (self.memory[self.pc], self.memory[self.pc + 1]).as_instruction();
        self.pc += 2;
        debug!("{instruction:?}");
        instruction
    }

    fn clear_screen(&mut self) {
        for col in self.display.iter_mut() {
            for pixel in col.iter_mut() {
                *pixel = false;
            }
        }
    }

    fn jump(&mut self, to: usize) {
        self.pc = to;
    }

    fn set_reg(&mut self, x: usize, val: u16) {
        self.reg[x] = val as u8;
    }

    fn add_to_reg(&mut self, x: usize, val: u16) {
        self.reg[x] = self.reg[x].wrapping_add(val as u8);
    }

    fn set_index(&mut self, val: usize) {
        self.i = val;
    }

    fn add_to_index(&mut self, x: usize) {
        self.i += self.reg[x] as usize;
    }

    fn draw(&mut self, x: usize, y: usize, height: u8) {
        self.reg[0xf] = 0;

        let vx = self.reg[x] as usize % WIDTH;
        let vy = self.reg[y] as usize % HEIGHT;

        for r in 0..height as usize {
            let row = self.memory[self.i + r];
            for c in 0..8 {
                if ((row << c) & 0b10000000) > 0 {
                    if let Some(col) = self.display.get_mut(vx + c) {
                        if let Some(p) = col.get_mut(vy + r) {
                            *p ^= true;

                            if !*p {
                                self.reg[0xf] = 1;
                            }
                        }
                    }
                }
            }
        }
    }

    fn skip_if(&mut self, skip: bool) {
        if skip {
            self.fetch();
        }
    }

    fn sub(&mut self, at: usize) {
        self.stack.push_front(self.pc);
        self.pc = at;
    }

    fn ret(&mut self) {
        self.pc = self
            .stack
            .pop_front()
            .expect("Stack shouldn't be empty when `ret()` is called");
    }

    fn assign(&mut self, x: usize, y: usize) {
        self.reg[x] = self.reg[y];
    }

    fn or(&mut self, x: usize, y: usize) {
        self.reg[x] |= self.reg[y];
    }

    fn and(&mut self, x: usize, y: usize) {
        self.reg[x] &= self.reg[y];
    }

    fn xor(&mut self, x: usize, y: usize) {
        self.reg[x] ^= self.reg[y];
    }

    fn plus(&mut self, x: usize, y: usize) {
        let (sum, overflow) = self.reg[x].overflowing_add(self.reg[y]);
        self.reg[x] = sum;
        self.reg[0xf] = if overflow { 1 } else { 0 };
    }

    fn minus(&mut self, x: usize, y: usize) {
        let (diff, underflow) = self.reg[x].overflowing_sub(self.reg[y]);
        self.reg[x] = diff;
        self.reg[0xf] = if !underflow { 1 } else { 0 };
    }

    fn shr(&mut self, x: usize) {
        self.reg[0xf] = if self.reg[x] & 0b00000001 > 0 { 1 } else { 0 };
        self.reg[x] >>= 1;
    }

    fn diff(&mut self, x: usize, y: usize) {
        let (diff, underflow) = self.reg[y].overflowing_sub(self.reg[x]);
        self.reg[x] = diff;
        self.reg[0xf] = if !underflow { 1 } else { 0 };
    }

    fn shl(&mut self, x: usize) {
        self.reg[0xf] = if self.reg[x] & 0b10000000 > 0 { 1 } else { 0 };
        self.reg[x] <<= 1;
    }

    fn dump(&mut self, x: usize) {
        self.memory[self.i..=(self.i + x)].copy_from_slice(&self.reg[0..=x])
    }

    fn load(&mut self, x: usize) {
        self.reg[0..=x].copy_from_slice(&self.memory[self.i..=(self.i + x)]);
    }

    fn bcd(&mut self, x: usize) {
        let mut vx = self.reg[x];
        let mut digits = [0u8; 3];

        digits[0] = vx / 100;
        vx %= 100;
        digits[1] = vx / 10;
        vx %= 10;
        digits[2] = vx;
        self.memory[self.i..(self.i + 3)].copy_from_slice(&digits);
    }

    fn char(&mut self, x: usize) {
        self.i = FONT_START + (self.reg[x] as usize * 5); // each char is 5 bytes
    }

    fn delay(&mut self, x: usize) {
        self.delay = Timer::new(self.reg[x]);
    }

    fn get_delay(&mut self, x: usize) {
        self.reg[x] = self.delay.val();
    }

    fn sound(&mut self, x: usize) {
        self.sound = Timer::new(self.reg[x]);
    }

    fn get_key(&mut self, x: usize) {
        if let Some((i, _)) = self.input.iter().enumerate().find(|(_, k)| **k) {
            self.reg[x] = i as u8;
        } else {
            self.pc -= 2;
        }
    }

    fn and_rand(&mut self, x: usize, val: u16) {
        let r = random::<u8>() % 0xff;
        self.reg[x] = r & val as u8;
    }
}
