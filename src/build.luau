--!nocheck

--// Imports
local fs = require("@lune/fs")
local process = require("@lune/process")
local dateTime = require("@lune/datetime")
local util = require("./util")

--// Types
export type HookScript = {
	command: string?,
	script: string?,
	args: { string }?,
	env: { [string]: string }?,
}

export type TaskHooks = {
	preBuild: HookScript?,
	postBuild: HookScript?,
}

type BuildTask = {
	name: string,
	input: string,
	output: string,
	frame: string,
	darkluaConfig: string,
	prerelease: boolean?,
	hooks: TaskHooks?,
	[string]: any,
}

--// State
local currentBuildSource: string? = nil
local currentBuildConfig: string? = nil
local currentVersion: string? = nil
local currentVariant: string? = nil
local currentInput: string? = nil
local currentOutput: string? = nil
local currentPrerelease: boolean? = nil

--// Private Methods
local function resolveTokens(s: string): string
	local resolved = s
	resolved = resolved:gsub("{version}", currentVersion or "")
	resolved = resolved:gsub("{config}",  currentBuildConfig or "")
	resolved = resolved:gsub("{variant}", currentVariant or "")
	resolved = resolved:gsub("{input}",   currentInput or "")
	resolved = resolved:gsub("{output}",  currentOutput or "")
	resolved = resolved:gsub("{file}",    currentOutput or "")
	return resolved
end

local function buildHookEnv(): { [string]: string }
	local env: { [string]: string } = {}
	for k, v in pairs(process.env) do
		env[k] = v
	end

	env.PCMP_VERSION    = currentVersion or ""
	env.PCMP_CONFIG     = currentBuildConfig or ""
	env.PCMP_VARIANT    = currentVariant or ""
	env.PCMP_INPUT      = currentInput or ""
	env.PCMP_OUTPUT     = currentOutput or ""
	env.PCMP_PRERELEASE = currentPrerelease and "true" or "false"

	return env
end

local function runHook(hook: HookScript, label: string)
	local env = buildHookEnv()
	if hook.env then
		for k, v in pairs(hook.env) do
			env[k] = v
		end
	end

	local resolvedArgs: { string } = {}
	if hook.args then
		for _, a in ipairs(hook.args) do
			table.insert(resolvedArgs, resolveTokens(a))
		end
	end

	local result
	if hook.script then
		local lune_args = { "run", hook.script }
		if #resolvedArgs > 0 then
			table.insert(lune_args, "--")
			for _, a in ipairs(resolvedArgs) do
				table.insert(lune_args, a)
			end
		end
		result = process.exec("lune", lune_args, { env = env })
	elseif hook.command then
		local resolvedCmd = resolveTokens(hook.command)
		local parts: { string } = {}
		for part in resolvedCmd:gmatch("%S+") do
			table.insert(parts, part)
		end
		if #parts == 0 then return end
		local cmd = table.remove(parts, 1) :: string
		for _, a in ipairs(resolvedArgs) do
			table.insert(parts, a)
		end
		result = process.exec(cmd, parts, { env = env })
	else
		return
	end

	if not result.ok then
		local output = (result.stderr and result.stderr ~= "") and result.stderr or result.stdout or ""
		error(`[pcmp] Hook '{label}' failed (exit {result.code}):\n{output}`)
	end
end

local function runHookIfPresent(hook: HookScript?, label: string)
	if hook then
		runHook(hook, label)
	end
end

--// Macros
local function buildComposerMarkers(): { [string]: () -> string }
	return {
		-- Source
		["__COMPOSER.Insert(__COMPOSER.build)"] = function()
			return currentBuildSource or ""
		end,

		-- Date / time
		["__COMPOSER.Insert(__COMPOSER.genDate)"] = function()
			return string.format("%q", dateTime.now():toIsoDate())
		end,
		["__COMPOSER.Insert(__COMPOSER.timestamp)"] = function()
			return tostring(os.time())
		end,

		-- Build identity
		["__COMPOSER.Insert(__COMPOSER.cfg)"] = function()
			return string.format("%q", currentBuildConfig or "")
		end,
		["__COMPOSER.Insert(__COMPOSER.vers)"] = function()
			return string.format("%q", currentVersion or "")
		end,
		["__COMPOSER.Insert(__COMPOSER.variant)"] = function()
			return string.format("%q", currentVariant or "")
		end,

		-- File paths
		["__COMPOSER.Insert(__COMPOSER.input)"] = function()
			return string.format("%q", currentInput or "")
		end,
		["__COMPOSER.Insert(__COMPOSER.output)"] = function()
			return string.format("%q", currentOutput or "")
		end,

		-- Release flags
		["__COMPOSER.Insert(__COMPOSER.prerelease)"] = function()
			return currentPrerelease and "true" or "false"
		end,

		-- Runtime / environment
		["__COMPOSER.Insert(__COMPOSER.os)"] = function()
			return string.format("%q", process.os)
		end,
		["__COMPOSER.Insert(__COMPOSER.arch)"] = function()
			return string.format("%q", process.arch)
		end,
	}
end

--// Public Methods
local function setContext(cfg: string?, vers: string?)
	currentBuildConfig = cfg
	currentVersion = vers
end

local function processFrame(rawFrame: string, builtSource: string): string
	currentBuildSource = builtSource

	local markers = buildComposerMarkers()
	local result = rawFrame
	for marker, handler in pairs(markers) do
		local cleanMarker = util.escapePattern(marker)
		result = result:gsub(cleanMarker, handler)
	end

	return result
end

local function runTask(buildTask: BuildTask)
	currentVariant   = buildTask.name
	currentInput     = buildTask.input
	currentOutput    = buildTask.output
	currentPrerelease = buildTask.prerelease

	util.ensureDir(util.getDirName(buildTask.output))

	runHookIfPresent(buildTask.hooks and buildTask.hooks.preBuild, `preBuild:{buildTask.name}`)

	local result = process.exec("darklua", {
		"process",
		buildTask.input,
		buildTask.output,
		"-c",
		buildTask.darkluaConfig,
	})

	if not result.ok then
		error(result.stderr or "darklua returned a non-zero exit code")
	end

	local builtSource = fs.readFile(buildTask.output)
	local rawFrame    = fs.readFile(buildTask.frame)
	local composed    = processFrame(rawFrame, builtSource)
	fs.writeFile(buildTask.output, composed)

	runHookIfPresent(buildTask.hooks and buildTask.hooks.postBuild, `postBuild:{buildTask.name}`)
end

return {
	setContext  = setContext,
	runTask     = runTask,
	__runHook   = runHook,
}
