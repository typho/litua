local io = require("io")

Litua.on_setup(function ()
    Litua.global.file_code = io.open("src.code", "w")
    Litua.global.file_docu = io.open("src.docu", "w")
end)

Litua.read_new_node("code", function (node)
    for i=1,#node.content do
        Litua.global.file_code:write(tostring(node.content[i]))
    end
end)

Litua.read_new_node("docu", function (node)
    for i=1,#node.content do
        Litua.global.file_docu:write(tostring(node.content[i]))
    end
end)

Litua.on_teardown(function ()
    Litua.global.file_code:close()
    Litua.global.file_docu:close()
end)
