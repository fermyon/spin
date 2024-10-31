use helper::{ensure_matches, ensure_ok};

use bindings::wasi::keyvalue::store::{open, Error, KeyResponse};
use bindings::wasi::keyvalue::batch as wasi_batch;
use bindings::wasi::keyvalue::atomics as wasi_atomics;

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
        ensure_matches!(keys(&store.list_keys(Some("0"))), Err(Error::Other(_))); // "list_keys: cursor not supported"

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

        ensure_ok!(wasi_batch::set_many(&store, &[("bar".to_string(), b"bin".to_vec()), ("baz".to_string(), b"buzz".to_vec())]));
        ensure_ok!(wasi_batch::get_many(&store, &["bar".to_string(), "baz".to_string()]));
        ensure_ok!(wasi_batch::delete_many(&store, &["bar".to_string(), "baz".to_string()]));
        ensure_matches!(wasi_atomics::increment(&store, "counter", 10), Ok(v) if v == 10);
        ensure_matches!(wasi_atomics::increment(&store, "counter", 5), Ok(v) if v == 15);

        // successful compare and swap
        ensure_ok!(store.set("bar", b"wow"));
        let cas = ensure_ok!(wasi_atomics::Cas::new(&store, "bar"));
        ensure_matches!(cas.current(), Ok(Some(v)) if v == b"wow".to_vec());
        ensure_ok!(wasi_atomics::swap(cas, b"swapped"));
        ensure_matches!(store.get("bar"), Ok(Some(v)) if v == b"swapped");
        ensure_ok!(store.delete("bar"));

        Ok(())
    }
}

fn keys<E>(res: &Result<KeyResponse, E>) -> Result<&[String], &E> {
    res.as_ref().map(|kr| kr.keys.as_slice())
}
