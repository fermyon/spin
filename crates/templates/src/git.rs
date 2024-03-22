use std::io::ErrorKind;

// TODO: the following and the second half of plugins/git.rs are duplicates

pub(crate) enum GitError {
    ProgramFailed(Vec<u8>),
    ProgramNotFound,
    Other(anyhow::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProgramNotFound => f.write_str("`git` command not found - is git installed?"),
            Self::Other(e) => e.fmt(f),
            Self::ProgramFailed(stderr) => match std::str::from_utf8(stderr) {
                Ok(s) => f.write_str(s),
                Err(_) => f.write_str("(cannot get error)"),
            },
        }
    }
}

pub(crate) trait UnderstandGitResult {
    fn understand_git_result(self) -> Result<Vec<u8>, GitError>;
}

impl UnderstandGitResult for Result<std::process::Output, std::io::Error> {
    fn understand_git_result(self) -> Result<Vec<u8>, GitError> {
        match self {
            Ok(output) => {
                if output.status.success() {
                    Ok(output.stdout)
                } else {
                    Err(GitError::ProgramFailed(output.stderr))
                }
            }
            Err(e) => match e.kind() {
                // TODO: consider cases like insufficient permission?
                ErrorKind::NotFound => Err(GitError::ProgramNotFound),
                _ => {
                    let err = anyhow::Error::from(e).context("Failed to run `git` command");
                    Err(GitError::Other(err))
                }
            },
        }
    }
}
