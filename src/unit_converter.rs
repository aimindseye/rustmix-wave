//! Hardware-independent, offline fixed-point unit-conversion domain.
//!
//! The UI stores values in thousandths so on-device conversion does not rely
//! on floating-point formatting. Ratios use `i128` intermediates to keep the
//! bounded 0..=999_999 input range safe for every supported category.

/// Fixed-point units per rendered whole value.
pub const FIXED_SCALE: i64 = 1_000;
/// Maximum value accepted by the button-driven converter UI.
pub const MAX_INPUT_MILLI: i64 = 999_999 * FIXED_SCALE;
/// Minimum value accepted by the initial UI milestone.
pub const MIN_INPUT_MILLI: i64 = 0;
/// Available value-edit increments in fixed-point thousandths.
pub const STEP_MILLI: [i64; 4] = [100, 1_000, 10_000, 100_000];

/// Top-level conversion groups exposed by the Tools application.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UnitCategory {
    #[default]
    Length,
    Mass,
    Temperature,
    Volume,
}

impl UnitCategory {
    pub const ALL: [Self; 4] = [Self::Length, Self::Mass, Self::Temperature, Self::Volume];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Mass => "Mass",
            Self::Temperature => "Temperature",
            Self::Volume => "Volume",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Mass => "mass",
            Self::Temperature => "temperature",
            Self::Volume => "volume",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Length => Self::Mass,
            Self::Mass => Self::Temperature,
            Self::Temperature => Self::Volume,
            Self::Volume => Self::Length,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Length => Self::Volume,
            Self::Mass => Self::Length,
            Self::Temperature => Self::Mass,
            Self::Volume => Self::Temperature,
        }
    }
}

/// Concrete unit selected by the converter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unit {
    Millimeters,
    Centimeters,
    Meters,
    Kilometers,
    Inches,
    Feet,
    Yards,
    Miles,
    Grams,
    Kilograms,
    Ounces,
    Pounds,
    Celsius,
    Fahrenheit,
    Kelvin,
    Milliliters,
    Liters,
    Cups,
    FluidOunces,
    Pints,
    Quarts,
    Gallons,
}

const LENGTH_UNITS: [Unit; 8] = [
    Unit::Millimeters,
    Unit::Centimeters,
    Unit::Meters,
    Unit::Kilometers,
    Unit::Inches,
    Unit::Feet,
    Unit::Yards,
    Unit::Miles,
];
const MASS_UNITS: [Unit; 4] = [Unit::Grams, Unit::Kilograms, Unit::Ounces, Unit::Pounds];
const TEMPERATURE_UNITS: [Unit; 3] = [Unit::Celsius, Unit::Fahrenheit, Unit::Kelvin];
const VOLUME_UNITS: [Unit; 7] = [
    Unit::Milliliters,
    Unit::Liters,
    Unit::Cups,
    Unit::FluidOunces,
    Unit::Pints,
    Unit::Quarts,
    Unit::Gallons,
];

impl Unit {
    #[must_use]
    pub const fn category(self) -> UnitCategory {
        match self {
            Self::Millimeters
            | Self::Centimeters
            | Self::Meters
            | Self::Kilometers
            | Self::Inches
            | Self::Feet
            | Self::Yards
            | Self::Miles => UnitCategory::Length,
            Self::Grams | Self::Kilograms | Self::Ounces | Self::Pounds => UnitCategory::Mass,
            Self::Celsius | Self::Fahrenheit | Self::Kelvin => UnitCategory::Temperature,
            Self::Milliliters
            | Self::Liters
            | Self::Cups
            | Self::FluidOunces
            | Self::Pints
            | Self::Quarts
            | Self::Gallons => UnitCategory::Volume,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Millimeters => "Millimeters",
            Self::Centimeters => "Centimeters",
            Self::Meters => "Meters",
            Self::Kilometers => "Kilometers",
            Self::Inches => "Inches",
            Self::Feet => "Feet",
            Self::Yards => "Yards",
            Self::Miles => "Miles",
            Self::Grams => "Grams",
            Self::Kilograms => "Kilograms",
            Self::Ounces => "Ounces",
            Self::Pounds => "Pounds",
            Self::Celsius => "Celsius",
            Self::Fahrenheit => "Fahrenheit",
            Self::Kelvin => "Kelvin",
            Self::Milliliters => "Milliliters",
            Self::Liters => "Liters",
            Self::Cups => "Cups",
            Self::FluidOunces => "Fluid ounces",
            Self::Pints => "Pints",
            Self::Quarts => "Quarts",
            Self::Gallons => "Gallons",
        }
    }

    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Millimeters => "mm",
            Self::Centimeters => "cm",
            Self::Meters => "m",
            Self::Kilometers => "km",
            Self::Inches => "in",
            Self::Feet => "ft",
            Self::Yards => "yd",
            Self::Miles => "mi",
            Self::Grams => "g",
            Self::Kilograms => "kg",
            Self::Ounces => "oz",
            Self::Pounds => "lb",
            Self::Celsius => "C",
            Self::Fahrenheit => "F",
            Self::Kelvin => "K",
            Self::Milliliters => "mL",
            Self::Liters => "L",
            Self::Cups => "cups",
            Self::FluidOunces => "fl oz",
            Self::Pints => "pt",
            Self::Quarts => "qt",
            Self::Gallons => "gal",
        }
    }
}

