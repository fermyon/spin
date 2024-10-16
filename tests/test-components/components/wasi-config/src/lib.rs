use helper::ensure_matches;

use bindings::wasi::config::store::{get, get_all};

helper::define_component!(Component);

impl Component {
    fn main() -> Result<(), String> {
        ensure_matches!(get("variable"), Ok(Some(val)) if val == "value");
        ensure_matches!(get("non_existent"), Ok(None));

        let expected_all = vec![
            ("variable".to_owned(), "value".to_owned()),
        ];
        ensure_matches!(get_all(), Ok(val) if val == expected_all);

        ensure_matches!(get("invalid-name"), Ok(None));
        ensure_matches!(get("invalid!name"), Ok(None));
        ensure_matches!(get("4invalidname"), Ok(None));

        Ok(())
    }
}
