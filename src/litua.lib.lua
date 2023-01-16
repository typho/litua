Litua.error = function (action, expected, unexpected, fix)
    error("Litua lua error: while " .. tostring(action) .. " I expected " .. tostring(expected) .. " but got " .. tostring(unexpected) .. ". Please " .. tostring(fix))
end

Litua.log = function (component, msg)
    print("LOG[" .. component .. "]: " .. msg)
end

Litua.validate_call = function (call)
    if call:match("=") ~= nil then
        return Litua.error("a valid call name", "name with '='", "use a name without '='")
    end
    return nil
end

Litua.print_sequence_table = function (tbl)
    local i = 1
    while true do
        if tbl[i] == nil then
            break
        end
        print(tostring(i) .. " = " .. tostring(tbl[i]))
        i = i + 1
    end
end

Litua.print_table = function (tbl)
    print("<table>")
    for k, v in pairs(tbl) do
        print("<" .. tostring(k) .. ">" .. tostring(v) .. "</" .. tostring(k) .. ">")
    end
    print("</table>")
end