/// One of the five button-editable converter fields.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConverterField {
    #[default]
    Category,
    FromUnit,
    Value,
    ToUnit,
    StepSize,
}

impl ConverterField {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Category => "CATEGORY",
            Self::FromUnit => "FROM UNIT",
            Self::Value => "VALUE",
            Self::ToUnit => "TO UNIT",
            Self::StepSize => "STEP SIZE",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Category => Self::FromUnit,
            Self::FromUnit => Self::Value,
            Self::Value => Self::ToUnit,
            Self::ToUnit => Self::StepSize,
            Self::StepSize => Self::Category,
        }
    }
}

/// Result returned by one bounded conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversionResult {
    Value(i64),
    OverRange,
    Invalid,
}

/// Session-local, button-driven Unit Converter state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitConverterUiState {
    pub category: UnitCategory,
    pub from_index: usize,
    pub to_index: usize,
    pub value_milli: i64,
    pub step_index: usize,
    pub active_field: ConverterField,
}

impl Default for UnitConverterUiState {
    fn default() -> Self {
        Self {
            category: UnitCategory::Length,
            from_index: 7, // Miles
            to_index: 3,   // Kilometers
            value_milli: 5 * FIXED_SCALE,
            step_index: 1, // 1.0
            active_field: ConverterField::Category,
        }
    }
}

impl UnitConverterUiState {
    #[must_use]
    pub fn from_unit(self) -> Unit {
        unit_at(self.category, self.from_index)
    }

    #[must_use]
    pub fn to_unit(self) -> Unit {
        unit_at(self.category, self.to_index)
    }

    #[must_use]
    pub fn step_milli(self) -> i64 {
        STEP_MILLI[self.step_index.min(STEP_MILLI.len() - 1)]
    }

    #[must_use]
    pub fn result(self) -> ConversionResult {
        convert_milli(self.value_milli, self.from_unit(), self.to_unit())
    }

    pub fn select_next_field(&mut self) {
        self.active_field = self.active_field.next();
    }

    pub fn increase_active(&mut self) {
        match self.active_field {
            ConverterField::Category => self.select_category(self.category.next()),
            ConverterField::FromUnit => {
                self.from_index = (self.from_index + 1) % units_for(self.category).len();
            }
            ConverterField::Value => {
                self.value_milli = self
                    .value_milli
                    .saturating_add(self.step_milli())
                    .min(MAX_INPUT_MILLI);
            }
            ConverterField::ToUnit => {
                self.to_index = (self.to_index + 1) % units_for(self.category).len();
            }
            ConverterField::StepSize => {
                self.step_index = (self.step_index + 1) % STEP_MILLI.len();
            }
        }
    }

    pub fn decrease_active(&mut self) {
        match self.active_field {
            ConverterField::Category => self.select_category(self.category.previous()),
            ConverterField::FromUnit => {
                self.from_index = previous_index(self.from_index, units_for(self.category).len());
            }
            ConverterField::Value => {
                self.value_milli = self
                    .value_milli
                    .saturating_sub(self.step_milli())
                    .max(MIN_INPUT_MILLI);
            }
            ConverterField::ToUnit => {
                self.to_index = previous_index(self.to_index, units_for(self.category).len());
            }
            ConverterField::StepSize => {
                self.step_index = previous_index(self.step_index, STEP_MILLI.len());
            }
        }
    }

