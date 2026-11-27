--!strict
-- VaultSpec: mini framework de pruebas compartido por la bóveda, Studio (Play) y Luau puro.
-- Un archivo *.spec devuelve: function(t) t.test("nombre", function() t.expect(x).toBe(y) end) end

export type Result = { name: string, passed: boolean, error: string? }

local VaultSpec = {}

local function fmt(v: any): string
	if typeof(v) == "string" then return string.format("%q", v) end
	return tostring(v)
end

local function deepEqual(a: any, b: any): boolean
	if a == b then return true end
	if type(a) ~= "table" or type(b) ~= "table" then return false end
	for k, v in a do if not deepEqual(v, b[k]) then return false end end
	for k in b do if a[k] == nil then return false end end
	return true
end

function VaultSpec.expect(actual: any)
	local e = {}
	function e.toBe(expected: any)
		if actual ~= expected then error(("esperaba %s, obtuve %s"):format(fmt(expected), fmt(actual)), 2) end
	end
	function e.toEqual(expected: any)
		if not deepEqual(actual, expected) then error("las tablas no son iguales", 2) end
	end
	function e.toBeTruthy()
		if not actual then error(("esperaba un valor verdadero, obtuve %s"):format(fmt(actual)), 2) end
	end
	function e.toBeFalsy()
		if actual then error(("esperaba falso/nil, obtuve %s"):format(fmt(actual)), 2) end
	end
	function e.toBeCloseTo(expected: number, eps: number?)
		if math.abs(actual - expected) > (eps or 1e-6) then error(("esperaba ~%s, obtuve %s"):format(fmt(expected), fmt(actual)), 2) end
	end
	function e.toThrow(fragment: string?)
		local ok, err = pcall(actual)
		if ok then error("esperaba un error y no hubo", 2) end
		if fragment and not string.find(tostring(err), fragment, 1, true) then error(("el error no contiene %q: %s"):format(fragment, tostring(err)), 2) end
	end
	return e
end

-- Ejecuta una spec (función) y devuelve los resultados de cada prueba.
function VaultSpec.run(spec: (any) -> ()): { Result }
	local results: { Result } = {}
	local t = {
		expect = VaultSpec.expect,
		test = function(name: string, fn: () -> ())
			local ok, err = pcall(fn)
			table.insert(results, { name = name, passed = ok, error = if ok then nil else tostring(err) })
		end,
	}
	local ok, err = pcall(spec, t)
	if not ok then table.insert(results, { name = "(carga de la spec)", passed = false, error = tostring(err) }) end
	return results
end

return VaultSpec
