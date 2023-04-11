local io = require("io")

Litua.on_setup(function ()
    Litua.global.file_code = io.open("code.txt", "w")
    Litua.global.file_docu = io.open("docu.txt", "w")
end)

Litua.read_new_node("code", function (node)
    for i=1,#node.content do
        Litua.global.file_code:write(node:totext())

        Litua.global.file_docu:write("```\n")
        Litua.global.file_docu:write(node:totext())
        Litua.global.file_docu:write("\n```\n")
    end
end)

Litua.read_new_node("docu", function (node)
    for i=1,#node.content do
        Litua.global.file_docu:write(node:totext())
    end
end)

Litua.on_teardown(function ()
    Litua.global.file_code:close()
    Litua.global.file_docu:close()
end)
