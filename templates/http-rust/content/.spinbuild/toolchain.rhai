if !interactive() {
    return;
}

let lr = exec("rustup", ["target", "list", "--installed"]);

if lr.program_found {
    if lr.exit_code == 0 && lr.stdout.contains("wasm32-wasi") {
        // all is well
    } else {
        print();
        print("This application requires the Rust `wasm32-wasi` target.");
        if ask_yn("You do not have this target installed. Would you like Spin to install it for you?") {
            let ir = exec("rustup", ["target", "install", "wasm32-wasi"]);
            print(ir.stdout);
            print(ir.stderr);
        } else {
            print("You can install it by running `rustup target install wasm32-wasi`.");
        }
    }
} else {
    print();
    print("This application requires Rust with the `wasm32-wasi` target. Please:");
    print("1. Install Rust from https://rustup.rs/");
    print("2. Then run `rustup target install wasm32-wasi`.");
}
