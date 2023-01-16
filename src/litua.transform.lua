local string = require("string")

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


Litua.recurse_predebug = function (node, depth)
    local err

    for i, hook in pairs(Litua.hooks["pre-debug"]) do
        if hook.filter(node.call) then
            err = hook.func(node.call, depth, #node.args, #node.content)
            if err ~= nil then
                Litua.error("pre-debug hook", "nil return value", err, "make hook return non-error")
                return err
            end
        end
    end

    for argkey, argvalues in pairs(node.args) do
        for i, argvalue in pairs(argvalues) do
            if argvalue.is_node then
                err = Litua.recurse_predebug(argvalue, depth + 1)
                if err ~= nil then
                    Litua.error("pre-debug hook", "nil return value", err, "make hook return non-error")
                    return err
                end
            end
        end
    end

    for _, value in pairs(node.content) do
        if value.is_node then
            err = Litua.recurse_predebug(value, depth + 1)
            if err ~= nil then
                Litua.error("pre-debug hook", "nil return value", err, "make hook return non-error")
                return err
            end
        end
    end
end

Litua.recurse_modify_node = function (node, depth)
    local err

    for i, hook in pairs(Litua.hooks["modify-node"]) do
        if hook.filter(node.call) then
            node, err = hook.func(node, depth)
            if err ~= nil then
                Litua.error("modify-node hook", "nil return value", err, "make hook return non-error")
                return nil, err
            end
        end
    end

    for argkey, argvalues in pairs(node.args) do
        for i, argvalue in pairs(argvalues) do
            if argvalue.is_node then
                node.args[argkey][i], err = Litua.recurse_modify_node(argvalue, depth + 1)
                if err ~= nil then
                    Litua.error("modify-node hook", "nil return value", err, "make hook return non-error")
                    return nil, err
                end
            end
        end
    end

    for i, value in pairs(node.content) do
        if value.is_node then
            node.content[i], err = Litua.recurse_modify_node(value, depth + 1)
            if err ~= nil then
                Litua.error("modify-node hook", "nil return value", err, "make hook return non-error")
                return nil, err
            end
        end
    end

    return node, nil
end

Litua.recurse_node_to_string = function (node, depth)
    -- NOTE this implementation needs to resolve its children first,
    --      then generate its own string representation

    local apply_hook = function (n)
        for _, hook in pairs(Litua.hooks["node-to-string"]) do
            if hook.filter(n.call) then
                local result_string, err = hook.func(n, depth)
                if err ~= nil then
                    Litua.error("node-to-string hook", "nil return value", err, "make hook return non-error")
                    return nil, err
                end
                if type(result_string) ~= "string" then
                    Litua.error(
                        "node-to-string hook " .. tostring(hook),
                        "string return value",
                        type(result_string),
                        "return a string for node-to-string hook"
                    )
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

Litua.postdebug = function (complete_string)
    local err

    for i, hook in pairs(Litua.hooks["post-debug"]) do
        if hook.filter(complete_string) then
            err = hook.func(complete_string)
            if err ~= nil then
                Litua.error("post-debug hook", "string return value", err, "make hook return string")
                return err
            end
        end
    end
end

Litua.transform = function (tree)
    local err, repr

    -- (0) take tree data and convert it into Node objects
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
    --print(dump_tree(tree))

    local root = Litua.tree_to_nodes(tree)

    -- root has a special string representation
    root.tostring = function (self)
        local out = ""
        for i = 1,#self.content do
            out = out .. tostring(self.content[i])
        end
        return out
    end

    -- (1) pre-debug hooks
    err = Litua.recurse_predebug(root, 0)
    if err ~= nil then
        return err
    end

    -- (2) node-manipulation hooks
    root, err = Litua.recurse_modify_node(root, 0)
    if err ~= nil then
        return err
    end

    -- (3) node-to-string hooks
    repr, err = Litua.recurse_node_to_string(root, 0)
    if err ~= nil then
        return err
    end

    -- (4) post-debug hooks
    err = Litua.postdebug(repr)
    if err ~= nil then
        return err
    end

    return repr
end