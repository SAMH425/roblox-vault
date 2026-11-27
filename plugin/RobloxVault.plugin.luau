--!strict
-- ROBLOX VAULT · plugin de Roblox Studio
-- Lo instala la app ROBLOX VAULT (Settings → Instalar plugin), que rellena el puerto y el token.
-- Comunicación solo con 127.0.0.1. Nunca ejecuta código recibido: los scripts que no estén
-- VERIFIED se insertan desactivados y todo se inserta solo tras tu confirmación.

local HttpService = game:GetService("HttpService")
local Selection = game:GetService("Selection")
local SerializationService = game:GetService("SerializationService")
local EncodingService = game:GetService("EncodingService")
local ChangeHistoryService = game:GetService("ChangeHistoryService")
local StudioService = game:GetService("StudioService")
local StudioTestService = game:GetService("StudioTestService")
local ServerScriptService = game:GetService("ServerScriptService")

local PORT = __VAULT_PORT__
local TOKEN = "__VAULT_TOKEN__"
local BASE = "http://127.0.0.1:" .. tostring(PORT)

local C = {
	bg = Color3.fromHex("101014"), surface = Color3.fromHex("191922"), panel = Color3.fromHex("282535"),
	violet = Color3.fromHex("9065FF"), accent = Color3.fromHex("C5B5FF"), ok = Color3.fromHex("36C991"),
	text = Color3.fromHex("E4E4E7"), muted = Color3.fromHex("8B8B96"), bad = Color3.fromHex("F87171"), warn = Color3.fromHex("FCD34D"),
}

---------------------------------------------------------------- red
type Reply = { ok: boolean, data: any, err: string? }

local function request(method: string, path: string, body: any?): Reply
	local ok, res = pcall(function()
		return HttpService:RequestAsync({
			Url = BASE .. path,
			Method = method,
			Headers = { ["Content-Type"] = "application/json", ["X-Vault-Token"] = TOKEN },
			Body = if body ~= nil then HttpService:JSONEncode(body) else nil,
		})
	end)
	if not ok then
		return { ok = false, data = nil, err = "ROBLOX VAULT no responde. ¿Está abierta la app? (" .. tostring(res) .. ")" }
	end
	local decoded: any = nil
	pcall(function() decoded = HttpService:JSONDecode(res.Body) end)
	if res.StatusCode ~= 200 then
		return { ok = false, data = decoded, err = if decoded and decoded.error then decoded.error else ("HTTP " .. res.StatusCode) }
	end
	return { ok = true, data = decoded, err = nil }
end

local function urlencode(s: string): string
	return (string.gsub(s, "[^%w%-_%.~]", function(c) return string.format("%%%02X", string.byte(c)) end))
end

local function toBase64(b: buffer): string
	return buffer.tostring(EncodingService:Base64Encode(b))
end
local function fromBase64(s: string): buffer
	return EncodingService:Base64Decode(buffer.fromstring(s))
end

---------------------------------------------------------------- interfaz
local toolbar = plugin:CreateToolbar("ROBLOX VAULT")
local button = toolbar:CreateButton("RobloxVaultToggle", "Abrir ROBLOX VAULT", "rbxasset://textures/StudioToolbox/AssetPreview/more.png", "Vault")
button.ClickableWhenViewportHidden = true

local widget = plugin:CreateDockWidgetPluginGui("RobloxVaultWidget", DockWidgetPluginGuiInfo.new(Enum.InitialDockState.Right, false, false, 360, 560, 300, 360))
widget.Title = "ROBLOX VAULT"
button.Click:Connect(function() widget.Enabled = not widget.Enabled end)
widget:GetPropertyChangedSignal("Enabled"):Connect(function() button:SetActive(widget.Enabled) end)

local function make(class: string, props: { [string]: any }, parent: Instance?): any
	local o = Instance.new(class)
	for k, v in props do (o :: any)[k] = v end
	if parent then o.Parent = parent end
	return o
end
local function corner(p: Instance, r: number?) make("UICorner", { CornerRadius = UDim.new(0, r or 8) }, p) end
local function pad(p: Instance, n: number) make("UIPadding", { PaddingTop = UDim.new(0, n), PaddingBottom = UDim.new(0, n), PaddingLeft = UDim.new(0, n), PaddingRight = UDim.new(0, n) }, p) end

