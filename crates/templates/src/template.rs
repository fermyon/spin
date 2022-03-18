use crate::{
    args::TemplateArgs,
    config::TemplateConfig,
    emoji,
    files::{self, FileMatcher, ShouldInclude},
    filters, hooks, progressbar, spinner, CONFIG_FILE_NAME,
};
use anyhow::{Context, Result};
use console::style;
use git2::{Config as GitConfig, Repository as GitRepository};
use indicatif::{MultiProgress, ProgressBar};
use liquid_core::Value;
use std::{cell::RefCell, collections::HashMap, fs, path::Path, rc::Rc};
use walkdir::{DirEntry, WalkDir};

pub(crate) async fn expand(
    template_dir: impl AsRef<Path>,
    mut config: TemplateConfig,
    args: &TemplateArgs,
    overrides: HashMap<String, toml::Value>,
) -> Result<()> {
    let object = config.object(args.noprompt, overrides)?;
    let mut obj = Rc::new(RefCell::new(object));

    let tpl_dir = template_dir.as_ref();

    hooks::exec_pre(&mut config, &tpl_dir, Rc::clone(&obj))?;
    files::remove_unneeded_files(tpl_dir, &config.ignore, args.verbose)?;

    let mut pbar = progressbar();

    // SAFETY: We gave a clone of the Rc to `execute_pre_hooks` which by now has already been dropped. Therefore, there
    // is no other pointer into this Rc which makes it safe to `get_mut`.
    let obj_ref = Rc::get_mut(&mut obj).unwrap().get_mut();

    println!(
        "{} {} {}",
        emoji::WRENCH,
        style("Expanding template").bold(),
        style("...").bold()
    );

    walk_dir(tpl_dir, &mut config, obj_ref, &mut pbar)?;

    hooks::exec_post(&config, &tpl_dir, Rc::clone(&obj))?;
    files::remove_dir_files(config.all_hooks(), false);

    pbar.join().unwrap();

    Ok(())
}

fn parser() -> liquid::Parser {
    liquid::ParserBuilder::with_stdlib()
        .filter(filters::KebabCaseFilterParser)
        .filter(filters::PascalCaseFilterParser)
        .filter(filters::SnakeCaseFilterParser)
        .build()
        .expect("can't fail due to no partials support")
}

fn walk_dir(
    tpl_dir: &Path,
    config: &mut TemplateConfig,
    object: &liquid::Object,
    mp: &mut MultiProgress,
) -> Result<()> {
    fn is_git_metadata(entry: &DirEntry) -> bool {
        entry
            .path()
            .components()
            .any(|c| c == std::path::Component::Normal(".git".as_ref()))
    }

    let mut permanently_excluded = config.all_hooks();
    permanently_excluded.push(CONFIG_FILE_NAME.to_string());

    let matcher = FileMatcher::new(config, tpl_dir, &permanently_excluded)?;
    let parser = parser();
    let spinner = spinner();

    let mut files_with_errors = vec![];
    let inputs = WalkDir::new(&tpl_dir)
        .sort_by_file_name()
        .contents_first(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !is_git_metadata(e))
        .filter(|e| e.path() != tpl_dir)
        .collect::<Vec<_>>();

    let total = inputs.len().to_string();

    let src = &tpl_dir;

    for (progress, entry) in inputs.into_iter().enumerate() {
        let pb = mp.add(ProgressBar::new(50));
        pb.set_style(spinner.clone());
        pb.set_prefix(format!(
            "[{:width$}/{}]",
            progress + 1,
            total,
            width = total.len()
        ));

        let file_path = entry.path();
        let rel_path = file_path.strip_prefix(&src)?;
        let f = rel_path.display();
        pb.set_message(format!("Processing: {}", f));

        match matcher.should_include(rel_path) {
            ShouldInclude::Include => {
                if entry.file_type().is_file() {
                    match process_template_file(object, &parser, file_path) {
                        Ok(new_contents) => {
                            let new_filename =
                                files::substitute_filename(file_path, &parser, object)
                                    .with_context(|| {
                                        format!(
                                            "{} {} `{}`",
                                            emoji::ERROR,
                                            style("Error templating a filename").bold().red(),
                                            style(file_path.display()).bold()
                                        )
                                    })?;
                            pb.inc(25);
                            let relative_path = new_filename.strip_prefix(src)?;
                            let f = relative_path.display();
                            fs::create_dir_all(new_filename.parent().unwrap()).unwrap();
                            fs::write(new_filename.as_path(), new_contents).with_context(|| {
                                format!(
                                    "{} {} `{}`",
                                    emoji::ERROR,
                                    style("Error writing rendered file.").bold().red(),
                                    style(new_filename.display()).bold()
                                )
                            })?;
                            pb.inc(50);
                            pb.finish_with_message(format!("Done: {}", f));
                        }
                        Err(e) => {
                            files_with_errors.push((file_path.display().to_string(), e.clone()));
                        }
                    }
                } else {
                    let new_filename = files::substitute_filename(file_path, &parser, object)?;
                    let relative_path = new_filename.strip_prefix(&src)?;
                    let f = relative_path.display();
                    pb.inc(50);
                    if file_path != new_filename {
                        fs::remove_dir_all(file_path)?;
                    }
                    pb.inc(50);
                    pb.finish_with_message(format!("Done: {}", f));
                }
            }
            ShouldInclude::Exclude => {
                pb.finish_with_message(format!("Skipped: {}", f));
            }
            ShouldInclude::Ignore => {
                pb.finish_with_message(format!("Ignored: {}", f));
            }
        }
    }

    if !files_with_errors.is_empty() {
        print_files_with_errors_warning(files_with_errors);
    }

    Ok(())
}

