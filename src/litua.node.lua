Litua.Node = {}

local identity_string = function (node)
    local args_string = ""
    local content_string = ""

    for argkey, argvalues in pairs(node.args) do
        if argkey:match("=") == nil then
            args_string = args_string .. "[" .. argkey .. "="
            for _, argvalue in pairs(argvalues) do
                args_string = args_string .. tostring(argvalue)
            end
            args_string = args_string .. "]"
        end
    end

    if #node.content > 0 then
        for i, value in ipairs(node.content) do
            content_string = content_string .. tostring(value)
        end
    end

    if args_string == "" and content_string == "" then
        return "{" .. node.call .. "}"
    elseif args_string == "" then
        return "{" .. node.call .. " " .. content_string .. "}"
    elseif content_string == "" then
        return "{" .. node.call .. args_string .. "}"
    else
        return "{" .. node.call .. args_string .. "\n" .. content_string .. "}"
    end
end

Litua.Node.Api = { "call", "args", "content", "copy", "is_node", "tostring" }

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
            for _, argvalue in pairs(argvalues) do
                if argvalue.is_node then
                    table.insert(new_args[argkey], argvalue.copy())
                else
                    table.insert(new_args[argkey], tostring(argvalue))
                end
            end
        end

        local new_content = {}
        for _, value in pairs(self.content) do
            if value.is_node then
                table.insert(new_content, value.copy())
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
    for _, key in pairs(Litua.Node.Api) do
        if index == key then
            return rawget(self, index)
        end
    end
    Litua.error("cannot fetch property '" .. tostring(index) .. "' of node '" .. self.call .. "'")
end

Litua.Node.__newindex = function (self, index, value)
    for _, key in pairs(Litua.Node.Api) do
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
