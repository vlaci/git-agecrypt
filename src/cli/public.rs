use std::path::PathBuf;

use crate::Result;

use crate::config::Validated;
use crate::{
    config::{AgeIdentities, AgeIdentity, Container},
    ctx::Context,
};

pub(crate) struct CommandContext<C: Context> {
    ctx: C,
}

impl<C: Context> CommandContext<C> {
    pub fn new(ctx: C) -> Self {
        Self { ctx }
    }

    pub(crate) fn init(&self) -> Result<()> {
        self.ctx.configure_filter()?;
        Ok(())
    }

    pub(crate) fn deinit(&self) -> Result<()> {
        self.ctx.deconfigure_filter()?;
        self.ctx.remove_sidecar_files()?;
        Ok(())
    }

    pub(crate) fn list_identities(&self) -> Result<()> {
        self.print_identities()
    }

    pub(crate) fn status(&self) -> Result<()> {
        self.list_identities()
    }

    pub(crate) fn add_identity(&self, identity: PathBuf) -> Result<()> {
        AgeIdentities::new(&self.ctx).add(AgeIdentity::try_from(identity)?)?;
        Ok(())
    }

    pub(crate) fn remove_identity(&self, identity: PathBuf) -> Result<()> {
        AgeIdentities::new(&self.ctx).remove(AgeIdentity::try_from(identity)?)?;
        Ok(())
    }

    fn print_identities(&self) -> Result<()> {
        let identities = AgeIdentities::new(&self.ctx).list()?;

        let padding = identities.iter().map(|i| i.path.len()).max().unwrap_or(0);
        println!("The following identities are currently configured:");
        for i in &identities {
            if let Err(err) = i.validate() {
                println!("    ⨯ {:padding$} -- {:?}", i.path, err, padding = padding);
            } else {
                println!("    ✓ {}", i.path);
            }
        }
        Ok(())
    }
}
