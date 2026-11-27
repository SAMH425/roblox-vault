--!strict
-- Inventario puro (sin servicios de Roblox): pilas, capacidad, serialización para guardar.
export type Stack = { id: string, count: number }
export type ItemDef = { maxStack: number }

local Inventory = {}
Inventory.__index = Inventory

export type Inventory = typeof(setmetatable({} :: { capacity: number, slots: { Stack }, defs: { [string]: ItemDef } }, Inventory))

function Inventory.new(capacity: number, defs: { [string]: ItemDef }): Inventory
	assert(capacity > 0, "capacity debe ser > 0")
	return setmetatable({ capacity = capacity, slots = {}, defs = defs }, Inventory)
end

function Inventory.count(self: Inventory, id: string): number
	local n = 0
	for _, s in self.slots do if s.id == id then n += s.count end end
	return n
end

-- Cuánto de `id` cabe todavía.
function Inventory.spaceFor(self: Inventory, id: string): number
	local def = self.defs[id]
	if not def then return 0 end
	local space = 0
	for _, s in self.slots do if s.id == id then space += def.maxStack - s.count end end
	return space + (self.capacity - #self.slots) * def.maxStack
end

-- Añade todo o nada. Devuelve true si se añadió.
function Inventory.add(self: Inventory, id: string, amount: number): boolean
	local def = self.defs[id]
	if not def or amount <= 0 or amount ~= math.floor(amount) then return false end
	if self:spaceFor(id) < amount then return false end
	for _, s in self.slots do
		if amount == 0 then break end
		if s.id == id and s.count < def.maxStack then
			local put = math.min(def.maxStack - s.count, amount)
			s.count += put; amount -= put
		end
	end
	while amount > 0 do
		local put = math.min(def.maxStack, amount)
		table.insert(self.slots, { id = id, count = put }); amount -= put
	end
	return true
end

-- Quita todo o nada (desde las últimas pilas).
function Inventory.remove(self: Inventory, id: string, amount: number): boolean
	if amount <= 0 or self:count(id) < amount then return false end
	for i = #self.slots, 1, -1 do
		local s = self.slots[i]
		if s.id == id then
			local take = math.min(s.count, amount)
			s.count -= take; amount -= take
			if s.count == 0 then table.remove(self.slots, i) end
			if amount == 0 then break end
		end
	end
	return true
end

function Inventory.serialize(self: Inventory): { Stack }
	local out = {}
	for _, s in self.slots do table.insert(out, { id = s.id, count = s.count }) end
	return out
end

-- Carga datos guardados descartando objetos que ya no existen o pilas inválidas.
function Inventory.load(capacity: number, defs: { [string]: ItemDef }, data: { Stack }?): Inventory
	local inv = Inventory.new(capacity, defs)
	for _, s in data or {} do
		if type(s) == "table" and type(s.id) == "string" and type(s.count) == "number" then inv:add(s.id, s.count) end
	end
	return inv
end

return Inventory
