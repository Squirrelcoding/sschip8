use std::time::UNIX_EPOCH;
const USE_NEW_SHIFTING_CONVENTIONS: bool = true;
const USE_NEW_MEMOPS_CONVENTIONS: bool = false;

pub const FONT: [u8; 80] = [
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

/// A struct representing the CHIP-8 CPU and RAM
#[allow(clippy::upper_case_acronyms)]
pub struct CPU {
    /// 4K of memory for the CHIP-8
    pub mem: [u8; 4096],

    /// The program counter
    pub pc: u16,

    /// The 'I' register to store memory addresses
    pub i_reg: u16,

    /// The stack for the CHIP-8
    pub stack: [u16; 16],

    /// The registers for the CPU
    pub registers: [u8; 16],

    /// The stack pointer
    pub sp: u8,

    /// The delay timer
    pub delay_timer: u8,

    /// The sound timer
    pub sound_timer: u8,

    /// The VF register
    pub vf: u8,

    /// The display buffer
    pub buf: [u8; 2048],

    // Variables for helping with internals, not meant for instruction use.
    pub last_st_write: u128,
    pub last_dt_write: u128,
    pub last_cpu_cycle: u128,

    pub is_key_pressed: bool,
    pub key_code: i16,
}

impl CPU {
    /// Initiate a new instance of the CPU struct

    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut mem: [u8; 4096] = [0; 4096];

        // Write the font to mem
        mem[0x050..(0x09F + 1)].copy_from_slice(&FONT[..]);

        CPU {
            registers: [0; 16],
            pc: 0x200,
            sp: 0,
            mem,
            stack: [0; 16],
            i_reg: 0x200,
            delay_timer: 255,
            sound_timer: 255,
            vf: 0,
            buf: [0; 2048],
            last_st_write: 0,
            last_dt_write: 0,
            last_cpu_cycle: 0,
            is_key_pressed: false,
            key_code: 0,
        }
    }

    pub fn new_with_memory(program_memory: &[u8]) -> Self {
        let mut mem: [u8; 4096] = [0; 4096];
        // Write the font to mem
        mem[0x050..(0x09F + 1)].copy_from_slice(&FONT[..]);

        // Load the program in memory
        mem[0x200..(program_memory.len() + 0x200)].copy_from_slice(program_memory);

        CPU {
            registers: [0; 16],
            pc: 0x200,
            sp: 0,
            mem,
            stack: [0; 16],
            i_reg: 0x200,
            delay_timer: 255,
            sound_timer: 255,
            vf: 0,
            buf: [0; 2048],
            last_st_write: 0,
            last_dt_write: 0,
            last_cpu_cycle: 0,
            is_key_pressed: false,
            key_code: 0,
        }
    }

    /// Decodes two bytes into 4 seperate nibbles
    pub fn decode(&self, upper_byte: u8, lower_byte: u8) -> (u8, u8, u8, u8) {
        let upper_high = ((upper_byte & 0xF0) >> 4) as u8;
        let upper_low = (upper_byte & 0x0F) as u8;

        let lower_high = ((lower_byte & 0xF0) >> 4) as u8;
        let lower_low = (lower_byte & 0x0F) as u8;

        (upper_high, upper_low, lower_high, lower_low)
    }

    /// Runs the CHIP-8
    pub fn run(&mut self) {
        loop {
            let instruction =
                self.decode(self.mem[self.pc as usize], self.mem[self.pc as usize + 1]);

            let now = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            if now - self.last_cpu_cycle >= 17 && self.sound_timer != 0 {
                // Play audio or something
                unsafe {
                    winapi::um::utilapiset::Beep(750, 100);
                }

                self.sound_timer -= 1;
            }
            // Increment the program counter
            self.pc += 2;

            match instruction {
                // 0x00E0 - clr
                (0x0, 0x0, 0xE, 0x0) => {
                    #[cfg(feature = "show_commands")]
                    println!("0x00E0: clr");

                    self.cls00e0();
                }
                // 0x1nnn - jp
                (0x1, nnn_a, nnn_b, nnn_c) => {
                    let addr = self.to_nnn(nnn_a, nnn_b, nnn_c);

                    #[cfg(feature = "show_commands")]
                    println!("0x1nnn: jp to {addr}");

                    self.jp1nnn(addr);
                }

                // 0x6xnn - set
                (0x6, x, upper_nibble, lower_nibble) => {
                    let nn = (upper_nibble << 4) | lower_nibble;

                    #[cfg(feature = "show_commands")]
                    println!("0x6xnn: set V{x} to {nn}");

                    self.set6xnn(x, nn);
                }

                // 0x7Xnn - add
                (0x7, x, upper_nibble, lower_nibble) => {
                    let nn = (upper_nibble << 4) | lower_nibble;

                    #[cfg(feature = "show_commands")]
                    println!("0x7xnn: add {nn} to V{x}");

                    self.add7xnn(x, nn);
                }

                // 0xAnnn - set
                (0xA, nnn_a, nnn_b, nnn_c) => {
                    let nn = self.to_nnn(nnn_a, nnn_b, nnn_c);

                    #[cfg(feature = "show_commands")]
                    println!("0xAnnn: set I to {nn}");

                    self.setannn(nn);
                }

                // 0xDxyn - draw
                (0xD, x, y, n) => {
                    #[cfg(feature = "show_commands")]
                    println!("0xDxyn: draw sprite at address {n} at ({x}, {y})");

                    self.drwdxyn(x, y, n);
                }

                // 0x2nnn - call
                (0x2, nnn_a, nnn_b, nnn_c) => {
                    let addr = self.to_nnn(nnn_a, nnn_b, nnn_c);

                    #[cfg(feature = "show_commands")]
                    println!("0x2nnn: call {addr}");

                    self.call2nnn(addr);
                }

                // 0x00EE - return
                (0x0, 0x0, 0xE, 0xE) => {
                    #[cfg(feature = "show_commands")]
                    println!("return from subroutine");

                    self.ret00ee();
                }

                // 0x3xnn - se
                (0x3, x, upper_nibble, lower_nibble) => {
                    let nn = (upper_nibble << 4) | lower_nibble;

                    self.se3xnn(x, nn);
                }

                // 0x4xnn - sne
                (0x4, x, upper_nibble, lower_nibble) => {
                    let nn = (upper_nibble << 4) | lower_nibble;
                    self.sne4xnn(x, nn);
                }

                // 0x5xy0 - se
                (0x5, x, y, 0x0) => {
                    self.se5xy0(x, y);
                }

                // 0x9xy0 - sne
                (0x9, x, y, 0x0) => {
                    self.sne9xy0(x, y);
                }

                // 0x8xy0 - ld
                (0x8, x, y, 0x0) => {
                    #[cfg(feature = "show_commands")]
                    println!("setting V{x} to value of V{y}");

                    self.ld8xy0(x, y);
                }

                // 0x8xy1 - bitwise OR
                (0x8, x, y, 0x1) => {
                    #[cfg(feature = "show_commands")]
                    println!("V{x} = V{x} OR V{y}");

                    self.or8xy1(x, y);
                }

                // 0x8xy2 - bitwise AND
                (0x8, x, y, 0x2) => {
                    #[cfg(feature = "show_commands")]
                    println!("V{x} = V{x} AND V{y}");
                    self.and8xy2(x, y);
                }

                // 0x8xy3 - bitwise XOR
                (0x8, x, y, 0x3) => {
                    #[cfg(feature = "show_commands")]
                    println!("V{x} = V{x} XOR V{y}");

                    self.xor8xy3(x, y);
                }

                // 0x8xy4 - ADD
                (0x8, x, y, 0x4) => {
                    #[cfg(feature = "show_commands")]
                    println!("Set V{x} = V{x} + V{y}, set VF = carry");

                    self.add8xy4(x, y);
                }

                // 0x8xy5 - SUB
                (0x8, x, y, 0x5) => {
                    #[cfg(feature = "show_commands")]
                    println!("Set V{x} = V{x} - V{y}, set VF = NOT borrow.");

                    self.sub8xy5(x, y);
                }

                // 0x8xy7 - SUB
                (0x8, x, y, 0x7) => {
                    #[cfg(feature = "show_commands")]
                    println!("Set V{x} = V{y} - V{x}, set VF = NOT borrow.");

                    self.sub8xy7(x, y);
                }

                // 0x8xy6 - shr
                (0x8, x, y, 0x6) => match USE_NEW_SHIFTING_CONVENTIONS {
                    true => {
                        #[cfg(feature = "show_commands")]
                        println!("Set V{x} = V{x} SHR 1.");
                        self.shr8xy6_usex(x, y);
                    }
                    false => {
                        #[cfg(feature = "show_commands")]
                        println!("Set V{x} = V{x} SHR 1.");
                        self.shr8xy6_usey(x, y);
                    }
                },

                // 0x8xy6 - shl
                (0x8, x, y, 0xE) => {
                    if USE_NEW_SHIFTING_CONVENTIONS {
                        #[cfg(feature = "show_commands")]
                        println!("Set V{x} = V{x} SHL 1.");
                        self.shl8xye_usex(x, y);
                    } else {
                        #[cfg(feature = "show_commands")]
                        println!("Set V{x} = V{x} SHL 1.");
                        self.shl8xye_usey(x, y);
                    }
                }

                // 0xBnnn - jp
                (0xB, nnn_a, nnn_b, nnn_c) => {
                    let nnn = self.to_nnn(nnn_a, nnn_b, nnn_c);
                    #[cfg(feature = "show_commands")]
                    println!("jp to {nnn} + V0");

                    self.jpbnnn(nnn);
                }

                // 0xCxnn - rnd
                (0xC, x, upper_nibble, lower_nibble) => {
                    let nn = (upper_nibble << 4) | lower_nibble;

                    #[cfg(feature = "show_commands")]
                    println!("get rnd | {nn}");

                    self.rndcxnn(x, nn);
                }

                // 0xFx07 - ldf
                (0xF, x, 0x0, 0x7) => {
                    #[cfg(feature = "show_commands")]
                    println!("Set V{x} = delay timer val");

                    self.ldfx07(x);
                }

                // 0xFx15 - ld
                (0xF, x, 0x1, 0x5) => {
                    #[cfg(feature = "show_commands")]
                    println!("Set delay timer to V{x}");

                    self.ldfx15(x);
                }

                // 0xFx18 - ld
                (0xF, x, 0x1, 0x8) => {
                    #[cfg(feature = "show_commands")]
                    println!("Set sound timer to V{x}");

                    self.ldfx18(x);
                }

                // 0xFx1E - add
                (0xF, x, 0x1, 0xE) => {
                    #[cfg(feature = "show_commands")]
                    println!("I = I + V{x}");

                    self.addfx1e(x);
                }

                // 0xFx29 - ld
                (0xF, x, 0x2, 0x9) => {
                    #[cfg(feature = "show_commands")]
                    println!("Set I = location of sprite for digit V{x}.");

                    self.ldfx29(x);
                }

                // 0xFx33 - ld
                (0xF, x, 0x3, 0x3) => {
                    #[cfg(feature = "show_commands")]
                    println!("Store BCD representation of Vx in memory locations I, I+1, and I+2.");

                    self.ldfx33(x);
                }

                // 0xFx55 - ld
                (0xF, x, 0x5, 0x5) => {
                    #[cfg(feature = "show_commands")]
                    println!("Store registers V0 through Vx in memory starting at location I.");

                    if USE_NEW_MEMOPS_CONVENTIONS {
                        self.ldfx55(x);
                    } else {
                        self.ldfx55_old(x);
                    }
                }

                (0xF, x, 0x6, 0x5) => {
                    #[cfg(feature = "show_commands")]
                    println!("Read registers V0 through Vx from memory starting at location I.");

                    if USE_NEW_MEMOPS_CONVENTIONS {
                        self.ldfx65(x);
                    } else {
                        self.ldfx65_old(x);
                    }
                }

                (0xF, x, 0x0, 0xA) => {
                    #[cfg(feature = "show_commands")]
                    println!("Waiting for keypress and writing result to V{x}");

                    self.ldfx0a(x);
                }

                (0xE, x, 0xA, 0x1) => {
                    #[cfg(feature = "show_commands")]
                    println!("Skip next instruction if key with the value of V{x} not is pressed. V{x} btw: 0x{:x}", 
                            self.registers[x as usize]);

                    self.skpexa1(x);
                }

                (0xE, x, 0x9, 0xE) => {
                    #[cfg(feature = "show_commands")]
                    println!("Skip next instruction if key with the value of V{x} is pressed. V{x} btw: 0x{:x}", 
                            self.registers[x as usize]);

                    self.skpex9e(x);
                }

                (a, b, c, d) => {
                    unimplemented!("0x{:x}{:x}{:x}{:x}", a, b, c, d);
                }
            }
            self.update();

            // Update the status of the last cpu cycle
            self.last_cpu_cycle = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
        }
    }

    /// A helper function to convert 8 nibbles into one u16 value
    fn to_nnn(&self, a: u8, b: u8, c: u8) -> u16 {
        let mut byte = ((a << 4) | b) as u16;
        byte = (byte << 4) | c as u16;

        byte
    }

    /// A function for checking if a key is currently pressed using `winapi`
    pub fn is_key_pressed(&self, key: u8) -> bool {
        let key_code = match key {
            0x0 => 0x30,
            0x1 => 0x31,
            0x2 => 0x32,
            0x3 => 0x33,
            0x4 => 0x34,
            0x5 => 0x35,
            0x6 => 0x36,
            0x7 => 0x37,
            0x8 => 0x38,
            0x9 => 0x39,
            0xA => 0x41,
            0xB => 0x42,
            0xC => 0x43,
            0xD => 0x44,
            0xE => 0x45,
            0xF => 0x46,
            _ => 0x30,
        };

        unsafe {
            let is_key_pressed = winapi::um::winuser::GetAsyncKeyState(key_code);
            
            // Checking if the MSB of `is_key_pressed`, which indicates if the key was pressed during the 
            // function call
            ((is_key_pressed >> 15) & 1) == 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::{Duration, SystemTime};

    fn new_cpu() -> CPU {
        CPU::new()
    }

    #[test]
    fn decode_instruction_success() {
        let upper_byte = 0xAB;
        let lower_byte = 0xCD;
        let cpu = new_cpu();
        let decoded_instruction = cpu.decode(upper_byte, lower_byte);

        assert_eq!(decoded_instruction.0, 0xA);
        assert_eq!(decoded_instruction.1, 0xB);
        assert_eq!(decoded_instruction.2, 0xC);
        assert_eq!(decoded_instruction.3, 0xD);
    }

    #[test]
    fn decode_instruction_fail() {
        let upper_byte = 0xAB;
        let lower_byte = 0xCD;
        let cpu = new_cpu();
        let decoded_instruction = cpu.decode(upper_byte, lower_byte);

        assert_ne!(decoded_instruction.0, 0x6);
        assert_ne!(decoded_instruction.1, 0x9);
        assert_ne!(decoded_instruction.2, 0x4);
        assert_ne!(decoded_instruction.3, 0x2);
    }

    #[test]
    fn test_to_nnn() {
        let a = 0xA;
        let b = 0xB;
        let c = 0xC;
        let cpu = new_cpu();

        let new_val = cpu.to_nnn(a, b, c);

        assert_eq!(new_val, 0xABC);
    }

    #[test]
    fn test_jp1nnn() {
        let mut cpu = new_cpu();

        let arbitrary_value = 69;

        cpu.jp1nnn(arbitrary_value);

        assert_eq!(cpu.pc, arbitrary_value)
    }

    #[test]
    fn test_set6xnn() {
        let mut cpu = new_cpu();

        let arbitrary_value = 69;

        cpu.set6xnn(0, arbitrary_value);

        assert_eq!(cpu.registers[0], arbitrary_value);
    }

    #[test]
    fn test_add7xnn() {
        let mut cpu = new_cpu();

        let old_value = 50;

        // Previous value of register 0
        cpu.set6xnn(0, old_value);

        let added_value = 69;

        cpu.add7xnn(0, added_value);

        assert_eq!(cpu.registers[0], old_value + added_value);
    }

    #[test]
    fn test_setannn() {
        let mut cpu = new_cpu();

        let arbitrary_value = 69;

        cpu.setannn(arbitrary_value);

        assert_eq!(cpu.i_reg, arbitrary_value);
    }

    #[test]
    fn test_call2nnn() {
        let mut cpu = new_cpu();

        let arbitrary_address = 500;

        cpu.call2nnn(arbitrary_address);

        assert_eq!(cpu.sp, 1);
        assert_eq!(cpu.pc, arbitrary_address);
        assert_eq!(cpu.stack[cpu.sp as usize], arbitrary_address);
    }

    #[test]
    fn test_se3xnn() {
        let mut cpu = new_cpu();

        let arbitrary_value = 69;

        cpu.set6xnn(0, arbitrary_value);

        // Set the PC to a random address
        cpu.pc = 500;

        // This should increment cpu.pc by 2.
        cpu.se3xnn(0, arbitrary_value);

        assert_eq!(cpu.pc, 502);
    }

    #[test]
    fn call_and_return_subroutine() {
        let mut cpu = new_cpu();

        let arbitrary_subroutine_address = 500;

        cpu.call2nnn(arbitrary_subroutine_address);
        cpu.ret00ee();

        assert_eq!(cpu.sp, 0);
    }

    #[test]
    fn test_sne4xnn() {
        let mut cpu = new_cpu();

        let arbitrary_value = 69;

        cpu.set6xnn(0, arbitrary_value);

        // Set the PC to a random address
        cpu.pc = 500;

        // This should increment cpu.pc by 2.
        cpu.sne4xnn(0, 42);

        assert_eq!(cpu.pc, 502);
    }

    #[test]
    fn test_se5xy0() {
        let mut cpu = new_cpu();

        let x_val = 69;
        let y_val = 69;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.pc = 500;

        cpu.se5xy0(0, 1);

        assert_eq!(cpu.pc, 502);
    }

    #[test]
    fn test_sne9xy0() {
        let mut cpu = new_cpu();

        let x_val = 69;
        let y_val = 42;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.pc = 500;

        cpu.sne9xy0(0, 1);

        assert_eq!(cpu.pc, 502);
    }

    #[test]
    fn test_ld8xy0() {
        let mut cpu = new_cpu();

        let x_val = 69;
        let y_val = 42;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.ld8xy0(0, 1);

        assert_eq!(cpu.registers[0], cpu.registers[1])
    }

    #[test]
    fn test_or8xy1() {
        let mut cpu = new_cpu();

        let x_val = 0b1000101;
        let y_val = 0b101010;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.or8xy1(0, 1);

        assert_eq!(cpu.registers[0], x_val | y_val);
    }

    #[test]
    fn test_and8xy2() {
        let mut cpu = new_cpu();

        let x_val = 0b1000101;
        let y_val = 0b101010;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.and8xy2(0, 1);

        assert_eq!(cpu.registers[0], x_val & y_val);
    }

    #[test]
    fn test_xor8xy3() {
        let mut cpu = new_cpu();

        let x_val = 0b1000101;
        let y_val = 0b101010;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.xor8xy3(0, 1);

        assert_eq!(cpu.registers[0], x_val ^ y_val);
    }

    #[test]
    fn test_add8xy4_without_overflow() {
        let mut cpu = new_cpu();

        let x_val = 10;
        let y_val = 10;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.add8xy4(0, 1);

        assert_eq!(cpu.vf, 0);
        assert_eq!(cpu.registers[0], 20);
    }

    #[test]
    fn test_add8xy4_with_overflow() {
        let mut cpu = new_cpu();

        let x_val = 255;
        let y_val = 255;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.add8xy4(0, 1);

        assert_eq!(cpu.vf, 1);
    }

    #[test]
    fn test_sub8xy5_without_underflow() {
        let mut cpu = new_cpu();

        let x_val = 50;
        let y_val = 25;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.sub8xy5(0, 1);

        assert_eq!(cpu.vf, 1);
        assert_eq!(cpu.registers[0], 25);
    }

    #[test]
    fn test_sub8xy5_with_underflow() {
        let mut cpu = new_cpu();

        let x_val = 25;
        let y_val = 50;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.sub8xy5(0, 1);

        assert_eq!(cpu.vf, 0);
        assert_eq!(cpu.registers[0], 231);
    }

    #[test]
    fn test_sub8xy7_without_underflow() {
        let mut cpu = new_cpu();

        let x_val = 25;
        let y_val = 50;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.sub8xy7(0, 1);

        assert_eq!(cpu.vf, 1);
        assert_eq!(cpu.registers[0], 25);
    }

    #[test]
    fn test_sub8xy7_with_underflow() {
        let mut cpu = new_cpu();

        let x_val = 50;
        let y_val = 25;

        cpu.set6xnn(0, x_val);
        cpu.set6xnn(1, y_val);

        cpu.sub8xy7(0, 1);

        assert_eq!(cpu.vf, 0);
        assert_eq!(cpu.registers[0], 231);
    }

    #[test]
    fn test_shr8xy6_usey() {
        let mut cpu = new_cpu();

        let byte = 0b1000101;

        cpu.set6xnn(1, byte);

        cpu.shr8xy6_usey(0, 1);

        assert_eq!(cpu.registers[0], byte >> 1);
        assert_eq!(cpu.vf, 1);
    }

    #[test]
    fn test_shr8xy6_usex() {
        let mut cpu = new_cpu();

        let byte = 0b1000101;

        cpu.set6xnn(0, byte);

        cpu.shr8xy6_usey(0, 0);

        assert_eq!(cpu.registers[0], byte >> 1);
        assert_eq!(cpu.vf, 1);
    }

    #[test]
    fn test_shl8xye_usey() {
        let mut cpu = new_cpu();

        let byte = 0b1000_1010;

        cpu.set6xnn(1, byte);

        cpu.shl8xye_usey(0, 1);

        assert_eq!(cpu.registers[0], byte << 1);
        assert_eq!(cpu.vf, 1);
    }

    #[test]
    fn test_shl8xye_usex() {
        let mut cpu = new_cpu();

        let byte = 0b1000_1010;

        cpu.set6xnn(0, byte);

        cpu.shl8xye_usex(0, 0);

        assert_eq!(cpu.registers[0], byte << 1);
        assert_eq!(cpu.vf, 1);
    }

    #[test]
    fn test_jpbnnn() {
        let mut cpu = new_cpu();

        cpu.jp1nnn(1024);
        cpu.set6xnn(0, 255);

        cpu.jpbnnn(1024);

        assert_eq!(cpu.pc, 1279)
    }

    #[test]
    fn test_delay_instructions() {
        let mut cpu = new_cpu();

        cpu.set6xnn(0, 69);
        cpu.ldfx15(0);
        let time = SystemTime::now();
        thread::sleep(Duration::from_millis(20));
        cpu.ldfx07(0);
        assert_eq!(time.elapsed().unwrap().as_millis() as u8, cpu.registers[0]);
    }

    #[test]
    fn test_addfx1e() {
        let mut cpu = new_cpu();

        cpu.setannn(500);
        cpu.set6xnn(0, 255);

        cpu.addfx1e(0);

        assert_eq!(755, cpu.i_reg);
    }

    #[test]
    fn test_ldfx29() {
        let mut cpu = new_cpu();

        cpu.set6xnn(0, 0x0);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50);

        cpu.set6xnn(0, 0x1);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50 + 5);

        cpu.set6xnn(0, 0x2);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50 + 10);

        cpu.set6xnn(0, 0x3);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50 + 15);

        cpu.set6xnn(0, 0x4);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50 + 20);

        cpu.set6xnn(0, 0x5);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50 + 25);

        cpu.set6xnn(0, 0x6);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50 + 30);

        cpu.set6xnn(0, 0x7);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50 + 35);

        cpu.set6xnn(0, 0x8);
        cpu.ldfx29(0);
        assert_eq!(cpu.i_reg, 0x50 + 40);
    }

    #[test]
    fn test_ldfx33() {
        let mut cpu = new_cpu();

        cpu.set6xnn(0, 123);

        cpu.i_reg = 1024;
        cpu.ldfx33(0);

        assert_eq!(cpu.mem[1024], 1);
        assert_eq!(cpu.mem[1025], 2);
        assert_eq!(cpu.mem[1026], 3);
    }

    #[test]
    fn test_ldfx55() {
        let mut cpu = new_cpu();

        cpu.set6xnn(0, 0);
        cpu.set6xnn(1, 1);
        cpu.set6xnn(2, 2);
        cpu.set6xnn(3, 3);
        cpu.set6xnn(4, 4);
        cpu.set6xnn(5, 5);

        cpu.i_reg = 1024;
        cpu.ldfx55(5);

        assert_eq!(cpu.mem[1024], 0);
        assert_eq!(cpu.mem[1025], 1);
        assert_eq!(cpu.mem[1026], 2);
        assert_eq!(cpu.mem[1027], 3);
        assert_eq!(cpu.mem[1028], 4);
        assert_eq!(cpu.mem[1029], 5);
    }

    #[test]
    fn test_ldfx65() {
        let mut cpu = new_cpu();

        cpu.mem[1024] = 0;
        cpu.mem[1025] = 1;
        cpu.mem[1026] = 2;
        cpu.mem[1027] = 3;
        cpu.mem[1028] = 4;
        cpu.mem[1029] = 5;

        cpu.i_reg = 1024;
        cpu.ldfx65(5);

        assert_eq!(cpu.registers[0], 0);
        assert_eq!(cpu.registers[1], 1);
        assert_eq!(cpu.registers[2], 2);
        assert_eq!(cpu.registers[3], 3);
        assert_eq!(cpu.registers[4], 4);
        assert_eq!(cpu.registers[5], 5);
    }
}