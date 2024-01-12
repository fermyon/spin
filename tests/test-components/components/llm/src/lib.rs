use helper::{ensure, ensure_eq, ensure_ok};

use bindings::fermyon::spin2_0_0::llm;

helper::define_component!(Component);

impl Component {
    fn main() -> Result<(), String> {
        let param = llm::InferencingParams {
            max_tokens: 1,
            repeat_penalty: 0.0,
            repeat_penalty_last_n_token_count: 0,
            temperature: 0.0,
            top_k: 1,
            top_p: 1.0,
        };
        let inference = ensure_ok!(llm::infer(
            &"llama2-chat".to_owned(),
            "say hello",
            Some(param)
        ));

        ensure!(!inference.text.is_empty());
        ensure_eq!(inference.usage.generated_token_count, 1);

        Ok(())
    }
}
