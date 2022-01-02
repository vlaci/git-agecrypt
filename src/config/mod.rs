mod age_identities;
mod app;
mod git;

pub(crate) use age_identities::{AgeIdentities, AgeIdentity};
pub(crate) use app::AppConfig;

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

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) trait Validated {
    fn validate(&self) -> Result<()>;
}

pub(crate) trait Container {
    type Item;

    fn add(&mut self, item: Self::Item) -> Result<()>;

    fn remove(&mut self, item: Self::Item) -> Result<()>;

    fn list(&self) -> Result<Vec<Self::Item>>;
}
