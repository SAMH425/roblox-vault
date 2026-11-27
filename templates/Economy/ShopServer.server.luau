-- ServerScriptService/Economy/ShopServer (Script)
-- Crea ReplicatedStorage.ShopRemotes.Purchase (RemoteFunction) y valida cada compra en el servidor.
-- Los saldos viven en memoria; para guardarlos entre sesiones conecta el perfil con el sistema DataSave (ver instalación).
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")

local CurrencyLogic = require(script.Parent.CurrencyLogic)
local ShopConfig = require(script.Parent.ShopConfig)

local remotes = ReplicatedStorage:FindFirstChild("ShopRemotes") or Instance.new("Folder")
remotes.Name = "ShopRemotes"
remotes.Parent = ReplicatedStorage
local purchase = remotes:FindFirstChild("Purchase") or Instance.new("RemoteFunction")
purchase.Name = "Purchase"
purchase.Parent = remotes

local profiles: { [Player]: any } = {}
local lastRequest: { [Player]: number } = {}

local function leaderstats(player: Player, profile)
	local ls = player:FindFirstChild("leaderstats") or Instance.new("Folder")
	ls.Name = "leaderstats"
	for name, value in profile.currencies do
		local v = ls:FindFirstChild(name) or Instance.new("IntValue")
		v.Name = name
		v.Value = value
		v.Parent = ls
	end
	ls.Parent = player
end

Players.PlayerAdded:Connect(function(player)
	profiles[player] = CurrencyLogic.newProfile(ShopConfig.StartingCurrencies)
	leaderstats(player, profiles[player])
end)
Players.PlayerRemoving:Connect(function(player)
	profiles[player] = nil
	lastRequest[player] = nil
end)

purchase.OnServerInvoke = function(player: Player, itemId: any)
	-- límite simple contra spam
	local now = os.clock()
	if lastRequest[player] and now - lastRequest[player] < 0.25 then return false, "demasiado rápido" end
	lastRequest[player] = now
	local profile = profiles[player]
	if not profile then return false, "perfil no cargado" end
	local ok, err = CurrencyLogic.purchase(profile, ShopConfig.Items, itemId)
	if ok then leaderstats(player, profile) end
	return ok, err
end
