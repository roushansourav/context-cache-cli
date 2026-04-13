use tree_sitter::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    JavaScript,
    TypeScript,
    Tsx,
    Python,
    Rust,
    Go,
    Java,
    C,
    Cpp,
    Ruby,
    Kotlin,
    Scala,
    Swift,
    Solidity,
    Dart,
    R,
    Perl,
    Vue,
    Notebook,
    Bash,
    Php,
    CSharp,
    Lua,
}

pub fn detect_language(file_path: &str) -> Option<Lang> {
    let fp = file_path.to_ascii_lowercase();
    if fp.ends_with(".js") || fp.ends_with(".jsx") || fp.ends_with(".mjs") || fp.ends_with(".cjs") {
        return Some(Lang::JavaScript);
    }
    if fp.ends_with(".ts") {
        return Some(Lang::TypeScript);
    }
    if fp.ends_with(".tsx") {
        return Some(Lang::Tsx);
    }
    if fp.ends_with(".py") {
        return Some(Lang::Python);
    }
    if fp.ends_with(".rs") {
        return Some(Lang::Rust);
    }
    if fp.ends_with(".go") {
        return Some(Lang::Go);
    }
    if fp.ends_with(".java") {
        return Some(Lang::Java);
    }
    if fp.ends_with(".c") || fp.ends_with(".h") {
        return Some(Lang::C);
    }
    if fp.ends_with(".cpp")
        || fp.ends_with(".cc")
        || fp.ends_with(".cxx")
        || fp.ends_with(".hpp")
        || fp.ends_with(".hh")
    {
        return Some(Lang::Cpp);
    }
    if fp.ends_with(".rb") {
        return Some(Lang::Ruby);
    }
    if fp.ends_with(".kt") || fp.ends_with(".kts") {
        return Some(Lang::Kotlin);
    }
    if fp.ends_with(".scala") {
        return Some(Lang::Scala);
    }
    if fp.ends_with(".swift") {
        return Some(Lang::Swift);
    }
    if fp.ends_with(".sol") {
        return Some(Lang::Solidity);
    }
    if fp.ends_with(".dart") {
        return Some(Lang::Dart);
    }
    if fp.ends_with(".r") {
        return Some(Lang::R);
    }
    if fp.ends_with(".pl") || fp.ends_with(".pm") || fp.ends_with(".t") || fp.ends_with(".xs") {
        return Some(Lang::Perl);
    }
    if fp.ends_with(".vue") {
        return Some(Lang::Vue);
    }
    if fp.ends_with(".ipynb") {
        return Some(Lang::Notebook);
    }
    if fp.ends_with(".sh") || fp.ends_with(".bash") || fp.ends_with(".zsh") {
        return Some(Lang::Bash);
    }
    if fp.ends_with(".php") {
        return Some(Lang::Php);
    }
    if fp.ends_with(".cs") {
        return Some(Lang::CSharp);
    }
    if fp.ends_with(".lua") {
        return Some(Lang::Lua);
    }
    None
}

pub fn language_for(lang: Lang) -> Option<Language> {
    match lang {
        Lang::JavaScript => Some(tree_sitter_javascript::language()),
        Lang::TypeScript => Some(tree_sitter_typescript::language_typescript()),
        Lang::Tsx => Some(tree_sitter_typescript::language_tsx()),
        Lang::Python => Some(tree_sitter_python::language()),
        Lang::Rust => Some(tree_sitter_rust::language()),
        Lang::Go => Some(tree_sitter_go::language()),
        Lang::Java => Some(tree_sitter_java::language()),
        Lang::C => Some(tree_sitter_c::language()),
        Lang::Cpp => Some(tree_sitter_cpp::language()),
        Lang::Ruby => Some(tree_sitter_ruby::language()),
        Lang::Kotlin => None,
        Lang::Scala => None,
        Lang::Swift => None,
        Lang::Solidity => None,
        Lang::Dart => None,
        Lang::R => None,
        Lang::Perl => None,
        Lang::Vue => None,
        Lang::Notebook => None,
        Lang::Bash => None,
        Lang::Php => None,
        Lang::CSharp => Some(tree_sitter_c_sharp::language()),
        Lang::Lua => None,
    }
}

pub fn lang_to_name(lang: Lang) -> &'static str {
    match lang {
        Lang::JavaScript => "javascript",
        Lang::TypeScript => "typescript",
        Lang::Tsx => "tsx",
        Lang::Python => "python",
        Lang::Rust => "rust",
        Lang::Go => "go",
        Lang::Java => "java",
        Lang::C => "c",
        Lang::Cpp => "cpp",
        Lang::Ruby => "ruby",
        Lang::Kotlin => "kotlin",
        Lang::Scala => "scala",
        Lang::Swift => "swift",
        Lang::Solidity => "solidity",
        Lang::Dart => "dart",
        Lang::R => "r",
        Lang::Perl => "perl",
        Lang::Vue => "vue",
        Lang::Notebook => "notebook",
        Lang::Bash => "bash",
        Lang::Php => "php",
        Lang::CSharp => "csharp",
        Lang::Lua => "lua",
    }
}
