-- Code transpiled & optimized from Pilot to Lua 5.4 / LuaJIT
-- Optimizations: local register bindings, numeric range loops, constant folding

local Priority = {
    Low = 1,
    Medium = 2,
    High = 3,
}
local function Circle(r)
    return { _tag = "Circle", r = r }
end
local function Rect(w, h)
    return { _tag = "Rect", w = w, h = h }
end
local function doubled(self)
    return self * 2
end
local Counter = {}
Counter.__index = Counter
function Counter.new(init)
    return setmetatable(init or {}, Counter)
end
function Counter:step(x, delta)
    return x + delta
end
local function gcd(a, b)
    local x = a
    local y = b
    while y ~= 0 do
        local rem = x % y
        x = y
        y = rem
    end
    return x
end
local function compute_weighted_sum(limit, scale)
    local base_bonus = (limit + 2) * scale
    local total = base_bonus
    for i = 1, limit do
        if i % 2 == 0 then
            total = total + (i * scale)
        elseif i == 5 then
            total = total + (10)
        else
            total = total + (i)
        end
    end
    return total
end
local BASE_SCALE <const> = 3
local g = gcd(48, 18)
local weighted = compute_weighted_sum(5, BASE_SCALE)
local squares = (function() local __res = {}; for x = 1, 6 do if x > 2 then table.insert(__res, x * x) end end; return __res end)()
assert(g == 6)
assert(weighted == 53)
print(string.format("GCD=%s, Weighted=%s", tostring(g), tostring(weighted)))
