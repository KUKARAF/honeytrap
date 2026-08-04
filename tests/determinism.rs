use honeytrap::render::render_response;
use honeytrap::template::TemplateStore;
use std::path::Path;

fn store() -> TemplateStore {
    TemplateStore::load(Path::new("tests/fixtures/templates")).expect("load fixture templates")
}

#[test]
fn same_ip_host_identical_output() {
    let store = store();
    let a = render_response(&store, "small.tmpl", "1.2.3.4", "example.com", "s").unwrap();
    let b = render_response(&store, "small.tmpl", "1.2.3.4", "example.com", "s").unwrap();
    assert_eq!(a.bytes, b.bytes);
}

#[test]
fn different_host_diverges() {
    let store = store();
    let a = render_response(&store, "small.tmpl", "1.2.3.4", "example.com", "s").unwrap();
    let b = render_response(&store, "small.tmpl", "1.2.3.4", "other.com", "s").unwrap();
    assert_ne!(a.bytes, b.bytes);
}

#[test]
fn different_ip_diverges() {
    let store = store();
    let a = render_response(&store, "small.tmpl", "1.2.3.4", "example.com", "s").unwrap();
    let b = render_response(&store, "small.tmpl", "9.9.9.9", "example.com", "s").unwrap();
    assert_ne!(a.bytes, b.bytes);
}

#[test]
fn different_salt_diverges() {
    let store = store();
    let a = render_response(&store, "small.tmpl", "1.2.3.4", "example.com", "s1").unwrap();
    let b = render_response(&store, "small.tmpl", "1.2.3.4", "example.com", "s2").unwrap();
    assert_ne!(a.bytes, b.bytes);
}