pub struct Authors {
    pub author: String,
    pub username: String,
}

/// Taken from cargo and thus (c) 2020 Cargo Developers
///
/// cf. <https://github.com/rust-lang/cargo/blob/2d5c2381e4e50484bf281fc1bfe19743aa9eb37a/src/cargo/ops/cargo_new.rs#L769-L851>
pub fn get_authors() -> Result<Authors> {
    fn get_environment_variable(variables: &[&str]) -> Option<String> {
        variables
            .iter()
            .filter_map(|var| std::env::var(var).ok())
            .next()
    }

    fn discover_author() -> Result<(String, Option<String>)> {
        let git_config = find_real_git_config();
        let git_config = git_config.as_ref();

        let name_variables = [
            "GIT_AUTHOR_NAME",
            "GIT_COMMITTER_NAME",
            "USER",
            "USERNAME",
            "NAME",
        ];
        let name = get_environment_variable(&name_variables[0..3])
            .or_else(|| git_config.and_then(|g| g.get_string("user.name").ok()))
            .or_else(|| get_environment_variable(&name_variables[3..]));

        let name = match name {
            Some(name) => name,
            None => {
                let username_var = if cfg!(windows) { "USERNAME" } else { "USER" };
                anyhow::bail!(
                    "could not determine the current user, please set ${}",
                    username_var
                )
            }
        };
        let email_variables = ["GIT_AUTHOR_EMAIL", "GIT_COMMITTER_EMAIL", "EMAIL"];
        let email = get_environment_variable(&email_variables[0..3])
            .or_else(|| git_config.and_then(|g| g.get_string("user.email").ok()))
            .or_else(|| get_environment_variable(&email_variables[3..]));

        let name = name.trim().to_string();
        let email = email.map(|s| {
            let mut s = s.trim();

            // In some cases emails will already have <> remove them since they
            // are already added when needed.
            if s.starts_with('<') && s.ends_with('>') {
                s = &s[1..s.len() - 1];
            }

            s.to_string()
        });

        Ok((name, email))
    }

    fn find_real_git_config() -> Option<GitConfig> {
        match std::env::current_dir() {
            Ok(cwd) => GitRepository::discover(cwd)
                .and_then(|repo| repo.config())
                .or_else(|_| GitConfig::open_default())
                .ok(),
            Err(_) => GitConfig::open_default().ok(),
        }
    }

    let author = match discover_author()? {
        (name, Some(email)) => Authors {
            author: format!("{} <{}>", name, email),
            username: name,
        },
        (name, None) => Authors {
            author: name.clone(),
            username: name,
        },
    };

    Ok(author)
}

pub(crate) fn render_string_gracefully(
    object: &liquid::Object,
    parser: &liquid::Parser,
    content: &str,
) -> liquid_core::Result<String> {
    let template = parser.parse(content)?;

    match template.render(object) {
        Ok(content) => Ok(content),
        Err(e) => {
            // handle it gracefully
            let msg = e.to_string();
            if msg.contains("requested variable") {
                // so, we miss a variable that is present in the file to render
                let requested_var =
                    regex::Regex::new(r"(?P<p>.*requested\svariable=)(?P<v>.*)").unwrap();
                let captures = requested_var.captures(msg.as_str()).unwrap();

                if let Some(Some(req_var)) = captures.iter().last() {
                    let missing_variable = req_var.as_str().to_string();
                    // try again with this variable added to the context
                    let mut context = object.clone();
                    context.insert(missing_variable.into(), Value::scalar("".to_string()));

                    // now let's parse again to see if we have all variables declared now
                    return render_string_gracefully(&context, parser, content);
                }
            }

            // todo: find nice way to have this happening outside of this fn
            // println!(
            //     "{} {} `{}`",
            //     emoji::ERROR,
            //     style("Error rendering template, file has been copied without rendering.")
            //         .bold()
            //         .red(),
            //     style(filename.display()).bold()
            // );
            // todo: end

            // fallback: no rendering, keep things original
            Ok(content.to_string())
        }
    }
}

fn process_template_file(
    object: &liquid::Object,
    parser: &liquid::Parser,
    file_path: impl AsRef<Path>,
) -> liquid_core::Result<String> {
    let content =
        fs::read_to_string(file_path).map_err(|e| liquid_core::Error::with_msg(e.to_string()))?;
    render_string_gracefully(object, parser, content.as_str())
}

fn print_files_with_errors_warning(files_with_errors: Vec<(String, liquid_core::Error)>) {
    let mut msg = format!(
        "\n{} {}",
        emoji::WARN,
        style("Substitution skipped, found invalid syntax in\n")
            .bold()
            .red(),
    );
    for file_error in files_with_errors {
        msg.push('\t');
        msg.push_str(&file_error.0);
        msg.push('\n');
    }
    let read_more = "Learn more: https://github.com/fermyon/spin#include--exclude.\n\n";
    let hint = style("Consider adding these files to a `spin-generate.toml` in the template repo to skip substitution on these files.").bold();

    println!("{}\n{}\n\n{}", msg, hint, read_more);
}
