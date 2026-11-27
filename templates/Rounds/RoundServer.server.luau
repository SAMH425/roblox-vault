-- ServerScriptService/Rounds/RoundServer (Script)
-- Publica el estado en ReplicatedStorage.RoundState (StringValue) y RoundTime (IntValue).
-- Cambia las funciones teleportToArena/teleportToLobby por las de tu mapa.
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local RoundMachine = require(script.Parent.RoundMachine)

local machine = RoundMachine.new({ minPlayers = 2, intermission = 15, roundLength = 120, endScreen = 8 })

local stateValue = ReplicatedStorage:FindFirstChild("RoundState") or Instance.new("StringValue")
stateValue.Name = "RoundState"; stateValue.Value = machine.state; stateValue.Parent = ReplicatedStorage
local timeValue = ReplicatedStorage:FindFirstChild("RoundTime") or Instance.new("IntValue")
timeValue.Name = "RoundTime"; timeValue.Parent = ReplicatedStorage

local function spawnAt(player: Player, name: string)
	local spawn = workspace:FindFirstChild(name, true)
	local root = player.Character and player.Character:FindFirstChild("HumanoidRootPart") :: BasePart?
	if spawn and spawn:IsA("BasePart") and root then root.CFrame = spawn.CFrame + Vector3.new(0, 4, 0) end
end

machine:onChange(function(_, new)
	stateValue.Value = new
	if new == "Playing" then
		for _, p in Players:GetPlayers() do
			spawnAt(p, "ArenaSpawn")
			local hum = p.Character and p.Character:FindFirstChildOfClass("Humanoid")
			if hum then hum.Died:Once(function() machine:eliminate(p) end) end
		end
	elseif new == "Lobby" then
		for _, p in Players:GetPlayers() do spawnAt(p, "LobbySpawn") end
	end
end)

while true do
	local dt = task.wait(1)
	machine:step(dt, Players:GetPlayers())
	timeValue.Value = math.max(0, math.ceil(machine.timeLeft))
end
