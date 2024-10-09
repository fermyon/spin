use helper::{ensure_matches, ensure_ok};

use bindings::wasi::keyvalue::store::{Error, open, KeyResponse};

helper::define_component!(Component);

impl Component {
    fn main() -> Result<(), String> {

        ensure_matches!(open("forbidden"), Err(Error::AccessDenied));

        let store = ensure_ok!(open("default"));

        // Ensure nothing set in `bar` key
        ensure_ok!(store.delete("bar"));
        ensure_matches!(store.exists("bar"), Ok(false));
        ensure_matches!(store.get("bar"), Ok(None));
        ensure_matches!(keys(&store.list_keys(None)), Ok(&[]));

        // Set `bar` key
        ensure_ok!(store.set("bar", b"baz"));
        ensure_matches!(store.exists("bar"), Ok(true));
        ensure_matches!(store.get("bar"), Ok(Some(v)) if v == b"baz");
        ensure_matches!(keys(&store.list_keys(None)), Ok([bar]) if bar == "bar");
        ensure_matches!(keys(&store.list_keys(Some(0))), Ok([bar]) if bar == "bar");

        // Override `bar` key
        ensure_ok!(store.set("bar", b"wow"));
        ensure_matches!(store.exists("bar"), Ok(true));
        ensure_matches!(store.get("bar"), Ok(Some(wow)) if wow == b"wow");
        ensure_matches!(keys(&store.list_keys(None)), Ok([bar]) if bar == "bar");

        // Set another key
        ensure_ok!(store.set("qux", b"yay"));
        ensure_matches!(keys(&store.list_keys(None)), Ok(c) if c.len() == 2 && c.contains(&"bar".into()) && c.contains(&"qux".into()));

        // Delete everything
        ensure_ok!(store.delete("bar"));
        ensure_ok!(store.delete("bar"));
        ensure_ok!(store.delete("qux"));
        ensure_matches!(store.exists("bar"), Ok(false));
        ensure_matches!(store.get("qux"), Ok(None));
        ensure_matches!(keys(&store.list_keys(None)), Ok(&[]));

        Ok(())
    }
}

fn keys<E>(res: &Result<KeyResponse, E>) -> Result<&[String], &E> {
    res.as_ref().map(|kr| kr.keys.as_slice())
}
