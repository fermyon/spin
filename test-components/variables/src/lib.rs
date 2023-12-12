use helper::ensure_matches;

use bindings::fermyon::spin2_0_0::variables::{get, Error};

helper::define_component!(Component);

impl Component {
    fn main() -> Result<(), String> {
        ensure_matches!(get("variable"), Ok(val) if val == "value");
        ensure_matches!(get("non_existent"), Err(Error::Undefined(_)));

        ensure_matches!(get("invalid-name"), Err(Error::InvalidName(_)));
        ensure_matches!(get("invalid!name"), Err(Error::InvalidName(_)));
        ensure_matches!(get("4invalidname"), Err(Error::InvalidName(_)));

        Ok(())
    }
}
