--!strict
-- Detección de velocidad y teletransporte imposibles (pura). El servidor le pasa muestras de posición.
-- No castiga sola: devuelve "sospechas" para que decidas (devolver al jugador, registrar, expulsar tras N).
export type Sample = { x: number, y: number, z: number, t: number }
export type Config = { maxSpeed: number, tolerance: number, maxTeleport: number, strikesToFlag: number }

local MovementCheck = {}
MovementCheck.__index = MovementCheck

export type Checker = typeof(setmetatable({} :: { config: Config, last: Sample?, strikes: number, allowUntil: number }, MovementCheck))

function MovementCheck.new(config: Config): Checker
	return setmetatable({ config = config, last = nil, strikes = 0, allowUntil = -math.huge }, MovementCheck)
end

-- Úsalo cuando TU juego mueve al jugador (teletransporte legítimo, lanzadera, respawn).
function MovementCheck.allowFor(self: Checker, now: number, seconds: number)
	self.allowUntil = now + seconds
	self.last = nil
end

-- Devuelve (sospechoso, motivo). Solo cuenta la distancia horizontal para no castigar caídas.
function MovementCheck.sample(self: Checker, s: Sample): (boolean, string?)
	local prev = self.last
	self.last = s
	if not prev or s.t <= prev.t or s.t < self.allowUntil then return false, nil end
	local dx, dz = s.x - prev.x, s.z - prev.z
	local dist = math.sqrt(dx * dx + dz * dz)
	local dt = s.t - prev.t
	local c = self.config
	if dist > c.maxTeleport then
		self.strikes += c.strikesToFlag
		return true, ("teletransporte de %.1f studs"):format(dist)
	end
	local speed = dist / dt
	if speed > c.maxSpeed * c.tolerance then
		self.strikes += 1
		if self.strikes >= c.strikesToFlag then return true, ("velocidad %.1f > %.1f"):format(speed, c.maxSpeed) end
	else
		self.strikes = math.max(0, self.strikes - 0.25)
	end
	return false, nil
end

return MovementCheck
