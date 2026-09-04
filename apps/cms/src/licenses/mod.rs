//! The `cms licenses` command: who wrote what the deployables are built out of.
//!
//! Two registries answer the same question in different shapes, so the record normalises both
//! onto Package URL (purl) rather than inventing an identity scheme. purl is what SPDX and
//! CycloneDX already key an SBOM by, which means its escaping rules and its registry
//! vocabulary are somebody else's settled problem instead of ours.
//!
//! The texts themselves are content addressed and published like any other asset -- the
//! registry never appears in a path. Package coordinates are not one shape across registries
//! (`@scope/name`, `group:artifact`, a whole module URL), and encoding them into keys means
//! inventing an escaping scheme that can never be changed afterwards. Addressing the bytes
//! sidesteps all of it, and deduplicates for free: several hundred crates ship the same
//! Apache-2.0 text byte for byte. See spec/architecture/data.md on content addressing.

pub mod cargo;
pub mod npm;

use crate::image::cid;
use crate::image::store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 4;

/// Extensions that say a file is code or configuration whatever it is called.
///
/// A blocklist rather than a list of permitted extensions, because the two mistakes are not
/// equal: publishing one stray file is untidy, and missing a real licence is the failure this
/// whole record exists to prevent. An allowlist would also have to guess at
/// `LICENSE-APACHE-2.0`, whose extension reads as `0`.
const NOT_A_LICENSE: [&str; 14] = [
	"js", "mjs", "cjs", "ts", "mts", "cts", "json", "toml", "yaml", "yml", "rs", "lock",
	// `LICENSE-3rdparty.csv` is a package's own machine-readable inventory of what it depends
	// on. Publishing it under a heading that says it is a licence text states something untrue
	// about a document that is not one.
	"csv", "tsv",
];

/// The file names a package uses to ship its terms.
///
/// `NOTICE` is here because Apache-2.0 makes it a second obligation, separate from the copy of
/// the licence itself: section 4(d) says the attribution notices in that file travel with every
/// distribution of the work. A package that ships one is therefore saying something the licence
/// text does not, and dropping it would leave a requirement unmet rather than merely lose a
/// file. Four packages in the current tree ship one, and all four ship their licence beside it.
fn is_license_file(name: &str) -> bool {
	let lower = name.to_ascii_lowercase();
	let named = lower.starts_with("license")
		|| lower.starts_with("licence")
		|| lower.starts_with("copying")
		|| lower.starts_with("notice");
	// `license_check.js` and `license.ts` are real files in real packages, and neither is a
	// licence. The name alone cannot tell them apart from `LICENSE-MIT`; the extension can.
	let extension =
		Path::new(&lower).extension().and_then(|value| value.to_str()).unwrap_or_default().to_owned();
	named && !NOT_A_LICENSE.contains(&extension.as_str())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
	/// The file name as the package shipped it.
	///
	/// Kept because it is the only thing that says which half of a dual licence a text is:
	/// `MIT OR Apache-2.0` arrives as `LICENSE-MIT` beside `LICENSE-APACHE`, and the texts
	/// themselves do not name the expression they satisfy.
	pub name: String,
	pub cid: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
	pub name: String,
	/// A GitHub login the manifest identifies explicitly, never one inferred from the name.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub github: Option<String>,
}

