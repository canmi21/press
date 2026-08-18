//! Content management operations shared by the command-line and desktop adapters.
//! See spec/architecture/cms.md.

mod alt;
mod check;
mod classify;
mod embed;
pub mod favicon;
mod gc;
mod i18n;
pub mod image;
mod licenses;
mod locale;
mod media;
mod opengraph;
pub mod paths;
mod port;
mod refs;
mod summary;
mod tags;
pub mod twitter;
pub mod urls;

pub mod articles;
pub mod cli;
pub mod derived;
pub mod overview;
pub mod task;
