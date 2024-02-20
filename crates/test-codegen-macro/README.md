# Test Codegen Macro

A macro for automatically producing `#[test]` annotated functions based on file directory structure. This is used by the runtime tests so that when adding a runtime test, you're not required to also add a test function corresponding to that runtime test. 