local string = require("string")

Litua = {
    ["hooks"] = {
        ["pre-debug"] = {},
        ["node-to-string"] = {},
        ["modify-node"] = {},
        ["post-debug"] = {},
    },
    ["global"] = {},
}

Litua.add_hook = function (filter, hook_id, hook_function)
    if not filter.is_filter then
        Litua.error("adding a hook", "Litua.Filter as first argument", type(filter), "provide a filter like Litua.Filter.any")
    end
    if Litua.hooks[hook_id] == nil then
        Litua.error("adding a hook", "a known hook identifier", "'" .. tostring(hook_id) .. "'", "provide a hook id like 'pre-debug'")
    end
    if type(hook_function) ~= "function" then
        Litua.error("adding a hook", "function as hook", "'" .. type(hook_function) .. "' as hook", "provide a function to call")
    end

    table.insert(Litua.hooks[hook_id], { ["filter"] = filter, ["func"] = hook_function })
end

-- transform will be inserted by litua_transform.lua
