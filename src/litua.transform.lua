local string = require("string")

-- FUNCTION given a nested structure of tables, return a nested structure of Litua.Node tables
Litua.tree_to_nodes = function (tree)
    local new_call = tostring(tree.call)
    local new_args = {}
    local new_content = {}

    for argkey, argvalues in pairs(tree.args) do
        local new_arg = {}
        for _, argvalue in pairs(argvalues) do
            if type(argvalue) == "table" then
                table.insert(new_arg, Litua.tree_to_nodes(argvalue))
            else
                table.insert(new_arg, tostring(argvalue))
            end
        end
        new_args[argkey] = new_arg
    end

    for _, value in pairs(tree.content) do
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
    for _, call in pairs(calls) do
        if Litua.hooks[hook_name][call] ~= nil then
            for i, hook in pairs(Litua.hooks[hook_name][call]) do
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

    for argkey, argvalues in pairs(node.args) do
        for i, argvalue in pairs(argvalues) do
            if argvalue.is_node then
                err = Litua.recurse_reading(argvalue, depth + 1, hook_name)
                if err ~= nil then
                    return err
                end
            end
        end
    end

    for _, value in pairs(node.content) do
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
    for _, call in pairs(calls) do
        if Litua.hooks[hook_name][call] ~= nil then
            for i, hook in pairs(Litua.hooks[hook_name][call]) do
                node, err = hook.impl(node, depth, call)
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

    for argkey, argvalues in pairs(node.args) do
        for i, argvalue in pairs(argvalues) do
            if argvalue.is_node then
                node.args[argkey][i], err = Litua.recurse_modify_node(argvalue, depth + 1, hook_name)
                if err ~= nil then
                    return nil, err
                end
            end
        end
    end

    for i, value in pairs(node.content) do
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

    local apply_hook = function (n)
        local calls = { node.call, "" }
        for _, call in pairs(calls) do
            if Litua.hooks[hook_name][call] ~= nil and Litua.hooks[hook_name][call][1] ~= nil then
                local hook = Litua.hooks[hook_name][call][1]

                local result_string, err = hook.impl(n, depth)
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
                if type(result_string) ~= "string" then
                    Litua.error(tostring(hook_name) .. " hook returned non-string value as first return value", {
                        ["context"] = tostring(hook_name) .. " hooks must return two values (string representation and error)",
                        ["expected"] = "string representation return value to be a string",
                        ["actual"] = "string representation return value is '" .. type(result_string) .. "'",
                        ["fix"] = "make hook return a string",
                        ["source"] = hook.src,
                    })
                    return nil, err
                end
    
                return result_string, nil
            end
        end
        return tostring(node), nil
    end

    -- ASSUMPTION: type(node.call) == "string"
    local args_as_strings = {}
    for k, arg in pairs(node.args) do
        if arg.is_node then
            args_as_strings[k] = apply_hook(arg)
        else
            args_as_strings[k] = tostring(arg)
        end
    end
    node.args = args_as_strings

    local content_as_strings = {}
    for i = 1,#node.content do
        if node.content[i].is_node then
            args_as_strings[i] = apply_hook(node.content[i])
        else
            args_as_strings[i] = tostring(node.content[i])
        end
    end
    node.content = args_as_strings

    return tostring(node), nil
end

Litua.transform = function (tree)
    local err, repr

    -- (0) run setup hooks
    local hook = "setup"
    for i=1,#Litua.hooks[hook][""] do
        print("INFO: ran " .. Litua.hooks[hook][""][i].src)
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
        for arg, value in pairs(t.args) do
            args = args .. "  :" .. arg .. " !" .. type(arg)
        end

        local out = indent .. "{" .. t.call .. "}" .. args .. " ["
        for i, c in pairs(t.content) do
            out = out .. type(c) .. " | "
        end
        out = out:sub(1, #out - 3) .. "]\n"

        local children_out = ""
        for _, c in pairs(t.content) do
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

    -- (2) modify nodes
    err = Litua.recurse_reading(root, 0, "read-new-node")
    if err ~= nil then
        return err
    end

    root, err = Litua.recurse_modify_node(root, 0, "modify-node")
    if err ~= nil then
        return err
    end

    err = Litua.recurse_reading(root, 0, "read-modified-node")
    if err ~= nil then
        return err
    end

    -- (3) convert nodes to strings
    repr, err = Litua.recurse_node_to_string(root, 0, "convert-node-to-string")
    if err ~= nil then
        return err
    end

    for i=1,#Litua.hooks["modify-final-string"] do
        repr, err = Litua.hooks["modify-final-string"][i].impl(repr)
        if err ~= nil then
            return err
        end
    end

    -- (6) run teardown hooks
    -- TODO: this hook must run even if previous hooks failed
    hook = "teardown"
    for i=1,#Litua.hooks[hook][""] do
        print("INFO: ran " .. Litua.hooks[hook][""][i].src)
        err = Litua.hooks[hook][""][i].impl()
        if err ~= nil then
            Litua.error(tostring(hook) .. " hook returned non-nil value", {
                ["expected"] = tostring(hook) .. " hooks must return nil",
                ["actual"] = "return value is '" .. tostring(err) .. "'",
                ["source"] = Litua.hooks[hook][""][i].src,
            })
        end
    end

    return repr
end