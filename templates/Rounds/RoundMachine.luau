--!strict
-- Máquina de estados de rondas (pura): Lobby → Intermission → Playing → Ended → Lobby.
export type State = "Lobby" | "Intermission" | "Playing" | "Ended"
export type Config = { minPlayers: number, intermission: number, roundLength: number, endScreen: number }

local RoundMachine = {}
RoundMachine.__index = RoundMachine

export type Machine = typeof(setmetatable({} :: {
	config: Config, state: State, timeLeft: number, alive: { [any]: boolean }, winners: { any }, listeners: { (State, State) -> () },
}, RoundMachine))

function RoundMachine.new(config: Config): Machine
	return setmetatable({ config = config, state = "Lobby" :: State, timeLeft = 0, alive = {}, winners = {}, listeners = {} }, RoundMachine)
end

function RoundMachine.onChange(self: Machine, fn: (State, State) -> ())
	table.insert(self.listeners, fn)
end

function RoundMachine._set(self: Machine, s: State, time: number)
	local old = self.state
	self.state = s; self.timeLeft = time
	for _, fn in self.listeners do fn(old, s) end
end

function RoundMachine.aliveCount(self: Machine): number
	local n = 0
	for _, a in self.alive do if a then n += 1 end end
	return n
end

-- Avanza el tiempo. `players` = jugadores conectados ahora mismo.
function RoundMachine.step(self: Machine, dt: number, players: { any })
	local c = self.config
	if self.state == "Lobby" then
		if #players >= c.minPlayers then self:_set("Intermission", c.intermission) end
		return
	end
	self.timeLeft -= dt
	if self.state == "Intermission" then
		if #players < c.minPlayers then self:_set("Lobby", 0); return end
		if self.timeLeft <= 0 then
			self.alive = {}
			for _, p in players do self.alive[p] = true end
			self.winners = {}
			self:_set("Playing", c.roundLength)
		end
	elseif self.state == "Playing" then
		-- los que se fueron quedan eliminados
		local present = {}
		for _, p in players do present[p] = true end
		for p in self.alive do if not present[p] then self.alive[p] = false end end
		local alive = self:aliveCount()
		if self.timeLeft <= 0 or alive <= 1 then
			self.winners = {}
			for p, a in self.alive do if a then table.insert(self.winners, p) end end
			self:_set("Ended", c.endScreen)
		end
	elseif self.state == "Ended" then
		if self.timeLeft <= 0 then self:_set("Lobby", 0) end
	end
end

function RoundMachine.eliminate(self: Machine, player: any)
	if self.state == "Playing" and self.alive[player] then self.alive[player] = false end
end

return RoundMachine
