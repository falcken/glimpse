local M = {}

local RENDERER_PORT = 42069
local RENDERER_URL = "http://localhost:" .. RENDERER_PORT .. "/update"
local LISTENER_PORT = 42070

local APP_NAME = "Glimpse"
local DEBOUNCE_MS = 100

M.debounce_timer = nil
M.server = nil

function M.launch_renderer()
  if vim.g.glimpse_started == nil then
    print("Starting Glimpse renderer...")
    vim.fn.jobstart({ 'open', '-a', APP_NAME })
    vim.g.glimpse_started = 1
  end
end

local function do_send_update()
  local file_name = vim.api.nvim_buf_get_name(0)
  local content = table.concat(vim.api.nvim_buf_get_lines(0, 0, -1, false), "\n")
  local cursor_line = vim.api.nvim_win_get_cursor(0)[1]

  local payload = {
    fileName = file_name,
    content = content,
    cursorLine = cursor_line,
  }
  
  local json_payload = vim.fn.json_encode(payload)

  vim.fn.jobstart({
    'curl',
    '-X', 'POST',
    '-s', -- silent
    '-f', -- fail on server error
    RENDERER_URL,
    '--header', 'Content-Type: application/json',
    '-d', json_payload
  }, {
    on_exit = function(_, exit_code)
      if exit_code ~= 0 then
        vim.notify(
          "Failed to send update. (curl exit code: " .. exit_code .. ")",
          vim.log.levels.WARN
        )
      end
    end
  })

  M.debounce_timer = nil
end

function M.send_to_renderer()
  if vim.g.glimpse_started == nil then
    return
  end

  if M.debounce_timer then
    vim.fn.timer_stop(M.debounce_timer)
  end

  M.debounce_timer = vim.fn.timer_start(DEBOUNCE_MS, do_send_update)
end

local function handle_jump_request(client_socket, data)
  if not data then
    client_socket:close()
    return
  end

  local json_string = data:match("^[^\r\n]+")
  if not json_string then return end

  local ok, decoded = pcall(vim.fn.json_decode, json_string)
  if not ok then
    vim.notify("Glimpse: Received invalid JSON: " .. json_string, vim.log.levels.WARN)
    client_socket:close()
    return
  end

  if decoded.line then
    local line_num = tonumber(decoded.line)
    if line_num then
      vim.schedule(function()
        local last_line = vim.api.nvim_buf_line_count(0)
        
        if line_num < 1 then line_num = 1 end
        if line_num > last_line then line_num = last_line end
        
        vim.api.nvim_win_set_cursor(0, { line_num, 0 })
        
      end)
    end
  end

  client_socket:close()
end

function M.launch_listener()
  if M.server then
    vim.notify("Listener already running", vim.log.levels.INFO)
    return
  end

  M.server = vim.loop.new_tcp()

  local bind_ok = M.server:bind('127.0.0.1', LISTENER_PORT)
  if not bind_ok then
    vim.notify("Error: Port already in use.", vim.log.levels.ERROR)
    M.server = nil
    return
  end

  M.server:listen(128, function(err)
    if err then
      vim.schedule(function()
        vim.notify("Glimpse Listener Error: " .. err, vim.log.levels.ERROR)
      end)
      return
    end

    local client = vim.loop.new_tcp()
    if not client then return end

    local accept_ok = M.server:accept(client)
    if not accept_ok then
      client:close()
      return
    end

    client:read_start(function(read_err, data)
      if read_err then
        client:close()
        return
      end
      vim.schedule(function()
        handle_jump_request(client, data)
      end)
    end)
  end)

  vim.notify("Glimpse: Started listener on port " .. LISTENER_PORT, vim.log.levels.INFO)
end

return M