local root = make("Frame", { Size = UDim2.fromScale(1, 1), BackgroundColor3 = C.bg, BorderSizePixel = 0 }, widget)
pad(root, 10)
make("UIListLayout", { Padding = UDim.new(0, 8), SortOrder = Enum.SortOrder.LayoutOrder }, root)

local function label(text: string, order: number, color: Color3?, size: number?): TextLabel
	return make("TextLabel", { Text = text, LayoutOrder = order, Size = UDim2.new(1, 0, 0, (size or 14) + 6), BackgroundTransparency = 1, TextColor3 = color or C.text,
		Font = Enum.Font.GothamMedium, TextSize = size or 14, TextXAlignment = Enum.TextXAlignment.Left, TextWrapped = true, AutomaticSize = Enum.AutomaticSize.Y }, root)
end
local function btn(text: string, parent: Instance, color: Color3?): TextButton
	local b = make("TextButton", { Text = text, Size = UDim2.new(0, 0, 0, 28), AutomaticSize = Enum.AutomaticSize.X, BackgroundColor3 = color or C.panel, TextColor3 = C.text,
		Font = Enum.Font.GothamMedium, TextSize = 13, AutoButtonColor = true }, parent)
	corner(b); make("UIPadding", { PaddingLeft = UDim.new(0, 10), PaddingRight = UDim.new(0, 10) }, b)
	return b
end
local function row(order: number): Frame
	local f = make("Frame", { LayoutOrder = order, Size = UDim2.new(1, 0, 0, 28), BackgroundTransparency = 1 }, root)
	make("UIListLayout", { FillDirection = Enum.FillDirection.Horizontal, Padding = UDim.new(0, 6), SortOrder = Enum.SortOrder.LayoutOrder }, f)
	return f
end

label("ROBLOX VAULT", 1, C.accent, 16)
local status = label("Conectando…", 2, C.muted, 12)
local projectLabel = label("", 3, C.muted, 12)

local r1 = row(4)
local linkBtn = btn("Vincular este juego", r1, C.panel)
local sendBtn = btn("Enviar selección", r1, C.violet)
local importBtn = btn("Importar archivo", r1, C.panel)

local searchBox = make("TextBox", { LayoutOrder = 5, Size = UDim2.new(1, 0, 0, 30), BackgroundColor3 = C.surface, TextColor3 = C.text, PlaceholderText = "Buscar en la bóveda (nombre o código)…",
	PlaceholderColor3 = C.muted, Text = "", ClearTextOnFocus = false, Font = Enum.Font.Gotham, TextSize = 13, TextXAlignment = Enum.TextXAlignment.Left }, root)
corner(searchBox); make("UIPadding", { PaddingLeft = UDim.new(0, 8) }, searchBox)
local r2 = row(6)
local scopeBtn = btn("Ámbito: todo", r2, C.panel)
local systemsBtn = btn("Sistemas", r2, C.panel)

local confirm = make("Frame", { LayoutOrder = 7, Size = UDim2.new(1, 0, 0, 0), AutomaticSize = Enum.AutomaticSize.Y, BackgroundColor3 = C.panel, Visible = false }, root)
corner(confirm); pad(confirm, 8)
make("UIListLayout", { Padding = UDim.new(0, 6), SortOrder = Enum.SortOrder.LayoutOrder }, confirm)
local confirmText = make("TextLabel", { LayoutOrder = 1, Size = UDim2.new(1, 0, 0, 0), AutomaticSize = Enum.AutomaticSize.Y, BackgroundTransparency = 1, TextColor3 = C.text, TextWrapped = true,
	Font = Enum.Font.Gotham, TextSize = 12, TextXAlignment = Enum.TextXAlignment.Left }, confirm)
local confirmRow = make("Frame", { LayoutOrder = 2, Size = UDim2.new(1, 0, 0, 28), BackgroundTransparency = 1 }, confirm)
make("UIListLayout", { FillDirection = Enum.FillDirection.Horizontal, Padding = UDim.new(0, 6) }, confirmRow)
local yesBtn = btn("Insertar", confirmRow, C.violet)
local noBtn = btn("Cancelar", confirmRow, C.panel)

local list = make("ScrollingFrame", { LayoutOrder = 8, Size = UDim2.new(1, 0, 1, -260), BackgroundTransparency = 1, BorderSizePixel = 0, ScrollBarThickness = 6,
	CanvasSize = UDim2.new(), AutomaticCanvasSize = Enum.AutomaticSize.Y }, root)
