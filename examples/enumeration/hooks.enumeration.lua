Litua.on_setup(function ()
    Litua.global.enum = 0
end)

Litua.convert_node_to_string("item", function (node)
    Litua.global.enum = Litua.global.enum + 1

    return "(" .. tostring(Litua.global.enum) .. ")"
end)
