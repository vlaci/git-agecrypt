use std::path::PathBuf;

use crate::Result;

use crate::config::Validated;
use crate::{config::AgeIdentity, ctx::Context};

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
        self.list_identities()?;
        println!();
        self.list_recipients()?;
        Ok(())
    }

    pub(crate) fn add_identity(&self, identity: PathBuf) -> Result<()> {
        self.ctx
            .age_identities()
            .add(AgeIdentity::try_from(identity)?)?;
        Ok(())
    }

    pub(crate) fn remove_identity(&self, identity: PathBuf) -> Result<()> {
        self.ctx
            .age_identities()
            .remove(AgeIdentity::try_from(identity)?)?;
        Ok(())
    }

    fn print_identities(&self) -> Result<()> {
        let identities = self.ctx.age_identities().list()?;

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

    pub fn add_recipients(&self, recipients: Vec<String>, paths: Vec<PathBuf>) -> Result<()> {
        let mut cfg = self.ctx.config()?;

        cfg.add(recipients, paths)?;

        cfg.save()?;
        Ok(())
    }

    pub fn remove_recipients(&self, recipients: Vec<String>, paths: Vec<PathBuf>) -> Result<()> {
        let mut cfg = self.ctx.config()?;
        cfg.remove(recipients, paths)?;
        cfg.save()?;
        Ok(())
    }

    pub fn list_recipients(&self) -> Result<()> {
        let cfg = self.ctx.config()?;
        let recipients = cfg.list();

        println!("The following recipients are configured:");
        for (p, r) in recipients {
            println!("    {}: {}", p, r);
        }
        Ok(())
    }
}
