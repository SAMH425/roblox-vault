-- StarterPlayerScripts/MobileControls (LocalScript)
-- Botones táctiles para acciones del juego usando ContextActionService (en PC funcionan con teclas).
-- Añade acciones en ACTIONS; el servidor recibe ReplicatedStorage.MobileActions:FireServer(nombre).
local ContextActionService = game:GetService("ContextActionService")
local ReplicatedStorage = game:GetService("ReplicatedStorage")

local remote = ReplicatedStorage:WaitForChild("MobileActions", 10) :: RemoteEvent?

local ACTIONS = {
	{ name = "Sprint", title = "Correr", keys = { Enum.KeyCode.LeftShift, Enum.KeyCode.ButtonL3 }, position = UDim2.new(1, -170, 1, -150) },
	{ name = "Interact", title = "Usar", keys = { Enum.KeyCode.E, Enum.KeyCode.ButtonX }, position = UDim2.new(1, -95, 1, -210) },
}

local function handler(actionName: string, state: Enum.UserInputState, _input: InputObject)
	if state == Enum.UserInputState.Begin and remote then
		remote:FireServer(actionName)
	end
	return Enum.ContextActionResult.Sink
end

for _, a in ACTIONS do
	ContextActionService:BindAction(a.name, handler, true, table.unpack(a.keys))
	ContextActionService:SetTitle(a.name, a.title)
	ContextActionService:SetPosition(a.name, a.position)
end
