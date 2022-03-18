use crate::{config::TemplateConfig, emoji, template, CONFIG_FILE_NAME};
use anyhow::Result;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{self, Path, PathBuf};

#[derive(Default)]
pub(crate) struct FileMatcher(Option<FileMatcherKind>, Vec<String>);

pub(crate) enum ShouldInclude {
    Include,
    Exclude,
    Ignore,
}

pub(crate) enum FileMatcherKind {
    Include(Gitignore),
    Exclude(Gitignore),
}

impl FileMatcher {
    pub fn new(
        mut template_config: &mut TemplateConfig,
        project_dir: &Path,
        permanent_excluded: &[String],
    ) -> Result<Self> {
        if template_config.include.is_some() && template_config.exclude.is_some() {
            template_config.exclude = None;
            println!(
                "{0} Your {1} contains both an include and exclude list. \
                    Only the include list will be considered. \
                    You should remove the exclude list for clarity. {0}",
                emoji::WARN,
                CONFIG_FILE_NAME
            )
        }

        let kind = match (&template_config.exclude, &template_config.include) {
            (None, None) => None,
            (None, Some(it)) => Some(FileMatcherKind::Include(Self::create_matcher(
                project_dir,
                it,
            )?)),
            (Some(it), None) => Some(FileMatcherKind::Exclude(Self::create_matcher(
                project_dir,
                it,
            )?)),
            (Some(_), Some(_)) => unreachable!(
                "BUG: template config has both include and exclude specified: {:?}",
                template_config
            ),
        };
        Ok(Self(kind, permanent_excluded.into()))
    }

    fn create_matcher(project_dir: &Path, patterns: &[String]) -> Result<Gitignore> {
        let mut builder = GitignoreBuilder::new(project_dir);
        for rule in patterns {
            builder.add_line(None, rule)?;
        }
        Ok(builder.build()?)
    }

    pub fn should_include(&self, relative_path: &Path) -> ShouldInclude {
        if self
            .1
            .iter()
            .any(|e| relative_path.to_str().map(|p| p == e).unwrap_or_default())
        {
            return ShouldInclude::Ignore;
        }

        // "Include" and "exclude" options are mutually exclusive.
        // if no include is made, we will default to ignore_exclude
        // which if there is no options, matches everything
        if match &self.0 {
            Some(FileMatcherKind::Exclude(it)) => {
                !it.matched_path_or_any_parents(relative_path, /* is_dir */ false)
                    .is_ignore()
            }
            Some(FileMatcherKind::Include(it)) => {
                it.matched_path_or_any_parents(relative_path, /* is_dir */ false)
                    .is_ignore()
            }
            None => true,
        } {
            ShouldInclude::Include
        } else {
            ShouldInclude::Exclude
        }
    }
}

pub(crate) fn remove_dir_files(files: impl IntoIterator<Item = impl Into<PathBuf>>, verbose: bool) {
    for item in files
        .into_iter()
        .map(|i| i.into() as PathBuf)
        .filter(|file| file.exists())
    {
        let ignore_message = format!("Ignoring: {}", &item.display());
        if item.is_dir() {
            std::fs::remove_dir_all(&item).unwrap();
            if verbose {
                println!("{}", ignore_message);
            }
        } else if item.is_file() {
            std::fs::remove_file(&item).unwrap();
            if verbose {
                println!("{}", ignore_message);
            }
        } else {
            println!(
                "The given paths are neither files nor directories! {}",
                &item.display()
            );
        }
    }
}

/// Takes the directory path and removes the files/directories specified in the
/// `ignore` section of the `spin-generate.toml` file. It handles all errors internally.
pub(crate) fn remove_unneeded_files(
    path: &Path,
    ignored_files: &Option<Vec<String>>,
    verbose: bool,
) -> Result<()> {
    let mut ignored = vec![path.join(CONFIG_FILE_NAME)];
    if let Some(ignored_files) = ignored_files {
        for f in ignored_files {
            let mut p = PathBuf::new();
            p.push(path);
            p.push(f);
            ignored.push(p);
        }
    }
    remove_dir_files(&ignored, verbose);
    Ok(())
}

pub(crate) fn substitute_filename(
    filepath: &Path,
    parser: &liquid::Parser,
    context: &liquid::Object,
) -> Result<PathBuf> {
    let mut path = PathBuf::new();
    for elem in filepath.components() {
        match elem {
            path::Component::Normal(e) => {
                let parsed =
                    template::render_string_gracefully(context, parser, e.to_str().unwrap())?;
                let parsed = sanitize_filename(parsed.as_str());
                path.push(parsed);
            }
            other => path.push(other),
        }
    }
    Ok(path)
}

