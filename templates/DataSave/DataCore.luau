--!strict
-- Núcleo de guardado: bloqueo de sesión, reintentos con espera creciente, plantilla por defecto
-- y reconciliación. Recibe el DataStore y el reloj inyectados para poder probarlo sin Roblox.
export type Store = { UpdateAsync: (self: any, key: string, fn: (any) -> any) -> any }
export type Env = { store: Store, now: () -> number, wait: (number) -> (), jobId: string, lockTimeout: number?, retries: number? }
export type Session = { key: string, data: { [string]: any }, released: boolean }

local DataCore = {}

local function deepCopy(v: any): any
	if type(v) ~= "table" then return v end
	local t = {}
	for k, x in v do t[k] = deepCopy(x) end
	return t
end

-- Añade las claves nuevas de la plantilla sin borrar datos existentes.
function DataCore.reconcile(data: { [string]: any }, template: { [string]: any }): { [string]: any }
	for k, v in template do
		if data[k] == nil then data[k] = deepCopy(v)
		elseif type(v) == "table" and type(data[k]) == "table" and next(v) ~= nil and #v == 0 then DataCore.reconcile(data[k], v) end
	end
	return data
end

local function retry<T>(env: Env, fn: () -> T): (boolean, T | string)
	local tries = env.retries or 5
	local last: any
	for i = 1, tries do
		local ok, res = pcall(fn)
		if ok then return true, res end
		last = res
		if i < tries then env.wait(math.min(2 ^ (i - 1), 8)) end
	end
	return false, tostring(last)
end

-- Carga y bloquea. Si otra sesión tiene el bloqueo reciente, falla (evita duplicar objetos).
function DataCore.load(env: Env, key: string, template: { [string]: any }): (Session?, string?)
	local timeout = env.lockTimeout or 600
	local blockedBy: string? = nil
	local ok, res = retry(env, function()
		blockedBy = nil
		return env.store:UpdateAsync(key, function(old: any)
			old = old or { data = deepCopy(template), lock = nil }
			if old.lock and old.lock.job ~= env.jobId and env.now() - old.lock.time < timeout then
				blockedBy = old.lock.job
				return nil -- no escribir
			end
			old.lock = { job = env.jobId, time = env.now() }
			return old
		end)
	end)
	if not ok then return nil, "DataStore no disponible: " .. tostring(res) end
	if blockedBy then return nil, "datos abiertos en otro servidor" end
	local record = res :: any
	return { key = key, data = DataCore.reconcile(record.data, template), released = false }, nil
end

-- Guarda; con release=true libera el bloqueo (al salir el jugador o cerrar el servidor).
function DataCore.save(env: Env, s: Session, release: boolean): (boolean, string?)
	if s.released then return false, "sesión ya liberada" end
	local stolen = false
	local ok, err = retry(env, function()
		stolen = false
		return env.store:UpdateAsync(s.key, function(old: any)
			if old and old.lock and old.lock.job ~= env.jobId then stolen = true; return nil end
			return { data = s.data, lock = if release then nil else { job = env.jobId, time = env.now() } }
		end)
	end)
	if not ok then return false, tostring(err) end
	if stolen then return false, "otra sesión tomó los datos" end
	if release then s.released = true end
	return true, nil
end

return DataCore
