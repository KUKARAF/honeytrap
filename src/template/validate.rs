use crate::errors::StartupValidationError;
use crate::render;
use crate::template::TemplateStore;
use std::collections::HashSet;

/// Synthetic (ip, host) pairs used purely to exercise every template at
/// startup — never derived from real request data.
const REPRESENTATIVE_TARGETS: &[(&str, &str)] = &[
    ("203.0.113.1", "startup-check-a.invalid"),
    ("198.51.100.7", "startup-check-b.invalid"),
    ("::1", "startup-check-c.invalid"),
];

/// Renders every known template against several fixed synthetic seeds and
/// checks the output against `max_bytes`. Called before the server starts
/// serving (and before `gen` prints anything), so an oversized template is a
/// loud startup failure rather than a runtime surprise.
pub fn run(
    store: &TemplateStore,
    salt: &str,
    max_bytes: usize,
) -> Result<(), StartupValidationError> {
    let mut violations = Vec::new();
    let unique_templates: HashSet<&str> = store.template_names().collect();

    for template_name in unique_templates {
        for (ip, host) in REPRESENTATIVE_TARGETS {
            match render::render_response(store, template_name, ip, host, salt) {
                Ok(rendered) => {
                    if rendered.bytes.len() > max_bytes {
                        violations.push(format!(
                            "{template_name}: rendered {} bytes (seed host {host:?}) — {} bytes over cap",
                            rendered.bytes.len(),
                            rendered.bytes.len() - max_bytes
                        ));
                    }
                }
                Err(e) => {
                    violations.push(format!("{template_name}: failed to render: {e}"));
                }
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(StartupValidationError {
            max_bytes,
            details: violations,
        })
    }
}
