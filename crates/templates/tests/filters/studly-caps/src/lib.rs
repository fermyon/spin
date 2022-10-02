wit_bindgen_rust::export!("../../../wit/custom-filter.wit");

struct CustomFilter;

impl custom_filter::CustomFilter for CustomFilter {
    fn exec(source: String) -> Result<String, String> {
        let mapped: String = (0..).zip(source.chars()).map(|(index, c)|
            if c.is_ascii_alphabetic() {
                if index % 2 == 0 {
                    c.to_ascii_lowercase()
                } else {
                    c.to_ascii_uppercase()
                }
            } else {
                c
            }
        ).collect();
        Ok(mapped)
    }
}
