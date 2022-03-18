use crate::{config::TemplateConfig, emoji};
use anyhow::{Context, Result};
use console::style;
use rhai::EvalAltResult;
use std::{cell::RefCell, env, path::Path, rc::Rc};

pub fn exec_pre(
    cfg: &mut TemplateConfig,
    dir: impl AsRef<Path>,
    obj: Rc<RefCell<liquid::Object>>,
) -> Result<()> {
    let engine = engine(dir.as_ref(), obj);
    let hooks = cfg.pre_hooks();
    eval(dir.as_ref(), engine, &hooks)
}

pub fn exec_post(
    cfg: &TemplateConfig,
    dir: impl AsRef<Path>,
    obj: Rc<RefCell<liquid::Object>>,
) -> Result<()> {
    let engine = engine(dir.as_ref(), obj);
    let hooks = cfg.post_hooks();
    eval(dir.as_ref(), engine, &hooks)
}

fn eval(dir: &Path, engine: rhai::Engine, scripts: &[String]) -> Result<()> {
    struct CleanupJob<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> CleanupJob<F> {
        pub fn new(f: F) -> Self {
            Self(Some(f))
        }
    }

    impl<F: FnOnce()> Drop for CleanupJob<F> {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }

    let cwd = std::env::current_dir()?;
    let _ = CleanupJob::new(move || {
        env::set_current_dir(cwd).ok();
    });
    env::set_current_dir(dir)?;

    for script in scripts {
        engine
            .eval_file::<()>(script.into())
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .context(format!(
                "{} {} {}",
                emoji::ERROR,
                style("Failed executing script:").bold().red(),
                style(script.to_owned()).yellow(),
            ))?;
    }

    Ok(())
}

fn engine(dir: &Path, obj: Rc<RefCell<liquid::Object>>) -> rhai::Engine {
    let mut engine = rhai::Engine::new();

    let module = variable_mod::create_module(obj);
    engine.register_static_module("variable", module.into());

    let module = file_mod::create_module(dir);
    engine.register_static_module("file", module.into());

    engine.register_result_fn(
        "abort",
        |error: &str| -> Result<String, Box<EvalAltResult>> { Err(error.into()) },
    );

    engine
}

mod variable_mod {
    use crate::variable::{StringEntry, Variable, VariableInfo};
    use liquid::{Object, ValueView};
    use liquid_core::Value;
    use regex::Regex;
    use rhai::{Array, Dynamic, EvalAltResult, Module};
    use std::cell::RefCell;
    use std::rc::Rc;

    type Result<T> = anyhow::Result<T, Box<EvalAltResult>>;

