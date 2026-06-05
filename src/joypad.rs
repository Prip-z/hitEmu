use macroquad::input::{KeyCode::{self}, is_key_down};


const BUTTON_MASK: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];

pub struct Joypad {
    state_of_buttons: u8,
    strobe: bool,
    index_of_buttons: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad { state_of_buttons: 0, strobe: false, index_of_buttons: 0 }
    }

    pub fn step(&mut self, value: u8) {
        self.strobe = (value & 0x01) != 0;
        if self.strobe {
            self.index_of_buttons = 0;
            
            let mut current_input = 0u8;
            if is_key_down(KeyCode::Z)      { current_input |= BUTTON_MASK[0]; } 
            if is_key_down(KeyCode::X)      { current_input |= BUTTON_MASK[1]; } 
            if is_key_down(KeyCode::Space)  { current_input |= BUTTON_MASK[2]; } 
            if is_key_down(KeyCode::Enter)  { current_input |= BUTTON_MASK[3]; } 
            if is_key_down(KeyCode::Up)     { current_input |= BUTTON_MASK[4]; }
            if is_key_down(KeyCode::Down)   { current_input |= BUTTON_MASK[5]; }
            if is_key_down(KeyCode::Left)   { current_input |= BUTTON_MASK[6]; }
            if is_key_down(KeyCode::Right)  { current_input |= BUTTON_MASK[7]; }

            self.state_of_buttons = current_input; 
        }
    }

    pub fn read_state(&mut self) -> u8 {
        if self.index_of_buttons >= 8 {
            return 1;
        }
        
        let button_is_pressed = (self.state_of_buttons & BUTTON_MASK[self.index_of_buttons as usize]) != 0;
        let value = if button_is_pressed { 1 } else { 0 };

        if !self.strobe {
            self.index_of_buttons += 1;
        }
        value
    }
}