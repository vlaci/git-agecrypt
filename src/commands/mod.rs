use crate::ctx::Context;

pub mod internal;
pub mod public;

pub(crate) struct Commands<C: Context> {
    pub ctx: C,
}
