local function is_valid_xml_element_name_or_attribute(name)
    -- implementation follows the XML 1.0 standard
    -- https://www.w3.org/TR/2008/REC-xml-20081126/#NT-Name
    local is_name_start_char = function (codepoint)
        -- NameStartChar ::= ":" | [A-Z] | "_" | [a-z] | [#xC0-#xD6] | [#xD8-#xF6]
        --                 | [#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF]
        --                 | [#x200C-#x200D] | [#x2070-#x218F] | [#x2C00-#x2FEF]
        --                 | [#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD]
        --                 | [#x10000-#xEFFFF]
        if codepoint == 58 or codepoint == 95 then return true end
        local admissible_ranges = {
            { 65, 90 }, { 97, 122 }, { 0xC0, 0xD6 }, { 0xD8, 0xF6 },
            { 0xF8, 0x2FF }, { 0x370, 0x37D }, { 0x37F, 0x1FFF },
            { 0x200C, 0x200D }, { 0x2070, 0x218F }, { 0x2C00, 0x2FEF },
            { 0x3001, 0xD7FF }, { 0xF900, 0xFDCF }, { 0xFDF0, 0xFFFD },
            { 0x10000, 0xEFFFF }
        }

        for _, range in ipairs(admissible_ranges) do
            local from = range[1]
            local to = range[2]
            if from <= codepoint and codepoint <= to then
                return true
            end
        end

        return false
    end

    local is_name_char = function (codepoint)
        -- NameChar      ::= NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
        if codepoint == 45 or codepoint == 46 or codepoint == 0xB7 then return true end
        if is_name_start_char(codepoint) then return true end
        local admissible_ranges = {
            { 48, 57 }, { 0x300, 0x36F }, { 0x203F, 0x2040 }
        }

        for _, range in ipairs(admissible_ranges) do
            local from = range[1]
            local to = range[2]
            if from <= codepoint and codepoint <= to then
                return true
            end
        end

        return false
    end


    for i, chr in utf8.codes(name) do
        if i == 1 and not is_name_start_char(chr) then
            return false
        elseif i > 1 and not is_name_char(chr) then
            return false
        end
    end

    return true
end

local function escape_text_for_xml(str)
    return str:gsub("&", "&amp;"):gsub("<", "&lt;"):gsub(">", "&gt;"):gsub("'", "&apos;"):gsub('"', "&quot;")
end

local SUB_ELEMENT_START = "\x02"
local SUB_ELEMENT_END = "\x03"
local SUB_ATTR_START = "\x0F"
local SUB_ATTR_END = "\x0E"

local node_to_xml = function (node)
    if not is_valid_xml_element_name_or_attribute(node.call) then
        Litua.error("Call '" .. tostring(node.call) .. "' is not XML-serializable (and likely neither HTML5-serializable)", {
            ["expected"] = "a valid XML element name (sorry, this implementation implements XML, not HTML5)",
            ["fix"] = "use call which can be serialized to HTML5"
        })
    end

    -- attach element name
    local out = SUB_ELEMENT_START .. node.call

    -- attach attributes
    local attributes = ""
    for attr, values in pairs(node.args) do
        local value = ""
        for i=1,#values do
            value = value .. tostring(values[i])
        end

        -- NOTE: skip attributes like "=whitespace" which are provided
        --       as special attributes by the lexer
        if attr:find("^=") == nil then
            if not is_valid_xml_element_name_or_attribute(attr) then
                Litua.error("Attribute '" .. tostring(attr) .. "' is not XML-serializable (and likely neither HTML5-serializable)", {
                    ["expected"] = "a valid XML attribute name (sorry, this implementation implements XML, not HTML5)",
                    ["fix"] = "use attribute which can be serialized to HTML5"
                })
            end
      
            attributes = attributes .. " " .. attr .. "=" .. SUB_ATTR_START .. escape_text_for_xml(value) .. SUB_ATTR_END
        end
    end
    if #node.content == 0 then
        -- empty element
        return out .. attributes .. " /" .. SUB_ELEMENT_END, nil
    else
        out = out .. attributes .. SUB_ELEMENT_END
        if node.args["=whitespace"] ~= nil and node.args["=whitespace"][1] ~= " " then
            out = out .. tostring(node.args["=whitespace"][1])
        end
    end

    -- attach content
    for _, content in ipairs(node.content) do
        out = out .. escape_text_for_xml(tostring(content))
    end

    -- attach closing xml element
    return out .. SUB_ELEMENT_START .. "/" .. node.call .. SUB_ELEMENT_END, nil
end

Litua.convert_node_to_string("", node_to_xml)
Litua.convert_node_to_string("document", function (node)
    local nonempty_content_nodes = {}

    for _, content in ipairs(node.content) do
        if content:find("[\x09\x0A\x0B\x0C\x0D\x20\x85\xA0]*$") == 1 then
            table.insert(nonempty_content_nodes, content)
        end
    end

    if #nonempty_content_nodes == 1 then
        return node.content[1]:gsub(SUB_ELEMENT_START, "<"):gsub(SUB_ELEMENT_END, ">"):gsub(SUB_ATTR_START, '"'):gsub(SUB_ATTR_END, '"'), nil
    else
        Litua.error("this document cannot be serialized to HTML5", {
            ["expected"] = "document with exactly one top-level call (and no text nodes)",
            ["actual"] = "found " .. tostring(#nonempty_content_nodes) .. " top-level nodes"
        })
    end
end)
