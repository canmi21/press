//! Finding `data/public` from wherever the command was run.
//!
//! Walking up for the marker rather than taking a compile-time path, because the binary is
//! run from anywhere in the tree and `CARGO_MANIFEST_DIR` would bake in whichever machine
//! built it.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
	"could not find data/public above the current directory -- run this from inside the repository"
)]
pub struct NotFound;

/// The repository root, found by looking for `data/public` above the working directory.
///
/// Every command joins its own paths onto this rather than being handed one of them: the
/// article tree, the originals and the published tree are all siblings, and a command that
/// only knew about `data/public` could not read the articles that decide what belongs there.
pub fn repo_root() -> Result<PathBuf, NotFound> {
	let start = std::env::current_dir().map_err(|_| NotFound)?;
	find_upwards(&start)
		.and_then(|public| public.parent().and_then(Path::parent).map(Path::to_path_buf))
		.ok_or(NotFound)
}

fn find_upwards(start: &Path) -> Option<PathBuf> {
	start
		.ancestors()
		.map(|directory| directory.join("data").join("public"))
		.find(|candidate| candidate.is_dir())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn finds_the_marker_from_a_nested_directory() {
		let root = std::env::temp_dir().join(format!("cms-paths-{}", std::process::id()));
		let nested = root.join("apps").join("cms").join("src");
		std::fs::create_dir_all(&nested).unwrap();
		std::fs::create_dir_all(root.join("data").join("public")).unwrap();

		assert_eq!(find_upwards(&nested), Some(root.join("data").join("public")));

		std::fs::remove_dir_all(&root).unwrap();
	}

	#[test]
	fn returns_nothing_when_there_is_no_marker() {
		let root = std::env::temp_dir().join(format!("cms-empty-{}", std::process::id()));
		std::fs::create_dir_all(&root).unwrap();
		// A temp directory has no data/public above it, and finding one would mean the walk
		// escaped into somebody's home.
		assert_eq!(find_upwards(&root), None);
		std::fs::remove_dir_all(&root).unwrap();
	}
}
