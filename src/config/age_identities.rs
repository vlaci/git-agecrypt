use std::{fmt::Display, path::PathBuf};

use anyhow::Context as AnyhowContext;

use crate::ctx::Context;

use super::{git::GitConfig, Container, Result, Validated};

pub(crate) struct AgeIdentity {
    pub path: String,
}

impl TryFrom<PathBuf> for AgeIdentity {
    type Error = anyhow::Error;

    fn try_from(value: PathBuf) -> std::result::Result<Self, Self::Error> {
        Ok(AgeIdentity {
            path: String::from(
                value
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Unsupported path {:?}", &value))?,
            ),
        })
    }
}

impl Display for AgeIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path.fmt(f)
    }
}

impl Validated for AgeIdentity {
    fn validate(&self) -> Result<()> {
        Ok(crate::age::validate_identity(&self.path)
            .with_context(|| format!("The file '{}' is not a valid age identity", self.path))?)
    }
}

pub(crate) struct AgeIdentities<'a, C: Context>(pub GitConfig<'a, C>);

impl<C: Context> Container for AgeIdentities<'_, C> {
    type Item = AgeIdentity;

    fn add(&mut self, identity: Self::Item) -> Result<()> {
        identity.validate()?;
        self.0.add(identity.path.into())?;
        Ok(())
    }

    fn remove(&mut self, identity: Self::Item) -> Result<()> {
        self.0.remove(identity.path.into())?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<Self::Item>> {
        let identities = self.0.list()?;
        Ok(identities
            .into_iter()
            .map(move |c| AgeIdentity { path: c.into() })
            .collect())
    }
}

impl<'a, C: Context> AgeIdentities<'a, C> {
    pub fn new(ctx: &'a C) -> Self {
        let cfg = GitConfig::new(ctx, "identity".into());
        Self(cfg)
    }
}
