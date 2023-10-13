use std::path::Path;

use ui_testing::{Failed, Normalizer, UiTestsRunner};

fn main() -> anyhow::Result<()> {
    let mut runner = UiTestsRunner::default();
    for entry in glob::glob("tests/ui/**/*.toml")? {
        let path = entry?.canonicalize()?;
        let test_name = format!("ui::{}", path.file_stem().unwrap().to_string_lossy());
        let snapshot_path = path.with_extension("json");
        runner.add_test(test_name, snapshot_path, move |n| run_test(&path, n));
    }
    for entry in glob::glob("tests/ui/v1/*.toml")? {
        let path = entry?.canonicalize()?;
        let test_name = format!(
            "ui::v1_to_v2::{}",
            path.file_stem().unwrap().to_string_lossy()
        );
        let snapshot_path = path.with_extension("toml.v2");
        runner.add_test(test_name, snapshot_path, move |n| {
            run_v1_to_v2_test(&path, n)
        });
    }

    runner.run_tests()
}

fn run_test(input: &Path, _normalizer: &mut Normalizer) -> Result<String, Failed> {
    let manifest = spin_manifest::manifest_from_file(input)?;
    Ok(serde_json::to_string_pretty(&manifest).expect("serialization should work"))
}

fn run_v1_to_v2_test(input: &Path, _normalizer: &mut Normalizer) -> Result<String, Failed> {
    let manifest_str =
        std::fs::read_to_string(input).unwrap_or_else(|err| panic!("reading {input:?}: {err:?}"));
    let v1_manifest = toml::from_str(&manifest_str)
        .unwrap_or_else(|err| panic!("parsing v1 manifest {input:?}: {err:?}"));
    let v2_manifest = spin_manifest::compat::v1_to_v2_app(v1_manifest)?;
    Ok(toml::to_string(&v2_manifest).expect("serialization should work"))
}
