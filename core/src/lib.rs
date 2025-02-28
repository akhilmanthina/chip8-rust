use std::collections::VecDeque;
use std::collections::HashMap;

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

}

impl Core {
    pub fn new() -> Self {
        let mut new_core = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            stack: VecDeque::new(),
            display: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            d_timer: 0,
            s_timer: 0,
            i_reg: 0,
            v_reg: [0; NUM_REG]
        };
        new_core.load_sprites();
        new_core
    }

    pub fn load_rom(&mut self, program: &[u8]) {
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
            0xF0, 0x80, 0xF0, 0x80, 0x80]  // F
        );
    }

    pub fn cycle(&mut self) {
        let instruction = self.fetch();
        self.decode_and_exec(instruction);
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

    // TODO: make private after testing
    pub fn decode_and_exec(&mut self, instruction: u16) {
        // 1st 4 bit "nibble"
        let nibble = (instruction & 0xF000) >> 12;
        let rest = instruction & 0x0FFF;
        match nibble {
            0x0 => match rest {
                    0x000 => self.noop(),
                    0x0E0 => self.clear_screen(),
                    0x0EE => (), //return from subroutine
                    _ => {dbg!("invalid op"); dbg!(instruction);}
                },
            0x1 => self.jump(rest),
            0x2 => self.call(rest),
            0x6 => self.set_v(rest),
            0x7 => self.add_v_no_carry(rest),
            0xA => self.set_i(rest),
            0xD => self.draw_sprite(rest),

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

    fn set_v(&mut self, rest: u16) {
        // Gather first 4 bits out of 12 remaining and shift it right to extract
        let num_reg = ((rest & 0xF00) >> 8) as usize;
        self.v_reg[num_reg] = (rest & 0xFF) as u8;
    }

    fn add_v_no_carry(&mut self, rest: u16){
        // Gather first 4 bits out of 12 remaining and shift it right to extract
        let num_reg = ((rest & 0xF00) >> 8) as usize;
        self.v_reg[num_reg] = self.v_reg[num_reg].wrapping_add((rest & 0xFF) as u8);
    }

    fn set_i(&mut self, addr: u16) {
        self.i_reg = addr;
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
                }
                // If sprite is on and pixel is off, turn on pixel
                else if sprite_pixel && !*display_pixel {
                    *display_pixel = true;
                }
            }
        }
    }

}