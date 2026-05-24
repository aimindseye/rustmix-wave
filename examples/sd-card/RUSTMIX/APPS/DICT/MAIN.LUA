-- Rustmix Dictionary prefix-shard app repair.
--
-- Reads the splitter output:
--   /RUSTMIX/APPS/DICT/INDEX.TXT
--   /RUSTMIX/APPS/DICT/DATA/<PREFIX>.JSN
--
-- Important behavior:
--   * does not assume single-letter G.JSN exists
--   * treats * as wildcard/search action, not as a literal key
--   * scans only selected <=16 KiB shard files

local M = {}

local ROOT = "/RUSTMIX/APPS/DICT"
local INDEX_PATH = ROOT .. "/INDEX.TXT"
local SHARD_LIMIT = 16384
local MARKER = "rustmix-lua-dictionary-pack-integrity-ok"

local state = {
  query = "",
  wildcard = false,
  selected = 1,
  message = "Type a word. * does prefix search.",
  word = "",
  body = "",
  shard = "",
}

local keys = {
  {"A", "B", "C", "D", "E", "F"},
  {"G", "H", "I", "J", "K", "L"},
  {"M", "N", "O", "P", "Q", "R"},
  {"S", "T", "U", "V", "W", "X"},
  {"Y", "Z", "DEL", "CLR", "GO", "*"},
}

