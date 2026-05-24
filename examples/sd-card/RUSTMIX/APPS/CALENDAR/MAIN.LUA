-- Rustmix Lua sample app: Calendar
-- X4 SD path: /RUSTMIX/APPS/CALENDAR/MAIN.LUA
-- Logical app id: calendar
-- Required capabilities: display, input, storage, time

-- Optional VM proof expression. Build firmware with --features lua-vm to use it.
-- 1 selects the first non-comment record in EVENTS.TXT.
vm_event_index_expression = "return 1 + 0"

-- The visible event data is read from /RUSTMIX/APPS/CALENDAR/EVENTS.TXT.
-- EVENTS.TXT records use | as a separator:
-- YYYY-MM-DD|Title|Optional detail

return {
  id = "calendar",
  name = "Calendar",
}
