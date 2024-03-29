--- Is the given `name` a valid XML element name and valid attribute?
-- We assume `name` is an UTF-8 string. Now we determine whether `name`
-- can be accepted as valid XML element as well as valid XML attribute.
-- The implementation follows the XML 1.0 standard even though this module
-- should illustrate HTML5. Why? Just because I did not care to read
-- the HTML5 standard when reading this module. The definition in XML
-- can be found here:
--   https://www.w3.org/TR/2008/REC-xml-20081126/#NT-Name
-- @param name  the UTF-8 string to validate
-- @return is valid (true) or invalid (false)
local function is_valid_xml_element_name_and_attribute(name)
    -- does the codepoint of the first Unicode scalar `codepoint`
    -- satisfy the criteria for an XML element?
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

    -- does the codepoint of the second-or-later Unicode scalar `codepoint`
    -- satisfy the criteria for an XML element?
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

    -- evaluation
    for i, chr in utf8.codes(name) do
        if i == 1 and not is_name_start_char(chr) then
            return false
        elseif i > 1 and not is_name_char(chr) then
            return false
        end
    end

    return true
end

--- Escape string `str` for use in in XML text nodes and XML attribute values
-- Let `str` be an UTF-8 string. Escape the content to be used for XML text nodes
-- and XML attribute values.
-- @param str  UTF-8 string to escape
-- @return  escaped string
local function escape_text_for_xml(str)
    return str:gsub("&", "&amp;"):gsub("<", "&lt;"):gsub(">", "&gt;"):gsub("'", "&apos;"):gsub('"', "&quot;")
end

-- substitution characters
local SUB_ELEMENT_START = "\x02"
local SUB_ELEMENT_END = "\x03"
local SUB_ATTR_START = "\x0F"
local SUB_ATTR_END = "\x0E"

--- Hook function to represent `node` as string
-- Takes a Litua node and converts it into a string. The node is not represented
-- with the XML literals “<”, “>”, and “"” but with the substitution characters.
-- Thus the output is neither XML nor HTML5. But a simple replacement of the final
-- string will lead to it. This function assumes that the text nodes in `node`
-- do not contain the substitution characters itself.
-- @param node  A Litua.Node table to represent
-- @return  string representation
local node_to_xml = function (node)
    if not is_valid_xml_element_name_and_attribute(node.call) then
        Litua.error("Call '" .. tostring(node.call) .. "' is not XML-serializable (and likely neither HTML5-serializable)", {
            ["expected"] = "a valid XML element name (sorry, this implementation implements XML, not HTML5)",
            ["fix"] = "use call which can be serialized to HTML5"
        })
    end

    -- attach element name
    local out = SUB_ELEMENT_START .. node.call

    -- attach attributes
    local attributes = ""

    -- NOTE: sort to make it deterministic
    local attribs = {}
    for attr, _ in pairs(node.args) do
        table.insert(attribs, attr)
    end
    table.sort(attribs)

    for i = 1, #attribs do
        local attr = attribs[i]
        local values = node.args[attr]

        local value = ""
        for j = 1, #values do
            local subnode = values[j]
            if subnode.is_node then
                value = value .. tostring(values[j])
            else
                local text = tostring(values[j])
                if text:find(SUB_ELEMENT_START) ~= nil or text:find(SUB_ELEMENT_END) ~= nil or text:find(SUB_ATTR_START) ~= nil or text:find(SUB_ATTR_END) ~= nil then
                    Litua.error("Text content contains a substitution character - sorry a shortcoming of this implementation leads to this error", {
                        ["expected"] = "text nodes do not contain the substitution characters of this implementation",
                    })
                end
                value = value .. text
            end
        end

        -- NOTE: skip attributes like "=whitespace" which are provided
        --       as special attributes by the lexer
        if attr:find("^=") == nil then
            if not is_valid_xml_element_name_and_attribute(attr) then
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

-- call hook for every element
Litua.convert_node_to_string("", node_to_xml)

-- 
Litua.convert_node_to_string("document", function (node)
    local nonempty_content_nodes = {}

    for _, content in ipairs(node.content) do
        local is_non_empty = content:find("^[\x09\x0A\x0B\x0C\x0D\x20\x85\xA0]*$") == nil
        if is_non_empty then
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
