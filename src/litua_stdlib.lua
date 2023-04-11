--- Generate an error object and invoke error() to terminate with an error
-- @tparam string errmsg  string to explain issue
-- @tparam table info  more detailed context information like expected and actual value in a table
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

--- Generate a message and log it to stdout for human consumption
-- @tparam string component  the component where this issue occurs
-- @tparam string msg  the string explaining the issue
Litua.log = function (component, msg)
    print("LOG[" .. component .. "]: " .. msg)
end

--- Represent a table as a string without traversing recursively.
--- It calls `tostring()` on each key and value to retrieve its string representation.
-- @tparam table tbl  the table to represent
Litua.represent_table = function (tbl)
    local repr = "{ "
    for key, value in pairs(tbl) do
        repr = repr .. "[" .. tostring(key) .. "] = " .. tostring(value) .. ", "
    end
    return repr:sub(#repr - 1) .. " }"
end

--- Iterate over values of a table, call `tostring`, and concatenate its output
-- @tparam table tbl  the table to iterate over (index 1, index 2, … until its value is nil)
-- @treturn string string concatenation of table values
Litua.concat_table_values = function (tbl)
    local concat = ""
    for c = 1,#tbl do
      concat = concat .. tostring(tbl[c])
    end
    return concat
end

--- Replace a single-quote with a backslash-single-quote sequence
-- @param text  the text to replace
-- @treturn string string representation
Litua.escape_single_quote_text = function (text)
    local replaced = tostring(text):gsub("'", "\\'")
    return replaced
end

--- Format a string provided as argument by replacing strings like '%1', '%2', … with the respective argument
--- provided. Only 9 arguments are supported, so '%1' up to '%9'.
-- @tparam string format_string  formatting string to insert values into
-- @treturn string  format_string with replaced values
Litua.format = function (format_string, ...)
    -- determine number of arguments
    local count_args = select('#', ...)
    if count_args > 9 then
        Litua.error("formatting with format string '" .. Litua.escape_single_quote_text(format_string) ..
            "' is provided " .. count_args .. " arguments, but only 9 are supported"
        )
        count_args = 9
    end

    -- collect arguments
    local func_args = {}
    for i=1,count_args do
        local val = select(i, ...)
        local value = tostring(val)  -- tostring(…) for number/function/CFunction/userdata
        if val == nil then
            value = "nil"
        elseif type(val) == "string" then
            value = "'" .. Litua.escape_single_quote_text(value) .. "'"
        elseif type(val) == "table" then
            value = Litua.represent_table(val)
        end
        func_args[tostring(i)] = value
    end

    -- replace placeholders
    return format_string:gsub("%%(%d)", func_args)
end
