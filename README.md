# litua

<dl>
<dt>author:</dt><dd>tajpulo</dd>
<dt>version:</dt><dd>2.0.0</dd>
<dt>badges:</dt><dd><img src="https://github.com/typho/litua/actions/workflows/release.yml/badge.svg" alt="state of the release process"/> <img src="https://github.com/typho/litua/actions/workflows/build.yml/badge.svg" alt="state of the build process"/></dd>
</dl>

Read a text document, receive its tree in [Lua](https://www.lua.org/) and manipulate it before representing it as string.

## What is it about?

### The input

Text documents occur in many contexts. Actually, we like them as a simple means to document ideas and concepts. They help us communicate. But sometimes, we want to transform them to other text formats or process its content. litua helps with that in a particular way.

You can write a text document like this:

```
In olden times when wishing still helped one, there lived a king whose daughters were all beautiful; and the youngest was so beautiful that the sun itself, which has seen so much, was astonished whenever it shone in her face.
```

But this text is boring. You usually care about markup. Markup are special instructions which annotate text:

```
In olden times when wishing still helped one, there lived a {bold king} whose daughters were all {italic beautiful}; and the youngest was so beautiful that the sun itself, which has seen so much, was astonished whenever it shone in her face.
```

In this case, the text ``{bold X}`` and ``{italic Y}`` has some special meaning. For example, it could mean that the text is represented with a special style (e.g. X in a bold font and Y in cursive script). In general, we define *litua input syntax* in the following manner:

```
{element[attr1=value1][attribute2=val2] text content of element}
```

* *element* is the name of the markup element. Its name indicates its semantics. In terms of litua, we can define the semantics ourselves.
* *attr1* and *attr2* are attributes of this markup element. It gives more details about the markup element. For example, it could name the fontface used to represent these markup elements. In essence, we have an attribute *attr1* here which is associated with the value *value1*. We also have attribute *attr2* which is associated with *val2*.
* *content* is the text affected by this markup.

And finally, I will tell you a secret: *value1*, *val2*, and *text content of element* need not be text, but can also be an element itself. Thus, the following is permitted in *litua input syntax*:

```
{bold[font-face=Bullshit Sans] {italic Blockchain managed information density}}
```

In this sense, litua input syntax is very similar to XML (`<element attr1="value1" attribute2="val2">text content of element</element>`), LISP (e.g. `(element :attr1 "value1" :attribute2 "val2" "text content of element")`), and markup languages in general. By the way, if you literally need a ``{`` or ``}`` in your document, you can escape these semantics by writing ``{left-curly-brace}`` or ``{right-curly-brace}`` respectively instead. litua input syntax files must always be encoded in UTF-8.

### Processing the document

Let us put the element-example in *litua input syntax* into a text document (``doc.lit``). Then we can invoke `litua`:

```
bash$  litua doc.lit
```

The output is in the file with extension ``out``: ``doc.out``. And it is super-boring: It is exactly the input:

```
bash$  cat doc.out
{element[attr1=value1][attribute2=val2] text content of element}
```

It becomes interesting, if I tell you that there is a representation of this element in Lua:

```lua
local node = {
    -- the string giving the node type
    ["call"] = "element",
    -- the key-value pairs of arguments.
    -- values are sequences of strings or nodes
    ["args"] = { ["attr1"] = { [1] = "value1" }, ["attribute2"] = { [1] = "val2" } },
    -- the sequence of elements occuring in the body of a node.
    -- the items of content can be strings or nodes themselves
    ["content"] = {
        [1] = "text content of element"
    },
}
```

For example, ``node.call`` allows you to access the name of the markup element. ``node.content[1]`` allows you to access the string which is the first and only content member of `element` in [Lua](https://www.lua.org/docs.html). Remember that in Lua, the first element in a collection type is stored at index 1 (not 0 as in the majority of programming languages).

Now create a Lua file ``hooks.lua`` in the same directory (the name must start with `hooks` and must end with `.lua`) with the following content:

```lua
Litua.convert_node_to_string("element", function (node)
    return "The " .. tostring(node.call) .. " said: " .. tostring(node.content[1])
end)
```

Now let us invoke `litua` again:

```
bash$  litua doc.lit
[…]
bash$  cat doc.out
The element said: text content of element
```

Wow, we just modified the behavior how to process the document 😍

### Hooks

In fact, we used a concept called *hook* to modify the behavior. We register a hook with ``convert_node_to_string`` to trigger the hook whenever litua tries to convert a node to a string. A hook is a Lua function. Let us read the Lua syntax:

```lua
Litua.convert_node_to_string("element", function (node)
    return "The " .. tostring(node.call) .. " said: " .. tostring(node.content[1])
end)
```

* ``Litua.convert_node_to_string`` is a function, which is defined by litua whenever you run ``litua``.
* The first argument must be a string, namely ``"element"``, which tells litua **when** to call the second argument.
* The second argument starts with the keyword ``function`` and ends with the keyword ``end``. This is the **hook**. It is a function and takes one argument called ``node``. It can run arbitrary code and specifically it returns a string in the end which is built from the data in the `node` variable. ``..`` is the string concatenation operator in Lua and ``tostring`` is a builtin Lua function which converts any value into a string object.

The complete set of hooks is given here:

* ``Litua.on_setup`` <br/> **purpose:** registers a hook which is run initially and meant to optionally initialize the ``Litua.global`` variable as you need it <br/> **default behavior:** does nothing <br/> **hook:** The hook takes no argument, and returns nil
* ``Litua.modify_initial_string`` <br/> **purpose:** registers a hook which is run after ``on_setup`` and meant to optionally pre-process the source code of the text document <br/>  **default behavior:** returns the original source code <br/> **hook:** The hook takes the source code as a string, the filepath of the source file as a string, and returns the updated source code as string
* ``Litua.read_new_node`` <br/> **purpose:** registers a hook which is run after turning the document into a hierarchy of elements. It allows you to look at some node before modifying it <br/> **default behavior:** does nothing <br/> **hook:** The hook takes a copy of the current node, the tree depth as integer, and returns nil
* ``Litua.modify_node`` <br/> **purpose:** registers a hook which is run after ``read_new_node`` and allows you to actually modify a node <br/> **default behavior:** returns the original node <br/> **hook:** The hook takes the current node, the tree depth as integer, and the filter name, and returns (some node or string) and nil
* ``Litua.read_modified_node`` <br/> **purpose:** registers a hook which is run after ``modify_node``. It allows you to look at some node after modifying it <br/> **default behavior:** does nothing <br/> **hook:** The hook takes a copy of the current node, the tree depth as integer, and returns nil
* ``Litua.convert_node_to_string`` <br/> **purpose:** registers a hook which defines how to represent a node as a string <br/> **default behavior:** returns its original string representation in litua input syntax <br/> **hook:** The hook takes the current node, the tree depth as integer, and the filter name, and returns a string and nil
* ``Litua.modify_final_string`` <br/> **purpose:** registers a hook once the hierarchy has been converted into a string and meant to optionally post-process the source code of the text document <br/> **default behavior:** returns the provided string representation <br/> **hook:** The hook takes the string representation as a string, and returns a string
* ``Litua.on_teardown`` <br/> **purpose:** registers a hook which is run finally and meant to tear down variables in ``Litua.global`` as you need it <br/> **default behavior:** does nothing <br/> **hook:** The hook takes no argument, and returns nil

Be aware that the document always lives within one invisible top-level node called ``document``. So if you use a ``document`` element in your input file and define a hook for the element ``document`` as well, don't be surprised about the additional invocation of this hook.

### Examples

I highly recommend to go through the examples in this order to get an idea how to use the hooks:

1. [enumeration](examples/enumeration) – replace a call with an incrementing counter
2. [replacements](examples/replacements) – first define substitution pairs and then apply them
3. [literate-programming](examples/literate-programming) – define documentation and code block and write them to different files
4. [markup](examples/markup) – serialize the tree to HTML5

## Why should I use it?

Litua is a simple text processing utility for text documents with a hierarchical structure. It reminds of tools like XSLT, but people often complain about XSLT being too foreign to common programming languages. As an alternative, I provide litua with a parser for the litua input syntax, a map of data from rust to Lua, a runtime in Lua, and writer for text files.

## How to install

This is a single static executable. It only depends on basic system libraries like pthread, math and libc. It ships the entire Lua 5.4 interpreter with the executable. I expect it to work out-of-the-box on your operating system.

## How to run

Call the litua executable with ``-h`` to get information about additional arguments:

```
litua -h
```

## Litua input specification

The following document defines the syntax (see also [``design/litua-lexer-state-diagram.jpg``](design/litua-lexer-state-diagram.jpg)):

```
Node       = (Text | RawString | Function){0,…}
Text       = (NOT the symbols "{" or "}"){1,…}
RawString  = "{<" Whitespace (NOT the string Whitespace-and-">}") Whitespace ">}"
           | "{<<" Whitespace (NOT the string Whitespace-and-">>}") Whitespace ">>}"
           | "{<<<" Whitespace (NOT the string Whitespace-and-">>>}") Whitespace ">>>}"
           … continue up to 126 "<" characters
Function   = "{" Call "}"
           | "{" Call Whitespace "}"
           | "{" Call Whitespace Node "}"
           | "{" Call ( "[" Key "=" Node "]" ){1,…} "}"
           | "{" Call ( "[" Key "=" Node "]" ){1,…} Whitespace "}"
           | "{" Call ( "[" Key "=" Node "]" ){1,…} Whitespace Node "}"

Call       = (NOT the symbols "}", "[" or "<")(NOT the symbols "[" or "<"){0,…}
Key        = (NOT the symbol "="){1,…}
Whitespace = any of the 25 Unicode Whitespace characters
```

In essence, don't use "<" or "[" in function call names, or "=" in argument keys.
Keep the number of opening and closing braces balanced (though this is not enforced by the syntax).

## Improvements

The following parts can be improved:

* the ``.parsed.expected`` files are not checked in the testsuite, because rust's HashMap representation is not consistent across builds.
* verify that error handling for user code works well
* improved error reporting for syntax errors (tokens position, etc)

## Source Code

The source code is available at [Github](https://github.com/typho/litua).

## License

See [the LICENSE file](LICENSE) (Hint: MIT license).

## Changelog

<dl>
<dt>0.9</dt> <dd>first public release with raw strings and four examples</dd>
<dt>1.0.0</dt> <dd>improves stdout/stderr, improved documentation, CI builds, upload to crates.io</dd>
<dt>1.1.0</dt> <dd>bugfix third argument of modify-node hook, modify-hook may now also return strings</dd>
<dt>1.1.1</dt> <dd>bugfix: interrupted '>' sequences inside raw string content can be used again, removed hook checks from testsuite</dd>
<dt>2.0</dt> <dd>improved docs, require whitespace before ">" in raw strings</dd>
</dl>

## Issues

Please report any issues on the [Github issues page](https://github.com/typho/litua/issues).