/// A licence somebody worked out by reading the package, because the package never said.
///
/// Kept apart from the generated record and committed, because it is a judgement rather than
/// an observation: `svelte-toolbelt` ships an MIT text and omits the manifest field, and
/// deciding that the text governs is a decision a person made once and should not have to
/// make again silently. `note` is the evidence, so the decision can be checked rather than
/// merely trusted.
#[derive(Debug, Clone, Deserialize)]
pub struct Assertion {
	pub spdx: String,
	#[allow(dead_code, reason = "the note is evidence for a reader of the file, not an input")]
	#[serde(default)]
	pub note: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Assertions {
	#[serde(default)]
	pub asserted: BTreeMap<String, Assertion>,
}

pub fn assertions_path(repo: &Path) -> PathBuf {
	repo.join("data").join("licenses.yaml")
}

pub fn read_assertions(repo: &Path) -> Result<Assertions, String> {
	let path = assertions_path(repo);
	let text = match std::fs::read_to_string(&path) {
		Ok(text) => text,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Assertions::default()),
		Err(error) => return Err(format!("could not read {}: {error}", path.display())),
	};
	serde_yaml_ng::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Package {
	/// The SPDX expression, verbatim. Absent when nothing declares or asserts one.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub spdx: Option<String>,
	/// True when the expression came from `data/licenses.yaml` rather than from the package.
	///
	/// Carried into the record so the published page can say which it is. Presenting a
	/// judgement as the package's own declaration would be the one dishonest thing this
	/// record could do.
	#[serde(default, skip_serializing_if = "std::ops::Not::not")]
	pub asserted: bool,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub authors: Vec<Person>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub homepage: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub documentation: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub repository: Option<String>,
	/// One shortest resolved path from each workspace root that reaches this package.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub origins: BTreeMap<String, Vec<String>>,
	/// Every package that depends on this one directly, as purls and `workspace:` labels.
	///
	/// Direct edges only. The indirect dependents are the transitive closure of these and are
	/// derived where they are displayed. See spec/architecture/data.md.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub dependents: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub texts: Vec<Text>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
	pub version: u32,
	/// Keyed by purl, which sorts the two registries into one list without a tie-break rule.
	pub packages: BTreeMap<String, Package>,
}

impl Default for Record {
	fn default() -> Self {
		Self { version: VERSION, packages: BTreeMap::new() }
	}
}

/// One package as a collector found it on disk, before it becomes a record entry.
#[derive(Debug, Clone)]
pub struct Found {
	pub purl: String,
	pub spdx: Option<String>,
	pub authors: Vec<Person>,
	pub description: Option<String>,
	pub homepage: Option<String>,
	pub documentation: Option<String>,
	pub repository: Option<String>,
	pub origins: BTreeMap<String, Vec<String>>,
	/// A set while collecting: the same parent is reached again through every other path to it.
	pub dependents: BTreeSet<String>,
	pub directory: PathBuf,
}

/// Keep one deterministic shortest path for a workspace root.
pub fn prefer_origin(
	origins: &mut BTreeMap<String, Vec<String>>,
	root: &str,
	candidate: Vec<String>,
) {
	let replace = match origins.get(root) {
		None => true,
		Some(current) => {
			candidate.len() < current.len() || (candidate.len() == current.len() && candidate < *current)
		}
	};
	if replace {
		origins.insert(root.to_owned(), candidate);
	}
}

pub fn record_path(repo: &Path) -> PathBuf {
	repo.join("data").join("build").join("licenses.json")
}

/// `license/{ab}/{cd}/{cid}.txt` under the published root.
///
/// The same fanout the image store uses, for the same reason: R2 has no directory to
/// overflow, but a filesystem mirror does, and the layout should not have to change if the
/// bytes move.
pub fn text_path(public_root: &Path, cid: &str) -> PathBuf {
	let first = cid.get(..2).unwrap_or(cid);
	let second = cid.get(2..4).unwrap_or("");
	public_root.join("license").join(first).join(second).join(format!("{cid}.txt"))
}

/// `license/full.txt` under the published root.
///
/// Named rather than content addressed, like the OpenGraph cards: it is an aggregate that is
/// rewritten whenever the dependency tree moves, and a reader asks for it by what it is. The
/// alternative is a Worker fetching several hundred objects to concatenate them per request.
pub fn full_path(public_root: &Path) -> PathBuf {
	public_root.join("license").join("full.txt")
}

/// Percent-encode one purl segment.
///
/// Only the characters purl reserves are escaped; everything else is left legible, because a
/// key nobody can read is a key nobody can check. `@` is the one that matters in practice --
/// an npm scope is a namespace whose name begins with it.
fn encode(segment: &str) -> String {
	segment
		.chars()
		.map(|c| match c {
			'@' => "%40".to_owned(),
			':' => "%3A".to_owned(),
			'#' => "%23".to_owned(),
			'?' => "%3F".to_owned(),
			other => other.to_string(),
		})
		.collect()
}

/// `pkg:{type}/{namespace}/{name}@{version}`, with the namespace omitted when there is none.
pub fn purl(kind: &str, namespace: Option<&str>, name: &str, version: &str) -> String {
	match namespace {
		Some(space) => format!("pkg:{kind}/{}/{}@{}", encode(space), encode(name), encode(version)),
		None => format!("pkg:{kind}/{}@{}", encode(name), encode(version)),
	}
}

/// The name out of a `Name <someone@example.com> (https://example.com)` author entry.
///
/// npm's field packs three things into one string and cargo's packs two. Only the name is
/// attribution -- it is what a copyright line carries -- and the other two are contact details
/// nobody offered for republication, so both are dropped wherever the string ends.
pub fn author_name(entry: &str) -> Option<String> {
	let head = entry.split(['<', '(']).next().unwrap_or(entry);
	// Some entries put the address in the name position with no brackets around it at all --
	// `contact@geoffroycouprie.com`, or `Rich Geldreich richgel99@gmail.com`. Dropping any word
	// carrying an `@` catches both, and no name contains one, so nothing real is lost.
	let name =
		head.split_whitespace().filter(|word| !word.contains('@')).collect::<Vec<_>>().join(" ");
	(!name.is_empty()).then_some(name)
}

/// Attribution plus a GitHub identity the manifest itself makes unambiguous.
///
/// A profile URL and GitHub's own no-reply address both name one account. Other homepages and
/// email addresses remain discarded, and a person's name is never searched or matched to an
/// account: a plausible avatar on the wrong person would be worse than no avatar at all.
pub fn author(entry: &str) -> Option<Person> {
	let name = author_name(entry)?;
	Some(Person { name, github: github_profile(entry).or_else(|| github_noreply(entry)) })
}

pub fn github_profile(value: &str) -> Option<String> {
	let start = [value.find("https://"), value.find("http://")].into_iter().flatten().min()?;
	let candidate =
		value[start..].split([')', '>', ' ', '\t', '\r', '\n']).next().unwrap_or_default();
	let url = url::Url::parse(candidate).ok()?;
	let github = url::Url::parse(crate::urls::EXTERNAL_GITHUB_WEB).ok()?;
	if url.host_str() != github.host_str() || url.query().is_some() || url.fragment().is_some() {
		return None;
	}
	let segments = url.path_segments()?.filter(|segment| !segment.is_empty()).collect::<Vec<_>>();
	match segments.as_slice() {
		[login] if github_login(login) => Some((*login).to_owned()),
		_ => None,
	}
}

pub fn github_noreply(value: &str) -> Option<String> {
	let address = value
		.split_whitespace()
		.map(|part| part.trim_matches(['<', '>', '(', ')', ',', ';']))
		.find(|part| part.ends_with("@users.noreply.github.com"))?;
	let local = address.strip_suffix("@users.noreply.github.com")?;
	let login = local.rsplit_once('+').map_or(local, |(_, login)| login);
	github_login(login).then(|| login.to_owned())
}

fn github_login(value: &str) -> bool {
	!value.is_empty()
		&& value.len() <= 39
		&& !value.starts_with('-')
		&& !value.ends_with('-')
		&& value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// A manifest URL that is safe and useful as a browser link.
pub fn web_url(value: Option<String>) -> Option<String> {
	let value = value?.trim().to_owned();
	let parsed = url::Url::parse(&value).ok()?;
	matches!(parsed.scheme(), "http" | "https").then_some(value)
}

/// Read the licence texts a package ships, in file-name order.
fn texts_in(directory: &Path) -> std::io::Result<Vec<(String, Vec<u8>)>> {
	let mut found = Vec::new();
	let entries = match std::fs::read_dir(directory) {
		Ok(entries) => entries,
		// A package directory that is gone is a package that cannot be read, which the caller
		// reports as having no text rather than treating as a failure of the whole run.
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(found),
		Err(error) => return Err(error),
	};
	for entry in entries {
		let entry = entry?;
		if !entry.file_type()?.is_file() {
			continue;
		}
		let name = entry.file_name().to_string_lossy().into_owned();
		if is_license_file(&name) {
			found.push((name, std::fs::read(entry.path())?));
		}
	}
	found.sort_by(|a, b| a.0.cmp(&b.0));
	Ok(found)
}

pub struct Written {
	pub record: Record,
	pub objects: usize,
	/// Packages that declare terms but ship no text to go with them.
	pub textless: Vec<String>,
	/// Packages that declare nothing and that nothing asserts for. Somebody has to look.
	pub undeclared: Vec<String>,
	/// Assertions for packages that now declare their own terms, or that are no longer here.
	pub stale: Vec<String>,
}

/// Turn what the collectors found into published objects and one record.
///
/// Texts are stored exactly as they were shipped. Normalising line endings would improve
/// deduplication and would also mean publishing a licence its author never wrote, which is
/// not a trade available on a legal text.
pub fn write(
	public_root: &Path,
	found: Vec<Found>,
	assertions: &Assertions,
) -> std::io::Result<Written> {
	let mut record = Record::default();
	let mut objects = BTreeSet::new();
	let mut textless = Vec::new();
	let mut undeclared = Vec::new();
	let mut used = BTreeSet::new();

	for package in found {
		let mut texts = Vec::new();
		for (name, bytes) in texts_in(&package.directory)? {
			let id = cid(&bytes);
			if objects.insert(id.clone()) {
				store::write(&text_path(public_root, &id), &bytes)?;
			}
			texts.push(Text { name, cid: id });
		}

		// An assertion only ever fills a gap. A package that declares its own terms is never
		// overridden, because then the file could quietly disagree with the package it
		// describes and nothing would say so.
		let (spdx, asserted) = match package.spdx {
			Some(declared) => (Some(declared), false),
			None => match assertions.asserted.get(&package.purl) {
				Some(assertion) => {
					used.insert(package.purl.clone());
					(Some(assertion.spdx.clone()), true)
				}
				None => {
					undeclared.push(package.purl.clone());
					(None, false)
				}
			},
		};
		if spdx.is_some() && texts.is_empty() {
			textless.push(package.purl.clone());
		}

		record.packages.insert(
			package.purl,
			Package {
				spdx,
				asserted,
				authors: package.authors,
				description: package.description,
				homepage: package.homepage,
				documentation: package.documentation,
				repository: package.repository,
				origins: package.origins,
				dependents: package.dependents.into_iter().collect(),
				texts,
			},
		);
	}

	let stale = assertions.asserted.keys().filter(|purl| !used.contains(*purl)).cloned().collect();

	Ok(Written { record, objects: objects.len(), textless, undeclared, stale })
}

/// The whole attribution notice as one document.
///
/// This is the artefact the permissive licences actually ask for -- the copyright notices and
/// permission texts of everything being distributed, reproducible in one fetch. It carries no
/// header of its own: the sentence about this repository's own licence belongs to the route
/// that serves it, and the object stays exactly what it claims to be.
pub fn full_document(public_root: &Path, record: &Record) -> std::io::Result<String> {
	let mut out = String::new();
	for (purl, package) in &record.packages {
		out.push_str(&"=".repeat(80));
		out.push('\n');
		out.push_str(purl);
		out.push('\n');
		if !package.authors.is_empty() {
			out.push_str(&format!(
				"Authors: {}\n",
				package.authors.iter().map(|author| author.name.as_str()).collect::<Vec<_>>().join(", ")
			));
		}
		match (&package.spdx, package.asserted) {
			// Said plainly rather than folded into the same sentence as a declaration: a
			// reader deciding whether they can rely on this needs to know which of the two
			// they are looking at.
			(Some(spdx), true) => out.push_str(&format!(
				"License: {spdx} (not declared by the package; read from what it ships)\n"
			)),
			(Some(spdx), false) => out.push_str(&format!("License: {spdx}\n")),
			(None, _) => out.push_str("License: not declared by the package\n"),
		}
		if package.texts.is_empty() {
			// Present rather than omitted. A package vanishing from this document reads as an
			// oversight; a package saying it shipped no text is a fact about the package.
			out.push_str("\nNo license text is distributed with this package.\n\n");
			continue;
		}
		for text in &package.texts {
			out.push_str(&format!("\n--- {} ---\n\n", text.name));
			let bytes = std::fs::read(text_path(public_root, &text.cid))?;
			out.push_str(&String::from_utf8_lossy(&bytes));
			if !out.ends_with('\n') {
				out.push('\n');
			}
		}
		out.push('\n');
	}
	Ok(out)
}

/// Every content id the record keeps alive, for `cms gc`.
pub fn referenced(record: &Record) -> BTreeSet<String> {
	record
		.packages
		.values()
		.flat_map(|package| package.texts.iter().map(|text| text.cid.clone()))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn builds_a_purl_for_each_registry_shape() {
		assert_eq!(purl("cargo", None, "serde", "1.0.219"), "pkg:cargo/serde@1.0.219");
		assert_eq!(purl("npm", Some("@sveltejs"), "kit", "2.0.0"), "pkg:npm/%40sveltejs/kit@2.0.0");
	}

	#[test]
	fn keeps_the_name_and_drops_the_contact_details() {
		assert_eq!(author_name("Ada <ada@example.com>").as_deref(), Some("Ada"));
		assert_eq!(author_name("  Ada  ").as_deref(), Some("Ada"));
		assert_eq!(author_name("<only@example.com>"), None);
		// npm packs a homepage in after the address, and plenty of packages give only that --
		// `The Babel Team (https://babel.dev/team)` is the one that surfaced this.
		assert_eq!(
			author_name("The Babel Team (https://babel.example/team)").as_deref(),
			Some("The Babel Team")
		);
		assert_eq!(author_name("Ada <ada@example.com> (https://example.com)").as_deref(), Some("Ada"));
		// Written with no brackets at all, which several crates and wrangler both do.
		assert_eq!(
			author_name("Rich Geldreich richgel99@gmail.com").as_deref(),
			Some("Rich Geldreich")
		);
		assert_eq!(author_name("contact@geoffroycouprie.com"), None);
	}

	#[test]
	fn keeps_only_explicit_github_identities() {
		assert_eq!(
			author("Ada <12345+octo-cat@users.noreply.github.com>").and_then(|person| person.github),
			Some("octo-cat".to_owned())
		);
		let github = crate::urls::EXTERNAL_GITHUB_WEB;
		assert_eq!(
			author(&format!("Ada ({github}/octo-cat)")).and_then(|person| person.github),
			Some("octo-cat".to_owned())
		);
		assert_eq!(
			author(&format!("Ada ({github}/octo-cat/project)")).and_then(|person| person.github),
			None
		);
		assert_eq!(
			author("Ada (https://not-github.example/octo-cat)").and_then(|person| person.github),
			None
		);
		assert_eq!(author("Ada <ada@example.com>").unwrap().github, None);
	}

	#[test]
	fn keeps_only_browser_urls() {
		assert_eq!(
			web_url(Some("https://example.com/project".to_owned())).as_deref(),
			Some("https://example.com/project")
		);
		assert_eq!(web_url(Some("javascript:alert(1)".to_owned())), None);
	}

	#[test]
	fn keeps_the_shortest_stable_origin_for_each_root() {
		let mut origins = BTreeMap::new();
		prefer_origin(
			&mut origins,
			"site",
			vec!["pkg:npm/z@1".to_owned(), "pkg:npm/target@1".to_owned()],
		);
		prefer_origin(
			&mut origins,
			"site",
			vec!["pkg:npm/a@1".to_owned(), "pkg:npm/target@1".to_owned()],
		);
		prefer_origin(
			&mut origins,
			"site",
			vec!["pkg:npm/long@1".to_owned(), "pkg:npm/path@1".to_owned(), "pkg:npm/target@1".to_owned()],
		);
		assert_eq!(origins["site"], ["pkg:npm/a@1", "pkg:npm/target@1"]);
	}

	#[test]
	fn recognises_the_names_a_license_is_shipped_under() {
		for name in ["LICENSE", "LICENSE-MIT", "licence.md", "COPYING", "NOTICE"] {
			assert!(is_license_file(name), "{name}");
		}
		// Named for a licence, but code or data. Every one of these is shipped by a package in
		// the current tree; `LICENSE-3rdparty.csv` is a machine-readable dependency inventory.
		for name in
			["README.md", "Cargo.toml", "license_check.js", "license.ts", "LICENSE-3rdparty.csv"]
		{
			assert!(!is_license_file(name), "{name}");
		}
		// A notice is not a licence and is collected anyway: Apache-2.0 section 4(d) makes
		// carrying it a requirement of its own.
		assert!(is_license_file("NOTICE"));
		// The extension reads as `0`, which an allowlist of extensions would have rejected.
		assert!(is_license_file("LICENSE-APACHE-2.0"));
	}

	#[test]
	fn stores_one_object_for_a_text_two_packages_share() {
		let temporary = tempfile::tempdir().expect("temp");
		let root = temporary.path();
		let one = root.join("one");
		let two = root.join("two");
		std::fs::create_dir_all(&one).unwrap();
		std::fs::create_dir_all(&two).unwrap();
		std::fs::write(one.join("LICENSE"), b"MIT terms").unwrap();
		std::fs::write(two.join("LICENSE"), b"MIT terms").unwrap();

		let public = root.join("public");
		let written = write(
			&public,
			vec![
				Found {
					purl: purl("npm", None, "one", "1.0.0"),
					spdx: Some("MIT".to_owned()),
					authors: vec![],
					description: None,
					homepage: None,
					documentation: None,
					repository: None,
					origins: BTreeMap::new(),
					dependents: BTreeSet::new(),
					directory: one,
				},
				Found {
					purl: purl("npm", None, "two", "1.0.0"),
					spdx: Some("MIT".to_owned()),
					authors: vec![],
					description: None,
					homepage: None,
					documentation: None,
					repository: None,
					origins: BTreeMap::new(),
					dependents: BTreeSet::new(),
					directory: two,
				},
			],
			&Assertions::default(),
		)
		.unwrap();

		assert_eq!(written.objects, 1);
		assert_eq!(written.record.packages.len(), 2);
		assert_eq!(referenced(&written.record).len(), 1);

		std::fs::remove_dir_all(&root).unwrap();
	}

	#[test]
	fn reports_a_package_that_declares_terms_but_ships_none() {
		let temporary = tempfile::tempdir().expect("temp");
		let root = temporary.path();
		let bare = root.join("bare");
		std::fs::create_dir_all(&bare).unwrap();

		let written = write(
			&root.join("public"),
			vec![
				Found {
					purl: purl("npm", None, "bare", "1.0.0"),
					spdx: Some("MIT".to_owned()),
					authors: vec![],
					description: None,
					homepage: None,
					documentation: None,
					repository: None,
					origins: BTreeMap::new(),
					dependents: BTreeSet::new(),
					directory: bare.clone(),
				},
				Found {
					purl: purl("npm", None, "silent", "1.0.0"),
					spdx: None,
					authors: vec![],
					description: None,
					homepage: None,
					documentation: None,
					repository: None,
					origins: BTreeMap::new(),
					dependents: BTreeSet::new(),
					directory: bare,
				},
			],
			&Assertions::default(),
		)
		.unwrap();

		assert_eq!(written.textless, vec!["pkg:npm/bare@1.0.0"]);
		assert_eq!(written.undeclared, vec!["pkg:npm/silent@1.0.0"]);

		std::fs::remove_dir_all(&root).unwrap();
	}

	#[test]
	fn a_textless_package_still_appears_in_the_document() {
		let mut record = Record::default();
		record.packages.insert(
			"pkg:npm/bare@1.0.0".to_owned(),
			Package {
				spdx: Some("MIT".to_owned()),
				asserted: false,
				authors: vec![Person { name: "Ada".to_owned(), github: None }],
				description: None,
				homepage: None,
				documentation: None,
				repository: None,
				origins: BTreeMap::new(),
				dependents: vec![],
				texts: vec![],
			},
		);
		let document = full_document(Path::new("/nowhere"), &record).unwrap();
		assert!(document.contains("pkg:npm/bare@1.0.0"));
		assert!(document.contains("Authors: Ada"));
		assert!(document.contains("No license text is distributed"));
	}
}
