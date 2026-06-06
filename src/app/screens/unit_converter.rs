//! Offline fixed-point Unit Converter Tools application.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::{Text, UiTextStyle},
        widgets::{
            footer::draw_footer,
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    unit_converter::{format_milli, ConversionResult, ConverterField},
};

/// Render the interactive offline converter screen.
pub fn render_unit_converter(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let converter = state.unit_converter;
    let body = state.display.body_style();
    let heading = state.display.heading_style();
    let result = result_label(converter.result(), converter.to_unit().symbol());
    let value = format_milli(converter.value_milli);
    let step = format_milli(converter.step_milli());

    draw_header(
        display,
        state.display,
        "UNIT CONVERTER",
        "OFFLINE FIXED-POINT",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: converter.category.label(),
            middle: converter.active_field.label(),
            right: "OFFLINE",
        },
    )?;

    Text::new("Conversion", Point::new(22, 158), heading).draw(display)?;
    draw_field(
        display,
        184,
        "Category",
        converter.category.label(),
        converter.active_field == ConverterField::Category,
        body,
    )?;
    draw_field(
        display,
        246,
        "From",
        converter.from_unit().label(),
        converter.active_field == ConverterField::FromUnit,
        body,
    )?;
    draw_field(
        display,
        308,
        "Value",
        &value,
        converter.active_field == ConverterField::Value,
        body,
    )?;
    draw_field(
        display,
        370,
        "To",
        converter.to_unit().label(),
        converter.active_field == ConverterField::ToUnit,
        body,
    )?;
    draw_field(
        display,
        432,
        "Step",
        &step,
        converter.active_field == ConverterField::StepSize,
        body,
    )?;

    Rectangle::new(Point::new(22, 520), Size::new(436, 152))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    Text::new("RESULT", Point::new(42, 560), body).draw(display)?;
    Text::new(&result, Point::new(42, 624), state.display.large_style()).draw(display)?;

    draw_footer(
        display,
        state.display,
        "UP/DOWN  SELECT NEXT  HOLD BOOT BACK",
    )?;
    Ok(())
}

fn draw_field(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    value: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let border = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::On, 4)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(22, top), Size::new(436, 52))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(38, top + 34),
        style,
    )
    .draw(display)?;
    Text::new(label, Point::new(68, top + 34), style).draw(display)?;
    Text::new(value, Point::new(212, top + 34), style).draw(display)?;
    Ok(())
}

fn result_label(result: ConversionResult, symbol: &str) -> String {
    match result {
        ConversionResult::Value(value) => format!("{} {symbol}", format_milli(value)),
        ConversionResult::OverRange => "OVER RANGE".into(),
        ConversionResult::Invalid => "INVALID".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::render_unit_converter;
    use crate::{app::AppState, framebuffer::FrameBuffer, orientation::OrientedFrameBuffer};

    #[test]
    fn renders_default_offline_converter() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_unit_converter(&mut display, &AppState::default()).unwrap();
    }
}
