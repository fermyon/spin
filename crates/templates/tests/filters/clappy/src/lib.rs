wit_bindgen_rust::export!("../../../wit/custom-filter.wit");

const CLAP: char = '👏';

struct CustomFilter;

impl custom_filter::CustomFilter for CustomFilter {
    fn exec(source: String) -> Result<String, String> {
        let mut builder = String::with_capacity(source.len() * 2);
        for c in source.chars() {
            builder.push(c);
            builder.push(CLAP);
        }
        let result = builder.trim_end_matches(CLAP);
        Ok(result.to_owned())
    }
}
