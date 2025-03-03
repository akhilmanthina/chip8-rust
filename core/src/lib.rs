use std::collections::VecDeque;
use rand::random;

const RAM_SIZE: usize = 4096;
const NUM_REG: usize = 16;
const START_ADDR: u16 = 0x200;

const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;

pub struct Core {
    pc: u16,
    ram: [u8; RAM_SIZE],
    stack: VecDeque<u16>,
    pub display: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    d_timer: u8,
    s_timer: u8,
    i_reg: u16,
    v_reg: [u8; NUM_REG],
    // legacy mode for programs written for original COSMAC VIP interpreter
    legacy: bool,
}

impl Core {
    pub fn new(program: &[u8], legacy: bool) -> Self {
        let mut new_core = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            stack: VecDeque::new(),
            display: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            d_timer: 0,
            s_timer: 0,
            i_reg: 0,
            v_reg: [0; NUM_REG],
            legacy: legacy,
        };
        new_core.load_sprites();
        new_core.load_rom(program);
        new_core
    }

    fn load_rom(&mut self, program: &[u8]) {
        // 0x200 is where instructions start in chip8
        // From 0x200 to the end of program length in ram, copy in the program
        self.ram[0x200..0x200 + program.len()].copy_from_slice(program);
    }

    fn load_sprites(&mut self) {
        self.ram[0x50..0xA0].copy_from_slice(
            &[0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
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
            0xF0, 0x80, 0xF0, 0x80, 0x80] // F
        );
    }

    pub fn decrement_timers(&mut self) {
        if self.d_timer > 0 {self.d_timer -= 1};
        if self.s_timer > 0 {self.s_timer -= 1};
    }

    pub fn cycle(&mut self, keys: &[u8]) {
        let instruction = self.fetch();
        self.decode_and_exec(instruction, keys);
    }

    fn fetch(&mut self) -> u16 {
        // Each instruction takes two bytes
        let mut high_byte: u16 = self.ram[self.pc as usize].into();
        let low_byte: u16 = self.ram[self.pc as usize + 1].into();
        
        // Increment pc here to avoid having to do this later
        self.pc += 2;
        high_byte = high_byte << 8;
        high_byte + low_byte
    }

    fn decode_and_exec(&mut self, instruction: u16, keys: &[u8]) {
        // 1st 4 bit "nibble"
        let nibble = (instruction & 0xF000) >> 12;
        let rest = instruction & 0x0FFF;
        match nibble {
            // Some instructions out of order to group similar opcodes
            0x0 => match rest {
                    0x000 => self.noop(),
                    0x0E0 => self.clear_screen(),
                    0x0EE => self.ret_subroutine(),

                    _ => {dbg!("invalid op"); dbg!(instruction);}
                },
            0x1 => self.jump(rest),
            0x2 => self.call(rest),
            0x3 => self.skip_eq_val(rest),
            0x4 => self.skip_neq_val(rest),
            0x5 => self.skip_eq_reg(rest),
            0x9 => self.skip_neq_reg(rest),
            0x6 => self.set_v(rest),
            0x7 => self.add_v_no_carry(rest),
            // Logical and arithmetic instructions determined by last nibble
            0x8 => match rest & 0x00F {
                0x0 => self.set(rest),
                0x1 => self.or(rest),
                0x2 => self.and(rest),
                0x3 => self.xor(rest),
                0x4 => self.add(rest),
                //sub function handles which to exec
                0x5 => self.sub(rest), //sub x-y
                0x7 => self.sub(rest), //sub y-x
                0x6 => self.right_shift(rest),
                0xE => self.left_shift(rest),
                _ => {dbg!("invalid op"); dbg!(instruction);}
            }
            0xA => self.set_i(rest),
            0xB => self.jump_offset(rest),
            0xC => self.rand(rest),
            0xD => self.draw_sprite(rest),
            0xE => self.key_skip(rest, keys), //skip if key
            0xF => match rest & 0x0FF {
                //timers
                0x07 => self.v_reg[((rest & 0xF00) >> 8) as usize] = self.d_timer,
                0x15 => self.d_timer = self.v_reg[((rest & 0xF00) >> 8) as usize],
                0x18 => self.s_timer = self.v_reg[((rest & 0xF00) >> 8) as usize],
                
                0x0A => self.await_key(rest, keys),
                0x1E => self.i_reg += self.v_reg[((rest & 0xF00) >> 8) as usize] as u16,
                0x29 => self.set_i_font(rest),
                0x33 => self.bcd(rest),
                0x55 => self.store_mem(rest),
                0x65 => self.fill_mem(rest),
                _ => {dbg!("invalid op"); dbg!(instruction);}
            },

            _ => {dbg!("invalid op"); dbg!(instruction);}
        };
    }

    //TODO ensure ordering of instructions based on opcode sheet

    fn noop(&self) {}
    
    fn clear_screen(&mut self) {
        self.display = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    fn jump(&mut self, addr: u16) {
        self.pc = addr;
    }

    fn call(&mut self, addr: u16) {
        self.stack.push_back(self.pc);
        self.pc = addr;
    }

    fn ret_subroutine(&mut self) {
        self.pc = self.stack.pop_back().expect("Stack underflow: expected return address in stack");
    }
    
    fn skip_eq_val(&mut self, rest: u16) {
        let x= ((rest & 0xF00) >> 8) as usize;
        let val: u8 = ((rest & 0x0FF)) as u8;

        if self.v_reg[x] == val {
            self.pc += 2;
        }
    }

    fn skip_neq_val(&mut self, rest: u16) {
        let x= ((rest & 0xF00) >> 8) as usize;
        let val: u8 = (rest & 0x0FF) as u8;

        if self.v_reg[x] != val {
            self.pc += 2;
        }
    }

    fn skip_eq_reg(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let y = ((rest & 0x0F0) >> 4) as usize;

        if self.v_reg[x] == self.v_reg[y] {
            self.pc += 2;
        }
    }

    fn skip_neq_reg(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let y = ((rest & 0x0F0) >> 4) as usize;

        if self.v_reg[x] != self.v_reg[y] {
            self.pc += 2;
        }
    }

    fn set_v(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        self.v_reg[x] = (rest & 0xFF) as u8;
    }

    fn add_v_no_carry(&mut self, rest: u16){
        let x = ((rest & 0xF00) >> 8) as usize;
        self.v_reg[x] = self.v_reg[x].wrapping_add((rest & 0xFF) as u8);
    }

    fn set(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let y = ((rest & 0x0F0) >> 4) as usize;
        self.v_reg[x] = self.v_reg[y];
    }

    fn or(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let y = ((rest & 0x0F0) >> 4) as usize;
        self.v_reg[x] = self.v_reg[x] | self.v_reg[y];
    }

    fn and(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let y = ((rest & 0x0F0) >> 4) as usize;
        self.v_reg[x] = self.v_reg[x] & self.v_reg[y];
    }

    fn xor(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let y = ((rest & 0x0F0) >> 4) as usize;
        self.v_reg[x] = self.v_reg[x] ^ self.v_reg[y];
    }

    fn add(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let y = ((rest & 0x0F0) >> 4) as usize;
        let sum = self.v_reg[x] as u16 + self.v_reg[y] as u16;

        self.v_reg[0xF] = if sum < 256 {0} else {1};
        self.v_reg[x] = (sum & 0x00FF) as u8;
    }

    fn sub(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let y = ((rest & 0x0F0) >> 4) as usize;

        let minuend = if (rest & 0x00F) == 5 { x } else { y };
        let subtrahend = if (rest & 0x00F) == 5 { y } else { x };

        // set VF to 1 if there's no borrow, i.e. left > right
        self.v_reg[0xF] = if self.v_reg[minuend] >= self.v_reg[subtrahend] { 1 } else { 0 };
        self.v_reg[x] = self.v_reg[minuend].wrapping_sub(self.v_reg[subtrahend]);
    }

    fn right_shift(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        if self.legacy {
            let y: usize = ((rest & 0x0F0) >> 4) as usize;
            self.v_reg[x] = self.v_reg[y];
        }
        // extract lsb
        self.v_reg[0xF] = self.v_reg[x] & 1;
        self.v_reg[x] >>= 1;
    }

    fn left_shift(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        if self.legacy {
            let y: usize = ((rest & 0x0F0) >> 4) as usize;
            self.v_reg[x] = self.v_reg[y];
        }
        // extract msb
        self.v_reg[0xF] = self.v_reg[x] >> 7;
        self.v_reg[x] <<= 1;
    }

    fn set_i(&mut self, addr: u16) {
        self.i_reg = addr;
    }

    fn jump_offset(&mut self, rest: u16) {
        self.pc = (self.v_reg[0] as u16) + rest;
    }

    fn rand(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let rand: u8 = random();
        self.v_reg[x] = rand & ((rest & 0x0FF) as u8);
    }

    fn draw_sprite(&mut self, rest: u16) {
        let init_x = self.v_reg[((rest & 0xF00) >> 8) as usize] % SCREEN_WIDTH as u8;
        let init_y = self.v_reg[((rest & 0x0F0) >> 4) as usize] % SCREEN_HEIGHT as u8;

        let sprite_height = rest & 0x00F;
        let sprite_ptr = self.i_reg;
        self.v_reg[0xF] = 0;

        // For every row in sprite
        for r in 0..sprite_height {
            let sprite_row = self.ram[(sprite_ptr + r) as usize];
            let display_row_pos = (init_y + (r as u8)) as usize;
            if display_row_pos >= SCREEN_HEIGHT {
                break;
            }
            // for every bit in sprite row (byte)
            for c in 0..8 {
                // Shifting mask to extract only the specific pixel of the sprite we are on
                // Then check to see if it's not 0 at the masked bit
                let sprite_pixel = sprite_row & (0b10000000 >> c) != 0;
                let display_col_pos = (init_x + c) as usize;
                // If it reaches right edge of screen, stop row
                if display_col_pos >= SCREEN_WIDTH {
                    break;
                }
                let display_index = SCREEN_WIDTH * display_row_pos + display_col_pos;
                let display_pixel = &mut self.display[display_index];
                // If both sprite and pixel are on, turn off pixel and set VF to 1
                if sprite_pixel &&  *display_pixel {
                    *display_pixel = false;
                    self.v_reg[0xF] = 1;
                } else if sprite_pixel && !*display_pixel {
                    // If sprite is on and pixel is off, turn on pixel
                    *display_pixel = true;
                }
            }
        }
    }

    fn key_skip(&mut self, rest: u16, keys: &[u8]) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let key_pressed = keys.contains(&self.v_reg[x]);
        

        if rest & 0x0FF == 0x9E {
            if key_pressed { self.pc += 2 };
        } else if rest & 0x0FF == 0xA1 {
            if !key_pressed { self.pc += 2 }
        }
    }

    fn await_key(&mut self, rest: u16, keys: &[u8]) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let key_pressed = !keys.is_empty();

        if key_pressed { 
            self.v_reg[x] = keys[0];
        } else {
            self.pc -= 2;
        }
    }

    fn set_i_font(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let addr = 0x50 + (self.v_reg[x] * 5);
        self.i_reg = addr as u16;
    }

    // Binary-coded decimal conversion
    fn bcd(&mut self, rest: u16) {
        let mut x = self.v_reg[((rest & 0xF00) >> 8) as usize];
        let mut digits = [0u8; 3];

        // Extract all 3 digits and store them individually in array
        for i in (0..3).rev() {
            digits[i] = x % 10;
            x /= 10;
        }
        //store digits in memory at i
        let i: usize = self.i_reg as usize;
        self.ram[i..i+3].copy_from_slice(&digits);
    }

    fn store_mem(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let i: usize = self.i_reg as usize;
        self.ram[i..=i+x].copy_from_slice(&self.v_reg[0..=x]);
    }

    fn fill_mem(&mut self, rest: u16) {
        let x = ((rest & 0xF00) >> 8) as usize;
        let i: usize = self.i_reg as usize;
        self.v_reg[0..=x].copy_from_slice(& self.ram[i..=i+x]);
    }
}