    pub fn create_module(liquid_object: Rc<RefCell<Object>>) -> Module {
        let mut module = Module::new();

        module.set_native_fn("is_set", {
            let liquid_object = liquid_object.clone();
            move |name: &str| -> Result<bool> {
                match liquid_object.get_value(name) {
                    NamedValue::NonExistant => Ok(false),
                    _ => Ok(true),
                }
            }
        });

        module.set_native_fn("get", {
            let liquid_object = liquid_object.clone();
            move |name: &str| -> Result<Dynamic> {
                match liquid_object.get_value(name) {
                    NamedValue::NonExistant => Ok(Dynamic::from(String::from(""))),
                    NamedValue::Bool(v) => Ok(Dynamic::from(v)),
                    NamedValue::String(v) => Ok(Dynamic::from(v)),
                }
            }
        });

        module.set_native_fn("set", {
            let liquid_object = liquid_object.clone();

            move |name: &str, value: &str| -> Result<()> {
                match liquid_object.get_value(name) {
                    NamedValue::NonExistant | NamedValue::String(_) => {
                        liquid_object.borrow_mut().insert(
                            name.to_string().into(),
                            Value::Scalar(value.to_string().into()),
                        );
                        Ok(())
                    }
                    _ => Err(format!("Variable {} not a String", name).into()),
                }
            }
        });

        module.set_native_fn("set", {
            let liquid_object = liquid_object.clone();

            move |name: &str, value: bool| -> Result<()> {
                match liquid_object.get_value(name) {
                    NamedValue::NonExistant | NamedValue::Bool(_) => {
                        liquid_object
                            .borrow_mut()
                            .insert(name.to_string().into(), Value::Scalar(value.into()));
                        Ok(())
                    }
                    _ => Err(format!("Variable {} not a bool", name).into()),
                }
            }
        });

        module.set_native_fn("set", {
            move |name: &str, value: Array| -> Result<()> {
                match liquid_object.get_value(name) {
                    NamedValue::NonExistant => {
                        let val = rhai_to_liquid_value(Dynamic::from(value))?;
                        liquid_object
                            .borrow_mut()
                            .insert(name.to_string().into(), val);
                        Ok(())
                    }
                    _ => Err(format!("Variable {} not an array", name).into()),
                }
            }
        });

        module.set_native_fn("prompt", {
            move |prompt: &str, default_value: bool| -> Result<bool> {
                let value = Variable::prompt(&Variable {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VariableInfo::Bool {
                        default: Some(default_value),
                    },
                });

                match value {
                    Ok(v) => Ok(v.parse::<bool>().map_err(|_| "Unable to parse into bool")?),
                    Err(e) => Err(e.to_string().into()),
                }
            }
        });

        module.set_native_fn("prompt", {
            move |prompt: &str| -> Result<String> {
                let value = Variable::prompt(&Variable {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VariableInfo::String {
                        entry: Box::new(StringEntry {
                            default: None,
                            choices: None,
                            regex: None,
                        }),
                    },
                });

                match value {
                    Ok(v) => Ok(v),
                    Err(e) => Err(e.to_string().into()),
                }
            }
        });

        module.set_native_fn("prompt", {
            move |prompt: &str, default_value: &str| -> Result<String> {
                let value = Variable::prompt(&Variable {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VariableInfo::String {
                        entry: Box::new(StringEntry {
                            default: Some(default_value.into()),
                            choices: None,
                            regex: None,
                        }),
                    },
                });

                match value {
                    Ok(v) => Ok(v),
                    Err(e) => Err(e.to_string().into()),
                }
            }
        });

        module.set_native_fn("prompt", {
            move |prompt: &str, default_value: &str, regex: &str| -> Result<String> {
                let value = Variable::prompt(&Variable {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VariableInfo::String {
                        entry: Box::new(StringEntry {
                            default: Some(default_value.into()),
                            choices: None,
                            regex: Some(Regex::new(regex).map_err(|_| "Invalid regex")?),
                        }),
                    },
                });

                match value {
                    Ok(v) => Ok(v),
                    Err(e) => Err(e.to_string().into()),
                }
            }
        });

        module.set_native_fn("prompt", {
            move |prompt: &str, default_value: &str, choices: rhai::Array| -> Result<String> {
                let value = Variable::prompt(&Variable {
                    prompt: prompt.into(),
                    var_name: "".into(),
                    var_info: VariableInfo::String {
                        entry: Box::new(StringEntry {
                            default: Some(default_value.into()),
                            choices: Some(
                                choices
                                    .iter()
                                    .map(|d| d.to_owned().into_string().unwrap())
                                    .collect(),
                            ),
                            regex: None,
                        }),
                    },
                });

                match value {
                    Ok(v) => Ok(v),
                    Err(e) => Err(e.to_string().into()),
                }
            }
        });

        module
    }

    enum NamedValue {
        NonExistant,
        Bool(bool),
        String(String),
    }

    trait GetNamedValue {
        fn get_value(&self, name: &str) -> NamedValue;
    }

