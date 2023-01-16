Litua.convert_node_to_string("let", function (node)
    for arg, val in pairs(node.args) do
        local content = ""
        for i=1,#val do
            content = content .. tostring(val[i])
        end

        Litua.convert_node_to_string(arg, function (_) return content end)
    end
    return ""
end)