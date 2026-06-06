//! Generic category list screen with bounded paging.

use core::convert::Infallible;

use embedded_graphics::prelude::Point;

use crate::app::typography::Text;

use crate::{
    app::{
        menu::{category_entries, CATEGORY_PAGE_SIZE},
        state::AppState,
        widgets::{
            footer::draw_footer,
            header::draw_header,
            menu_row::draw_menu_row,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
};

pub fn render_category(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let route = state.active_route();
    let entries = category_entries(route);
    let selected = state.category_selection(route);
    let page_start = (selected / CATEGORY_PAGE_SIZE) * CATEGORY_PAGE_SIZE;
    let pages = entries.len().max(1).div_ceil(CATEGORY_PAGE_SIZE);
    let page = format!("{}/{}", (page_start / CATEGORY_PAGE_SIZE) + 1, pages);
    let title = route.label().to_ascii_uppercase();
    let heading = state.display.heading_style();
    let body = state.display.body_style();

    draw_header(display, state.display, &title, "CATEGORY MENU")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "CATEGORY",
            middle: &format!("{} entries", entries.len()),
            right: &page,
        },
    )?;
    Text::new("Select an entry", Point::new(22, 158), heading).draw(display)?;
    Text::new("Hold BOOT to go back.", Point::new(22, 188), body).draw(display)?;

    for (visible_index, entry) in entries
        .iter()
        .copied()
        .skip(page_start)
        .take(CATEGORY_PAGE_SIZE)
        .enumerate()
    {
        draw_menu_row(
            display,
            214 + visible_index as i32 * 86,
            entry,
            page_start + visible_index == selected,
            state.display,
        )?;
    }

    draw_footer(
        display,
        state.display,
        "UP/DOWN MOVE  SELECT OPEN  HOLD BOOT BACK",
    )?;
    Ok(())
}
