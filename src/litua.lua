local string = require("string")
local debug = require("debug")

Litua = {
    ["hooks"] = {
        ["setup"] = {},
        ["read-new-node"] = {},
        ["modify-node"] = {},
        ["read-modified-node"] = {},
        ["convert-node-to-string"] = {},
        ["modify-final-string"] = {},
        ["teardown"] = {},
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
    local call_repr = "'" .. tostring(scope) .. "' hook from '" .. tostring(source_file) .. "' at " .. tostring(line_number)

    -- validate arguments
    if type(filter) ~= "string" then
        Litua.error("filter argument must be a string", {
            ["source"] = call_repr,
            ["expected"] = "a string with the name of the call",
            ["actual"] = "'" .. tostring(filter) .. "'",
            ["fix"] = "change the filter argument to a call name as a string",
        })
    end
    if type(hook_impl) ~= "function" then
        Litua.error("hook argument must be a function", {
            ["source"] = call_repr,
            ["expected"] = "a function as implementation of the hook",
            ["actual"] = "'" .. tostring(hook_impl) .. "'",
            ["fix"] = "change the hook argument to a function",
        })
    end
    if filter:match("[%s]") ~= nil then
        Litua.error("filter argument must not contain a whitespace", {
            ["source"] = call_repr,
            ["expected"] = "a valid call name, since the filter argument must be a call name",
            ["actual"] = "'" .. tostring(filter) .. "'",
        })
    end
    if filter:match("[\\[]") ~= nil then
        Litua.error("filter argument must not contain square braces", {
            ["source"] = call_repr,
            ["expected"] = "a valid call name, since the filter argument must be a call name",
            ["actual"] = "'" .. tostring(filter) .. "'",
        })
    end
    if type(Litua.hooks[hook_name]) ~= "table" then
        Litua.error("unknown hook '" .. tostring(hook_name) .. "'", {
            ["source"] = call_repr,
            ["expected"] = "hook name like 'read-new-node' or 'modify-node'",
            ["actual"] = "'" .. tostring(filter) .. "'",
        })
    end

    -- everything fine, let's insert the hook!
    if type(Litua.hooks[hook_name][filter]) == "nil" then
        Litua.hooks[hook_name][filter] = {}
    end
    table.insert(Litua.hooks[hook_name][filter], {
        ["src"] = call_repr,
        ["impl"] = hook_impl,
    })

    if hook_name == "modify-string" and #Litua.hooks[hook_name][filter] > 1 then
        Litua.error("hook 'modify-string' must only be registered once for call '" .. filter .. "'", {
            ["source"] = call_repr,
        })
    end
end

-- hooks API
Litua.read_new_node = function (filter, hook) Litua.register_hook("read-new-node", filter, hook) end
Litua.modify_node = function (filter, hook) Litua.register_hook("modify-node", filter, hook) end
Litua.read_modified_node = function (filter, hook) Litua.register_hook("read-modified-node", filter, hook) end
Litua.convert_node_to_string = function (filter, hook) Litua.register_hook("convert-node-to-string", filter, hook) end
Litua.modify_final_string = function (filter, hook) Litua.register_hook("modify-final-string", filter, hook) end

Litua.on_setup = function (hook) Litua.register_hook("setup", "", hook) end
Litua.on_teardown = function (hook) Litua.register_hook("teardown", "", hook) end

-- Litua.transform will be inserted later
