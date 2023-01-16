local string = require("string")

-- FUNCTION given a nested structure of tables, return a nested structure of Litua.Node tables
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

-- FUNCTION recurse into elements of the tree and provide nodes
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

Litua.recurse_modify_node = function (node, depth, hook_name)
    local err

    local calls = { node.call, "" }
    for _, call in ipairs(calls) do
        if Litua.hooks[hook_name][call] ~= nil then
            for i, hook in ipairs(Litua.hooks[hook_name][call]) do
                Litua.log("transform", "ran " .. Litua.hooks[hook_name][call][i].src .. " for call '" .. node.call .. "'")
                node, err = hook.impl(node, depth, call)
                if node == nil or (not node.is_node) then
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

    return node, nil
end

Litua.recurse_node_to_string = function (node, depth, hook_name)
    -- NOTE this implementation needs to resolve its children first,
    --      then generate its own string representation
    local err

    local apply_hooks_to_node = function (this_node)
        if this_node.call == "left-curly-brace" then return "{", nil end
        if this_node.call == "right-curly-brace" then return "}", nil end

        -- NOTE: hooks for "" should always be called
        local calls = { this_node.call, "" }
        for _, call in ipairs(calls) do
            local hooks = Litua.hooks[hook_name][call]
            if hooks ~= nil and hooks[1] ~= nil then
                local hook = hooks[1]
                Litua.log("transform", "ran " .. hook.src .. " for call '" .. this_node.call .. "'")

                local result_string, err2 = hook.impl(this_node, depth)
                if err2 ~= nil then
                    Litua.error(tostring(hook_name) .. " hook returned non-nil value as second value", {
                        ["context"] = tostring(hook_name) .. " hooks must return two values (node and error)",
                        ["expected"] = "error return value to be nil",
                        ["actual"] = "error return value is '" .. tostring(err2) .. "'",
                        ["fix"] = "make hook return no error",
                        ["source"] = hook.src,
                    })
                    return nil, err
                end
                if type(result_string) ~= "string" then
                    Litua.error(tostring(hook_name) .. " hook returned non-string value as first return value", {
                        ["context"] = tostring(hook_name) .. " hooks must return two values (string representation and error)",
                        ["expected"] = "string representation return value to be a string",
                        ["actual"] = "string representation return value is '" .. type(result_string) .. "'",
                        ["fix"] = "make hook return a string",
                        ["source"] = hook.src,
                    })
                    return "error", err2
                end
    
                return result_string, nil
            end
        end
        return tostring(this_node), nil
    end

    -- ASSUMPTION: type(node.call) == "string"
    local args_as_strings = {}
    for k, arg in ipairs(node.args) do
        if arg.is_node then
            args_as_strings[k], err = apply_hooks_to_node(arg)
            if err ~= nil then
                return args_as_strings[k], err
            end
        else
            args_as_strings[k] = tostring(arg)
        end
    end
    node.args = args_as_strings

    local content_as_strings = {}
    for i = 1,#node.content do
        if node.content[i].is_node then
            content_as_strings[i], err = apply_hooks_to_node(node.content[i])
            if err ~= nil then
                return content_as_strings[i], err
            end
        else
            content_as_strings[i] = tostring(node.content[i])
        end
    end
    node.content = content_as_strings

    return tostring(node), nil
end

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

    -- (1) take tree data and convert it into Node objects
    --[[
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

    local root = Litua.tree_to_nodes(tree)

    -- root has a special string representation
    root.tostring = function (self)
        local out = ""
        for i = 1,#self.content do
            out = out .. tostring(self.content[i])
        end
        return out
    end

    err = (function()
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
    end)()

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
    if err ~= nil then
        return err
    end

    return repr
end