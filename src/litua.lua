local string = require("string")
local debug = require("debug")

Litua = {
    ["hooks"] = {
        ["init"] = {},
        ["pre-debug"] = {},
        ["modify-node"] = {},
        ["node-to-string"] = {},
        ["post-debug"] = {},
        ["final"] = {},
    },
    ["global"] = {},
    ["config"] = {},
}

Litua.register_hook = function (hook_name, filter, hook_impl)
    local levels = 2 -- how many calls above is the user scope?

    -- create a string representation of the call location
    local line_number = debug.getinfo(levels).currentline
    local source_file = debug.getinfo(levels).source
    local scope = debug.getinfo(levels).name
    local call_repr = "hook from '" .. tostring(scope) .. "' in '" .. tostring(source_file) .. "' at " .. tostring(line_number)

    -- validate arguments
    if type(filter) ~= "string" or type(hook_impl) ~= "function" then
        print("ERROR: when registering a hook, the 1st argument must be a string and the 2nd argument be a function. I received " .. type(filter) .. " and " .. type(hook_impl))
    end
    if filter:match("[%s]") ~= nil then
        print("ERROR: hook names must not contain whitespaces, but received '" .. tostring(filter) .. "' in " .. call_repr)
    end
    if filter:match("[\\[]") ~= nil then
        print("ERROR: hook names must not contain square braces, but received '" .. tostring(filter) .. "' in " .. call_repr)
    end

    -- register hook
    if Litua.hooks[hook_name] ~= nil then
        print("WARN: overwriting existing hook.", Litua.hooks[hook_name].src, "overwritten by", call_repr)
    end

    Litua.hooks[hook_name] = {
        ["match"] = (function(f)
            -- match will return true if the argument matches `filter`
            return function (name) return name == f end
        end)(filter),
        ["src"] = call_repr,
        ["impl"] = hook_impl,
    }
end

-- hooks API
Litua.on_init = function (filter, hook) Litua.register_hook("init", filter, hook) end
Litua.look_at_new_node = function (filter, hook) Litua.register_hook("pre-debug", filter, hook) end
Litua.modify_node = function (filter, hook) Litua.register_hook("modify-node", filter, hook) end
Litua.convert_node_to_string = function (filter, hook) Litua.register_hook("node-to-string", filter, hook) end
Litua.look_at_finished_node = function (filter, hook) Litua.register_hook("post-debug", filter, hook) end
Litua.modify_final_string = function (filter, hook) Litua.register_hook("final", filter, hook) end

-- Litua.transform will be inserted later