    fn select_category(&mut self, category: UnitCategory) {
        self.category = category;
        let (from_index, to_index, value_milli) = defaults_for(category);
        self.from_index = from_index;
        self.to_index = to_index;
        self.value_milli = value_milli;
    }
}

/// Convert one fixed-point value between compatible units.
#[must_use]
pub fn convert_milli(value_milli: i64, from: Unit, to: Unit) -> ConversionResult {
    if from.category() != to.category() {
        return ConversionResult::Invalid;
    }
    if value_milli < MIN_INPUT_MILLI || value_milli > MAX_INPUT_MILLI {
        return ConversionResult::Invalid;
    }
    let converted = if from.category() == UnitCategory::Temperature {
        convert_temperature(value_milli, from, to)
    } else {
        convert_ratio(value_milli, from, to)
    };
    match converted {
        Some(value) if value.unsigned_abs() <= MAX_INPUT_MILLI as u64 => {
            ConversionResult::Value(value)
        }
        Some(_) => ConversionResult::OverRange,
        None => ConversionResult::Invalid,
    }
}

fn convert_ratio(value_milli: i64, from: Unit, to: Unit) -> Option<i64> {
    let (from_num, from_den) = base_ratio(from)?;
    let (to_num, to_den) = base_ratio(to)?;
    let numerator = i128::from(value_milli)
        .checked_mul(from_num)?
        .checked_mul(to_den)?;
    let denominator = from_den.checked_mul(to_num)?;
    i64::try_from(div_round(numerator, denominator)).ok()
}

fn convert_temperature(value_milli: i64, from: Unit, to: Unit) -> Option<i64> {
    let celsius = match from {
        Unit::Celsius => i128::from(value_milli),
        Unit::Fahrenheit => div_round((i128::from(value_milli) - 32_000) * 5, 9),
        Unit::Kelvin => {
            if value_milli < 0 {
                return None;
            }
            i128::from(value_milli) - 273_150
        }
        _ => return None,
    };
    let absolute_zero_celsius = -273_150;
    if celsius < absolute_zero_celsius {
        return None;
    }
    let converted = match to {
        Unit::Celsius => celsius,
        Unit::Fahrenheit => div_round(celsius * 9, 5) + 32_000,
        Unit::Kelvin => celsius + 273_150,
        _ => return None,
    };
    i64::try_from(converted).ok()
}

fn base_ratio(unit: Unit) -> Option<(i128, i128)> {
    match unit {
        Unit::Millimeters => Some((1, 1)),
        Unit::Centimeters => Some((10, 1)),
        Unit::Meters => Some((1_000, 1)),
        Unit::Kilometers => Some((1_000_000, 1)),
        Unit::Inches => Some((254, 10)),
        Unit::Feet => Some((3_048, 10)),
        Unit::Yards => Some((9_144, 10)),
        Unit::Miles => Some((1_609_344, 1)),
        Unit::Grams => Some((1, 1)),
        Unit::Kilograms => Some((1_000, 1)),
        Unit::Ounces => Some((28_349_523_125, 1_000_000_000)),
        Unit::Pounds => Some((45_359_237, 100_000)),
        Unit::Milliliters => Some((1, 1)),
        Unit::Liters => Some((1_000, 1)),
        Unit::Cups => Some((2_365_882_365, 10_000_000)),
        Unit::FluidOunces => Some((29_573_529_563, 1_000_000_000)),
        Unit::Pints => Some((473_176_473, 1_000_000)),
        Unit::Quarts => Some((946_352_946, 1_000_000)),
        Unit::Gallons => Some((3_785_411_784, 1_000_000)),
        Unit::Celsius | Unit::Fahrenheit | Unit::Kelvin => None,
    }
}

fn div_round(numerator: i128, denominator: i128) -> i128 {
    debug_assert!(denominator > 0);
    if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        -((-numerator + denominator / 2) / denominator)
    }
}

#[must_use]
pub fn units_for(category: UnitCategory) -> &'static [Unit] {
    match category {
        UnitCategory::Length => &LENGTH_UNITS,
        UnitCategory::Mass => &MASS_UNITS,
        UnitCategory::Temperature => &TEMPERATURE_UNITS,
        UnitCategory::Volume => &VOLUME_UNITS,
    }
}

