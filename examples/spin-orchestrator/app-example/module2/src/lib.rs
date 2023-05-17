wit_bindgen::generate!({
    world: "spin-orchestrator-module2",
    path: "spin-orchestrator-module2.wit"
});

struct SpinModule;

impl SpinOrchestratorModule2 for SpinModule {
    fn handle_init(module1_output: String) -> String {            
        let timeout = config::get_config("timeout").unwrap();
        let retries = config::get_config("retries").unwrap();
        
        let params = format!("Module2, Init with {module1_output}, timeout: {timeout}, retries: {retries}");
        
        println!("{}", params);
        params
    }
}

export_spin_orchestrator_module2!(SpinModule);
