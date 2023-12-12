use helper::{ensure_matches, ensure_ok};

use bindings::fermyon::spin2_0_0::key_value::{Error, Store};

helper::define_component!(Component);

impl Component {
    fn main() -> Result<(), String> {
        ensure_matches!(Store::open("forbidden"), Err(Error::AccessDenied));

        let store = ensure_ok!(Store::open("default"));

        // Ensure nothing set in `bar` key
        ensure_ok!(store.delete("bar"));
        ensure_matches!(store.exists("bar"), Ok(false));
        ensure_matches!(store.get("bar"), Ok(None));
        ensure_matches!(store.get_keys().as_deref(), Ok(&[]));

        // Set `bar` key
        ensure_ok!(store.set("bar", b"baz"));
        ensure_matches!(store.exists("bar"), Ok(true));
        ensure_matches!(store.get("bar"), Ok(Some(v)) if v == b"baz");
        ensure_matches!(store.get_keys().as_deref(), Ok([bar]) if bar == "bar");

        // Override `bar` key
        ensure_ok!(store.set("bar", b"wow"));
        ensure_matches!(store.exists("bar"), Ok(true));
        ensure_matches!(store.get("bar"), Ok(Some(wow)) if wow == b"wow");
        ensure_matches!(store.get_keys().as_deref(), Ok([bar]) if bar == "bar");

        // Set another key
        ensure_ok!(store.set("qux", b"yay"));
        ensure_matches!(store.get_keys().as_deref(), Ok(c) if c.len() == 2 && c.contains(&"bar".into()) && c.contains(&"qux".into()));

        // Delete everything
        ensure_ok!(store.delete("bar"));
        ensure_ok!(store.delete("bar"));
        ensure_ok!(store.delete("qux"));
        ensure_matches!(store.exists("bar"), Ok(false));
        ensure_matches!(store.get("qux"), Ok(None));
        ensure_matches!(store.get_keys().as_deref(), Ok(&[]));

        Ok(())
    }
}
