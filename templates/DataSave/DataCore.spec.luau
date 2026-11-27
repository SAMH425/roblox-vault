return function(t, load)
	local DataCore = load("DataCore")
	local function fakeStore(failTimes: number?)
		local db, fails = {}, failTimes or 0
		return {
			db = db,
			UpdateAsync = function(_, key, fn)
				if fails > 0 then fails -= 1; error("502: DataStore caído") end
				local copy = db[key] and table.clone(db[key]) or nil
				local new = fn(copy)
				if new ~= nil then db[key] = new end
				return db[key]
			end,
		}
	end
	local clock = 1000
	local function env(store, job)
		return { store = store, now = function() return clock end, wait = function() end, jobId = job or "A" }
	end
	local template = { Coins = 100, Inventory = {}, Settings = { Music = true } }

	t.test("jugador nuevo recibe la plantilla", function()
		local s = DataCore.load(env(fakeStore()), "u1", template)
		t.expect(s.data.Coins).toBe(100)
		t.expect(s.data.Settings.Music).toBe(true)
	end)
	t.test("guarda y vuelve a cargar", function()
		local st = fakeStore()
		local s = DataCore.load(env(st), "u1", template)
		s.data.Coins = 555
		t.expect((DataCore.save(env(st), s, true))).toBe(true)
		local s2 = DataCore.load(env(st, "B"), "u1", template)
		t.expect(s2.data.Coins).toBe(555)
	end)
	t.test("bloqueo: otro servidor no puede abrir los mismos datos", function()
		local st = fakeStore()
		DataCore.load(env(st, "A"), "u1", template)
		local s2, err = DataCore.load(env(st, "B"), "u1", template)
		t.expect(s2).toBe(nil)
		t.expect(err).toBe("datos abiertos en otro servidor")
	end)
	t.test("el bloqueo caduca si el servidor murió", function()
		local st = fakeStore()
		DataCore.load(env(st, "A"), "u1", template)
		clock += 601
		t.expect(DataCore.load(env(st, "B"), "u1", template) ~= nil).toBe(true)
	end)
	t.test("reintenta cuando el DataStore falla", function()
		local st = fakeStore(3)
		t.expect(DataCore.load(env(st), "u2", template) ~= nil).toBe(true)
		local s, err = DataCore.load(env(fakeStore(10)), "u3", template)
		t.expect(s).toBe(nil)
		t.expect(string.find(err, "DataStore no disponible", 1, true) ~= nil).toBe(true)
	end)
	t.test("reconcilia claves nuevas sin borrar las viejas", function()
		local d = DataCore.reconcile({ Coins = 7, Settings = { Music = false } }, { Coins = 0, Gems = 5, Settings = { Music = true, Sfx = true } })
		t.expect(d.Coins).toBe(7); t.expect(d.Gems).toBe(5)
		t.expect(d.Settings.Music).toBe(false); t.expect(d.Settings.Sfx).toBe(true)
	end)
	t.test("no guarda si otra sesión robó el bloqueo", function()
		local st = fakeStore()
		local s = DataCore.load(env(st, "A"), "u1", template)
		clock += 601
		DataCore.load(env(st, "B"), "u1", template)
		local ok, err = DataCore.save(env(st, "A"), s, false)
		t.expect(ok).toBe(false); t.expect(err).toBe("otra sesión tomó los datos")
	end)
end