local flat_keys = {}
for _, row in ipairs(keys) do
  for _, key in ipairs(row) do
    flat_keys[#flat_keys + 1] = key
  end
end

local function upper(s)
  s = tostring(s or "")
  local out = {}
  for i = 1, #s do
    local b = string.byte(s, i)
    if b >= 97 and b <= 122 then
      out[#out + 1] = string.char(b - 32)
    else
      out[#out + 1] = string.char(b)
    end
  end
  return table.concat(out)
end

local function trim(s)
  return tostring(s or ""):match("^%s*(.-)%s*$") or ""
end

local function starts_with(s, prefix)
  return string.sub(s, 1, #prefix) == prefix
end

local function normalize_query(q)
  q = trim(upper(q or ""))
  q = q:gsub("[%*_]+$", "")
  return trim(q)
end

local function alpha_prefix(q)
  q = normalize_query(q)
  if q == "" then
    return ""
  end
  local first = string.sub(q, 1, 1)
  if not first:match("%a") then
    return "OTHERS"
  end
  local out = {}
  for i = 1, #q do
    local ch = string.sub(q, i, i)
    if not ch:match("%a") then
      break
    end
    out[#out + 1] = ch
    if #out >= 5 then
      break
    end
  end
  if #out == 0 then
    return "OTHERS"
  end
  return table.concat(out)
end

local function shard_base(name)
  name = upper(name or "")
  local base = name:gsub("%d+$", "")
  if base == "" then
    return name
  end
  return base
end

local function read_file(ctx, path)
  if ctx and type(ctx.read_file) == "function" then
    local ok, data = pcall(ctx.read_file, path)
    if ok and data then return data end
  end
  if ctx and ctx.storage and type(ctx.storage.read_file) == "function" then
    local ok, data = pcall(ctx.storage.read_file, path)
    if ok and data then return data end
  end
  if _G.rustmix and type(_G.rustmix.read_file) == "function" then
    local ok, data = pcall(_G.rustmix.read_file, path)
    if ok and data then return data end
  end
  if _G.storage and type(_G.storage.read_file) == "function" then
    local ok, data = pcall(_G.storage.read_file, path)
    if ok and data then return data end
  end
  if io and type(io.open) == "function" then
    local f = io.open(path, "rb") or io.open(path:gsub("^/", ""), "rb")
    if f then
      local data = f:read("*a")
      f:close()
      return data
    end
  end
  return nil
end

local function read_index(ctx)
  local data = read_file(ctx, INDEX_PATH)
  if not data then
    data = read_file(ctx, "INDEX.TXT")
  end
  if not data then
    return nil, "missing INDEX.TXT"
  end
  local rows = {}
  for line in tostring(data):gmatch("[^\r\n]+") do
    line = trim(line)
    if line ~= "" and string.sub(line, 1, 1) ~= "#" then
      local sep = line:find("|", 1, true)
      if not sep then
        return nil, "bad index line"
      end
      local name = trim(string.sub(line, 1, sep - 1))
      local rel = trim(string.sub(line, sep + 1))
      if name ~= "" and rel ~= "" then
        rows[#rows + 1] = { name = upper(name), rel = rel }
      end
    end
  end
  if #rows == 0 then
    return nil, "empty INDEX.TXT"
  end
  return rows, nil
end

local function row_matches(row_name, prefix)
  local base = shard_base(row_name)
  if prefix == "OTHERS" then
    return base == "OTHERS"
  end
  return starts_with(base, prefix) or starts_with(prefix, base)
end

local function candidate_rows(rows, q)
  local prefix = alpha_prefix(q)
  if prefix == "" then
    return {}
  end
  local out = {}
  for _, row in ipairs(rows) do
    if row_matches(row.name, prefix) then
      out[#out + 1] = row
    end
  end
  table.sort(out, function(a, b)
    local abase = shard_base(a.name)
    local bbase = shard_base(b.name)
    if #abase ~= #bbase then
      return #abase > #bbase
    end
    return a.name < b.name
  end)
  return out
end

local function compose_path(rel)
  rel = tostring(rel or "")
  if string.sub(rel, 1, 1) == "/" then
    return rel
  end
  return ROOT .. "/" .. rel
end

local function skip_ws(s, pos)
  while pos <= #s do
    local c = string.sub(s, pos, pos)
    if c ~= " " and c ~= "\t" and c ~= "\r" and c ~= "\n" then
      break
    end
    pos = pos + 1
  end
  return pos
end

local function parse_string_end(s, pos)
  local i = pos + 1
  local escaped = false
  while i <= #s do
    local c = string.sub(s, i, i)
    if escaped then
      escaped = false
    elseif c == "\\" then
      escaped = true
    elseif c == '"' then
      return i
    end
    i = i + 1
  end
  return nil
end

local function parse_value_end(s, pos)
  local first = string.sub(s, pos, pos)
  if first == '"' then
    local e = parse_string_end(s, pos)
    if not e then return nil end
    return e + 1
  end
  if first ~= "{" and first ~= "[" then
    local i = pos
    while i <= #s do
      local c = string.sub(s, i, i)
      if c == "," or c == "}" or c == "]" then
        return i
      end
      i = i + 1
    end
    return i
  end

  local obj = 0
  local arr = 0
  local instr = false
  local escaped = false
  local i = pos
  while i <= #s do
    local c = string.sub(s, i, i)
    if instr then
      if escaped then
        escaped = false
      elseif c == "\\" then
        escaped = true
      elseif c == '"' then
        instr = false
      end
    else
      if c == '"' then
        instr = true
      elseif c == "{" then
        obj = obj + 1
      elseif c == "}" then
        obj = obj - 1
        if obj == 0 and arr == 0 then return i + 1 end
      elseif c == "[" then
        arr = arr + 1
      elseif c == "]" then
        arr = arr - 1
        if obj == 0 and arr == 0 then return i + 1 end
      end
    end
    i = i + 1
  end
  return nil
end

local function unquote_json_string(s)
  s = tostring(s or "")
  if string.sub(s, 1, 1) == '"' and string.sub(s, -1) == '"' then
    s = string.sub(s, 2, -2)
  end
  s = s:gsub('\\"', '"'):gsub('\\n', ' '):gsub('\\r', ' '):gsub('\\t', ' ')
  return s
end

local function extract_json_entry(json, q, prefix_mode)
  q = normalize_query(q)
  local pos = skip_ws(json, 1)
  if string.sub(json, pos, pos) ~= "{" then
    return nil, nil, "parse failed: shard JSON"
  end
  pos = pos + 1
  while pos <= #json do
    pos = skip_ws(json, pos)
    local c = string.sub(json, pos, pos)
    if c == "}" then
      return nil, nil, nil
    end
    if c ~= '"' then
      return nil, nil, "parse failed: shard key"
    end
    local key_end = parse_string_end(json, pos)
    if not key_end then return nil, nil, "parse failed: shard key" end
    local key = string.sub(json, pos + 1, key_end - 1)
    pos = skip_ws(json, key_end + 1)
    if string.sub(json, pos, pos) ~= ":" then
      return nil, nil, "parse failed: shard separator"
    end
    pos = skip_ws(json, pos + 1)
    local value_start = pos
    local value_end = parse_value_end(json, pos)
    if not value_end then return nil, nil, "parse failed: shard value" end

    if key == q or (prefix_mode and starts_with(key, q)) then
      return key, string.sub(json, value_start, value_end - 1), nil
    end

    pos = skip_ws(json, value_end)
    local sep = string.sub(json, pos, pos)
    if sep == "," then
      pos = pos + 1
    elseif sep == "}" then
      return nil, nil, nil
    else
      return nil, nil, "parse failed: shard delimiter"
    end
  end
  return nil, nil, "parse failed: shard EOF"
end

local function compact_definition(raw)
  raw = trim(raw or "")
  if raw == "" then return "" end
  if string.sub(raw, 1, 1) == '"' then
    return unquote_json_string(raw)
  end
  local meaning = raw:match('"meaning"%s*:%s*"(.-)"')
    or raw:match('"definition"%s*:%s*"(.-)"')
    or raw:match('"def"%s*:%s*"(.-)"')
    or raw:match('"text"%s*:%s*"(.-)"')
  if meaning then
    return unquote_json_string(meaning)
  end
  raw = raw:gsub('[{}%[%]"]', ''):gsub(',', ', '):gsub(':', ': ')
  if #raw > 180 then
    raw = string.sub(raw, 1, 177) .. "..."
  end
  return raw
end

local function lookup(ctx, q, prefix_mode)
  q = normalize_query(q)
  if q == "" then
    return nil, nil, nil, "type a word first"
  end

  local rows, idx_err = read_index(ctx)
  if not rows then
    return nil, nil, nil, idx_err
  end

  local candidates = candidate_rows(rows, q)
  if #candidates == 0 then
    return nil, nil, nil, "no shard in INDEX.TXT for " .. alpha_prefix(q)
  end

  local missing = {}
  local bad = {}
  local readable = 0

  local function try_rows(as_prefix)
    for _, row in ipairs(candidates) do
      local path = compose_path(row.rel)
      local data = read_file(ctx, path)
      if data and #data <= SHARD_LIMIT then
        readable = readable + 1
        local word, raw, parse_err = extract_json_entry(data, q, as_prefix)
        if word then
          return word, compact_definition(raw), row.name, nil
        elseif parse_err then
          bad[#bad + 1] = row.name .. ": " .. parse_err
        end
      elseif data and #data > SHARD_LIMIT then
        bad[#bad + 1] = row.name .. ": shard too large " .. tostring(#data)
      else
        missing[#missing + 1] = row.name
      end
    end
    return nil, nil, nil, nil
  end

  local word, body, shard, err = try_rows(prefix_mode)
  if word then
    return word, body, shard, err
  end

  if not prefix_mode then
    word, body, shard, err = try_rows(true)
    if word then
      return word, body, shard, err
    end
  end

  if readable == 0 and #missing > 0 then
    return nil, nil, nil, "dictionary pack incomplete: missing " .. missing[1]
  end
  if #bad > 0 then
    return nil, nil, nil, bad[1]
  end
  if #missing > 0 then
    return nil, nil, nil, "word not found; skipped missing " .. missing[1]
  end
  return nil, nil, nil, "word not found"
end

local function do_lookup(ctx, prefix_mode)
  local q = normalize_query(state.query)
  state.wildcard = prefix_mode
  state.word = ""
  state.body = ""
  state.shard = ""
  local word, body, shard, err = lookup(ctx, q, prefix_mode)
  if word then
    state.word = word
    state.body = body
    state.shard = shard or ""
    state.message = "found in " .. state.shard
  else
    state.message = err or "not found"
    if shard then state.shard = shard end
  end
end

local function select_key(ctx, key)
  if key == "DEL" then
    state.query = string.sub(state.query, 1, math.max(0, #state.query - 1))
    state.wildcard = false
  elseif key == "CLR" then
    state.query = ""
    state.wildcard = false
    state.word = ""
    state.body = ""
    state.shard = ""
    state.message = "cleared"
  elseif key == "GO" then
    do_lookup(ctx, false)
  elseif key == "*" then
    do_lookup(ctx, true)
  else
    if #state.query < 32 then
      state.query = state.query .. key
      state.wildcard = false
    end
  end
end

local function move_selection(delta)
  local n = #flat_keys
  state.selected = ((state.selected - 1 + delta) % n) + 1
end

local function move_row(delta)
  move_selection(delta * 6)
end

function M.on_key(ctx, key)
  key = tostring(key or ""):lower()
  if key == "left" or key == "l" then move_selection(-1)
  elseif key == "right" or key == "r" then move_selection(1)
  elseif key == "up" or key == "u" then move_row(-1)
  elseif key == "down" or key == "d" then move_row(1)
  elseif key == "ok" or key == "select" or key == "enter" then select_key(ctx, flat_keys[state.selected])
  elseif key == "back" or key == "cancel" then return { action = "back" }
  end
  return M.render(ctx)
end

function M.on_event(ctx, event)
  if type(event) == "table" then
    return M.on_key(ctx, event.key or event.button or event.code or "")
  end
  return M.on_key(ctx, event)
end

local function keyboard_lines()
  local lines = {}
  local cursor = 1
  for _, row in ipairs(keys) do
    local parts = {}
    for _, key in ipairs(row) do
      if cursor == state.selected then
        parts[#parts + 1] = "[" .. key .. "]"
      else
        parts[#parts + 1] = key
      end
      cursor = cursor + 1
    end
    lines[#lines + 1] = table.concat(parts, " ")
  end
  return lines
end

function M.render(_ctx)
  local search = state.query
  if state.wildcard and search ~= "" then
    search = search .. "*"
  end
  if search == "" then search = "_" end

  local lines = {
    "Search: " .. search,
    state.message,
  }
  if state.word ~= "" then
    lines[#lines + 1] = state.word
    lines[#lines + 1] = state.body
  else
    lines[#lines + 1] = "Use INDEX.TXT + DATA/*.JSN shards"
    lines[#lines + 1] = "Shard limit: 16384 bytes"
  end
  lines[#lines + 1] = ""
  for _, line in ipairs(keyboard_lines()) do
    lines[#lines + 1] = line
  end
  lines[#lines + 1] = ""
  lines[#lines + 1] = "Arrows move keyboard. OK key. Back Tools"
  lines[#lines + 1] = MARKER

  return {
    title = "Dictionary",
    status = "Prefix",
    lines = lines,
    footer = "Back    OK    <<    >>",
    marker = MARKER,
  }
end

function M.init(_ctx)
  return true
end

function M.update(ctx)
  return M.render(ctx)
end

function M.main(ctx)
  return M.render(ctx)
end

function M.lookup_for_test(ctx, q, prefix_mode)
  return lookup(ctx, q, prefix_mode)
end

return M
