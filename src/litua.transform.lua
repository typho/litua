--- Given a nested structure of tables, return a nested structure of Litua.Node tables
-- This function converts the hierarchy of tables into actual Litua.Node
-- (also in a hierarchical structure)
-- @param tree  the root node
-- @return  the Litua.Node instance
Litua.tree_to_nodes = function (tree)
    local new_call = tostring(tree.call)
    local new_args = {}
    local new_content = {}

    for argkey, argvalues in pairs(tree.args) do
        local new_arg = {}
        for _, argvalue in ipairs(argvalues) do
            if type(argvalue) == "table" then
                table.insert(new_arg, Litua.tree_to_nodes(argvalue))
            else
                table.insert(new_arg, tostring(argvalue))
            end
        end
        new_args[argkey] = new_arg
    end

    for _, value in ipairs(tree.content) do
        if type(value) == "table" then
            table.insert(new_content, Litua.tree_to_nodes(value))
        else
            table.insert(new_content, tostring(value))
        end
    end

    return Litua.Node.init(new_call, new_args, new_content)
end

--- Implementation of the read-new-node hooks
-- This function invokes the hook for the node and then recurses into any content
-- or arg nodes (unless it's a string and thus has no children nodes)
-- @param node  the current node to process
-- @param depth  the current recursion depth
-- @param hook_name  "read-new-node"
-- @return  error or nil
Litua.recurse_reading = function (node, depth, hook_name)
    local err

    local calls = { node.call, "" }
    for _, call in ipairs(calls) do
        if Litua.hooks[hook_name][call] ~= nil then
            for i, hook in ipairs(Litua.hooks[hook_name][call]) do
                Litua.log("transform", "ran " .. Litua.hooks[hook_name][call][i].src .. " for call '" .. node.call .. "'")
                err = hook.impl(node:copy(), depth)
                if err ~= nil then
                    Litua.error(tostring(hook_name) .. " hook #" .. tostring(i) .. " returned non-nil value", {
                        ["expected"] = "return value nil",
                        ["actual"] = err,
                        ["fix"] = "make hook return non-error",
                        ["source"] = hook.src,
                    })
                    return err
                end
            end
        end
    end

    for _, argvalues in ipairs(node.args) do
        for _, argvalue in ipairs(argvalues) do
            if argvalue.is_node then
                err = Litua.recurse_reading(argvalue, depth + 1, hook_name)
                if err ~= nil then
                    return err
                end
            end
        end
    end

    for _, value in ipairs(node.content) do
        if value.is_node then
            err = Litua.recurse_reading(value, depth + 1, hook_name)
            if err ~= nil then
                return err
            end
        end
    end
end

--- Implementation of the modify-node hooks
-- This function invokes the hook for the node and
-- then recurses into any content or arg nodes
-- @param node  the current node to process
-- @param depth  the current recursion depth
-- @param hook_name  "modify-node"
-- @return  (modified node, error or nil)
Litua.recurse_modify_node = function (node, depth, hook_name)
    local err

    local calls = { node.call, "" }
    for _, call in ipairs(calls) do
        if Litua.hooks[hook_name][call] ~= nil then
            for i, hook in ipairs(Litua.hooks[hook_name][call]) do
                Litua.log("transform", "ran " .. Litua.hooks[hook_name][call][i].src .. " for call '" .. node.call .. "'")
                node, err = hook.impl(node, depth, call)
                if node == nil or (not node.is_node and type(node) ~= "string") then
                    Litua.error(tostring(hook_name) .. " hook #" .. tostring(i) .. " returned nil value", {
                        ["expected"] = "return value node",
                        ["actual"] = "nil",
                        ["fix"] = "make hook return a proper node",
                        ["source"] = hook.src,
                    })
                    return err
                elseif err ~= nil then
                    Litua.error(tostring(hook_name) .. " hook #" .. tostring(i) .. " returned non-nil value", {
                        ["expected"] = "return value nil",
                        ["actual"] = err,
                        ["fix"] = "make hook return non-error",
                        ["source"] = hook.src,
                    })
                    return err
                end
            end
        end
    end

    if type(node) ~= "string" then
        for argkey, argvalues in pairs(node.args) do
            for i, argvalue in ipairs(argvalues) do
                if argvalue.is_node then
                    node.args[argkey][i], err = Litua.recurse_modify_node(argvalue, depth + 1, hook_name)
                    if err ~= nil then
                        return nil, err
                    end
                end
            end
        end

        for i, value in ipairs(node.content) do
            if value.is_node then
                node.content[i], err = Litua.recurse_modify_node(value, depth + 1, hook_name)
                if err ~= nil then
                    return nil, err
                end
            end
        end
    end

    return node, nil
end

--- Implementation of the convert-node-to-string hooks
-- This function invokes the hook for the args nodes,
-- then content nodes and finally for the node itself
-- @param node  the current node to process
-- @param depth  the current recursion depth
-- @param hook_name  "convert-node-to-string"
-- @return  (string representation, error or nil)
Litua.recurse_node_to_string = function (node, depth, hook_name)
    local err
    if node.call == "left-curly-brace" then return "{", nil end
    if node.call == "right-curly-brace" then return "}", nil end

    -- (1) resolve args to string
    for i, arg in ipairs(node.args) do
        if arg.is_node then
            node.args[i], err = Litua.recurse_node_to_string(arg, depth + 1, hook_name)
            if err ~= nil then
                return node.args[i], err
            end
        else
            node.args[i] = tostring(arg)
        end
    end

    -- (2) resolve content to string
    for i = 1,#node.content do
        if node.content[i].is_node then
            node.content[i], err = Litua.recurse_node_to_string(node.content[i], depth + 1, hook_name)
            if err ~= nil then
                return node.content[i], err
            end
        else
            node.content[i] = tostring(node.content[i])
        end
    end

    -- (3) call hooks for this node
    local calls = { node.call, "" }
    for _, call in ipairs(calls) do
        local hooks = Litua.hooks[hook_name][call]
        if hooks ~= nil and hooks[1] ~= nil then
            local hook = hooks[1]
            Litua.log("transform", "ran " .. hook.src .. " for call '" .. node.call .. "'")

            local result
            result, err = hook.impl(node, depth, call)
            if err ~= nil then
                Litua.error(tostring(hook_name) .. " hook returned non-nil value as second value", {
                    ["context"] = tostring(hook_name) .. " hooks must return two values (node and error)",
                    ["expected"] = "error return value to be nil",
                    ["actual"] = "error return value is '" .. tostring(err) .. "'",
                    ["fix"] = "make hook return no error",
                    ["source"] = hook.src,
                })
                return nil, err
            end
            if type(result) ~= "string" then
                Litua.error(tostring(hook_name) .. " hook returned non-string value as first return value", {
                    ["context"] = tostring(hook_name) .. " hooks must return two values (string representation and error)",
                    ["expected"] = "string representation return value to be a string",
                    ["actual"] = "string representation return value is '" .. type(result) .. "'",
                    ["fix"] = "make hook return a string",
                    ["source"] = hook.src,
                })
                return "error", err
            end

            return result, nil
        end
    end

    return tostring(node), nil
end

--- Transformation function taking a root element `tree`,
--- invoking all hooks, and return a string representation
-- @param tree  the Litua.Node instance of the root
-- @return  error or string representation
Litua.transform = function (tree)
    local err, repr

    -- (0) run setup hooks
    Litua.log("transform", "run setup hooks")
    local hook = "setup"
    for i=1,#Litua.hooks[hook][""] do
        Litua.log("transform", "ran " .. Litua.hooks[hook][""][i].src)
        err = Litua.hooks[hook][""][i].impl()
        if err ~= nil then
            Litua.error(tostring(hook) .. " hook returned non-nil value", {
                ["expected"] = tostring(hook) .. " hooks must return nil",
                ["actual"] = "return value is '" .. tostring(err) .. "'",
                ["source"] = Litua.hooks[hook][""][i].src,
            })
        end
    end

    --[[
    -- debug the tree generated
    local function dump_tree(t, depth)
        if depth == nil then depth = 0 end
        local indent = ("  "):rep(depth)

        local args = ""
        for arg, value in ipairs(t.args) do
            args = args .. "  :" .. arg .. " !" .. type(arg)
        end

        local out = indent .. "{" .. t.call .. "}" .. args .. " ["
        for i, c in ipairs(t.content) do
            out = out .. type(c) .. " | "
        end
        out = out:sub(1, #out - 3) .. "]\n"

        local children_out = ""
        for _, c in ipairs(t.content) do
            if type(c) == "table" then
                children_out = children_out .. dump_tree(c, depth + 1)
            end
        end

        return out .. children_out
    end
    print(dump_tree(tree))
    ]]

    -- (1) take tree data and convert it into Node objects
    local root = Litua.tree_to_nodes(tree)

    -- root has a special string representation
    root.tostring = function (self)
        local out = ""
        for i = 1,#self.content do
            out = out .. tostring(self.content[i])
        end
        return out
    end

    local function middle()
        -- (2) read-new-node hooks
        Litua.log("transform", "run read-new-node hooks")
        err = Litua.recurse_reading(root, 0, "read-new-node")
        if err ~= nil then
            return err
        end

        -- (3) modify-node hooks
        Litua.log("transform", "run modify-node hooks")
        root, err = Litua.recurse_modify_node(root, 0, "modify-node")
        if err ~= nil then
            return err
        end

        -- (4) read-modified-node hooks
        Litua.log("transform", "run read-modified-node hooks")
        err = Litua.recurse_reading(root, 0, "read-modified-node")
        if err ~= nil then
            return err
        end

        -- (5) convert-node-to-string hooks
        Litua.log("transform", "run convert-node-to-string hooks")
        repr, err = Litua.recurse_node_to_string(root, 0, "convert-node-to-string")
        if err ~= nil then
            return err
        end

        -- (6) modify-final-string hooks
        Litua.log("transform", "run modify-final-string hooks")
        for i=1,#Litua.hooks["modify-final-string"][""] do
            repr, err = Litua.hooks["modify-final-string"][""][i].impl(repr)
            if err ~= nil then
                return err
            end
        end
    end
    local middle_success, err = pcall(middle)

    -- (7) run teardown hooks
    hook = "teardown"
    for i=1,#Litua.hooks[hook][""] do
        Litua.log("transform", "ran " .. Litua.hooks[hook][""][i].src)
        local teardown_err = Litua.hooks[hook][""][i].impl()
        if teardown_err ~= nil then
            Litua.error(tostring(hook) .. " hook returned non-nil value", {
                ["expected"] = tostring(hook) .. " hooks must return nil",
                ["actual"] = "return value is '" .. tostring(teardown_err) .. "'",
                ["source"] = Litua.hooks[hook][""][i].src,
            })
        end
    end
    if not middle_success then
        return err
    end
    if err ~= nil then
        return err
    end

    return repr
end