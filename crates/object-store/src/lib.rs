pub mod file;

mod wit {
    wit_bindgen_wasmtime::export!("../../wit/ephemeral/spin-object-store.wit");
}

pub use wit::spin_object_store::add_to_linker;
