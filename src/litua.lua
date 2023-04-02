local debug = require("debug")

-- global variable with hooks, global user variables, and configuration
Litua = {
    ["hooks"] = {
        ["on_setup"] = { [""] = {} },
        ["modify_initial_string"] = { [""] = {} },
        ["read_new_node"] = {},
        ["modify_node"] = {},
        ["read_modified_node"] = {},
        ["convert_node_to_string"] = {},
        ["modify_final_string"] = { [""] = {} },
        ["on_teardown"] = { [""] = {} },
    },
    ["global"] = {},
    ["config"] = {},
}

--- A table implementation which logs any accesses to its items
--- but is completely transparent about other operations
AccessLoggingTable = {}
AccessLoggingTable.tablename = "Litua.global"
AccessLoggingTable.__index = function(table, key)
    Litua.log("internal", Litua.format("indexing %1 of %2", key, AccessLoggingTable.tablename))
    return rawget(table, key)
end
AccessLoggingTable.__newindex = function(table, key, value)
    rawset(table, key, value)
    Litua.log("internal", Litua.format("setting %1 of %2 to %3", key, AccessLoggingTable.tablename, value))
    return rawget(table, key)
end

--- Turn Litua.global into an AccessLoggingTable
setmetatable(Litua.global, AccessLoggingTable)

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
    local call_repr = Litua.format("%1 hook from %2 at line %3", scope, source_file, line_number)

    -- validate arguments
    if type(filter) ~= "string" then
        Litua.error("filter argument must be a string", {
            ["source"] = call_repr,
            ["expected"] = "a string with the name of the call",
            ["actual"] = Litua.format("%1", filter),
            ["fix"] = "change the filter argument to a call name as a string",
        })
    end
    if type(hook_impl) ~= "function" then
        Litua.error("hook argument must be a function", {
            ["source"] = call_repr,
            ["expected"] = "a function as implementation of the hook",
            ["actual"] = Litua.format("%1", hook_impl),
            ["fix"] = "change the hook argument to a function",
        })
    end
    if filter:match("[%s]") ~= nil then
        Litua.error("filter argument must not contain a whitespace", {
            ["source"] = call_repr,
            ["expected"] = "a valid call name, since the filter argument must be a call name",
            ["actual"] = Litua.format("%1", filter),
        })
    end
    if filter:match("[\\[]") ~= nil then
        Litua.error("filter argument must not contain square braces", {
            ["source"] = call_repr,
            ["expected"] = "a valid call name, since the filter argument must be a call name",
            ["actual"] = Litua.format("%1", filter),
        })
    end
    if type(Litua.hooks[hook_name]) ~= "table" then
        Litua.error("unknown hook '" .. tostring(hook_name) .. "'", {
            ["source"] = call_repr,
            ["expected"] = "hook name like 'read-new-node' or 'modify-node'",
            ["actual"] = Litua.format("%1", filter),
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
        local msg = "hook %1 must only be registered once for call %2"
        Litua.error(Litua.format(msg, "convert-node-to-string", filter), {
            ["source"] = call_repr,
        })
    end
end

-- hooks API

--- Register a new on_setup hook, invoked once after all nodes where just created
-- @param hook  hook like ``function () return nil end`` to invoke
Litua.on_setup = function (hook) Litua.register_hook("on_setup", "", hook) end

--- Register a new modify_initial_string hook, invoked after on_setup hooks
-- @param hook  hook like ``function (text) return text end`` to invoke
Litua.modify_initial_string = function (hook) Litua.register_hook("modify_initial_string", "", hook) end

--- Register a new read_new_node hook, invoked after turning the text document into a tree of nodes
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook like ``function (node_copy, depth) return nil end`` to invoke
Litua.read_new_node = function (filter, hook) Litua.register_hook("read_new_node", filter, hook) end

--- Register a new modify_node hook, invoked after read_new_node hooks
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook like ``function (node, depth, filter) return node, nil end`` to invoke
Litua.modify_node = function (filter, hook) Litua.register_hook("modify_node", filter, hook) end

--- Register a new read_modified_node hook, invoked after modify_node hooks
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook like ``function (node_copy, depth) return nil end`` to invoke
Litua.read_modified_node = function (filter, hook) Litua.register_hook("read_modified_node", filter, hook) end

--- Register a new convert_node_to_string hook, invoked after read_modified_node hooks
-- @param filter  call name to filter for, or "" to call hook for every call
-- @param hook  hook like ``function (node, depth, filter) return "…", nil end`` to invoke
Litua.convert_node_to_string = function (filter, hook) Litua.register_hook("convert_node_to_string", filter, hook) end

--- Register a new modify_final_string hook, invoked after the tree has been turned into a string again
-- @param hook  hook like ``function (text) return text end`` to invoke
Litua.modify_final_string = function (hook) Litua.register_hook("modify_final_string", "", hook) end

--- Register a new on_teardown hook, invoked once after modify_final_string hooks
-- @param hook  hook like ``function () return nil end`` to invoke
Litua.on_teardown = function (hook) Litua.register_hook("on_teardown", "", hook) end

--- Pre-processing functions are all hooks which run
--- without requiring the input as tree.
-- @param text  the text document content
-- @return  text document content
Litua.preprocess = function (text)
    local result, hook_name

    -- (0) run on_setup hooks
    hook_name = "on_setup"
    Litua.log("preprocess", "run " .. hook_name .. " hooks")
    for i=1,#Litua.hooks[hook_name][""] do
        Litua.log("preprocess", "ran " .. Litua.hooks[hook_name][""][i].src)
        result = Litua.hooks[hook_name][""][i].impl()
        if result ~= nil then
            Litua.error(Litua.format("%1 hook returned non-nil value", hook_name), {
                ["expected"] = Litua.format("%1 hooks must return nil", hook_name),
                ["actual"] = Litua.format("return value is %1", result),
                ["source"] = Litua.hooks[hook_name][""][i].src,
            })
        end
    end

    -- (1) modify modify_initial_string hooks
    hook_name = "modify_initial_string"
    Litua.log("preprocess", "run " .. hook_name .. " hooks")
    for i=1,#Litua.hooks[hook_name][""] do
        text = Litua.hooks[hook_name][""][i].impl(text)
        if type(text) ~= "string" then
            Litua.error(Litua.format("%1 hook returned non-string value as first return value", hook_name), {
                ["context"] = Litua.format("%1 hooks must return two values (string representation and error)", hook_name),
                ["expected"] = "string representation return value to be a string",
                ["actual"] = Litua.format("string representation return value is %1", type(text)),
                ["fix"] = "make hook return a string",
                ["source"] = Litua.hooks[hook_name][""][i].src,
            })
        end
    end

    return text
end

-- NOTE: Litua.transform will be inserted later from the file litua.transform.lua
