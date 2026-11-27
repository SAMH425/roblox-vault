-- ServerScriptService/MobileActionsServer (Script)
-- Crea el RemoteEvent y valida las acciones recibidas (nombre conocido + límite de frecuencia).
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local remote = ReplicatedStorage:FindFirstChild("MobileActions") or Instance.new("RemoteEvent")
remote.Name = "MobileActions"
remote.Parent = ReplicatedStorage

local ALLOWED = { Sprint = true, Interact = true }
local last: { [Player]: number } = {}

remote.OnServerEvent:Connect(function(player: Player, action: any)
	if type(action) ~= "string" or not ALLOWED[action] then return end
	local now = os.clock()
	if last[player] and now - last[player] < 0.1 then return end
	last[player] = now
	-- Conecta aquí la lógica de cada acción.
	if action == "Sprint" then
		local hum = player.Character and player.Character:FindFirstChildOfClass("Humanoid")
		if hum then hum.WalkSpeed = if hum.WalkSpeed > 16 then 16 else 24 end
	end
end)