make("UIListLayout", { Padding = UDim.new(0, 6), SortOrder = Enum.SortOrder.LayoutOrder }, list)

local noteBox = make("TextBox", { LayoutOrder = 9, Size = UDim2.new(1, 0, 0, 30), BackgroundColor3 = C.surface, TextColor3 = C.text, PlaceholderText = "Nota de cambio para el registro…",
	PlaceholderColor3 = C.muted, Text = "", ClearTextOnFocus = false, Font = Enum.Font.Gotham, TextSize = 12, TextXAlignment = Enum.TextXAlignment.Left }, root)
corner(noteBox); make("UIPadding", { PaddingLeft = UDim.new(0, 8) }, noteBox)
local r3 = row(10)
local logBtn = btn("Registrar cambio", r3, C.panel)
local testBtn = btn("Ejecutar pruebas", r3, C.violet)

local function setStatus(text: string, color: Color3?)
	status.Text = text
	status.TextColor3 = color or C.muted
end

---------------------------------------------------------------- proyecto
local projectScope = false
local function placeId(): number return game.PlaceId end

local function refreshProject()
	local ping = request("GET", "/ping")
	if not ping.ok then setStatus(ping.err or "sin conexión", C.bad) return end
	setStatus("Conectado a ROBLOX VAULT", C.ok)
	if placeId() == 0 then
		projectLabel.Text = "Este lugar no está publicado (PlaceId 0): publícalo para vincularlo a un proyecto."
		return
	end
	local r = request("GET", "/project?place_id=" .. placeId())
	if r.ok and r.data.project then
		projectLabel.Text = "Proyecto: " .. r.data.project.name
		linkBtn.Text = "Vinculado ✓"
	else
		projectLabel.Text = "Este juego aún no está vinculado a un proyecto."
		linkBtn.Text = "Vincular este juego"
	end
end

linkBtn.MouseButton1Click:Connect(function()
	if placeId() == 0 then setStatus("Publica el lugar primero (PlaceId 0).", C.warn) return end
	-- game.Name en Studio suele ser "Place1": usamos el nombre publicado de la experiencia
	local name = game.Name
	local okInfo, info = pcall(function() return game:GetService("MarketplaceService"):GetProductInfo(placeId()) end)
	if okInfo and info and info.Name and info.Name ~= "" then name = info.Name end
	local r = request("POST", "/project/link", { place_id = placeId(), name = name })
	if r.ok then setStatus("Vinculado a " .. r.data.project.name, C.ok) refreshProject() else setStatus(r.err or "error", C.bad) end
end)

scopeBtn.MouseButton1Click:Connect(function()
	projectScope = not projectScope
	scopeBtn.Text = if projectScope then "Ámbito: este proyecto" else "Ámbito: todo"
end)

---------------------------------------------------------------- insertar (con consentimiento)
local pending: (() -> ())? = nil
local function ask(text: string, action: () -> ())
	confirmText.Text = text
	confirm.Visible = true
	pending = action
end
yesBtn.MouseButton1Click:Connect(function()
	confirm.Visible = false
	local a = pending; pending = nil
	if a then a() end
end)
noBtn.MouseButton1Click:Connect(function() confirm.Visible = false; pending = nil end)

-- Desactiva los scripts de lo insertado si el recurso no está VERIFIED. Devuelve cuántos.
local function disableScripts(root: Instance, status: string): number
	if status == "VERIFIED" then return 0 end
	local n = 0
	local items = root:GetDescendants()
	table.insert(items, root)
	for _, d in items do
		if d:IsA("Script") or d:IsA("LocalScript") then
			(d :: any).Enabled = false
			n += 1
		end
	end
	return n
end

local function insertTarget(): Instance
	local sel = Selection:Get()
	if #sel == 1 and not sel[1]:IsA("LuaSourceContainer") then return sel[1] end
	return workspace
end

local function record(name: string, fn: () -> ())
	local id = ChangeHistoryService:TryBeginRecording(name)
	local ok, e = pcall(fn)
	if id then ChangeHistoryService:FinishRecording(id, if ok then Enum.FinishRecordingOperation.Commit else Enum.FinishRecordingOperation.Cancel) end
	if not ok then setStatus("Error: " .. tostring(e), C.bad) end
end

