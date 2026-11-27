--!strict
-- Lógica de monedas y compras (pura y validada en servidor). Sin servicios de Roblox.
export type Item = { price: number, currency: string, unique: boolean? }
export type Profile = { currencies: { [string]: number }, owned: { [string]: number } }

local CurrencyLogic = {}

local function valid(n: any): boolean
	return type(n) == "number" and n == n and n >= 0 and n < 1e15 and n == math.floor(n)
end

function CurrencyLogic.newProfile(start: { [string]: number }?): Profile
	local c = {}
	for k, v in start or {} do c[k] = v end
	return { currencies = c, owned = {} }
end

function CurrencyLogic.balance(p: Profile, currency: string): number
	return p.currencies[currency] or 0
end

function CurrencyLogic.add(p: Profile, currency: string, amount: number): (boolean, string?)
	if not valid(amount) or amount == 0 then return false, "cantidad inválida" end
	p.currencies[currency] = CurrencyLogic.balance(p, currency) + amount
	return true, nil
end

function CurrencyLogic.spend(p: Profile, currency: string, amount: number): (boolean, string?)
	if not valid(amount) or amount == 0 then return false, "cantidad inválida" end
	local have = CurrencyLogic.balance(p, currency)
	if have < amount then return false, "saldo insuficiente" end
	p.currencies[currency] = have - amount
	return true, nil
end

-- Compra con todas las validaciones del servidor. Nunca confíes en precios enviados por el cliente.
function CurrencyLogic.purchase(p: Profile, catalog: { [string]: Item }, itemId: any): (boolean, string?)
	if type(itemId) ~= "string" then return false, "petición inválida" end
	local item = catalog[itemId]
	if not item then return false, "el objeto no existe" end
	if item.unique and (p.owned[itemId] or 0) > 0 then return false, "ya lo tienes" end
	local ok, err = CurrencyLogic.spend(p, item.currency, item.price)
	if not ok then return false, err end
	p.owned[itemId] = (p.owned[itemId] or 0) + 1
	return true, nil
end

return CurrencyLogic
