Litua.Filter = {}

local Any = {
    __call = function (self, _) return true end,
    __tostring = function (self) return "filter matching any element" end,
}
local any = { ["is_filter"] = true }
setmetatable(any, Any)

Litua.Filter.any = any

local ByCall = {
    __call = function (self, call) return self.call == call end,
    __tostring = function (self) return "filter matching elements '" .. self.call .. "'" end,
}

Litua.Filter.by_call = function (call)
    local err = Litua.validate_call(call)
    if err ~= nil then
        Litua.error("defining filter by_call", "a proper call name", err, "call by_call with a proper call name")
    end

    local new = { ["call"] = tostring(call), ["is_filter"] = true }
    setmetatable(new, ByCall)
    return new
end
