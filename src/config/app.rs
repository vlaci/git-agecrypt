use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

use crate::age;

use super::Result;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct RecipientEntry {
    paths: Vec<PathBuf>,
    recipients: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    config: HashMap<PathBuf, Vec<String>>,
    #[serde(skip)]
    path: PathBuf,
    #[serde(skip)]
    prefix: PathBuf,
}

impl AppConfig {
    pub fn load(path: &Path, repo_prefix: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(contents) => {
                let mut cfg: AppConfig = toml::from_str(&contents).with_context(|| {
                    format!("Couldn't load configuration file '{}'", path.display())
                })?;
                cfg.path = path.into();
                cfg.prefix = repo_prefix.into();
                Ok(cfg)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self {
                config: HashMap::new(),
                path: path.into(),
                prefix: repo_prefix.into(),
            }),
            Err(err) => Ok(Err(err).with_context(|| {
                format!("Couldn't read configuration file '{}'", path.display())
            })?),
        }
    }

    pub fn save(&self) -> Result<()> {
        let cfg = toml::to_string_pretty(self).context("Coldn't format configuration as TOML")?;
        fs::write(&self.path, cfg).with_context(|| {
            format!("Couldn't save configuration file '{}'", self.path.display())
        })?;
        Ok(())
    }

    pub fn add(&mut self, recipients: Vec<String>, paths: Vec<PathBuf>) -> Result<()> {
        age::validate_public_keys(&recipients)?;
        let invalid_paths: Vec<String> = paths
            .iter()
            .filter(|&p| !p.is_file())
            .map(|f| f.to_string_lossy().to_string())
            .collect();
        if !invalid_paths.is_empty() {
            return Err(anyhow!(
                "The follwing files doesn't exist: {}",
                invalid_paths.join(", ")
            )
            .into());
        }
        for path in paths {
            let entry = self.config.entry(path).or_default();
            entry.extend(recipients.clone().into_iter());
            entry.dedup();
        }
        Ok(())
    }

    pub fn remove(&mut self, recipients: Vec<String>, paths: Vec<PathBuf>) -> Result<()> {
        if paths.is_empty() {
            for rs in self.config.values_mut() {
                rs.retain(|r| !recipients.contains(r));
            }
        } else {
            for path in paths {
                let rs = self.config.get_mut(&path).with_context(|| {
                    format!("No configuration entry found for {}", path.display())
                })?;
                if recipients.is_empty() {
                    rs.clear();
                } else {
                    rs.retain(|r| !recipients.contains(r));
                }
            }
        }

        self.config.retain(|_, rs| !rs.is_empty());

        Ok(())
    }

    pub fn list(&self) -> Vec<(String, String)> {
        let mut rv = vec![];
        for (p, rs) in &self.config {
            for r in rs {
                rv.push((p.to_string_lossy().to_string(), r.clone()));
            }
        }
        rv
    }

    pub fn get_public_keys(&self, path: &Path) -> Result<&[String]> {
        let pubk = self
            .config
            .get(path.strip_prefix(&self.prefix).with_context(|| {
                format!(
                    "Not a path inside git repository, path={path:?}, repo={:?}",
                    self.prefix
                )
            })?)
            .with_context(|| format!("No public key can be found for '{}'", path.display()))?;
        Ok(&pubk[..])
    }
}