fn unit_at(category: UnitCategory, index: usize) -> Unit {
    let units = units_for(category);
    units[index.min(units.len() - 1)]
}

fn previous_index(index: usize, count: usize) -> usize {
    index.checked_sub(1).unwrap_or(count - 1)
}

fn defaults_for(category: UnitCategory) -> (usize, usize, i64) {
    match category {
        UnitCategory::Length => (7, 3, 5 * FIXED_SCALE), // 5 mi -> km
        UnitCategory::Mass => (1, 3, FIXED_SCALE),       // 1 kg -> lb
        UnitCategory::Temperature => (1, 0, 32 * FIXED_SCALE), // 32 F -> C
        UnitCategory::Volume => (6, 1, FIXED_SCALE),     // 1 gal -> L
    }
}

/// Render a fixed-point value with up to three decimal places.
#[must_use]
pub fn format_milli(value_milli: i64) -> String {
    let negative = value_milli < 0;
    let absolute = value_milli.unsigned_abs();
    let whole = absolute / FIXED_SCALE as u64;
    let fraction = absolute % FIXED_SCALE as u64;
    if fraction == 0 {
        return if negative {
            format!("-{whole}")
        } else {
            whole.to_string()
        };
    }
    let mut fraction_text = format!("{fraction:03}");
    while fraction_text.ends_with('0') {
        fraction_text.pop();
    }
    if negative {
        format!("-{whole}.{fraction_text}")
    } else {
        format!("{whole}.{fraction_text}")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        convert_milli, format_milli, ConversionResult, ConverterField, Unit, UnitCategory,
        UnitConverterUiState, FIXED_SCALE, MAX_INPUT_MILLI,
    };

    #[test]
    fn converts_length_examples_with_thousandth_precision() {
        assert_eq!(
            convert_milli(FIXED_SCALE, Unit::Miles, Unit::Kilometers),
            ConversionResult::Value(1_609)
        );
        assert_eq!(
            convert_milli(5 * FIXED_SCALE, Unit::Miles, Unit::Kilometers),
            ConversionResult::Value(8_047)
        );
    }

    #[test]
    fn converts_mass_temperature_and_volume_examples() {
        assert_eq!(
            convert_milli(FIXED_SCALE, Unit::Kilograms, Unit::Pounds),
            ConversionResult::Value(2_205)
        );
        assert_eq!(
            convert_milli(32 * FIXED_SCALE, Unit::Fahrenheit, Unit::Celsius),
            ConversionResult::Value(0)
        );
        assert_eq!(
            convert_milli(100 * FIXED_SCALE, Unit::Celsius, Unit::Fahrenheit),
            ConversionResult::Value(212_000)
        );
        assert_eq!(
            convert_milli(FIXED_SCALE, Unit::Gallons, Unit::Liters),
            ConversionResult::Value(3_785)
        );
        assert_eq!(
            convert_milli(FIXED_SCALE, Unit::Cups, Unit::Milliliters),
            ConversionResult::Value(236_588)
        );
    }

    #[test]
    fn rejects_kelvin_below_absolute_zero_and_clamps_over_range_results() {
        assert_eq!(
            convert_milli(-1, Unit::Kelvin, Unit::Celsius),
            ConversionResult::Invalid
        );
        assert_eq!(
            convert_milli(MAX_INPUT_MILLI, Unit::Miles, Unit::Millimeters),
            ConversionResult::OverRange
        );
    }

    #[test]
    fn formats_values_without_unnecessary_trailing_zeroes() {
        assert_eq!(format_milli(8_000), "8");
        assert_eq!(format_milli(8_050), "8.05");
        assert_eq!(format_milli(-17_778), "-17.778");
    }

    #[test]
    fn ui_state_cycles_fields_categories_units_values_and_steps() {
        let mut state = UnitConverterUiState::default();
        assert_eq!(state.category, UnitCategory::Length);
        assert_eq!(state.active_field, ConverterField::Category);
        state.increase_active();
        assert_eq!(state.category, UnitCategory::Mass);
        state.select_next_field();
        state.increase_active();
        state.select_next_field();
        let original = state.value_milli;
        state.increase_active();
        assert!(state.value_milli > original);
        state.select_next_field();
        state.select_next_field();
        let step = state.step_milli();
        state.increase_active();
        assert_ne!(state.step_milli(), step);
    }
}
