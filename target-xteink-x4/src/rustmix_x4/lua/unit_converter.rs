//! Playable SD Lua Unit Converter model for the X4 Tools category.
//!
//! The app remains SD-loaded through `/RUSTMIX/APPS/UNITS`, but conversion
//! state is intentionally in-memory only for this first playable slice.

pub const LUA_UNIT_CONVERTER_PLAYABLE_APP_MARKER: &str = "rustmix-lua-unit-converter-playable-ok";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitConversion {
    pub name: &'static str,
    pub input_label: &'static str,
    pub output_label: &'static str,
    pub inputs: [&'static str; 3],
    pub outputs: [&'static str; 3],
}

pub const UNIT_CONVERSIONS: [UnitConversion; 6] = [
    UnitConversion {
        name: "Length",
        input_label: "Meters",
        output_label: "Feet",
        inputs: ["1 m", "5 m", "10 m"],
        outputs: ["3.28 ft", "16.40 ft", "32.81 ft"],
    },
    UnitConversion {
        name: "Weight",
        input_label: "Kilograms",
        output_label: "Pounds",
        inputs: ["1 kg", "5 kg", "10 kg"],
        outputs: ["2.20 lb", "11.02 lb", "22.05 lb"],
    },
    UnitConversion {
        name: "Temperature",
        input_label: "Celsius",
        output_label: "Fahrenheit",
        inputs: ["0 C", "25 C", "100 C"],
        outputs: ["32 F", "77 F", "212 F"],
    },
    UnitConversion {
        name: "Volume",
        input_label: "Liters",
        output_label: "Cups",
        inputs: ["1 L", "2 L", "5 L"],
        outputs: ["4.23 cups", "8.45 cups", "21.13 cups"],
    },
    UnitConversion {
        name: "Speed",
        input_label: "Kilometers/hour",
        output_label: "Miles/hour",
        inputs: ["10 km/h", "50 km/h", "100 km/h"],
        outputs: ["6.21 mph", "31.07 mph", "62.14 mph"],
    },
    UnitConversion {
        name: "Data",
        input_label: "Megabytes",
        output_label: "Kilobytes",
        inputs: ["1 MB", "16 MB", "128 MB"],
        outputs: ["1024 KB", "16384 KB", "131072 KB"],
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitConverterState {
    conversion_index: usize,
    amount_index: usize,
}

impl Default for UnitConverterState {
    fn default() -> Self {
        Self {
            conversion_index: 0,
            amount_index: 0,
        }
    }
}

impl UnitConverterState {
    pub const fn conversion_count() -> usize {
        UNIT_CONVERSIONS.len()
    }

    pub const fn amount_count() -> usize {
        3
    }

    pub fn load_default(&mut self) {
        self.reset();
    }

    pub fn load_units<T>(&mut self, _data: T) {
        self.reset();
    }

    pub fn reset(&mut self) {
        self.conversion_index = 0;
        self.amount_index = 0;
    }

    pub fn selected_index(&self) -> usize {
        self.conversion_index
    }

    pub fn amount_index(&self) -> usize {
        self.amount_index
    }

    pub fn conversion(&self) -> &'static UnitConversion {
        &UNIT_CONVERSIONS[self.conversion_index]
    }

    pub fn prev_conversion(&mut self) {
        if self.conversion_index == 0 {
            self.conversion_index = UNIT_CONVERSIONS.len() - 1;
        } else {
            self.conversion_index -= 1;
        }
    }

    pub fn next_conversion(&mut self) {
        self.conversion_index = (self.conversion_index + 1) % UNIT_CONVERSIONS.len();
    }

    pub fn prev_amount(&mut self) {
        if self.amount_index == 0 {
            self.amount_index = Self::amount_count() - 1;
        } else {
            self.amount_index -= 1;
        }
    }

    pub fn next_amount(&mut self) {
        self.amount_index = (self.amount_index + 1) % Self::amount_count();
    }

    pub fn input_value(&self) -> &'static str {
        self.conversion().inputs[self.amount_index]
    }

    pub fn output_value(&self) -> &'static str {
        self.conversion().outputs[self.amount_index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_conversion_is_meter_to_feet() {
        let state = UnitConverterState::default();
        assert_eq!(state.conversion().name, "Length");
        assert_eq!(state.input_value(), "1 m");
        assert_eq!(state.output_value(), "3.28 ft");
    }

    #[test]
    fn conversion_and_amount_cycle_without_allocations() {
        let mut state = UnitConverterState::default();
        state.next_conversion();
        assert_eq!(state.conversion().name, "Weight");
        state.prev_conversion();
        assert_eq!(state.conversion().name, "Length");
        state.prev_conversion();
        assert_eq!(state.conversion().name, "Data");
        state.next_amount();
        assert_eq!(state.input_value(), "16 MB");
        state.prev_amount();
        assert_eq!(state.input_value(), "1 MB");
    }
}
