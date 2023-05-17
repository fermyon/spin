wit_bindgen::generate!({
    world: "spin-orchestrator-module1",
    path: "spin-orchestrator-module1.wit"
});

struct SpinModule;

impl SpinOrchestratorModule1 for SpinModule {
    fn handle_init(start_input: String) -> String {
        let text = config::get_config("message").unwrap();
        let params = format!("Module1, Init with {start_input}, message: {text}");
        
        println!("{}", params);
        
        params
    }
}

export_spin_orchestrator_module1!(SpinModule);
