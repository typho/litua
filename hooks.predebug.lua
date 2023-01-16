local string = require('string')

--[[Litua.add_hook(Litua.Filter.any, "pre-debug",
    function (call, depth)
        print(("  "):rep(depth * 2), call)
    end
)]]