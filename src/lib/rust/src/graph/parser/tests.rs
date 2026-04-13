use super::language::{detect_language, lang_to_name};
use super::preprocess::normalize_source;

#[test]
fn detects_extended_languages() {
    assert_eq!(detect_language("a.kt").map(lang_to_name), Some("kotlin"));
    assert_eq!(detect_language("b.scala").map(lang_to_name), Some("scala"));
    assert_eq!(detect_language("c.vue").map(lang_to_name), Some("vue"));
    assert_eq!(
        detect_language("d.ipynb").map(lang_to_name),
        Some("notebook")
    );
}

#[test]
fn normalizes_vue_script_blocks() {
    let source = r#"<template><div /></template><script lang=\"ts\">export function a() { return 1; }</script>"#;
    let (out, lang) = normalize_source("Comp.vue", source);
    assert!(out.contains("export function a"));
    assert!(lang.is_some());
}

#[test]
fn normalizes_ipynb_code_cells() {
    let source = r#"{"cells":[{"cell_type":"code","source":["def a():\n","  return 1\n"]}],"metadata":{"language_info":{"name":"python"}}}"#;
    let (out, _lang) = normalize_source("nb.ipynb", source);
    assert!(out.contains("def a():"));
}
