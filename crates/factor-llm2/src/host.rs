use crate::{bindings::my_company::my_product::llm::Host, InstanceState};

impl Host for InstanceState {
    fn my_function(&mut self) -> String {
        String::from("world!")
    }
}
