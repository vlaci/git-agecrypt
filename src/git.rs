use std::{
    env, io,
    path::{Path, PathBuf},
    process,
};

use anyhow::{anyhow, Context};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{:?} already exists.", .0)]
    AlreadyExists(String),
    #[error("{:?} doesn't exist.", .0)]
    NotExist(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Self {
        Self::Other(anyhow!(err))
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Other(anyhow!(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) trait Repository {
    fn workdir(&self) -> &Path;

    fn path(&self) -> &Path;

    fn get_file_contents(&self, path: &Path) -> Result<Vec<u8>>;

    fn add_config(&self, key: &str, value: &str) -> Result<()>;

    fn contains_config(&self, key: &str, value: &str) -> bool;

    fn remove_config(&self, key: &str, value: &str) -> Result<()>;

    fn list_config(&self, key: &str) -> Result<Vec<String>>;

    fn set_config(&self, key: &str, value: &str) -> Result<()>;

    fn remove_config_section(&self, key: &str) -> Result<()>;

    fn list_filtered_files(&self, pathspecs: &[PathBuf]) -> Result<Vec<PathBuf>>;

    fn renormalize(&self, paths: &[PathBuf]) -> Result<()>;
}

pub(crate) struct LibGit2Repository {
    inner: git2::Repository,
}

impl LibGit2Repository {
    pub(crate) fn from_current_dir() -> Result<Self> {
        Self::from_dir(env::current_dir().context("Cannot determine current directory")?)
    }

    pub(crate) fn from_dir(path: PathBuf) -> Result<Self> {
        let inner = git2::Repository::discover(&path)
            .with_context(|| format!("'{}' Not a git repository", path.display()))?;
        if inner.is_bare() {
            return Err(anyhow!("Bare repositories are unsupported {}", path.display(),).into());
        }
        Ok(Self { inner })
    }
}

impl Repository for LibGit2Repository {
    fn workdir(&self) -> &Path {
        self.inner.workdir().unwrap() // None in case of bare repo
    }

    fn path(&self) -> &Path {
        self.inner.path()
    }

    fn get_file_contents(&self, path: &Path) -> Result<Vec<u8>> {
        let relpath = path.strip_prefix(self.workdir()).with_context(|| {
            format!(
                "Path {} is outside of git repository {}",
                path.display(),
                self.workdir().display()
            )
        })?;
        let head = match self.inner.head() {
            Ok(head) => head,
            Err(e)
                if matches!(
                    e.code(),
                    git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
                ) =>
            {
                return Err(Error::NotExist(
                    "HEAD does not point to a commit yet".into(),
                ))
            }
            Err(e) => {
                return Err(Error::Other(
                    anyhow!(e).context("Could not determine repository head"),
                ))
            }
        };
        let entry = head
            .peel_to_tree()?
            .get_path(relpath)
            .map_err(|e| match e.code() {
                git2::ErrorCode::NotFound => {
                    Error::NotExist(format!("Path {} is not found in HEAD", relpath.display(),))
                }
                _ => Error::Other(e.into()),
            })?;
        let contents = entry.to_object(&self.inner)?;
        let blob = contents.as_blob().ok_or_else(|| {
            Error::NotExist(format!("Path {} is not a file in HEAD", relpath.display()))
        })?;

        Ok(blob.content().into())
    }

    fn add_config(&self, key: &str, value: &str) -> Result<()> {
        if self.contains_config(key, value) {
            return Err(Error::AlreadyExists(value.into()));
        }

        let mut cfg = self.inner.config()?;

        cfg.set_multivar(key, "^$", value)?;

        Ok(())
    }

    fn contains_config(&self, key: &str, value: &str) -> bool {
        let entries = self.list_config(key).unwrap_or_default();
        entries.iter().any(|e| e == value)
    }

    fn remove_config(&self, key: &str, value: &str) -> Result<()> {
        if !self.contains_config(key, value) {
            return Err(Error::NotExist(value.into()));
        }

        let mut cfg = self.inner.config()?;
        let pattern = format!("^{}$", regex::escape(value));
        cfg.remove_multivar(key, &pattern)?;

        Ok(())
    }

    fn list_config(&self, key: &str) -> Result<Vec<String>> {
        let cfg = self.inner.config()?;
        let mut entries = Vec::new();

        match cfg.multivar(key, None) {
            Ok(entry_iter) => {
                entry_iter.for_each(|e| {
                    if let Some(v) = e.value() {
                        entries.push(v.into())
                    }
                })?;
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => {}
            Err(e) => return Err(Error::Other(anyhow!(e))),
        };

        Ok(entries)
    }

    fn set_config(&self, key: &str, value: &str) -> Result<()> {
        for v in self.list_config(key)? {
            self.remove_config(key, &v)?;
        }
        let mut cfg = self.inner.config()?;
        cfg.set_str(key, value)?;
        Ok(())
    }

    fn remove_config_section(&self, key: &str) -> Result<()> {
        // Unfortunately there is no `git config --remove-section <section>` equivalent in libgit2
        let mut command = process::Command::new("git");
        command
            .current_dir(self.workdir())
            .arg("config")
            .arg("--remove-section")
            .arg(key);
        let output = command.output()?;

        if !output.status.success() {
            log::error!(
                "Failed to execute command. This may not be an issue; command='{:?}' status='{}', stdout={:?}, stderr={:?}",
                command,
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(Error::NotExist(key.into()));
        }
        Ok(())
    }

    fn list_filtered_files(&self, pathspecs: &[PathBuf]) -> Result<Vec<PathBuf>> {
        // Collect tracked files matching the given pathspecs
        let ls_output = if pathspecs.is_empty() {
            let mut cmd = process::Command::new("git");
            cmd.current_dir(self.workdir()).arg("ls-files").arg("-z");
            cmd.output()?
        } else {
            let mut cmd = process::Command::new("git");
            cmd.arg("ls-files")
                .arg("-z")
                .arg("--full-name")
                .arg("--error-unmatch")
                .arg("--");
            for p in pathspecs {
                cmd.arg(p);
            }
            let output = cmd.output()?;
            if !output.status.success() {
                return Err(Error::Other(anyhow::anyhow!(
                    "git ls-files failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            output
        };

        let paths: Vec<PathBuf> = ls_output
            .stdout
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| {
                #[cfg(unix)]
                {
                    use std::os::unix::ffi::OsStrExt;
                    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(s)))
                }
                #[cfg(not(unix))]
                {
                    let s = String::from_utf8(s.to_vec())
                        .map_err(|e| Error::Other(anyhow::anyhow!("Non-UTF-8 path: {e}")))?;
                    Ok(PathBuf::from(s))
                }
            })
            .collect::<std::result::Result<Vec<_>, Error>>()?;

        if paths.is_empty() {
            return Ok(vec![]);
        }

        // Filter to only those with the git-agecrypt filter attribute
        let mut child = process::Command::new("git")
            .current_dir(self.workdir())
            .arg("check-attr")
            .arg("--stdin")
            .arg("-z")
            .arg("filter")
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::inherit())
            .spawn()?;

        let mut stdin = child.stdin.take().unwrap();
        let handle = std::thread::spawn(move || -> std::io::Result<()> {
            use std::io::Write;
            for p in &paths {
                let mut buf = Vec::new();
                #[cfg(unix)]
                {
                    use std::os::unix::ffi::OsStrExt;
                    buf.extend_from_slice(p.as_os_str().as_bytes());
                }
                #[cfg(not(unix))]
                {
                    let s = p.to_string_lossy();
                    buf.extend_from_slice(s.as_bytes());
                }
                buf.push(0);
                stdin.write_all(&buf)?;
            }
            Ok(())
        });

        let output = child.wait_with_output()?;
        handle
            .join()
            .map_err(|_| Error::Other(anyhow::anyhow!("write thread panicked")))??;

        // Parse triples: path\0attr\0value\0
        let stdout = output.stdout;
        let mut filtered = Vec::new();
        let mut i = 0;
        while i + 2 < stdout.len() {
            let path_end = stdout[i..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(stdout.len() - i);
            let attr_start = i + path_end + 1;
            let attr_end = stdout[attr_start..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(stdout.len() - attr_start);
            let val_start = attr_start + attr_end + 1;
            let val_end = stdout[val_start..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(stdout.len() - val_start);

            let value = &stdout[val_start..val_start + val_end];
            if value == b"git-agecrypt" {
                #[cfg(unix)]
                {
                    use std::os::unix::ffi::OsStrExt;
                    filtered.push(PathBuf::from(std::ffi::OsStr::from_bytes(
                        &stdout[i..i + path_end],
                    )));
                }
                #[cfg(not(unix))]
                {
                    filtered.push(PathBuf::from(
                        String::from_utf8_lossy(&stdout[i..i + path_end]).as_ref(),
                    ));
                }
            }

            i = val_start + val_end + 1;
        }

        Ok(filtered)
    }

    fn renormalize(&self, paths: &[PathBuf]) -> Result<()> {
        let mut cmd = process::Command::new("git");
        cmd.current_dir(self.workdir())
            .arg("--literal-pathspecs")
            .arg("add")
            .arg("--renormalize")
            .arg("--");
        for p in paths {
            cmd.arg(p);
        }
        let status = cmd.status()?;
        if !status.success() {
            return Err(Error::Other(anyhow::anyhow!(
                "git add --renormalize failed with {status}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, fs, ops::Deref};

    use anyhow::Result;
    use assert_fs::prelude::*;
    use assert_fs::{fixture::ChildPath, TempDir};
    use assert_matches::assert_matches;
    use rstest::fixture;
    use rstest::rstest;

    use super::*;

    struct Sandbox {
        _tmp: TempDir,
        home: PathBuf,
        work: PathBuf,
    }

    impl Sandbox {
        fn git(&self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) {
            snapbox::cmd::Command::new("git")
                .env_clear()
                .env("PATH", std::env::var_os("PATH").unwrap_or_default())
                .env("HOME", &self.home)
                .env("XDG_CONFIG_HOME", self.home.join(".config"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .current_dir(&self.work)
                .args(args)
                .assert()
                .success();
        }
    }

    #[fixture]
    fn sandbox() -> Sandbox {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let work = tmp.path().join("work");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&work).unwrap();
        fs::write(
            home.join(".gitconfig"),
            "[user]\n\tname = A U Thor\n\temail = author@example.com\n[init]\n\tdefaultBranch = main\n",
        )
        .unwrap();
        Sandbox {
            _tmp: tmp,
            home,
            work,
        }
    }

    struct Repo {
        inner: LibGit2Repository,
        sb: Sandbox,
    }

    impl Deref for Repo {
        type Target = LibGit2Repository;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    #[fixture]
    fn git_repo(sandbox: Sandbox) -> Repo {
        sandbox.git(["init"]);
        let inner = LibGit2Repository::from_dir(sandbox.work.clone()).unwrap();
        Repo { inner, sb: sandbox }
    }

    #[rstest]
    fn test_repo_can_be_loaded(git_repo: Repo) -> Result<()> {
        assert_eq!(git_repo.workdir(), git_repo.sb.work);
        assert_eq!(git_repo.path(), git_repo.sb.work.join(".git"));
        Ok(())
    }

    #[rstest]
    fn test_repo_required(sandbox: Sandbox) -> Result<()> {
        let repo = LibGit2Repository::from_dir(sandbox.work.clone()).err();
        assert_matches!(repo, Some(Error::Other(_)));
        Ok(())
    }

    #[rstest]
    fn test_bare_repo_is_error(sandbox: Sandbox) -> Result<()> {
        sandbox.git(["init", "--bare"]);
        let repo = LibGit2Repository::from_dir(sandbox.work.clone()).err();
        assert_matches!(repo, Some(Error::Other(_)));
        Ok(())
    }

    #[rstest]
    fn test_get_file_contents(git_repo: Repo) -> Result<()> {
        let path = "subdir/file.txt";
        let file_contents = "file contents";

        let repo_file = ChildPath::new(git_repo.sb.work.join(path));
        repo_file.write_str(file_contents)?;
        git_repo.sb.git(["add", path]);
        git_repo.sb.git(["commit", "-m", "adding file"]);

        assert_eq!(
            git_repo.get_file_contents(&git_repo.sb.work.join(path))?,
            file_contents.as_bytes()
        );

        repo_file.write_str("additional_contents")?;
        assert_eq!(
            git_repo.get_file_contents(&git_repo.sb.work.join(path))?,
            file_contents.as_bytes()
        );

        assert!(git_repo
            .get_file_contents(git_repo.sb.work.parent().unwrap())
            .is_err());

        Ok(())
    }

    #[rstest]
    fn test_get_file_contents_on_unborn_head(git_repo: Repo) -> Result<()> {
        assert_matches!(
            git_repo.get_file_contents(&git_repo.sb.work.join("x")),
            Err(Error::NotExist(_))
        );
        Ok(())
    }

    #[rstest]
    fn test_unset_key_has_no_values(git_repo: Repo) -> Result<()> {
        assert_eq!(git_repo.list_config("foo.bar")?, [] as [String; 0]);
        assert!(!git_repo.contains_config("foo.bar", "foobar"));
        Ok(())
    }

    #[rstest]
    fn test_set_config_rejects_invalid_key(git_repo: Repo) {
        assert_matches!(git_repo.set_config("invalidkey", "foobar"), Err(_));
    }

    #[rstest]
    fn test_list_config_returns_all_values_of_exact_key(git_repo: Repo) -> Result<()> {
        git_repo.set_config("foo.bar", "foobar")?;
        git_repo.add_config("foo.bar", "snafu")?;
        git_repo.set_config("foo.barx", "other")?;

        assert!(git_repo.contains_config("foo.bar", "foobar"));
        assert!(git_repo.contains_config("foo.bar", "snafu"));
        assert_eq!(git_repo.list_config("foo.bar")?, ["foobar", "snafu"]);
        Ok(())
    }

    #[rstest]
    fn test_set_config_replaces_all_values(git_repo: Repo) -> Result<()> {
        git_repo.set_config("foo.bar", "foobar")?;
        git_repo.add_config("foo.bar", "snafu")?;

        git_repo.set_config("foo.bar", "FOOBAR")?;

        assert!(git_repo.contains_config("foo.bar", "FOOBAR"));
        assert_eq!(git_repo.list_config("foo.bar")?, ["FOOBAR"]);
        Ok(())
    }

    #[rstest]
    fn test_add_config_rejects_duplicate_value(git_repo: Repo) -> Result<()> {
        git_repo.set_config("foo.bar", "foobar")?;

        assert_matches!(
            git_repo.add_config("foo.bar", "foobar"),
            Err(Error::AlreadyExists(value)) if value == "foobar"
        );
        Ok(())
    }

    #[rstest]
    fn test_remove_config_rejects_absent_value(git_repo: Repo) -> Result<()> {
        git_repo.set_config("foo.bar", "foobar")?;
        git_repo.remove_config("foo.bar", "foobar")?;

        assert_matches!(
            git_repo.remove_config("foo.bar", "foobar"),
            Err(Error::NotExist(value)) if value == "foobar"
        );
        Ok(())
    }

    #[rstest]
    fn test_remove_config_section_removes_its_keys(git_repo: Repo) -> Result<()> {
        git_repo.set_config("foo.bar", "foobar")?;
        git_repo.set_config("foo.barx", "other")?;

        assert_matches!(git_repo.remove_config_section("foo"), Ok(()));
        assert_eq!(git_repo.list_config("foo.bar")?, [] as [String; 0]);
        Ok(())
    }
}
