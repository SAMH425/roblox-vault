-- ServerScriptService/DataSave/DataService (ModuleScript)
-- Uso: local DataService = require(path.DataService); DataService.start({ Coins = 0 })
--      local data = DataService.get(player)  -- tabla viva, se guarda sola
local Players = game:GetService("Players")
local DataStoreService = game:GetService("DataStoreService")
local RunService = game:GetService("RunService")

local DataCore = require(script.Parent.DataCore)

local DataService = {}
local sessions: { [Player]: any } = {}
local loaded = Instance.new("BindableEvent")
DataService.PlayerLoaded = loaded.Event

local env = {
	store = DataStoreService:GetDataStore("PlayerData_v1"),
	now = os.time,
	wait = task.wait,
	jobId = if game.JobId ~= "" then game.JobId else "studio-" .. tostring(math.random(1, 1e9)),
}

function DataService.get(player: Player): { [string]: any }?
	local s = sessions[player]
	return s and s.data
end

local function onJoin(player: Player, template)
	local s, err = DataCore.load(env, "u_" .. player.UserId, template)
	if not s then
		warn("[DataSave]", player.Name, err)
		player:Kick("No se pudieron cargar tus datos (" .. tostring(err) .. "). Vuelve a entrar en un momento.")
		return
	end
	if not player.Parent then DataCore.save(env, s, true) return end -- salió mientras cargaba
	sessions[player] = s
	loaded:Fire(player, s.data)
end

local function onLeave(player: Player)
	local s = sessions[player]
	if not s then return end
	sessions[player] = nil
	local ok, err = DataCore.save(env, s, true)
	if not ok then warn("[DataSave] no se guardó", player.Name, err) end
end

function DataService.start(template: { [string]: any }, autosaveSeconds: number?)
	Players.PlayerAdded:Connect(function(p) onJoin(p, template) end)
	for _, p in Players:GetPlayers() do task.spawn(onJoin, p, template) end
	Players.PlayerRemoving:Connect(onLeave)
	task.spawn(function()
		while true do
			task.wait(autosaveSeconds or 120)
			for player, s in sessions do task.spawn(DataCore.save, env, s, false) end
		end
	end)
	game:BindToClose(function()
		if RunService:IsStudio() then task.wait(1) end
		local pending = 0
		for player in sessions do pending += 1; task.spawn(function() onLeave(player); pending -= 1 end) end
		local t0 = os.clock()
		while pending > 0 and os.clock() - t0 < 25 do task.wait() end
	end)
end

return DataService
