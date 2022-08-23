use super::cpu::CPU;

const WIDTH: u8 = 64;

impl CPU {
    /// Clears the display
    pub fn clear(&mut self) {
        self.buf = [0; 2048];
        self.update();
    }

    #[cfg(feature = "show_commands")]
    pub fn update(&mut self) {}

    #[cfg(not(feature = "show_commands"))]
    pub fn update(&mut self) {
        print!("\x1B[2J\x1B[1;1H");
        for (i, item) in self.buf.iter_mut().enumerate() {
            if i != 0 && (i - 1) % (WIDTH as usize) == 0 {
                println!();
            }
            if *item == 1 {
                print!("■");
            } else {
                print!(" ");
            }
        }
        println!();
    }

    pub fn draw(&mut self, x: u8, y: u8, n: usize) {
        let x = self.registers[x as usize] % 64;
        let y = self.registers[y as usize] % 32;
        self.vf = 0;

        let sprite = &self.mem[(self.i_reg as usize)..(self.i_reg as usize + n as usize)];

        // Convert the coordinates to an index in the frame buffer
        let mut pos_in_buf = x as usize + (WIDTH as usize * y as usize);

        // Loop through each row in the sprite
        sprite.iter().for_each(|byte_row| {
            // This loop pushes the bits one by one to the right for each iteration,
            // see if it's on or off (using the & 1) and then write it to the frame
            // buffer
            for j in (0..8).rev() {
                // The current bit value, can be 1 or 0
                let current_bit_value = (byte_row >> j) & 1;

                // Set VF to 1 if the current bit is already on and the updated bit is also on.
                // Also turn off the bit.
                if (self.buf[pos_in_buf] == 1) && (current_bit_value == 1) {
                    self.vf = 1;
                    self.buf[pos_in_buf] = 0;
                } else {
                    // Just write to the display buffer by default.
                    self.buf[pos_in_buf] = current_bit_value;
                }

                // If we reached the end of the screen, stop rendering the row
                if (pos_in_buf % (WIDTH as usize)) == 0 {
                    break;
                }

                // Incrementing X coordinate
                pos_in_buf += 1;
            }

            // Incrementing Y coordinate
            pos_in_buf += (WIDTH - 8) as usize;
        });
    }
}
