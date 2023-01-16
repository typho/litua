local string = require("string")

Litua = {
    ["hooks"] = {
        ["pre-debug"] = {},
        ["node-to-string"] = {},
        ["modify-node"] = {},
        ["post-debug"] = {},
    },
    ["lib"] = {},
    ["global"] = {},
}

-- TODO move to lib?
Litua.error = function (action, expected, unexpected, fix)
    return nil, "Litua lua error: while " .. action .. " I expected " .. expected .. " but got " .. unexpected .. ". Please " .. fix
end

-- TODO move to lib?
Litua.log = function (component, msg)
    print("[" .. component .. "]: " .. msg)
end

Litua.lib.validate_call = function (call)
    if call:match("=") ~= nil then
        return Litua.error("a valid call name", "name with '='", "use a name without '='")
    end
    return true
end

Litua.add_pre_debug_hook = function (name, hook)
    -- validate name
    if name == nil then
        name = '='
    elseif name:match("=") ~= nil then
        return Litua.error("adding a pre-debug hook", "a valid call name", "name with '='", "call add_pre_debug_hook without a name containing '=' (1st argument)")
    end
    -- validate hook
    if type(hook) ~= "string" then
        return Litua.error("adding a pre-debug hook", "a hook function", type(hook), "call add_pre_debug_hook with a function (as 2nd argument)")
    end

    -- actually add the hook
    Litua.env.hooks["pre-debug"][name] = hook
    return "hook added", nil
end

Litua.add_node_to_string_hook = function (name, hook)
    -- validate name
    if name == nil then
        name = '='
    elseif name:match("=") ~= nil then
        return Litua.error("adding a node-to-string hook", "a valid call name", "name with '='", "call add_node_to_string_hook without a name containing '=' (1st argument)")
    end
    -- validate hook
    if type(hook) ~= "string" then
        return Litua.error("adding a node-to-string hook", "a hook function", type(hook), "call add_node_to_string_hook with a function (as 2nd argument)")
    end

    -- actually add the hook
    Litua.env.hooks["node-to-string"][name] = hook
    return "hook added", nil
end

Litua.add_modify_node_hook = function (name, hook)
    -- validate name
    if name == nil then
        name = '='
    elseif name:match("=") ~= nil then
        return Litua.error("adding a modify-node hook", "a valid call name", "name with '='", "call add_modify_node_hook without a name containing '=' (1st argument)")
    end
    -- validate hook
    if type(hook) ~= "string" then
        return Litua.error("adding a modify-node hook", "a hook function", type(hook), "call add_modify_node_hook with a function (as 2nd argument)")
    end

    -- actually add the hook
    Litua.env.hooks["modify-node"][name] = hook
    return "hook added", nil
end

Litua.add_post_debug_hook = function (name, hook)
    -- validate name
    if name == nil then
        name = '='
    elseif name:match("=") ~= nil then
        return Litua.error("adding a post-debug hook", "a valid call name", "name with '='", "call add_post_debug_hook without a name containing '=' (1st argument)")
    end
    -- validate hook
    if type(hook) ~= "string" then
        return Litua.error("adding a post-debug hook", "a hook function", type(hook), "call add_post_debug_hook with a function (as 2nd argument)")
    end

    -- actually add the hook
    Litua.env.hooks["post-debug"][name] = hook
    return "hook added", nil
end

-- transform will be inserted by litua_transform.lua
