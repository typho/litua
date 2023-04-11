
--- Data structure to represent nodes of the tree
Litua.Node = {}

--- Identity string representation of a node
-- Considers the given node and represents it in litua input syntax.
-- It uses the "=whitespace" key to recover the original whitespace
-- character where any whitespace would have been accepted
-- @param node  A Litua.Node to represent
-- @return  node's string representation
local identity_string = function (node)
    local args_string = ""
    local content_string = ""

    local whitespace = " "
    local whitespace_after = " "
    for argkey, argvalues in pairs(node.args) do
        if argkey:match("=") == nil then
            args_string = args_string .. "[" .. argkey .. "="
            for _, argvalue in ipairs(argvalues) do
                args_string = args_string .. tostring(argvalue)
            end
            args_string = args_string .. "]"
        end
        if argkey == "=whitespace" then
            whitespace = tostring(argvalues[1])
        end
        if argkey == "=whitespace-after" then
            whitespace_after = tostring(argvalues[1])
        end
    end

    if #node.content > 0 then
        for i, value in ipairs(node.content) do
            content_string = content_string .. tostring(value)
        end
    end

    if node.call:match("<+") ~= nil then
        local length = #node.call
        return "{" .. node.call .. whitespace .. node.content[1] .. whitespace_after .. (">"):rep(length) .. "}"
    elseif args_string == "" and content_string == "" then
        return "{" .. node.call .. "}"
    elseif args_string == "" then
        return "{" .. node.call .. whitespace .. content_string .. "}"
    elseif content_string == "" then
        return "{" .. node.call .. args_string .. "}"
    else
        return "{" .. node.call .. args_string .. "\n" .. content_string .. "}"
    end
end

--- The set of admissible API call
Litua.Node.Api = { "call", "args", "content", "copy", "is_node", "tostring" }

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
    return identity_string(self)
end
