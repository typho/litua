Litua.error = function (errmsg, info)
    local out = "ERROR: " .. tostring(errmsg) .. "\n"
    if type(info) == "table" then
        if info.context ~= nil then
            out = out .. "   CONTEXT: " .. tostring(info.context) .. "\n"
        end
        if info.expected ~= nil then
            out = out .. "  EXPECTED: " .. tostring(info.expected) .. "\n"
        end
        if info.actual ~= nil then
            out = out .. "    ACTUAL: " .. tostring(info.actual) .. "\n"
        end
        if info.fix ~= nil then
            out = out .. "       FIX: " .. tostring(info.fix) .. "\n"
        end
        if info.source ~= nil then
            out = out .. "    SOURCE: " .. tostring(info.source) .. "\n"
        end
    end
    error(out)
end

Litua.log = function (component, msg)
    print("LOG[" .. component .. "]: " .. msg)
end

Litua.print_table = function (tbl)
    print("<table>")
    for k, v in pairs(tbl) do
        print("  <" .. tostring(k) .. ">" .. tostring(v) .. "</" .. tostring(k) .. ">")
    end
    print("</table>")
end
