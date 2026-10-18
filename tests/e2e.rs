// SPDX-FileCopyrightText: 2026 László Vaskó <opensource@vlaci.email>
//
// SPDX-License-Identifier: MPL-2.0

//! End-to-end tests driving real `git` with the built `git-agecrypt` binary.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use snapbox::cmd::{Command, OutputAssert};

use age::secrecy::ExposeSecret;

const BIN: &str = env!("CARGO_BIN_EXE_git-agecrypt");

struct Sandbox {
    tmp: Rc<assert_fs::TempDir>,
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
        let sb = Self {
            tmp: tmp.into(),
            home,
            repo,
        };
        sb.assert_run_git(["init"]);
        sb.run_agecrypt().arg("init").assert().success();
        sb
    }

    fn git_clone(&self, dir: &str) -> Self {
        let repo = &self.tmp.path().join(dir);
        self.assert_run_git(["clone", "--no-checkout", ".", repo.to_str().unwrap()]);
        Self {
            tmp: self.tmp.clone(),
            home: self.home.clone(),
            repo: repo.into(),
        }
    }

    fn command(&self, program: impl AsRef<OsStr>, dir: &Path) -> Command {
        let cmd = Command::new(program)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .current_dir(dir);

        match std::env::var_os("LLVM_PROFILE_FILE") {
            Some(file) => cmd.env("LLVM_PROFILE_FILE", file),
            None => cmd,
        }
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

    fn remove_recipient(&self, key: &Key, paths: &[&str]) {
        let mut args = vec!["config", "remove", "-r", &key.recipient];
        if !paths.is_empty() {
            args.push("-p");
            args.extend_from_slice(paths);
        }
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

    #[track_caller]
    fn assert_encrypted_to(&self, rev_path: &str, to: &Key, not_to: &Key) {
        let blob = self.blob(rev_path);
        assert!(
            decrypts_with(&blob, to),
            "{rev_path} should decrypt with the intended key"
        );
        assert!(
            !decrypts_with(&blob, not_to),
            "{rev_path} should NOT decrypt with the excluded key"
        );
    }

    fn remove_cache(&self) {
        fs::remove_dir_all(self.repo.join(".git/git-agecrypt")).unwrap();
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

#[test]
fn unrelated_identity_config_keys_are_ignored() {
    let sb = Sandbox::new();
    // Keys that merely contain "identity" in their name must not be read as age identities.
    sb.assert_run_git(["config", "--global", "advice.implicitIdentity", "false"]);
    sb.assert_run_git(["config", "user.signingidentity", "foo"]);

    let key = sb.key("k1");
    sb.add_identity(&key);
    sb.commit_secret(&key);

    fs::remove_file(sb.repo.join("secret")).unwrap();
    sb.assert_run_git(["checkout", "--", "secret"]);
    sb.assert_decrypted_secret();
}

#[test]
fn same_stem_and_same_directory_files_keep_their_own_recipients() {
    let sb = Sandbox::new();
    let key1 = sb.key("k1");
    let key2 = sb.key("k2");
    sb.add_identity(&key1);

    sb.write("a.json", "same\n");
    sb.write("a.yaml", "same\n");
    sb.write("conf.d/one", "same\n");
    sb.write("conf.d/two", "same\n");

    sb.add_recipient(&key1, &["a.json"]);
    sb.add_recipient(&key2, &["a.yaml"]);
    sb.add_recipient(&key1, &["conf.d/one"]);
    sb.add_recipient(&key2, &["conf.d/two"]);
    sb.set_gitattributes(&["a.json", "a.yaml", "conf.d/one", "conf.d/two"]);

    sb.assert_run_git([
        "add",
        ".gitattributes",
        "git-agecrypt.toml",
        "a.json",
        "a.yaml",
        "conf.d/one",
        "conf.d/two",
    ]);
    sb.assert_run_git(["commit", "-m", "add colliding files"]);

    sb.assert_encrypted_to("HEAD:a.yaml", &key2, &key1);
    sb.assert_encrypted_to("HEAD:conf.d/two", &key2, &key1);
}

#[test]
fn recipient_change_takes_effect_on_renormalize() {
    let sb = Sandbox::new();
    let key1 = sb.key("k1");
    let key2 = sb.key("k2");

    sb.add_identity(&key1);
    sb.add_identity(&key2);

    sb.commit_secret(&key1);

    sb.add_recipient(&key2, &["secret"]);
    sb.remove_recipient(&key1, &["secret"]);

    sb.assert_run_git(["add", "--renormalize", "."]);
    sb.assert_run_git(["commit", "-m", "rotate recipient"]);

    sb.assert_encrypted_to("HEAD:secret", &key2, &key1);
}

#[test]
fn fresh_clone_renormalize_produces_no_changes() {
    let sb = Sandbox::new();
    let key = sb.key("k1");
    sb.add_identity(&key);
    sb.commit_secret(&key);

    let clone = sb.git_clone("clone");

    clone.run_agecrypt().arg("init").assert().success();
    clone.add_identity(&key);
    clone.assert_run_git(["reset", "--hard", "HEAD"]);

    clone.assert_run_git(["add", "--renormalize", "."]);
    clone.assert_run_git(["diff", "--cached", "--quiet"]);
    clone
        .assert_run_git(["status", "--porcelain"])
        .stdout_eq("");
}

#[test]
fn rekey_reencrypts_after_cache_loss() {
    let sb = Sandbox::new();
    let key1 = sb.key("k1");
    let key2 = sb.key("k2");
    sb.add_identity(&key1);
    sb.add_identity(&key2);
    sb.commit_secret(&key1);

    sb.add_recipient(&key2, &["secret"]);
    sb.remove_recipient(&key1, &["secret"]);

    sb.remove_cache();

    sb.run_agecrypt()
        .arg("rekey")
        .assert()
        .success()
        .stdout_eq("Re-encrypted secret\n");

    sb.assert_run_git(["diff", "--cached", "--name-only"])
        .stdout_eq("secret\n");

    sb.assert_run_git(["commit", "-m", "rekey"]);

    sb.assert_encrypted_to("HEAD:secret", &key2, &key1);
}

#[test]
fn rekey_with_path_only_restages_that_path() {
    let sb = Sandbox::new();
    let key1 = sb.key("k1");
    let key2 = sb.key("k2");
    sb.add_identity(&key1);

    sb.write("s1", "first\n");
    sb.write("s2", "second\n");
    sb.add_recipient(&key1, &["s1", "s2"]);
    sb.set_gitattributes(&["s1", "s2"]);
    sb.assert_run_git(["add", ".gitattributes", "git-agecrypt.toml", "s1", "s2"]);
    sb.assert_run_git(["commit", "-m", "two encrypted files"]);

    sb.add_recipient(&key2, &["s1", "s2"]);
    sb.remove_recipient(&key1, &["s1", "s2"]);
    sb.remove_cache();

    sb.run_agecrypt()
        .args(["rekey", "s1"])
        .assert()
        .success()
        .stdout_eq("Re-encrypted s1\n");

    sb.assert_run_git(["diff", "--cached", "--name-only"])
        .stdout_eq("s1\n");
}

#[test]
fn rekey_without_matching_files_fails() {
    let sb = Sandbox::new();
    sb.run_agecrypt()
        .arg("rekey")
        .assert()
        .failure()
        .stderr_eq("Error: No tracked files using the git-agecrypt filter match the given paths\n");
}
