use tree_sitter::Language as TsLanguage;

pub fn language() -> TsLanguage {
    tree_sitter_go::LANGUAGE.into()
}
