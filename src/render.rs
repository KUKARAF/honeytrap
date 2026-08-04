use crate::errors::RenderError;
use crate::seed::{self, RngHandle};
use crate::template::{functions, TemplateStore};
use std::rc::Rc;

pub struct Rendered {
    pub bytes: Vec<u8>,
    pub template: String,
}

/// Renders the template registered for `template_name`, seeded from
/// `(client_ip, host, salt)`. Used by both `serve` and `gen` so CLI output
/// is guaranteed identical to what the server would have sent for the same
/// inputs.
pub fn render_response(
    store: &TemplateStore,
    template_name: &str,
    client_ip: &str,
    host: &str,
    salt: &str,
) -> Result<Rendered, RenderError> {
    let tmpl = store
        .env
        .get_template(template_name)
        .map_err(|_| RenderError::UnknownTemplate(template_name.to_string()))?;

    let seed = seed::derive_seed(client_ip, host, salt);
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
    })
}

/// Renders by request path rather than template name; used by the HTTP
/// handler and the `gen` CLI subcommand.
pub fn render_for_path(
    store: &TemplateStore,
    request_path: &str,
    client_ip: &str,
    host: &str,
    salt: &str,
) -> Option<Result<Rendered, RenderError>> {
    let template_name = store.resolve_path(request_path)?.to_string();
    Some(render_response(
        store,
        &template_name,
        client_ip,
        host,
        salt,
    ))
}
