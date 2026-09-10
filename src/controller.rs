use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ButtonFlags: u8 {
        const A = 0b00000001;
        const B = 0b00000010;
        const Select = 0b00000100;
        const Start = 0b00001000;
        const Up = 0b00010000;
        const Down = 0b00100000;
        const Left = 0b01000000;
        const Right = 0b10000000;
    }
}

pub struct Controller {
    button_state: ButtonFlags,
    latched_value: u8,
    shift_value: u8,
    pub update_latch: bool,
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller {
    fn new() -> Self {
        Controller {
            button_state: ButtonFlags::empty(),
            latched_value: 0,
            shift_value: 0,
            update_latch: false,
        }
    }

    pub fn update_button_state(&mut self, state: ButtonFlags) {
        self.button_state = state;

        if self.update_latch {
            self.update_latch_value();
        }
    }

    pub fn update_latch_value(&mut self) {
        self.latched_value = self.button_state.bits();
        self.shift_value = 0;
    }

    pub fn get_latched_value_and_shift(&mut self) -> u8 {
        if self.shift_value > 8 {
            1
        } else {
            let ret = (self.latched_value >> self.shift_value) & 0x01;
            self.shift_value += 1;
            ret
        }
    }
}
