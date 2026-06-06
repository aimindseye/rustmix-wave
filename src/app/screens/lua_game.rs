//! SD Lua application catalog and native-canvas preview screens.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Line, PrimitiveStyle, Rectangle},
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
    games::canvas::{CanvasTextStyle, DrawCommand},
    lua_runtime::LUA_CATALOG_PAGE_SIZE,
    orientation::OrientedFrameBuffer,
};

pub fn render_lua_apps(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let catalog = &state.lua_runtime.catalog;
    let selected = state.lua_runtime.selected;
    let page_start = (selected / LUA_CATALOG_PAGE_SIZE) * LUA_CATALOG_PAGE_SIZE;
    let pages = catalog.entries.len().max(1).div_ceil(LUA_CATALOG_PAGE_SIZE);
    let page = format!("{}/{}", (page_start / LUA_CATALOG_PAGE_SIZE) + 1, pages);
    let heading = state.display.heading_style();
    let body = state.display.body_style();

    draw_header(display, state.display, "SD LUA APPS", "RUST-OWNED CANVAS")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "CATALOG",
            middle: &format!("{} apps", catalog.entries.len()),
            right: &page,
        },
    )?;
    Text::new("Select an SD-loaded app", Point::new(22, 158), heading).draw(display)?;
    Text::new("Hold BOOT to go back.", Point::new(22, 188), body).draw(display)?;

    if catalog.entries.is_empty() {
        Rectangle::new(Point::new(22, 232), Size::new(436, 228))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)?;
        Text::new("No Lua apps found.", Point::new(44, 300), heading).draw(display)?;
        Text::new("Install /RUSTMIX/APPS/HGRID", Point::new(44, 350), body).draw(display)?;
        Text::new(
            catalog.warning.as_deref().unwrap_or("Catalog is empty"),
            Point::new(44, 400),
            state.display.detail_style(),
        )
        .draw(display)?;
    } else {
        for (visible_index, entry) in catalog
            .entries
            .iter()
            .skip(page_start)
            .take(LUA_CATALOG_PAGE_SIZE)
            .enumerate()
        {
            let top = 214 + visible_index as i32 * 86;
            let is_selected = page_start + visible_index == selected;
            let border = if is_selected {
                PrimitiveStyle::with_stroke(BinaryColor::On, 4)
            } else {
                PrimitiveStyle::with_stroke(BinaryColor::On, 1)
            };
            Rectangle::new(Point::new(22, top), Size::new(436, 76))
                .into_styled(border)
                .draw(display)?;
            Text::new(
                if is_selected { ">" } else { " " },
                Point::new(34, top + 35),
                body,
            )
            .draw(display)?;
            Text::new(
                &entry.manifest.name,
                Point::new(56, top + 31),
                state.display.navigation_style(),
            )
            .draw(display)?;
            Text::new(
                &format!(
                    "{}  {}  {}",
                    entry.directory_name,
                    entry.manifest.kind.marker().to_ascii_uppercase(),
                    entry.manifest.version
                ),
                Point::new(56, top + 62),
                body,
            )
            .draw(display)?;
            Text::new("SD", Point::new(408, top + 62), body).draw(display)?;
        }
    }

    draw_footer(
        display,
        state.display,
        "UP/DOWN MOVE  SELECT OPEN  HOLD BOOT BACK",
    )?;
    Ok(())
}

pub fn render_lua_game(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(session) = state.lua_runtime.session.as_ref() else {
        return render_lua_error(display, state);
    };
    for command in session.canvas.commands() {
        draw_command(display, state, command)?;
    }
    Ok(())
}

