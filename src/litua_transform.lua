local string = require("string")

Litua.recurse = function (node, depth)
    print(("  "):rep(depth) .. "<" .. node["=call"] .. " nr-args='" .. #node["=args"] .. "' nr-children='" .. #node["=content"] .. "'/>")
    for k, v in pairs(node["=content"]) do
        if type(v) == "table" then
            local err = Litua.recurse(v, depth + 1)
            if err ~= nil then
                Litua.log("pre-debug hook", err)
                return err
            end
        end
    end
end

Litua.recurse_predebug = function (node, depth)
    local call = tostring(node["=call"])
    local args_count = #node["=args"]
    local content_count = #node["=content"]

    for hook_call, hook in pairs(Litua.hooks["pre-debug"]) do
        if hook_call == '=' or call == hook_call then
            local err = hook(call, depth, args_count, content_count)
            if err ~= nil then
                Litua.log("pre-debug hook", err)
                return err
            end
        end
    end

    for k, v in pairs(node["=content"]) do
        if type(v) == "table" then
            Litua.recurse_predebug(v, depth + 1)
        end
    end

    for k, v in pairs(node["=content"]) do
        if type(v) == "table" then
            Litua.recurse_predebug(v, depth + 1)
        end
    end
end

Litua.recurse_modify_node = function (node, depth)
    -- (1) fetch old values
    local prev_node = {
        ["call"] = tostring(node["=call"]),
        ["content"] = node["=content"],
        ["args"] = {}
    }
    -- TODO proper deep copy?
    for arg, value in pairs(node) do
        if ~string:match("^=") then
            prev_node.args[arg] = value
        end
    end

    -- (2) call hooks
    for hook_call, hook in pairs(Litua.hooks["modify-node"]) do
        if hook_call == '=' or prev_node.call == hook_call then
            local call, args, content, err = hook(prev_node.content, prev_node.args, prev_node.call, depth)

            -- error handling
            if err ~= nil then
                return err
            end

            -- validate call
            if call ~= nil and type(call) ~= "string" then
                return Litua.error("receiving hook return value 'call'", "string", type(call), "return a string")
            end

            -- validate args
            if args ~= nil and type(args) ~= "function" then
                -- TODO validate that values are nodes
                return Litua.error("receiving hook return value 'args'", "function", type(args), "return a function")
            end

            -- validate content
            if content ~= nil and type(content) ~= "function" then
                -- TODO validate that children are nodes
                return Litua.error("receiving hook return value 'content'", "function", type(content), "return a function")
            end

            -- modify node
            prev_node = { ["call"] = call, ["args"] = args, ["content"] = content }
        end
    end

    node = { ["=call"] = prev_node.call, ["=content"] = {} }
    for k, v in pairs(prev_node.content) do
        if type(v) == "table" then
            local new_child = Litua.recurse_modify_node(v, depth + 1)
            table.insert(node["=content"], new_child)
        end
    end

    return node
end

Litua.recurse_node_to_string = function (node, depth)
end

Litua.recurse_postdebug = function (node, depth)
    local call = tostring(node["=call"])
    local args_count = #node["=args"]
    local content_count = #node["=content"]
    for hook_call, hook in pairs(Litua.hooks["post-debug"]) do
        if hook_call == '=' or call == hook_call then
            hook(call, depth, args_count, content_count)
        end
    end

    for k, v in pairs(node["=content"]) do
        if type(v) == "table" then
            local err = Litua.recurse_postdebug(v, depth + 1)
            if err ~= nil then
                Litua.log("pre-debug hook", err)
                return err
            end
        end
    end
end




Litua.transform = function (tree)
    local err

    -- (1) pre-debug hooks
    err = Litua.recurse_predebug(tree, 0)
    if err ~= nil then
        return err
    end

    -- (2) node-manipulation hooks
    Litua.recurse_modify_node(tree, 0)

    -- (3) node-to-string hooks
    Litua.recurse_node_to_string(tree, 0)

    -- (4) post-debug hooks
    err = Litua.recurse_postdebug(tree, 0)
    if err ~= nil then
        return err
    end

    print("transform is called with ", tostring(tree))
    return "hello world"
end