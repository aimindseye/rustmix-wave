-- ============================================================================
-- BABA IS YOU (1-Bit E-Paper Edition for Rustmix Wave / ESP32-S3 3.97" 800x480)
-- Loads levels dynamically from MicroSD: /apps/baba_is_you/levels.txt
-- ============================================================================

local SCREEN_W = 800
local SCREEN_H = 480
local GRID_W   = 12
local GRID_H   = 8
local CELL_SZ  = 52
local BOARD_W  = GRID_W * CELL_SZ -- 624px
local BOARD_H  = GRID_H * CELL_SZ -- 416px
local BOARD_X  = 16
local BOARD_Y  = 48
local HUD_X    = BOARD_X + BOARD_W + 16 -- 656px

local LEVEL_PATHS = {
  "/sd/apps/baba_is_you/levels.txt",
  "/sdcard/apps/baba_is_you/levels.txt",
  "/apps/baba_is_you/levels.txt",
  "levels.txt"
}

-- ----------------------------------------------------------------------------
-- Hardware Abstraction Layer (Display + MicroSD File I/O)
-- ----------------------------------------------------------------------------
local gfx = {}

function gfx.clear()
  if display and display.clear then display.clear(1)
  elseif gfx_clear then gfx_clear(1) end
end

function gfx.rect(x, y, w, h, color)
  if display and display.rect then display.rect(x, y, w, h, color)
  elseif gfx_rect then gfx_rect(x, y, w, h, color) end
end

function gfx.fill_rect(x, y, w, h, color)
  if display and display.fill_rect then display.fill_rect(x, y, w, h, color)
  elseif gfx_fill_rect then gfx_fill_rect(x, y, w, h, color) end
end

function gfx.line(x0, y0, x1, y1, color)
  if display and display.line then display.line(x0, y0, x1, y1, color)
  elseif gfx_line then gfx_line(x0, y0, x1, y1, color) end
end

function gfx.text(x, y, str, color, scale)
  if display and display.text then display.text(x, y, str, color or 0, scale or 1)
  elseif gfx_text then gfx_text(x, y, str, color or 0) end
end

function gfx.flush()
  if display and display.flush then display.flush("partial")
  elseif gfx_flush then gfx_flush() end
end

-- Reads raw text from MicroSD via Rustmix FS bindings or standard Lua io
local function read_sd_file()
  for _, path in ipairs(LEVEL_PATHS) do
    if fs and fs.read_text then
      local ok, data = pcall(fs.read_text, path)
      if ok and data and #data > 0 then return data, path end
    end
    if sd and sd.read_file then
      local ok, data = pcall(sd.read_file, path)
      if ok and data and #data > 0 then return data, path end
    end
    if sd_read then
      local ok, data = pcall(sd_read, path)
      if ok and data and #data > 0 then return data, path end
    end
    if io and io.open then
      local f = io.open(path, "r")
      if f then
        local data = f:read("*a")
        f:close()
        if data and #data > 0 then return data, path end
      end
    end
  end
  return nil, "FALLBACK"
end

-- ----------------------------------------------------------------------------
-- Token Dictionary & SD Level Pack Parser
-- ----------------------------------------------------------------------------
local TOKEN_MAP = {
  ["B"] = { name = "baba",  kind = "text", role = "noun", label = "BABA" },
  ["W"] = { name = "wall",  kind = "text", role = "noun", label = "WALL" },
  ["R"] = { name = "rock",  kind = "text", role = "noun", label = "ROCK" },
  ["F"] = { name = "flag",  kind = "text", role = "noun", label = "FLAG" },
  ["K"] = { name = "water", kind = "text", role = "noun", label = "WATR" },
  ["I"] = { name = "is",    kind = "text", role = "verb", label = " IS " },
  ["Y"] = { name = "you",   kind = "text", role = "prop", label = "YOU " },
  ["S"] = { name = "stop",  kind = "text", role = "prop", label = "STOP" },
  ["P"] = { name = "push",  kind = "text", role = "prop", label = "PUSH" },
  ["X"] = { name = "win",   kind = "text", role = "prop", label = "WIN " },
  ["N"] = { name = "sink",  kind = "text", role = "prop", label = "SINK" },

  ["b"] = { name = "baba",  kind = "obj", label = "baba"  },
  ["w"] = { name = "wall",  kind = "obj", label = "wall"  },
  ["r"] = { name = "rock",  kind = "obj", label = "rock"  },
  ["f"] = { name = "flag",  kind = "obj", label = "flag"  },
  ["k"] = { name = "water", kind = "obj", label = "water" },
}

local FALLBACK_TXT = [[
; 01: SD MISSING (FALLBACK)
BIY.....FI..
..........X.
..wwwwwwww..
..w......w..
..w.b..r.f..
..w......w..
..wwwwwwww..
WIS.....RIP.
]]

local LEVELS = {}
local sd_source_label = "SD:NONE"

local function parse_levels_txt(raw_text)
  local parsed = {}
  local current = nil

  for line in string.gmatch(raw_text .. "\n", "([^\r\n]*)\r?\n") do
    local trimmed = string.match(line, "^%s*(.-)%s*$")
    if trimmed ~= "" and string.sub(trimmed, 1, 1) ~= "#" then
      if string.sub(trimmed, 1, 1) == ";" then
        if current and #current.rows > 0 then
          table.insert(parsed, current)
        end
        local title = string.match( string.sub(trimmed, 2), "^%s*(.-)%s*$" )
        current = { title = title ~= "" and title or ("LEVEL " .. (#parsed + 1)), rows = {} }
      elseif current and #current.rows < GRID_H then
        -- Pad or trim row to exact GRID_W (12 chars)
        if #trimmed < GRID_W then
          trimmed = trimmed .. string.rep(".", GRID_W - #trimmed)
        elseif #trimmed > GRID_W then
          trimmed = string.sub(trimmed, 1, GRID_W)
        end
        table.insert(current.rows, trimmed)
      end
    end
  end

  if current and #current.rows > 0 then
    table.insert(parsed, current)
  end
  return parsed
end

local function load_level_pack_from_sd()
  local raw, src = read_sd_file()
  if raw then
    LEVELS = parse_levels_txt(raw)
    sd_source_label = "SD:OK (" .. tostring(#LEVELS) .. ")"
  end
  if #LEVELS == 0 then
    LEVELS = parse_levels_txt(FALLBACK_TXT)
    sd_source_label = "SD:FALLBACK"
  end
end

-- ----------------------------------------------------------------------------
-- Game State & Rule Engine
-- ----------------------------------------------------------------------------
local state = {
  level_idx    = 1,
  entities     = {},
  rules        = {},
  rule_strings = {},
  undo_stack   = {},
  turn_count   = 0,
  won          = false,
  lost         = false,
}

local function deep_copy_entities(list)
  local copy = {}
  for i, e in ipairs(list) do
    copy[i] = {
      id = e.id, name = e.name, kind = e.kind,
      role = e.role, label = e.label, x = e.x, y = e.y
    }
  end
  return copy
end

local function parse_rules()
  state.rules = { text = { push = true } }
  state.rule_strings = {}

  local function add_rule(subj, target, subj_label, target_label)
    state.rules[subj] = state.rules[subj] or {}
    if not state.rules[subj][target] then
      state.rules[subj][target] = true
      table.insert(state.rule_strings, subj_label .. " IS " .. target_label)
    end
  end

  local function get_texts_at(x, y)
    local res = {}
    for _, e in ipairs(state.entities) do
      if e.kind == "text" and e.x == x and e.y == y then
        table.insert(res, e)
      end
    end
    return res
  end

  for y = 1, GRID_H do
    for x = 1, GRID_W do
      for _, mid in ipairs(get_texts_at(x, y)) do
        if mid.role == "verb" and mid.name == "is" then
          for _, left in ipairs(get_texts_at(x - 1, y)) do
            if left.role == "noun" then
              for _, right in ipairs(get_texts_at(x + 1, y)) do
                if right.role == "prop" or right.role == "noun" then
                  add_rule(left.name, right.name, left.label, right.label)
                end
              end
            end
          end
          for _, up in ipairs(get_texts_at(x, y - 1)) do
            if up.role == "noun" then
              for _, down in ipairs(get_texts_at(x, y + 1)) do
                if down.role == "prop" or down.role == "noun" then
                  add_rule(up.name, down.name, up.label, down.label)
                end
              end
            end
          end
        end
      end
    end
  end
end

local function has_prop(e, prop)
  if e.kind == "text" and prop == "push" then return true end
  if e.kind == "obj" and state.rules[e.name] and state.rules[e.name][prop] then
    return true
  end
  return false
end

local function check_status()
  state.won = false
  local any_you = false
  for _, a in ipairs(state.entities) do
    if has_prop(a, "you") then
      any_you = true
      for _, b in ipairs(state.entities) do
        if a.x == b.x and a.y == b.y and has_prop(b, "win") then
          state.won = true
          return
        end
      end
    end
  end
  state.lost = not any_you
end

local function load_level(idx)
  state.level_idx  = ((idx - 1) % #LEVELS) + 1
  state.entities   = {}
  state.undo_stack = {}
  state.turn_count = 0
  state.won        = false
  state.lost       = false

  local lvl = LEVELS[state.level_idx]
  local next_id = 1

  for y = 1, GRID_H do
    local row = lvl.rows[y] or ""
    for x = 1, GRID_W do
      local ch = string.sub(row, x, x)
      local def = TOKEN_MAP[ch]
      if def then
        table.insert(state.entities, {
          id = next_id, name = def.name, kind = def.kind,
          role = def.role, label = def.label, x = x, y = y
        })
        next_id = next_id + 1
      end
    end
  end

  parse_rules()
  check_status()
end

-- ----------------------------------------------------------------------------
-- Turn Execution & Undo
-- ----------------------------------------------------------------------------
local function can_step(x, y, dx, dy, push_chain)
  local nx, ny = x + dx, y + dy
  if nx < 1 or nx > GRID_W or ny < 1 or ny > GRID_H then return false end

  local needs_push = false
  for _, e in ipairs(state.entities) do
    if e.x == nx and e.y == ny then
      if has_prop(e, "stop") and not has_prop(e, "push") then
        return false
      elseif has_prop(e, "push") then
        needs_push = true
        table.insert(push_chain, e)
      end
    end
  end

  if needs_push then
    return can_step(nx, ny, dx, dy, push_chain)
  end
  return true
end

local function step_turn(dx, dy)
  if state.won then
    load_level(state.level_idx + 1)
    return true
  end

  local movers = {}
  for _, e in ipairs(state.entities) do
    if has_prop(e, "you") then table.insert(movers, e) end
  end
  if #movers == 0 then return false end

  table.sort(movers, function(a, b)
    return (a.x * dx + a.y * dy) > (b.x * dx + b.y * dy)
  end)

  local snapshot = deep_copy_entities(state.entities)
  local moved_any = false
  local moved_ids = {}

  for _, you_ent in ipairs(movers) do
    local chain = {}
    if can_step(you_ent.x, you_ent.y, dx, dy, chain) then
      moved_any = true
      if not moved_ids[you_ent.id] then
        you_ent.x = you_ent.x + dx
        you_ent.y = you_ent.y + dy
        moved_ids[you_ent.id] = true
      end
      for _, pushed in ipairs(chain) do
        if not moved_ids[pushed.id] then
          pushed.x = pushed.x + dx
          pushed.y = pushed.y + dy
          moved_ids[pushed.id] = true
        end
      end
    end
  end

  if not moved_any then return false end

  table.insert(state.undo_stack, snapshot)
  if #state.undo_stack > 64 then table.remove(state.undo_stack, 1) end
  state.turn_count = state.turn_count + 1

  parse_rules()

  local nouns = { "baba", "wall", "rock", "flag", "water" }
  for _, e in ipairs(state.entities) do
    if e.kind == "obj" and state.rules[e.name] then
      for _, target_noun in ipairs(nouns) do
        if state.rules[e.name][target_noun] then
          e.name  = target_noun
          e.label = target_noun
          break
        end
      end
    end
  end

  for y = 1, GRID_H do
    for x = 1, GRID_W do
      local cell = {}
      local has_sink = false
      for _, e in ipairs(state.entities) do
        if e.x == x and e.y == y then
          table.insert(cell, e)
          if has_prop(e, "sink") then has_sink = true end
        end
      end
      if has_sink and #cell > 1 then
        local survivors = {}
        for _, e in ipairs(state.entities) do
          if not (e.x == x and e.y == y) then table.insert(survivors, e) end
        end
        state.entities = survivors
      end
    end
  end

  parse_rules()
  check_status()
  return true
end

local function undo_turn()
  if #state.undo_stack == 0 then return false end
  state.entities = table.remove(state.undo_stack)
  state.turn_count = math.max(0, state.turn_count - 1)
  parse_rules()
  check_status()
  return true
end

-- ----------------------------------------------------------------------------
-- 1-Bit E-Paper Renderer
-- ----------------------------------------------------------------------------
local function draw_entity(e)
  local px = BOARD_X + (e.x - 1) * CELL_SZ
  local py = BOARD_Y + (e.y - 1) * CELL_SZ

  if e.kind == "text" then
    gfx.fill_rect(px + 4, py + 4, CELL_SZ - 8, CELL_SZ - 8, 0)
    if e.role == "verb" then
      gfx.rect(px + 6, py + 6, CELL_SZ - 12, CELL_SZ - 12, 1)
    end
    gfx.text(px + 10, py + 20, e.label, 1, 1)
  else
    if e.name == "baba" then
      gfx.fill_rect(px + 14, py + 20, 24, 20, 0)
      gfx.fill_rect(px + 16, py + 8,  6,  12, 0)
      gfx.fill_rect(px + 30, py + 8,  6,  12, 0)
      gfx.fill_rect(px + 18, py + 24, 4,  4,  1)
      gfx.fill_rect(px + 30, py + 24, 4,  4,  1)
    elseif e.name == "wall" then
      gfx.rect(px + 4, py + 4, CELL_SZ - 8, CELL_SZ - 8, 0)
      gfx.line(px + 4, py + 20, px + CELL_SZ - 4, py + 20, 0)
      gfx.line(px + 4, py + 34, px + CELL_SZ - 4, py + 34, 0)
      gfx.line(px + 26, py + 4, px + 26, py + 20, 0)
      gfx.line(px + 16, py + 20, px + 16, py + 34, 0)
      gfx.line(px + 36, py + 20, px + 36, py + 34, 0)
    elseif e.name == "rock" then
      gfx.rect(px + 10, py + 10, CELL_SZ - 20, CELL_SZ - 20, 0)
      gfx.rect(px + 12, py + 12, CELL_SZ - 24, CELL_SZ - 24, 0)
      gfx.text(px + 18, py + 20, "()", 0, 1)
    elseif e.name == "flag" then
      gfx.fill_rect(px + 14, py + 8, 4, CELL_SZ - 16, 0)
      gfx.fill_rect(px + 18, py + 8, 22, 16, 0)
      gfx.fill_rect(px + 10, py + CELL_SZ - 10, 16, 4, 0)
    elseif e.name == "water" then
      gfx.text(px + 10, py + 12, "~~~~", 0, 1)
      gfx.text(px + 10, py + 28, "~~~~", 0, 1)
    end
  end
end

function render()
  gfx.clear()

  local lvl = LEVELS[state.level_idx]
  gfx.fill_rect(0, 0, SCREEN_W, 36, 0)
  gfx.text(16, 10, "BABA IS YOU  |  " .. lvl.title, 1, 1)
  gfx.text(SCREEN_W - 260, 10, sd_source_label .. " | STEPS:" .. tostring(state.turn_count), 1, 1)

  gfx.rect(BOARD_X - 2, BOARD_Y - 2, BOARD_W + 4, BOARD_H + 4, 0)
  for y = 0, GRID_H do
    for x = 0, GRID_W do
      gfx.fill_rect(BOARD_X + x * CELL_SZ - 1, BOARD_Y + y * CELL_SZ - 1, 2, 2, 0)
    end
  end

  for _, e in ipairs(state.entities) do
    if e.kind == "obj" then draw_entity(e) end
  end
  for _, e in ipairs(state.entities) do
    if e.kind == "text" then draw_entity(e) end
  end

  gfx.rect(HUD_X, BOARD_Y, SCREEN_W - HUD_X - 12, BOARD_H, 0)
  gfx.fill_rect(HUD_X, BOARD_Y, SCREEN_W - HUD_X - 12, 28, 0)
  gfx.text(HUD_X + 8, BOARD_Y + 6, "ACTIVE LAWS", 1, 1)

  for i, r_str in ipairs(state.rule_strings) do
    if i <= 10 then
      gfx.text(HUD_X + 8, BOARD_Y + 20 + i * 24, "* " .. r_str, 0, 1)
    end
  end

  gfx.line(HUD_X, BOARD_Y + BOARD_H - 96, SCREEN_W - 12, BOARD_Y + BOARD_H - 96, 0)
  gfx.text(HUD_X + 8, BOARD_Y + BOARD_H - 84, "TILT: Move", 0, 1)
  gfx.text(HUD_X + 8, BOARD_Y + BOARD_H - 60, "CCW : Undo", 0, 1)
  gfx.text(HUD_X + 8, BOARD_Y + BOARD_H - 36, "CW  : Next/Rst", 0, 1)

  if state.won then
    gfx.fill_rect(BOARD_X + 112, BOARD_Y + 150, 400, 100, 0)
    gfx.rect(BOARD_X + 116, BOARD_Y + 154, 392, 92, 1)
    gfx.text(BOARD_X + 180, BOARD_Y + 178, "CONGRATULATIONS!", 1, 1)
    gfx.text(BOARD_X + 148, BOARD_Y + 210, "TILT OR CLICK FOR NEXT LEVEL", 1, 1)
  elseif state.lost then
    gfx.fill_rect(BOARD_X + 112, BOARD_Y + 150, 400, 80, 0)
    gfx.text(BOARD_X + 148, BOARD_Y + 182, "NO [YOU] LEFT! ROTATE CCW TO UNDO", 1, 1)
  end

  gfx.flush()
end

-- ----------------------------------------------------------------------------
-- Lifecycle & Input Hooks
-- ----------------------------------------------------------------------------
function init()
  load_level_pack_from_sd()
  load_level(1)
  render()
end

function on_input(action)
  local dirty = false
  if action == "up" or action == "tilt_up" then
    dirty = step_turn(0, -1)
  elseif action == "down" or action == "tilt_down" then
    dirty = step_turn(0, 1)
  elseif action == "left" or action == "tilt_left" then
    dirty = step_turn(-1, 0)
  elseif action == "right" or action == "tilt_right" then
    dirty = step_turn(1, 0)
  elseif action == "undo" or action == "rotary_ccw" or action == "btn_back" then
    dirty = undo_turn()
  elseif action == "reset" or action == "rotary_cw" or action == "btn_select" then
    if state.won then
      load_level(state.level_idx + 1)
    else
      load_level(state.level_idx)
    end
    dirty = true
  elseif action == "reload_sd" then
    load_level_pack_from_sd()
    load_level(state.level_idx)
    dirty = true
  end

  if dirty then render() end
end

local last_tilt_ms = 0
function on_tilt(pitch, roll, now_ms)
  if now_ms and (now_ms - last_tilt_ms < 280) then return end
  local thresh = 18.0
  if math.abs(pitch) > math.abs(roll) then
    if pitch < -thresh then last_tilt_ms = now_ms or 0; on_input("up")
    elseif pitch > thresh then last_tilt_ms = now_ms or 0; on_input("down") end
  else
    if roll < -thresh then last_tilt_ms = now_ms or 0; on_input("left")
    elseif roll > thresh then last_tilt_ms = now_ms or 0; on_input("right") end
  end
end

init()
