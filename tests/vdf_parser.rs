//! Integration tests for the VDF/ACF parser. `vdf.rs` has no crate dependencies, so it is
//! included directly (the crate is binary-only and has no library target).

#[path = "../src/vdf.rs"]
#[allow(dead_code)]
mod vdf;

use vdf::{parse, Vdf};

const LIBRARYFOLDERS: &str = include_str!("fixtures/libraryfolders.vdf");

#[test]
fn libraryfolders_fixture_lists_all_libraries_and_apps() {
    let root = parse(LIBRARYFOLDERS);
    let lf = root.get("libraryfolders").expect("root key");
    let libs: Vec<_> = lf.entries();
    assert_eq!(libs.len(), 2);
    assert_eq!(
        lf.get("0").unwrap().str("path"),
        Some(r"C:\Program Files (x86)\Steam")
    );
    let apps = lf.get("1").unwrap().get("apps").unwrap().entries();
    let ids: Vec<&str> = apps.iter().map(|(k, _)| k.as_str()).collect();
    assert!(ids.contains(&"570") && ids.contains(&"440"));
}

#[test]
fn crlf_line_endings_parse_like_lf() {
    let crlf = LIBRARYFOLDERS.replace('\n', "\r\n");
    assert_eq!(parse(&crlf), parse(LIBRARYFOLDERS));
}

#[test]
fn escaped_quotes_and_backslashes_are_unescaped() {
    let v = parse(r#""a" { "k" "say \"hi\" C:\x" }"#);
    assert_eq!(v.get("a").unwrap().str("k"), Some(r#"say "hi" C:\x"#));
}

#[test]
fn lookup_is_case_insensitive_and_str_on_object_is_none() {
    let v = parse(r#""AppState" { "Name" "X" "sub" { } }"#);
    let st = v.get("appstate").unwrap();
    assert_eq!(st.str("NAME"), Some("X"));
    assert_eq!(st.str("sub"), None);
    assert!(matches!(st.get("sub"), Some(Vdf::Obj(_))));
}

#[test]
fn truncated_and_garbage_inputs_never_panic() {
    for s in [
        "",
        "\"",
        "\"a",
        "\"a\" {",
        "\"a\" { \"b\" \"c",
        "}",
        "{{{{",
        "\"a\" \"b\" }",
        "\0\0\0",
        "//only comment",
    ] {
        let _ = parse(s);
    }
    // Truncate the real fixture at every byte boundary that is valid UTF-8.
    for i in 0..LIBRARYFOLDERS.len() {
        if LIBRARYFOLDERS.is_char_boundary(i) {
            let _ = parse(&LIBRARYFOLDERS[..i]);
        }
    }
}
