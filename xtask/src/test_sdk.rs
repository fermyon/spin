/// Test runners for sdks
use crate::Result;
use crate::Shell;

impl crate::TestSdk {
    pub fn exec(&self, sh: &Shell) -> Result<()> {
        if self.go {
            go::run(sh)?;
        }

        Ok(())
    }
}

// go sdk
mod go {
    use std::ffi::OsStr;
    use crate::Result;
    use crate::cmd;
    use crate::Shell;

    enum Mode {
        Export,
        Import,
    }

    impl AsRef<OsStr> for Mode {
        fn as_ref(&self) -> &OsStr {
            OsStr::new(match self {
                Self::Export => "export",
                Self::Import => "import",
            })
        }
    }

    pub fn run(sh: &Shell) -> crate::Result<()> {
        let sdk_path = sh.current_dir().join("sdk").join("go");
        let wit_path = sh.current_dir().join("wit").join("ephemeral");

        let http = sdk_path.join("http");
        let redis = sdk_path.join("redis");
        let config = sdk_path.join("config");

        use Mode::*;
        let wit_bindgen = |wit_name, mode, outdir|->Result<()> {
            let wit = wit_path.join(wit_name).with_extension("wit");
            cmd!(sh, "wit-bindgen c --{mode} {wit} --out-dir {outdir}").run()?;
            Ok(())
        };

        wit_bindgen("wasi-outbound-http", Import, &http)?;
        wit_bindgen("spin-http", Export, &http)?;
        wit_bindgen("outbound-redis", Import, &redis)?;
        wit_bindgen("spin-redis", Export, &redis)?;
        wit_bindgen("spin-config", Import, &config)?;

        let testdata = http.join("tesdata").join("http-tinygo");
        let go_source = testdata.join("main.go");
        let wasm = go_source.with_extension("wasm");
        cmd!(sh, "tinygo build -wasm-abi=generic -target=wasi -gc=leaking -no-debug -o {wasm} {go_source}").run()?;

        cmd!(sh, "go test -v {sdk_path}").run()?;

        let tinygo_test = |dir| ->Result<()>{
            cmd!(
                sh,
                "tinygo test -wasm-abi=generic -target=wasi -gc=leaking -v {dir}"
            )
            .run()?;
            Ok(())
        };
        tinygo_test(http)?;
        tinygo_test(redis)?;
        Ok(())
    }
}
