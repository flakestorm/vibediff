use tree_sitter::Language as TsLanguage;

pub fn language() -> TsLanguage {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

pub fn anchor_kinds() -> &'static [&'static str] {
    &[
        "function_declaration",
        "function_expression",
        "arrow_function",
        "method_definition",
        "class_declaration",
        "interface_declaration",
        "type_alias_declaration",
    ]
}