pub fn render_lua_error(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    draw_header(display, state.display, "LUA APP ERROR", "NATIVE FALLBACK")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "SCRIPT",
            middle: "ERROR",
            right: "SAFE",
        },
    )?;
    Rectangle::new(Point::new(22, 210), Size::new(436, 270))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    Text::new("App could not open.", Point::new(44, 280), heading).draw(display)?;
    Text::new(
        state
            .lua_runtime
            .error
            .as_deref()
            .unwrap_or("No active app session"),
        Point::new(44, 350),
        body,
    )
    .draw(display)?;
    Text::new("Hold BOOT to return.", Point::new(44, 430), body).draw(display)?;
    draw_footer(display, state.display, "HOLD BOOT BACK")?;
    Ok(())
}

fn draw_command(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    command: &DrawCommand,
) -> Result<(), Infallible> {
    match command {
        DrawCommand::Clear => {}
        DrawCommand::Text { x, y, text, style } => {
            Text::new(text, Point::new(*x, *y), canvas_text_style(state, *style)).draw(display)?;
        }
        DrawCommand::Line { x1, y1, x2, y2 } => {
            Line::new(Point::new(*x1, *y1), Point::new(*x2, *y2))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
        }
        DrawCommand::Rect {
            x,
            y,
            width,
            height,
            filled,
        } => {
            let style = if *filled {
                PrimitiveStyle::with_fill(BinaryColor::On)
            } else {
                PrimitiveStyle::with_stroke(BinaryColor::On, 1)
            };
            Rectangle::new(
                Point::new(*x, *y),
                Size::new((*width).max(0) as u32, (*height).max(0) as u32),
            )
            .into_styled(style)
            .draw(display)?;
        }
        DrawCommand::Grid {
            x,
            y,
            columns,
            rows,
            cell_width,
            cell_height,
        } => {
            let width = i32::from(*columns) * *cell_width;
            let height = i32::from(*rows) * *cell_height;
            for column in 0..=i32::from(*columns) {
                let line_x = *x + column * *cell_width;
                Line::new(Point::new(line_x, *y), Point::new(line_x, *y + height))
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(display)?;
            }
            for row in 0..=i32::from(*rows) {
                let line_y = *y + row * *cell_height;
                Line::new(Point::new(*x, line_y), Point::new(*x + width, line_y))
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(display)?;
            }
        }
    }
    Ok(())
}

fn canvas_text_style(state: &AppState, style: CanvasTextStyle) -> UiTextStyle {
    match style {
        CanvasTextStyle::Body => state.display.body_style(),
        CanvasTextStyle::Heading => state.display.heading_style(),
        CanvasTextStyle::Detail => state.display.detail_style(),
    }
}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use crate::{
        app::{render_current_screen, AppState, ScreenRoute},
        framebuffer::FrameBuffer,
        games::{canvas::NativeGameCanvas, refresh_policy::GameRefreshPlan},
        lua_runtime::{
            event_bridge::LuaEventBridge,
            manifest::{LuaAppEntry, LuaAppKind, LuaAppManifest},
            LuaAppSession,
        },
    };

    #[test]
    fn renders_native_canvas_without_exposing_panel_transport() {
        let mut canvas = NativeGameCanvas::default();
        canvas.grid(80, 220, 4, 4, 64, 64).unwrap();
        let mut state = AppState::default();
        state.lua_runtime.session = Some(LuaAppSession {
            entry: LuaAppEntry {
                directory_name: "HGRID".into(),
                directory: std::path::PathBuf::from("/tmp/HGRID"),
                manifest: LuaAppManifest {
                    id: "hello_grid".into(),
                    name: "Hello Grid".into(),
                    kind: LuaAppKind::Game,
                    entry: "MAIN.LUA".into(),
                    version: "1.0".into(),
                    input: vec![],
                },
            },
            source_bytes: 10,
            canvas,
            refresh_plan: GameRefreshPlan::PartialFullscreen { regions: vec![] },
            event_bridge: LuaEventBridge::Static,
        });
        state.router.navigate_to(ScreenRoute::LuaGame);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        assert!(frame.is_black(Point::new(220, 399)).is_some());
    }
}
