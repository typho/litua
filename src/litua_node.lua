
--- Data structure to represent nodes of the tree
Litua.Node = {}

--- Identity string representation of a node
-- Considers the given node and represents it in litua input syntax.
-- It uses keys starting with "=" to recover its original representation.
-- e.g. value of key "=whitespace" stores the whitespace
-- where any whitespace would have been accepted
-- @param node  A Litua.Node to represent
-- @return  node's string representation
Litua.Node.identity_representation = function (node)
    -- read regular arguments and reconstruct the argument string
    local args_string = ""

    local args_keys = {}
    for argkey, _ in pairs(node.args) do
        if argkey:match("=") == nil then
            table.insert(args_keys, argkey)
        end
    end
    -- NOTE: we want to represent them in a sorted manner to get some
    --       deterministic behavior
    table.sort(args_keys)

    for i = 1,#args_keys do
        local argkey = tostring(args_keys[i])
        local argvalues = node.args[argkey]

        args_string = args_string .. "[" .. argkey .. "=" .. Litua.concat_table_values(argvalues) .. "]"
    end

    -- read special arguments
    local whitespace = ""
    local whitespace_after = ""
    for argkey, argvalues in pairs(node.args) do
        if argkey == "=whitespace" then
            whitespace = Litua.concat_table_values(argvalues)
        elseif argkey == "=whitespace-after" then
            whitespace_after = Litua.concat_table_values(argvalues)
        end
    end

    -- reconstruct content string
    local content_string = Litua.concat_table_values(node.content)

    -- NOTE: if =whitespace is not set, but there is some content_string,
    --       we still need some separating whitespace, U+0020 SPACE per default
    if whitespace == "" and #content_string > 0 then
        whitespace = " "
    end

    -- reconstruct entire function
    if node.call:match("<+") ~= nil then
        local length = #node.call
        return "{" .. node.call .. whitespace .. content_string .. whitespace_after .. (">"):rep(length) .. "}"
    else
        return "{" .. node.call .. args_string .. whitespace .. content_string .. whitespace_after .. "}"
    end
end

--- Text-only string representation of a node
-- Considers the given node and represents only its text content.
-- It discards any attributes and handles raw strings like regular strings.
-- Since the call name is not represented, also whitespace information is discarded.
-- @param node  A Litua.Node to represent
-- @return  node's string representation
Litua.Node.text_only_representation = function (node)
    if node.call:match("<+") ~= nil then
        -- Oh, a raw string. Then just return its only content item of type "string"
        return node.content[1]
    end

    -- reconstruct content string
    local content_string = ""
    for i=1,#node.content do
        if type(node.content[i]) == "table" then
            -- A content node which is a table? Then we need to get its text repr recursively
            content_string = content_string .. Litua.Node.text_only_representation(node.content[i])
        elseif type(node.content[i]) == "string" then
            -- A content node which is a string itself? Then use it.
            content_string = content_string .. tostring(node.content[i])
        end
    end

    return content_string
end

--- The set of admissible API call
Litua.Node.Api = { "call", "args", "content", "copy", "is_node", "tostring", "totext" }

--- Constructor for a new node
-- It takes the `call` name, arguments `args`, and a table `content`.
-- Here, `content` can be Nodes or strings themselves.
-- @param call  name of the call
-- @param args  the table with key-value associations defining arguments
-- @param content  the content of this element (enumerated table with Litua.Node or string instances)
-- @return  a Litua.Node instance
Litua.Node.init = function (call, args, content)
    local node = {
        ["call"] = tostring(call),
        ["args"] = args,
        ["content"] = content,
    }

    node.copy = function (self)
        local new_args = {}
        for argkey, argvalues in pairs(self.args) do
            new_args[argkey] = {}
            for _, argvalue in ipairs(argvalues) do
                if argvalue.is_node then
                    table.insert(new_args[argkey], argvalue:copy())
                else
                    table.insert(new_args[argkey], tostring(argvalue))
                end
            end
        end

        local new_content = {}
        for _, value in ipairs(self.content) do
            if value.is_node then
                table.insert(new_content, value:copy())
            else
                table.insert(new_content, tostring(value))
            end
        end

        return Litua.Node.init(call, new_args, new_content)
    end

    node.is_node = true
    node.totext = Litua.Node.text_only_representation
    return setmetatable(node, Litua.Node)
end

Litua.Node.__name = function (self, name)
    return self[name]
end

Litua.Node.__index = function (self, index)
    for _, key in ipairs(Litua.Node.Api) do
        if index == key then
            return rawget(self, index)
        end
    end
    Litua.error("cannot fetch property '" .. tostring(index) .. "' of node '" .. self.call .. "'")
end

Litua.Node.__newindex = function (self, index, value)
    for _, key in ipairs(Litua.Node.Api) do
        if index == key then
            rawset(self, index, value)
            return nil
        end
    end
    Litua.error("cannot modify property '" .. tostring(index) .. "' of node '" .. self.call .. "'")
end

Litua.Node.__tostring = function (self)
    if type(rawget(self, 'tostring')) == "function" then
        return rawget(self, 'tostring')(self)
    end
    return Litua.Node.identity_representation(self)
end
