use crate::errors::RenderError;
use crate::seed::{self, RngHandle};
use crate::template::{functions, TemplateStore};
use std::rc::Rc;

pub struct Rendered {
    pub bytes: Vec<u8>,
    pub template: String,
    /// True when this template was chosen as a fallback for a path with no
    /// exact template, rather than being the path's registered template.
    pub fallback: bool,
}

/// Renders a named template against a precomputed 32-byte seed. The single
/// place rendering actually happens — every other entry point derives a seed
/// and calls here.
fn render_with_seed(
    store: &TemplateStore,
    template_name: &str,
    seed: [u8; 32],
    fallback: bool,
) -> Result<Rendered, RenderError> {
    let tmpl = store
        .env
        .get_template(template_name)
        .map_err(|_| RenderError::UnknownTemplate(template_name.to_string()))?;

    let rng = Rc::new(RngHandle::new(seed));
    let out = functions::with_rng(rng, || tmpl.render(minijinja::context! {})).map_err(|e| {
        RenderError::Render {
            name: template_name.to_string(),
            source: e,
        }
    })?;

    Ok(Rendered {
        bytes: out.into_bytes(),
        template: template_name.to_string(),
        fallback,
    })
}

/// Renders a template by name, seeding on the template name itself as the
/// path component. Used by startup validation and tests, which care about a
/// template's output size/shape rather than a specific request path.
pub fn render_response(
    store: &TemplateStore,
    template_name: &str,
    client_ip: &str,
    host: &str,
    salt: &str,
) -> Result<Rendered, RenderError> {
    let seed = seed::derive_seed(client_ip, host, template_name, salt);
    render_with_seed(store, template_name, seed, false)
}

/// Renders the response for a request path, seeded on `(client_ip, host,
/// request_path, salt)`. Used by both `serve` and `gen` so CLI output is
/// identical to what the server would send.
///
/// If the path has a registered template, that template is used. Otherwise,
/// when `allow_fallback` is set, a template is deterministically picked from
/// the pool so the path gets plausible content instead of a 404 (which would
/// betray the honeypot). Returns `None` when there is no match and either
/// fallback is disabled or no templates are loaded.
pub fn render_for_path(
    store: &TemplateStore,
    request_path: &str,
    client_ip: &str,
    host: &str,
    salt: &str,
    allow_fallback: bool,
) -> Option<Result<Rendered, RenderError>> {
    let seed = seed::derive_seed(client_ip, host, request_path, salt);

    let (template_name, fallback) = match store.resolve_path(request_path) {
        Some(name) => (name.to_string(), false),
        None if allow_fallback => (store.pick_fallback(&seed)?.to_string(), true),
        None => return None,
    };

    Some(render_with_seed(store, &template_name, seed, fallback))
}
