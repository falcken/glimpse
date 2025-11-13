local ok, glimpse = pcall(require, 'glimpse')
if not ok then
  return
end

local glimpse_augroup = vim.api.nvim_create_augroup('GlimpseRenderer', { clear = true })

-- Launch the app when entering .md
vim.api.nvim_create_autocmd({ 'BufEnter' }, {
  pattern = { '*.md', '*.markdown' },
  group = glimpse_augroup,
  callback = function()
    glimpse.launch_renderer()
    glimpse.launch_listener()
  end
})

-- Send updates on text / cursor change
vim.api.nvim_create_autocmd({ 'CursorMoved', 'TextChanged', 'TextChangedI' }, {
  pattern = { '*.md', '*.markdown' },
  group = glimpse_augroup,
  callback = glimpse.send_to_renderer
})