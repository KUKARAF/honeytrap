use honeytrap::render::render_response;
use honeytrap::template::TemplateStore;
use regex::Regex;
use std::path::Path;

#[test]
fn shipped_aws_credentials_template_contains_valid_shaped_keys() {
    let store = TemplateStore::load(Path::new("templates")).expect("load shipped templates");
    let rendered = render_response(
        &store,
        ".aws__credentials.tmpl",
        "1.2.3.4",
        "example.com",
        "test-salt",
    )
    .expect("render .aws/credentials template");
    let body = String::from_utf8(rendered.bytes).unwrap();

    let re = Regex::new(r"AKIA[A-Z0-9]{16}").unwrap();
    let matches: Vec<&str> = re.find_iter(&body).map(|m| m.as_str()).collect();
    assert!(
        !matches.is_empty(),
        "no AWS-key-shaped strings found in rendered template"
    );
    for key in matches {
        assert!(key.contains("HTRAP"), "key {key} missing greppable marker");
    }
}