fn sanitize_filename(filename: &str) -> String {
    use sanitize_filename::sanitize_with_options;

    let options = sanitize_filename::Options {
        truncate: true,   // true by default, truncates to 255 bytes
        replacement: "_", // str to replace sanitized chars/strings
        ..sanitize_filename::Options::default()
    };

    sanitize_with_options(filename, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use liquid::model::Value;

    #[test]
    fn should_do_happy_path() {
        assert_eq!(
            substitute_filename("{{author}}.rs", prepare_context("sassman")).unwrap(),
            "sassman.rs"
        );
        #[cfg(unix)]
        assert_eq!(
            substitute_filename("/tmp/project/{{author}}.rs", prepare_context("sassman")).unwrap(),
            "/tmp/project/sassman.rs"
        );
        #[cfg(unix)]
        assert_eq!(
            substitute_filename(
                "/tmp/project/{{author}}/{{author}}.rs",
                prepare_context("sassman")
            )
            .unwrap(),
            "/tmp/project/sassman/sassman.rs"
        );
        #[cfg(windows)]
        assert_eq!(
            substitute_filename(
                "C:\\tmp\\project\\{{author}}.rs",
                prepare_context("sassman")
            )
            .unwrap(),
            "C:\\tmp\\project\\sassman.rs"
        );
        #[cfg(windows)]
        assert_eq!(
            substitute_filename(
                "C:\\tmp\\project\\{{author}}\\{{author}}.rs",
                prepare_context("sassman")
            )
            .unwrap(),
            "C:\\tmp\\project\\sassman\\sassman.rs"
        );
    }

    #[test]
    fn should_prevent_invalid_filenames() {
        #[cfg(unix)]
        assert_eq!(
            substitute_filename("/tmp/project/{{author}}.rs", prepare_context("s/a/s")).unwrap(),
            "/tmp/project/s_a_s.rs"
        );
        #[cfg(unix)]
        assert_eq!(
            substitute_filename(
                "/tmp/project/{{author}}/{{author}}.rs",
                prepare_context("s/a/s")
            )
            .unwrap(),
            "/tmp/project/s_a_s/s_a_s.rs"
        );
        #[cfg(windows)]
        assert_eq!(
            substitute_filename(
                "C:\\tmp\\project\\{{author}}.rs",
                prepare_context("s\\a\\s")
            )
            .unwrap(),
            "C:\\tmp\\project\\s_a_s.rs"
        );
        #[cfg(windows)]
        assert_eq!(
            substitute_filename(
                "C:\\tmp\\project\\{{author}}\\{{author}}.rs",
                prepare_context("s\\a\\s")
            )
            .unwrap(),
            "C:\\tmp\\project\\s_a_s\\s_a_s.rs"
        );
    }

    #[test]
    fn should_prevent_exploitation() {
        #[cfg(unix)]
        assert_eq!(
            substitute_filename(
                "/tmp/project/{{author}}.rs",
                prepare_context("../../etc/passwd")
            )
            .unwrap(),
            "/tmp/project/.._.._etc_passwd.rs"
        );
        #[cfg(unix)]
        assert_eq!(
            substitute_filename(
                "/tmp/project/{{author}}/main.rs",
                prepare_context("../../etc/passwd")
            )
            .unwrap(),
            "/tmp/project/.._.._etc_passwd/main.rs"
        );
        #[cfg(windows)]
        assert_eq!(
            substitute_filename(
                "C:\\tmp\\project\\{{author}}.rs",
                prepare_context("..\\..\\etc\\passwd")
            )
            .unwrap(),
            "C:\\tmp\\project\\.._.._etc_passwd.rs"
        );
        #[cfg(windows)]
        assert_eq!(
            substitute_filename(
                "C:\\tmp\\project\\{{author}}\\main.rs",
                prepare_context("..\\..\\etc\\passwd")
            )
            .unwrap(),
            "C:\\tmp\\project\\.._.._etc_passwd\\main.rs"
        );
    }

    fn prepare_context(value: &str) -> liquid::Object {
        let mut ctx = liquid::Object::default();
        ctx.entry("author")
            .or_insert(Value::scalar(value.to_string()));

        ctx
    }

    fn substitute_filename(f: &str, ctx: liquid::Object) -> Result<String> {
        let parser = liquid::Parser::default();

        super::substitute_filename(f.as_ref(), &parser, &ctx)
            .map(|p| p.to_str().unwrap().to_string())
    }
}
