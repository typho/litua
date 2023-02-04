local debug = require("debug")

-- global variable with hooks, global user variables, and configuration
Litua = {
    ["hooks"] = {
        ["setup"] = { [""] = {} },
        ["read-new-node"] = {},
        ["modify-node"] = {},
        ["read-modified-node"] = {},
        ["convert-node-to-string"] = {},
        ["modify-final-string"] = { [""] = {} },
        ["teardown"] = { [""] = {} },
    },
    ["global"] = {},
    ["config"] = {},
}

--- Register a new hook
-- Store the hook function `hook_impl` in the Litua hooks table
-- to trigger it for every call named `filter` (or every call if
-- filter is ""). The type of hook is defined by `hook_name`
-- @param hook_name  a hook name like read-new-node or setup
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook_impl  hook function to invoke
Litua.register_hook = function (hook_name, filter, hook_impl)
    local levels = 3 -- how many calls above is the user scope?

    -- create a string representation of the call location
    local line_number = debug.getinfo(levels).currentline
    local source_file = debug.getinfo(levels).source
    local scope = debug.getinfo(levels - 1).name
    local call_repr = "'" .. tostring(scope) .. "' hook from '" .. tostring(source_file) .. "' at line " .. tostring(line_number)

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
    Litua.log("register_hook", call_repr .. " registered")

    if hook_name == "convert-node-to-string" and #Litua.hooks[hook_name][filter] > 1 then
        Litua.error("hook 'convert-node-to-string' must only be registered once for call '" .. filter .. "'", {
            ["source"] = call_repr,
        })
    end
end

-- hooks API

--- Register a new read_new_node hook, invoked after the setup hook
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook ``function (node_copy, depth) return nil end`` to invoke
Litua.read_new_node = function (filter, hook) Litua.register_hook("read-new-node", filter, hook) end

--- Register a new modify_node hook, invoked after read_new_node hooks
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook ``function (node, depth, filter) return node, nil end`` to invoke
Litua.modify_node = function (filter, hook) Litua.register_hook("modify-node", filter, hook) end

--- Register a new read_modified_node hook, invoked after modify_node hooks
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook ``function (node_copy, depth) return nil end`` to invoke
Litua.read_modified_node = function (filter, hook) Litua.register_hook("read-modified-node", filter, hook) end

--- Register a new convert_node_to_string hook, invoked after read_modified_node hooks
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook ``function (node, depth, filter) return "…", nil end`` to invoke
Litua.convert_node_to_string = function (filter, hook) Litua.register_hook("convert-node-to-string", filter, hook) end

--- Register a new modify_final_string hook, invoked after convert_node_to_string hooks
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook ``function (repr) return repr end`` to invoke
Litua.modify_final_string = function (hook) Litua.register_hook("modify-final-string", "", hook) end

--- Register a new setup hook, invoked once after all nodes where just created
-- @param hook  hook ``function () return nil end`` to invoke
Litua.on_setup = function (hook) Litua.register_hook("setup", "", hook) end

--- Register a new teardown hook, invoked once after modify_final_string hooks
-- @param hook  hook ``function () return nil end`` to invoke
Litua.on_teardown = function (hook) Litua.register_hook("teardown", "", hook) end

-- NOTE: Litua.transform will be inserted later from the file litua.transform.lua
