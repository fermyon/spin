#[allow(warnings)]
mod bindings;
#[allow(warnings)]
mod fermyon;
#[allow(warnings)]
mod wasi;

struct Component;

bindings::export!(Component with_types_in bindings);

pub(crate) fn format_deny_error(s: &str) -> String {
    format!("{s:?} is not permitted")
}