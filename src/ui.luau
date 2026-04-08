--!nocheck

--// Imports

local io = require("@lune/stdio")
local task = require("@lune/task")
local process = require("@lune/process")

--// Constants

local SPINNER_FRAMES = { "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏" }

local BOX_H  = "─"
local BOX_TL = "┌"
local BOX_TR = "┐"
local BOX_BL = "└"
local BOX_BR = "┘"
local BOX_V  = "│"

--// Private Methods

local function padRight(s: string, width: number): string
	local len = #s
	if len >= width then
		return s
	end
	return s .. string.rep(" ", width - len)
end

--- Spawn a process detached from the current one so it does not block.
--- On Windows, non-.exe commands are routed through `cmd /c`.
--- Returns immediately — callers that need to wait should pass wait=true
--- to openFile instead, which uses process.exec directly.
local function spawnDetached(command: string, args: { string })
	local cmd = command
	local argv = args

	if process.os == "windows" and not cmd:lower():match("%.exe$") then
		argv = table.clone(argv)
		table.insert(argv, 1, cmd)
		table.insert(argv, 1, "/c")
		cmd = "cmd"
	end

	-- process.create is the non-blocking Lune API (available since 0.8).
	-- Fall back to spawning via task.spawn + process.exec for older runtimes.
	local create = (process :: any).create
	if create then
		-- process.create returns a handle; we intentionally discard it so the
		-- child runs independently and is not awaited.
		pcall(create, cmd, { args = argv })
	else
		task.spawn(function()
			pcall(process.exec, cmd, argv)
		end)
	end
end

--// Public Methods

local function withSpinner(msg: string, fn: () -> any, colorName: io.Color?): (boolean, any)
	local done = false
	local idx = 1

	task.spawn(function()
		while not done do
			local frame = SPINNER_FRAMES[idx]
			if colorName then
				io.write(io.color(colorName))
			end
			io.write("\r" .. frame .. " " .. msg)
			if colorName then
				io.write(io.color("reset"))
			end
			idx = (idx % #SPINNER_FRAMES) + 1
			task.wait(0.08)
		end
	end)

	local ok, result = pcall(fn)
	done = true

	io.write("\r\27[2K")
	return ok, result
end

local function printSuccess(msg: string)
	io.write(io.color("green"))
	print("✔ " .. msg)
	io.write(io.color("reset"))
end

local function printError(msg: string)
	io.write(io.color("red"))
	print("✘ " .. msg)
	io.write(io.color("reset"))
end

local function printWarn(msg: string)
	io.write(io.color("yellow"))
	print("⚠ " .. msg)
	io.write(io.color("reset"))
end

local function printInfo(msg: string)
	io.write(io.color("cyan"))
	print("ℹ " .. msg)
	io.write(io.color("reset"))
end

type SummaryRow = { label: string, value: string }

local function printSummaryTable(title: string, rows: { SummaryRow })
	local maxLabel = #title
	local maxValue = 0

	for _, row in ipairs(rows) do
		if #row.label > maxLabel then
			maxLabel = #row.label
		end
		if #row.value > maxValue then
			maxValue = #row.value
		end
	end

	local innerWidth     = maxLabel + 4 + maxValue
	local titlePadded    = " " .. title .. " "
	local titleDisplayLen = utf8.len(titlePadded) or #titlePadded
	local hLine          = string.rep(BOX_H, innerWidth)

	local leftFillCount  = math.floor((innerWidth - titleDisplayLen) / 2)
	local rightFillCount = innerWidth - titleDisplayLen - leftFillCount

	local topFill  = string.rep(BOX_H, leftFillCount)
	local topFillR = string.rep(BOX_H, rightFillCount)

	io.write(io.color("cyan"))
	print(BOX_TL .. topFill .. titlePadded .. topFillR .. BOX_TR)

	for _, row in ipairs(rows) do
		local labelPad = padRight(row.label, maxLabel)
		local valuePad = padRight(row.value, maxValue)
		print(BOX_V .. " " .. io.color("white") .. labelPad .. io.color("reset") .. "  " .. valuePad .. " " .. BOX_V)
	end

	print(BOX_BL .. hLine .. BOX_BR)
	io.write(io.color("reset"))
end

export type OpenFileConfig = {
	command: string,
	args: { string },
}

--- Open a file with the configured command.
--- `wait = true`  → blocks until the subprocess exits (good for editors).
--- `wait = false` → fires and forgets; returns immediately (good for viewers).
local function openFile(filePath: string, cfg: OpenFileConfig?, wait: boolean?)
	if not cfg then
		return
	end

	local resolvedArgs: { string } = {}
	for _, arg in ipairs(cfg.args) do
		table.insert(resolvedArgs, (arg:gsub("{file}", filePath)))
	end

	if wait then
		-- Blocking path — used for editors (release notes, etc.)
		local cmd = cfg.command
		local argv = resolvedArgs

		if process.os == "windows" and not cmd:lower():match("%.exe$") then
			argv = table.clone(argv)
			table.insert(argv, 1, cmd)
			table.insert(argv, 1, "/c")
			cmd = "cmd"
		end

		pcall(process.exec, cmd, argv)
	else
		-- Non-blocking path — used for output viewers
		spawnDetached(cfg.command, resolvedArgs)
	end
end

return {
	withSpinner       = withSpinner,
	printSuccess      = printSuccess,
	printError        = printError,
	printWarn         = printWarn,
	printInfo         = printInfo,
	printSummaryTable = printSummaryTable,
	openFile          = openFile,
}
