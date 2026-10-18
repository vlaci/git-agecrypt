//! End-to-end tests driving real `git` with the built `git-agecrypt` binary.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use snapbox::cmd::{Command, OutputAssert};

use age::secrecy::ExposeSecret;

const BIN: &str = env!("CARGO_BIN_EXE_git-agecrypt");

struct Sandbox {
    tmp: assert_fs::TempDir,
    home: PathBuf,
    repo: PathBuf,
}

struct Key {
    identity: age::x25519::Identity,
    path: PathBuf,
    recipient: String,
}

impl Sandbox {
    fn new() -> Self {
        let sb = Self::unborn();
        sb.assert_run_git(["commit", "--allow-empty", "-m", "init"]);
        sb
    }

    fn unborn() -> Self {
        let tmp = assert_fs::TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let repo = tmp.path().join("repo");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&repo).unwrap();
        fs::write(
            home.join(".gitconfig"),
            "[user]\n\tname = Test\n\temail = test@example.com\n[init]\n\tdefaultBranch = main\n",
        )
        .unwrap();
        let sb = Self { tmp, home, repo };
        sb.assert_run_git(["init"]);
        sb.run_agecrypt().arg("init").assert().success();
        sb
    }

    fn command(&self, program: impl AsRef<OsStr>, dir: &Path) -> Command {
        Command::new(program)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .current_dir(dir)
    }

    fn assert_run_git(&self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> OutputAssert {
        self.command("git", &self.repo)
            .args(args)
            .assert()
            .success()
    }

    fn run_agecrypt(&self) -> Command {
        self.command(BIN, &self.repo)
    }

    fn key(&self, name: &str) -> Key {
        let identity = age::x25519::Identity::generate();
        let dir = self.tmp.path().join("keys");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, format!("{}\n", identity.to_string().expose_secret())).unwrap();
        let recipient = identity.to_public().to_string();
        Key {
            identity,
            path,
            recipient,
        }
    }

    fn write(&self, rel: &str, contents: &str) {
        let path = self.repo.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.repo.join(rel)).unwrap()
    }

    fn set_gitattributes(&self, paths: &[&str]) {
        let contents: String = paths
            .iter()
            .map(|p| format!("{p} filter=git-agecrypt diff=git-agecrypt\n"))
            .collect();
        self.write(".gitattributes", &contents);
    }

    fn blob(&self, rev_path: &str) -> Vec<u8> {
        self.assert_run_git(["cat-file", "blob", rev_path])
            .get_output()
            .stdout
            .clone()
    }

    fn add_identity(&self, key: &Key) {
        self.run_agecrypt()
            .args(["config", "add", "-i"])
            .arg(&key.path)
            .assert()
            .success();
    }

    fn add_recipient(&self, key: &Key, paths: &[&str]) {
        let mut args = vec!["config", "add", "-r", &key.recipient, "-p"];
        args.extend_from_slice(paths);
        self.run_agecrypt().args(args).assert().success();
    }

    fn commit_secret(&self, key: &Key) {
        self.write("secret", "top secret\n");
        self.add_recipient(key, &["secret"]);
        self.set_gitattributes(&["secret"]);
        self.assert_run_git(["add", ".gitattributes", "git-agecrypt.toml", "secret"]);
        self.assert_run_git(["commit", "-m", "add secret"]);
    }

    fn assert_decrypted_secret(&self) {
        assert_eq!(self.read("secret"), "top secret\n");
    }
}

fn decrypts_with(blob: &[u8], key: &Key) -> bool {
    let Ok(age::Decryptor::Recipients(d)) = age::Decryptor::new(blob) else {
        return false;
    };
    let Ok(mut r) = d.decrypt(std::iter::once(&key.identity as &dyn age::Identity)) else {
        return false;
    };
    let mut out = vec![];
    std::io::Read::read_to_end(&mut r, &mut out).is_ok()
}

#[test]
fn round_trip_encrypts_in_history_and_decrypts_on_checkout() {
    let sb = Sandbox::new();
    let key = sb.key("k1");
    sb.add_identity(&key);
    sb.commit_secret(&key);

    let blob = sb.blob("HEAD:secret");
    assert!(blob.starts_with(b"age-encryption.org/v1"));
    assert!(decrypts_with(&blob, &key));

    fs::remove_file(sb.repo.join("secret")).unwrap();
    sb.assert_run_git(["checkout", "--", "secret"]);
    sb.assert_decrypted_secret();
}

#[test]
fn first_add_on_unborn_branch_succeeds() {
    let sb = Sandbox::unborn();
    let key = sb.key("k1");
    sb.commit_secret(&key);

    assert!(sb.blob("HEAD:secret").starts_with(b"age-encryption.org/v1"));
}