    impl GetNamedValue for Rc<RefCell<Object>> {
        fn get_value(&self, name: &str) -> NamedValue {
            match self.borrow().get(name) {
                Some(value) => value
                    .as_scalar()
                    .map(|scalar| {
                        scalar.to_bool().map_or_else(
                            || {
                                let v = scalar.to_kstr();
                                NamedValue::String(String::from(v.as_str()))
                            },
                            NamedValue::Bool,
                        )
                    })
                    .unwrap_or_else(|| NamedValue::NonExistant),
                None => NamedValue::NonExistant,
            }
        }
    }

    fn rhai_to_liquid_value(val: Dynamic) -> Result<Value> {
        val.as_bool()
            .map(Into::into)
            .map(Value::Scalar)
            .or_else(|_| val.clone().into_string().map(Into::into).map(Value::Scalar))
            .or_else(|_| {
                val.clone()
                    .try_cast::<Array>()
                    .ok_or_else(|| {
                        format!(
                            "expecting type to be string, bool or array but found a '{}' instead",
                            val.type_name()
                        )
                        .into()
                    })
                    .and_then(|arr| {
                        arr.into_iter()
                            .map(rhai_to_liquid_value)
                            .collect::<Result<_>>()
                            .map(Value::Array)
                    })
            })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_rhai_set() {
            let mut engine = rhai::Engine::new();
            let liquid_object = Rc::new(RefCell::new(liquid::Object::new()));

            let module = create_module(liquid_object.clone());
            engine.register_static_module("variable", module.into());

            engine
                .eval::<()>(
                    r#"
                let dependencies = ["some_dep", "other_dep"];

                variable::set("dependencies", dependencies);
            "#,
                )
                .unwrap();

            let liquid_object = liquid_object.borrow();

            assert_eq!(
                liquid_object.get("dependencies"),
                Some(&Value::Array(vec![
                    Value::Scalar("some_dep".into()),
                    Value::Scalar("other_dep".into())
                ]))
            );
        }
    }
}

mod file_mod {
    use console::style;
    use path_absolutize::Absolutize;
    use rhai::{Array, EvalAltResult, Module};
    use std::io::Write;
    use std::path::{Path, PathBuf};

    type Result<T> = anyhow::Result<T, Box<EvalAltResult>>;

    pub fn create_module(dir: &Path) -> Module {
        let dir = dir.to_owned();
        let mut module = Module::new();

        module.set_native_fn("rename", {
            let dir = dir.clone();

            move |from: &str, to: &str| -> Result<()> {
                let from = to_absolute_path(&dir, from)?;
                let to = to_absolute_path(&dir, to)?;
                std::fs::rename(from, to).map_err(|e| e.to_string())?;
                Ok(())
            }
        });

        module.set_native_fn("delete", {
            let dir = dir.clone();

            move |file: &str| -> Result<()> {
                let file = to_absolute_path(&dir, file)?;
                if file.is_file() {
                    std::fs::remove_file(file).map_err(|e| e.to_string())?;
                } else {
                    std::fs::remove_dir_all(file).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
        });

        module.set_native_fn("write", {
            let dir = dir.clone();

            move |file: &str, content: &str| -> Result<()> {
                let file = to_absolute_path(&dir, file)?;
                std::fs::write(file, content).map_err(|e| e.to_string())?;
                Ok(())
            }
        });

        module.set_native_fn("write", {
            move |file: &str, content: Array| -> Result<()> {
                let file = to_absolute_path(&dir, file)?;
                let mut file = std::fs::File::create(file).map_err(|e| e.to_string())?;
                for v in content.iter() {
                    writeln!(file, "{}", v).map_err(|e| e.to_string())?;
                }

                Ok(())
            }
        });

        module
    }

    fn to_absolute_path(base_dir: &Path, relative_path: &str) -> Result<PathBuf> {
        let joined = base_dir.join(relative_path);
        Ok(joined
            .absolutize_virtually(base_dir)
            .map_err(|_| invalid_path(relative_path))?
            .into_owned())
    }

    fn invalid_path(path: &str) -> String {
        format!(
            "{} {}",
            style("Path must be inside template dir:").bold().red(),
            style(path).yellow(),
        )
    }
}
