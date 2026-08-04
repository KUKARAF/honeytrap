use honeytrap::template::{validate, TemplateStore};
use std::path::Path;

#[test]
fn startup_validation_rejects_oversized_template() {
    let store =
        TemplateStore::load(Path::new("tests/fixtures/templates")).expect("load fixture templates");
    let result = validate::run(&store, "test-salt", 8192);
    let err = result.expect_err("expected oversized.tmpl to fail validation");
    assert!(
        err.details.iter().any(|d| d.contains("oversized.tmpl")),
        "error did not name the offending template: {err}"
    );
}

#[test]
fn shipped_templates_pass_validation_at_default_cap() {
    let store = TemplateStore::load(Path::new("templates")).expect("load shipped templates");
    validate::run(&store, "test-salt", 8192).expect("shipped templates must fit the default cap");
}
