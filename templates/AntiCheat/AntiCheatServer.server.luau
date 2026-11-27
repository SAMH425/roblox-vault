-- ServerScriptService/AntiCheat/AntiCheatServer (Script)
-- Muestrea la posición de cada personaje 5 veces por segundo. Ante sospecha: devuelve al jugador
-- a su última posición válida y lo registra. Ajusta maxSpeed si tu juego cambia WalkSpeed.
local Players = game:GetService("Players")
local MovementCheck = require(script.Parent.MovementCheck)

local CONFIG = { maxSpeed = 16, tolerance = 1.5, maxTeleport = 80, strikesToFlag = 4 }
local checkers: { [Player]: any } = {}
local safe: { [Player]: CFrame } = {}

Players.PlayerAdded:Connect(function(player)
	checkers[player] = MovementCheck.new(CONFIG)
	player.CharacterAdded:Connect(function(char)
		checkers[player]:allowFor(os.clock(), 2) -- respawn legítimo
		local hum = char:WaitForChild("Humanoid") :: Humanoid
		hum:GetPropertyChangedSignal("WalkSpeed"):Connect(function()
			checkers[player].config = table.clone(CONFIG)
			checkers[player].config.maxSpeed = math.max(CONFIG.maxSpeed, hum.WalkSpeed)
		end)
	end)
end)
Players.PlayerRemoving:Connect(function(p) checkers[p] = nil; safe[p] = nil end)

-- API para tus scripts: llama esto antes de teletransportar a alguien a propósito.
local allow = Instance.new("BindableFunction")
allow.Name = "AllowTeleport"
allow.OnInvoke = function(player: Player, seconds: number?)
	local c = checkers[player]
	if c then c:allowFor(os.clock(), seconds or 1) end
end
allow.Parent = script

while true do
	task.wait(0.2)
	for player, checker in checkers do
		local root = player.Character and player.Character:FindFirstChild("HumanoidRootPart") :: BasePart?
		if root then
			local p = root.Position
			local suspicious, why = checker:sample({ x = p.X, y = p.Y, z = p.Z, t = os.clock() })
			if suspicious then
				warn(("[AntiCheat] %s: %s"):format(player.Name, why))
				if safe[player] then root.CFrame = safe[player]; checker:allowFor(os.clock(), 0.5) end
				checker.strikes = 0
			else
				safe[player] = root.CFrame
			end
		end
	end
end
