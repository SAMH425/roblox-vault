-- Ejecuta todas las *.spec en Luau puro: luau run_specs.luau -a <carpeta_templates>
local VaultSpec = require("./_spec/VaultSpec")
local systems = { "Inventory", "Economy", "DataSave", "Rounds", "AntiCheat" }
local specs = {
	Inventory = { "Inventory" }, Economy = { "CurrencyLogic" }, DataSave = { "DataCore" },
	Rounds = { "RoundMachine" }, AntiCheat = { "MovementCheck" },
}
local total, failed = 0, 0
for _, sys in systems do
	for _, mod in specs[sys] do
		local spec = require("./" .. sys .. "/" .. mod .. ".spec")
		local results = VaultSpec.run(function(t) spec(t, function(name) return require("./" .. sys .. "/" .. name) end) end)
		for _, r in results do
			total += 1
			if not r.passed then failed += 1 end
			print((r.passed and "  ok  " or "  FALLO ") .. sys .. " · " .. r.name .. (r.error and (" → " .. r.error) or ""))
		end
	end
end
print(("%d pruebas, %d fallos"):format(total, failed))
if failed > 0 then error("hay pruebas fallidas") end