local function insertAsset(a: any)
	if a.format == "rbxm" or a.format == "rbxmx" then
		local r = request("GET", "/asset/" .. a.id .. "/rbxm")
		if not r.ok then setStatus(r.err or "error", C.bad) return end
		record("Vault: insertar " .. a.name, function()
			local items = SerializationService:DeserializeInstancesAsync(fromBase64(r.data.rbxm_base64))
			local parent = insertTarget()
			local off = 0
			for _, inst in items do
				off += disableScripts(inst, a.status)
				inst:SetAttribute("VaultAssetId", a.id)
				inst.Parent = parent
			end
			Selection:Set(items)
			setStatus(("Insertado %s v%d%s"):format(a.name, a.version, if off > 0 then (" · %d script(s) desactivados"):format(off) else ""), C.ok)
		end)
	else
		local r = request("GET", "/asset/" .. a.id .. "/script")
		if not r.ok then setStatus(r.err or "error", C.bad) return end
		if #r.data.syntax_errors > 0 then setStatus("No se inserta: el script tiene errores de sintaxis (" .. r.data.syntax_errors[1] .. ")", C.bad) return end
		record("Vault: insertar " .. a.name, function()
			local cls = r.data.class
			if cls ~= "Script" and cls ~= "LocalScript" then cls = "ModuleScript" end
			local s = Instance.new(cls)
			s.Name = a.name;
			(s :: any).Source = r.data.source
			s:SetAttribute("VaultAssetId", a.id)
			local off = disableScripts(s, a.status)
			s.Parent = insertTarget()
			Selection:Set({ s })
			setStatus(("Insertado %s (%s)%s"):format(a.name, cls, if off > 0 then " · desactivado hasta que lo revises" else ""), C.ok)
		end)
	end
end

---------------------------------------------------------------- resultados
local function clearList()
	for _, c in list:GetChildren() do if c:IsA("GuiObject") then c:Destroy() end end
end

local statusColor = { VERIFIED = C.ok, ["PARTIALLY VERIFIED"] = C.accent, PENDING = C.warn, FAILED = C.bad, DEPRECATED = C.muted }

local function showResults(assets: { any })
	clearList()
	if #assets == 0 then
		make("TextLabel", { Size = UDim2.new(1, 0, 0, 24), BackgroundTransparency = 1, Text = "Sin resultados.", TextColor3 = C.muted, Font = Enum.Font.Gotham, TextSize = 12 }, list)
		return
	end
	for i, a in assets do
		local card = make("Frame", { LayoutOrder = i, Size = UDim2.new(1, -8, 0, 50), BackgroundColor3 = C.surface }, list)
		corner(card)
		make("TextLabel", { Position = UDim2.fromOffset(10, 6), Size = UDim2.new(1, -110, 0, 18), BackgroundTransparency = 1, Text = a.name, TextColor3 = C.text,
			Font = Enum.Font.GothamMedium, TextSize = 13, TextXAlignment = Enum.TextXAlignment.Left, TextTruncate = Enum.TextTruncate.AtEnd }, card)
		make("TextLabel", { Position = UDim2.fromOffset(10, 26), Size = UDim2.new(1, -110, 0, 16), BackgroundTransparency = 1,
			Text = ("%s · v%d · %s"):format(string.upper(a.format), a.version, a.status), TextColor3 = statusColor[a.status] or C.muted,
			Font = Enum.Font.Gotham, TextSize = 11, TextXAlignment = Enum.TextXAlignment.Left }, card)
		if a.insertable then
			local b = btn("Insertar", card, C.violet)
			b.AnchorPoint = Vector2.new(1, 0.5); b.Position = UDim2.new(1, -8, 0.5, 0)
			b.MouseButton1Click:Connect(function()
				local warnText = if a.status == "VERIFIED" then "" elseif a.status == "FAILED" then "\n⚠ Marcado como FAILED." else "\nLos scripts se insertarán DESACTIVADOS (no está VERIFIED)."
				ask(("¿Insertar %s v%d (%s) en %s?%s"):format(a.name, a.version, a.status, insertTarget():GetFullName(), warnText), function() insertAsset(a) end)
			end)
		end
	end
end

