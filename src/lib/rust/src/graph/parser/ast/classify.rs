use super::super::language::Lang;

pub fn is_class_node(kind: &str, lang: Lang) -> bool {
    match lang {
        Lang::JavaScript | Lang::TypeScript | Lang::Tsx => {
            matches!(kind, "class_declaration" | "class")
        }
        Lang::Python => kind == "class_definition",
        Lang::Rust => kind == "struct_item" || kind == "trait_item" || kind == "impl_item",
        Lang::Go => kind == "type_spec",
        Lang::Java => kind == "class_declaration" || kind == "interface_declaration",
        Lang::C | Lang::Cpp => kind == "struct_specifier" || kind == "class_specifier",
        Lang::Ruby => kind == "class" || kind == "module",
        Lang::Php => kind == "class_declaration" || kind == "interface_declaration",
        Lang::CSharp => {
            kind == "class_declaration"
                || kind == "interface_declaration"
                || kind == "struct_declaration"
        }
        Lang::Kotlin | Lang::Scala | Lang::Swift | Lang::Solidity | Lang::Dart | Lang::R | Lang::Perl | Lang::Vue | Lang::Notebook | Lang::Bash => false,
        Lang::Lua => false,
    }
}

pub fn is_import_node(kind: &str, lang: Lang) -> bool {
    match lang {
        Lang::JavaScript | Lang::TypeScript | Lang::Tsx => kind == "import_statement",
        Lang::Python => kind == "import_statement" || kind == "import_from_statement",
        Lang::Rust => kind == "use_declaration",
        Lang::Go => kind == "import_declaration" || kind == "import_spec",
        Lang::Java => kind == "import_declaration",
        Lang::C | Lang::Cpp => kind == "preproc_include",
        Lang::Ruby => kind == "call",
        Lang::Php => kind == "namespace_use_declaration",
        Lang::CSharp => kind == "using_directive",
        Lang::Kotlin | Lang::Scala | Lang::Swift | Lang::Solidity | Lang::Dart | Lang::R | Lang::Perl | Lang::Vue | Lang::Notebook | Lang::Bash => false,
        Lang::Lua => kind == "function_call",
    }
}

pub fn is_call_node(kind: &str, lang: Lang) -> bool {
    match lang {
        Lang::JavaScript | Lang::TypeScript | Lang::Tsx => {
            matches!(kind, "call_expression" | "new_expression")
        }
        Lang::Python => kind == "call",
        Lang::Rust => kind == "call_expression" || kind == "macro_invocation",
        Lang::Go => kind == "call_expression",
        Lang::Java => kind == "method_invocation" || kind == "object_creation_expression",
        Lang::C | Lang::Cpp => kind == "call_expression",
        Lang::Ruby => kind == "call" || kind == "method_call",
        Lang::Php => {
            kind == "function_call_expression"
                || kind == "method_call_expression"
                || kind == "object_creation_expression"
        }
        Lang::CSharp => kind == "invocation_expression" || kind == "object_creation_expression",
        Lang::Kotlin | Lang::Scala | Lang::Swift | Lang::Solidity | Lang::Dart | Lang::R | Lang::Perl | Lang::Vue | Lang::Notebook | Lang::Bash => false,
        Lang::Lua => kind == "function_call",
    }
}
