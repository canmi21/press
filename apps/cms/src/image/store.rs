//! Where derived files land on disk.
//!
//! Paths are computed here and nowhere else, because the same layout has to be produced by
//! this tool and understood by the worker that serves it. Two spellings of one scheme is one
//! more than can be kept in step.

use std::path::{Path, PathBuf};

/// How many hex characters each level of the key fans out on.
const FAN: usize = 2;

/// `image/{ab}/{cd}/{cid}.{ext}` under the published root.
///
/// The two levels buy nothing on R2, which has no directory to overflow. They exist so the
/// same bytes can be moved to an object store that does care, without rewriting every key --
/// and the CDN hides them anyway, since a request names only the cid.
pub fn variant_path(public_root: &Path, cid: &str, extension: &str) -> PathBuf {
	let (first, second) = fanout(cid);
	public_root
		.join("image")
		.join(first)
		.join(second)
		.join(format!("{cid}.{extension}"))
}

/// `meta/{blake3}.json` under the published root.
///
/// Flat rather than fanned out: metadata is looked up by exact id and never listed, and
/// keeping it out of the `image/` tree means a sync of one does not walk the other. It is
/// separated by kind rather than by hash because a second kind of asset is expected, and
/// `meta/` will hold those too.
pub fn meta_path(public_root: &Path, blake3: &str) -> PathBuf {
	public_root.join("meta").join(format!("{blake3}.json"))
}

/// The two fanout segments of a content id.
fn fanout(cid: &str) -> (&str, &str) {
	let first = cid.get(..FAN).unwrap_or(cid);
	let second = cid.get(FAN..FAN * 2).unwrap_or("");
	(first, second)
}

pub fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	std::fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fans_a_variant_out_over_two_levels() {
		let path = variant_path(
			Path::new("/pub"),
			"44b6081deaf0242ca3bf83d62a3b6c95",
			"avif",
		);
		assert_eq!(
			path,
			Path::new("/pub/image/44/b6/44b6081deaf0242ca3bf83d62a3b6c95.avif")
		);
	}

	#[test]
	fn keeps_the_full_id_in_the_filename() {
		// The prefix directories are a copy of the first characters, not a substitute for
		// them: a file has to be identifiable from its own name alone once it is elsewhere.
		let path = variant_path(
			Path::new("/pub"),
			"abcdef0123456789abcdef0123456789",
			"webp",
		);
		assert!(
			path
				.file_name()
				.unwrap()
				.to_string_lossy()
				.starts_with("abcdef0123456789")
		);
	}

	#[test]
	fn puts_metadata_outside_the_image_tree() {
		let path = meta_path(Path::new("/pub"), "44b6081deaf0242ca3bf83d62a3b6c95");
		assert_eq!(
			path,
			Path::new("/pub/meta/44b6081deaf0242ca3bf83d62a3b6c95.json")
		);
	}

	#[test]
	fn survives_an_id_shorter_than_the_fanout() {
		// Never expected, but a panic here would come from a filename rather than from data,
		// which is a poor reason to lose a run partway through.
		let path = variant_path(Path::new("/pub"), "ab", "png");
		assert!(path.to_string_lossy().contains("ab.png"));
	}
}