local function search()
	local q = searchBox.Text
	local path = "/search?q=" .. urlencode(q)
	if projectScope then path ..= "&scope=project&place_id=" .. placeId() end
	setStatus("Buscando…")
	local r = request("GET", path)
	if not r.ok then setStatus(r.err or "error", C.bad) return end
	setStatus(("%d resultado(s)"):format(#r.data.assets), C.ok)
	showResults(r.data.assets)
end
searchBox.FocusLost:Connect(function(enter) if enter then search() end end)

---------------------------------------------------------------- enviar
local function sendInstance(inst: Instance): (boolean, string)
	local updateId = inst:GetAttribute("VaultAssetId")
	local base = { name = inst.Name, path = inst:GetFullName(), place_id = placeId(), update_id = updateId }
	local r: Reply
	if inst:IsA("LuaSourceContainer") then
		base.source = (inst :: any).Source
		base.class = inst.ClassName
		r = request("POST", "/script", base)
	else
		local ok, data = pcall(function() return SerializationService:SerializeInstancesAsync({ inst }) end)
		if not ok then return false, inst.Name .. ": " .. tostring(data) end
		base.rbxm_base64 = toBase64(data)
		r = request("POST", "/model", base)
	end
	if not r.ok then return false, inst.Name .. ": " .. (r.err or "error") end
	inst:SetAttribute("VaultAssetId", r.data.asset.id)
	local tag = if r.data.new_version then ("v%d"):format(r.data.asset.version) elseif r.data.duplicate then "sin cambios" else "nuevo"
	if r.data.syntax_errors and #r.data.syntax_errors > 0 then tag ..= " · ⚠ errores de sintaxis" end
	return true, inst.Name .. " (" .. tag .. ")"
end

sendBtn.MouseButton1Click:Connect(function()
	local sel = Selection:Get()
	if #sel == 0 then setStatus("Selecciona modelos o scripts en el Explorer.", C.warn) return end
	setStatus("Enviando " .. #sel .. "…")
	local done, failed = {}, {}
	for _, inst in sel do
		local ok, msg = sendInstance(inst)
		table.insert(if ok then done else failed, msg)
	end
	if #failed > 0 then setStatus("Fallaron: " .. table.concat(failed, "; "), C.bad)
	else setStatus("Guardado: " .. table.concat(done, ", "), C.ok) end
end)

---------------------------------------------------------------- alternativa por archivo
-- Si la conexión local no está disponible: en la app usa "Recuperar" para guardar el .rbxm y aquí "Importar archivo".
importBtn.MouseButton1Click:Connect(function()
	local ok, file = pcall(function() return StudioService:PromptImportFileAsync({ "rbxm" }) end)
	if not ok or not file then return end
	local content = (file :: any):GetBinaryContents()
	record("Vault: importar archivo", function()
		local items = SerializationService:DeserializeInstancesAsync(buffer.fromstring(content))
		local off = 0
		for _, inst in items do off += disableScripts(inst, "PENDING"); inst.Parent = insertTarget() end
		Selection:Set(items)
		setStatus(("Importado %s%s"):format((file :: any).Name, if off > 0 then (" · %d script(s) desactivados"):format(off) else ""), C.ok)
	end)
end)

---------------------------------------------------------------- registro de cambios
logBtn.MouseButton1Click:Connect(function()
	local note = noteBox.Text
	if note == "" then setStatus("Escribe una nota de cambio.", C.warn) return end
	local targets = {}
	for _, s in Selection:Get() do table.insert(targets, s:GetFullName()) end
	local r = request("POST", "/log", { action = "change", target = table.concat(targets, ", "), detail = note .. " · " .. game.Name })
	if r.ok then noteBox.Text = ""; setStatus("Cambio registrado.", C.ok) else setStatus(r.err or "error", C.bad) end
end)


---------------------------------------------------------------- sistemas completos
-- Crea (o reutiliza) la ruta "Servicio/Carpeta/Sub" y devuelve el contenedor final.
local function ensurePath(path: string): Instance
	local parts = string.split(path, "/")
	local current: Instance = game:GetService(parts[1])
	for i = 2, #parts do
		local child = current:FindFirstChild(parts[i])
		if not child then
			child = Instance.new("Folder")
			child.Name = parts[i]
			child.Parent = current
		end
		current = child
	end
	return current
end

local function insertSystem(sys: any)
	local r = request("GET", "/system/" .. sys.id)
	if not r.ok then setStatus(r.err or "error", C.bad) return end
	local skipped, created, off = {}, 0, 0
	for _, m in r.data.members do
		if m.syntax_errors and #m.syntax_errors > 0 then setStatus("No se inserta: " .. m.asset.name .. " tiene errores de sintaxis", C.bad) return end
	end
	record("Vault: sistema " .. r.data.name, function()
		for _, m in r.data.members do
			if m.source then
				local parent = ensurePath(m.location or "ServerScriptService")
				if parent:FindFirstChild(m.asset.name) then
					table.insert(skipped, m.asset.name) -- nunca sobrescribe
				else
					local cls = m.class
					if cls ~= "Script" and cls ~= "LocalScript" then cls = "ModuleScript" end
					local s = Instance.new(cls)
					s.Name = m.asset.name;
					(s :: any).Source = m.source
					s:SetAttribute("VaultAssetId", m.asset.id)
					off += disableScripts(s, m.asset.status)
					s.Parent = parent
					created += 1
				end
			end
		end
	end)
	local msg = ("Sistema %s: %d insertados"):format(r.data.name, created)
	if #skipped > 0 then msg ..= " · ya existían (no se tocaron): " .. table.concat(skipped, ", ") end
	if off > 0 then msg ..= (" · %d script(s) desactivados hasta probarlos"):format(off) end
	setStatus(msg, C.ok)
end

local function showSystems(systems: { any })
	clearList()
	for i, sys in systems do
		local card = make("Frame", { LayoutOrder = i, Size = UDim2.new(1, -8, 0, 50), BackgroundColor3 = C.surface }, list)
		corner(card)
		make("TextLabel", { Position = UDim2.fromOffset(10, 6), Size = UDim2.new(1, -110, 0, 18), BackgroundTransparency = 1, Text = sys.name, TextColor3 = C.text,
			Font = Enum.Font.GothamMedium, TextSize = 13, TextXAlignment = Enum.TextXAlignment.Left, TextTruncate = Enum.TextTruncate.AtEnd }, card)
		make("TextLabel", { Position = UDim2.fromOffset(10, 26), Size = UDim2.new(1, -110, 0, 16), BackgroundTransparency = 1,
			Text = ("SISTEMA · %d recursos · %s"):format(sys.members, sys.status), TextColor3 = statusColor[sys.status] or C.muted,
			Font = Enum.Font.Gotham, TextSize = 11, TextXAlignment = Enum.TextXAlignment.Left }, card)
		local b = btn("Insertar", card, C.violet)
		b.AnchorPoint = Vector2.new(1, 0.5); b.Position = UDim2.new(1, -8, 0.5, 0)
		b.MouseButton1Click:Connect(function()
			ask(("¿Insertar el sistema %s (%d recursos, %s)? Cada script irá a su servicio. Lo que ya exista no se toca."):format(sys.name, sys.members, sys.status), function() insertSystem(sys) end)
		end)
	end
	if #systems == 0 then make("TextLabel", { Size = UDim2.new(1, 0, 0, 24), BackgroundTransparency = 1, Text = "Sin sistemas. En la app: Systems Library → Instalar plantillas BASE.", TextColor3 = C.muted, TextWrapped = true, Font = Enum.Font.Gotham, TextSize = 12 }, list) end
end

systemsBtn.MouseButton1Click:Connect(function()
	local r = request("GET", "/systems?q=" .. urlencode(searchBox.Text))
	if not r.ok then setStatus(r.err or "error", C.bad) return end
	setStatus(("%d sistema(s)"):format(#r.data.systems), C.ok)
	showSystems(r.data.systems)
end)

---------------------------------------------------------------- pruebas reales en Play
-- Script temporal que solo actúa cuando lo lanza este plugin (GetTestArgs). Ejecuta todos los
-- ModuleScript "*.spec", recoge errores de consola y devuelve el informe con EndTest.
local RUNNER_SOURCE = [==[
local StudioTestService = game:GetService("StudioTestService")
if StudioTestService:GetTestArgs() ~= "RobloxVaultTests" then return end
local HttpService = game:GetService("HttpService")
local LogService = game:GetService("LogService")
local errors = {}
LogService.MessageOut:Connect(function(msg, kind)
	if kind == Enum.MessageType.MessageError and #errors < 50 then table.insert(errors, msg) end
end)
local function expect(actual)
	local e = {}
	function e.toBe(x) if actual ~= x then error(("esperaba %s, obtuve %s"):format(tostring(x), tostring(actual)), 2) end end
	function e.toEqual(x) if HttpService:JSONEncode(actual) ~= HttpService:JSONEncode(x) then error("las tablas no son iguales", 2) end end
	function e.toBeTruthy() if not actual then error("esperaba verdadero", 2) end end
	function e.toBeFalsy() if actual then error("esperaba falso", 2) end end
	function e.toBeCloseTo(x, eps) if math.abs(actual - x) > (eps or 1e-6) then error(("esperaba ~%s, obtuve %s"):format(tostring(x), tostring(actual)), 2) end end
	function e.toThrow(frag) local ok, err = pcall(actual); if ok then error("esperaba un error", 2) end
		if frag and not string.find(tostring(err), frag, 1, true) then error("el error no coincide: " .. tostring(err), 2) end end
	return e
end
local report = { specs = {}, errors = errors }
local roots = { game:GetService("ServerScriptService"), game:GetService("ReplicatedStorage"), game:GetService("ServerStorage") }
for _, root in roots do
	for _, m in root:GetDescendants() do
		if m:IsA("ModuleScript") and string.sub(m.Name, -5) == ".spec" then
			local results = {}
			local t = { expect = expect, test = function(name, fn)
				local ok, err = pcall(fn)
				table.insert(results, { name = name, passed = ok, error = if ok then nil else tostring(err) })
			end }
			local load = function(name)
				local target = m.Parent:FindFirstChild(name)
				assert(target and target:IsA("ModuleScript"), "no encuentro el módulo " .. name)
				return require(target)
			end
			local okReq, spec = pcall(require, m)
			if okReq then
				local okRun, err = pcall(spec, t, load)
				if not okRun then table.insert(results, { name = "(spec)", passed = false, error = tostring(err) }) end
			else
				table.insert(results, { name = "(cargar spec)", passed = false, error = tostring(spec) })
			end
			local target = m.Parent:FindFirstChild(string.sub(m.Name, 1, -6))
			table.insert(report.specs, { spec = m:GetFullName(), spec_id = m:GetAttribute("VaultAssetId"),
				target_id = if target then target:GetAttribute("VaultAssetId") else nil, results = results })
		end
	end
end
task.wait(3) -- deja correr los scripts del juego para detectar errores al arrancar
StudioTestService:EndTest(HttpService:JSONEncode(report))
]==]

local testing = false
testBtn.MouseButton1Click:Connect(function()
	if testing then return end
	ask("¿Ejecutar las pruebas? Se iniciará una sesión de Play automática, se ejecutarán todos los ModuleScript *.spec y se guardarán los resultados reales en la bóveda.", function()
		testing = true
		testBtn.Text = "Probando…"
		setStatus("Ejecutando pruebas en Play…")
		local old = ServerScriptService:FindFirstChild("RobloxVaultTestRunner")
		if old then old:Destroy() end
		local runner = Instance.new("Script")
		runner.Name = "RobloxVaultTestRunner"
		runner.Source = RUNNER_SOURCE
		runner.Parent = ServerScriptService
		local ok, result = pcall(function() return StudioTestService:ExecutePlayModeAsync("RobloxVaultTests") end)
		runner:Destroy()
		testing = false
		testBtn.Text = "Ejecutar pruebas"
		if not ok then setStatus("No se pudo iniciar Play: " .. tostring(result), C.bad) return end
		if type(result) ~= "string" then setStatus("La prueba terminó sin informe (¿la paraste a mano?).", C.warn) return end
		local report = HttpService:JSONDecode(result)
		local r = request("POST", "/tests", report)
		local passed, total = 0, 0
		for _, sp in report.specs do
			for _, t in sp.results do total += 1; if t.passed then passed += 1 end end
		end
		local msg = ("Pruebas: %d/%d OK en %d spec(s)"):format(passed, total, #report.specs)
		if #report.errors > 0 then msg ..= (" · %d error(es) en consola: %s"):format(#report.errors, report.errors[1]) end
		if not r.ok then msg ..= " · no se guardó en la bóveda: " .. (r.err or "") end
		setStatus(msg, if passed == total and #report.errors == 0 then C.ok else C.bad)
	end)
end)

widget:GetPropertyChangedSignal("Enabled"):Connect(function() if widget.Enabled then refreshProject() end end)
if widget.Enabled then refreshProject() end
