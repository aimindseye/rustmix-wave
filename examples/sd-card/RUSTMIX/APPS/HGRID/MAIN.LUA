-- Rustmix-Wave v0.18.0 bounded bootstrap sample.
-- These top-level ui calls are executed from SD into a Rust-owned canvas.
ui.clear()
ui.text(24, 92, "HELLO GRID", "heading")
ui.text(24, 132, "SD LUA FOUNDATION", "body")
ui.rect(24, 174, 432, 392, false)
ui.grid(88, 230, 4, 4, 64, 64)
ui.text(24, 640, "Native dirty-region canvas", "body")
ui.text(24, 682, "Partial transport stays Rust-owned", "detail")
ui.invalidate_rect(24, 174, 432, 392)
ui.request_refresh()
