//! The port the local UI binds.
//!
//! Read from the environment rather than compiled in, because Vite needs the same number to
//! proxy to and neither language can read the other's config. `mise.toml` holds it.

use std::fmt;

pub const VAR: &str = "CMS_PORT";

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	Missing,
	NotANumber(String),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Missing => write!(
				f,
				"{VAR} is not set. It is defined in mise.toml, so this usually means mise is not \
				 active in this shell."
			),
			Self::NotANumber(value) => write!(f, "{VAR} is not a port number: {value:?}"),
		}
	}
}

pub fn from_env() -> Result<u16, Error> {
	let raw = std::env::var(VAR).map_err(|_| Error::Missing)?;
	parse(&raw)
}

fn parse(raw: &str) -> Result<u16, Error> {
	raw
		.trim()
		.parse::<u16>()
		.map_err(|_| Error::NotANumber(raw.to_owned()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn accepts_a_plain_port() {
		assert_eq!(parse("26521"), Ok(26521));
	}

	#[test]
	fn tolerates_surrounding_whitespace() {
		assert_eq!(parse(" 26521\n"), Ok(26521));
	}

	#[test]
	fn rejects_a_non_number() {
		assert_eq!(
			parse("http://localhost:26521"),
			Err(Error::NotANumber("http://localhost:26521".into()))
		);
	}

	#[test]
	fn rejects_a_port_above_the_range() {
		// 65536 parses fine as an integer and is not a port. u16 is doing the checking here,
		// so this test is really pinning the choice of type.
		assert_eq!(parse("65536"), Err(Error::NotANumber("65536".into())));
	}
}
