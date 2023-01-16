Litua.global.enum = 0

Litua.add_hook(Litua.Filter.by_call("item"), "node-to-string", function (node)
  Litua.global.enum = Litua.global.enum + 1

  return "(" .. tostring(Litua.global.enum) .. ") "
end)
