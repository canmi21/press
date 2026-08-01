//! Turning a pair of coordinates into a place name, without asking anyone.
//!
//! GeoNames' `cities500` in `data/geo`, indexed once into an R-tree and searched for the
//! nearest settlement. Offline on purpose: a reverse geocoding service would make importing a
//! photograph depend on somebody else's uptime, their rate limit and their opinion about what
//! we may do with the answer -- for a fact that never changes once written. See
//! spec/architecture.md.
//!
//! The data is 39MB of text and lives outside git, like the photographs it describes. `mise
//! run geo` fetches it; without it, addresses are simply absent, which is the same state as an
//! image whose EXIF carried no position.

use super::exif::Address;
use rstar::{AABB, PointDistance, RTree, RTreeObject};
use std::collections::HashMap;
use std::path::Path;

/// One settlement, as much of it as the address needs.
#[derive(Debug, Clone)]
struct Place {
	lat: f64,
	lon: f64,
	name: String,
	country: String,
	admin1: String,
	admin2: String,
}

/// One postal area, by the point GeoNames gives for it.
///
/// A code is not unique on its own -- 27707 is a district of Eumseong and also a part of
/// Durham -- so these are found by position and never by string.
#[derive(Debug, Clone)]
struct Postal {
	lat: f64,
	lon: f64,
	code: String,
	country: String,
}

impl RTreeObject for Place {
	type Envelope = AABB<[f64; 2]>;
	fn envelope(&self) -> Self::Envelope {
		AABB::from_point([self.lon, self.lat])
	}
}

impl RTreeObject for Postal {
	type Envelope = AABB<[f64; 2]>;
	fn envelope(&self) -> Self::Envelope {
		AABB::from_point([self.lon, self.lat])
	}
}

impl PointDistance for Postal {
	fn distance_2(&self, point: &[f64; 2]) -> f64 {
		let dx = self.lon - point[0];
		let dy = self.lat - point[1];
		dx * dx + dy * dy
	}
}

impl PointDistance for Place {
	/// Squared degrees, which is wrong as a distance and right as an ordering.
	///
	/// A degree of longitude is shorter near the poles, so this is not metres and must never
	/// be reported as one. It only ever decides which of two candidates is closer, and for
	/// that the distortion has to be extreme before it changes the answer -- the nearest town
	/// to a photograph is rarely a close call between two on opposite bearings.
	fn distance_2(&self, point: &[f64; 2]) -> f64 {
		let dx = self.lon - point[0];
		let dy = self.lat - point[1];
		dx * dx + dy * dy
	}
}

pub struct Gazetteer {
	tree: RTree<Place>,
	/// ISO country code to (name, continent).
	countries: HashMap<String, (String, String)>,
	/// `US.NC` to the region's name.
	regions: HashMap<String, String>,
	/// `US.NC.183` to the county's name.
	subregions: HashMap<String, String>,
	/// Postal areas, when that file has been fetched.
	postal: Option<RTree<Postal>>,
	finder: tzf_rs::DefaultFinder,
}

/// Where the data lives, relative to the repository root.
pub const DIRECTORY: &str = "data/geo";

fn continent_of(code: &str) -> &'static str {
	match code {
		"AF" => "Africa",
		"AS" => "Asia",
		"EU" => "Europe",
		"NA" => "North America",
		"OC" => "Oceania",
		"SA" => "South America",
		"AN" => "Antarctica",
		_ => "",
	}
}

impl Gazetteer {
	/// Read the gazetteer, or `None` when it has not been fetched.
	pub fn open(repo: &Path) -> Option<Self> {
		let root = repo.join(DIRECTORY);
		let cities = std::fs::read_to_string(root.join("cities500.txt")).ok()?;

		let mut countries = HashMap::new();
		if let Ok(text) = std::fs::read_to_string(root.join("countryInfo.txt")) {
			for line in text.lines().filter(|l| !l.starts_with('#')) {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() > 8 {
					countries.insert(
						f[0].to_owned(),
						(f[4].to_owned(), continent_of(f[8]).to_owned()),
					);
				}
			}
		}

		let mut regions = HashMap::new();
		if let Ok(text) = std::fs::read_to_string(root.join("admin1CodesASCII.txt")) {
			for line in text.lines() {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() > 1 {
					regions.insert(f[0].to_owned(), f[1].to_owned());
				}
			}
		}

		let mut subregions = HashMap::new();
		if let Ok(text) = std::fs::read_to_string(root.join("admin2Codes.txt")) {
			for line in text.lines() {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() > 1 {
					subregions.insert(f[0].to_owned(), f[1].to_owned());
				}
			}
		}

		// Optional, and by far the largest of these files. Absent, postal codes are simply
		// missing, which is the state every image was in before it was fetched.
		let postal = std::fs::read_to_string(root.join("postal.txt"))
			.ok()
			.map(|text| {
				let points: Vec<Postal> = text
					.lines()
					.filter_map(|line| {
						let f: Vec<&str> = line.split('\t').collect();
						if f.len() < 11 {
							return None;
						}
						Some(Postal {
							lat: f[9].parse().ok()?,
							lon: f[10].parse().ok()?,
							code: f[1].to_owned(),
							country: f[0].to_owned(),
						})
					})
					.collect();
				RTree::bulk_load(points)
			});

		// Built once. Two hundred thousand points is a second of work and a lookup that costs
		// nothing after, which matters because a library of photographs is imported in batches.
		let places: Vec<Place> = cities
			.lines()
			.filter_map(|line| {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() < 18 {
					return None;
				}
				Some(Place {
					lat: f[4].parse().ok()?,
					lon: f[5].parse().ok()?,
					name: f[1].to_owned(),
					country: f[8].to_owned(),
					admin1: f[10].to_owned(),
					admin2: f[11].to_owned(),
				})
			})
			.collect();

		Some(Self {
			tree: RTree::bulk_load(places),
			countries,
			regions,
			subregions,
			postal,
			finder: tzf_rs::DefaultFinder::new(),
		})
	}

	/// The address for a position, as far as the data can say.
	///
	/// `district` stays absent. Naming a neighbourhood needs the full GeoNames dump, which is
	/// an order of magnitude larger than everything here put together, and deriving one from
	/// the nearest town would state something no source claimed.
	pub fn lookup(&self, lat: f64, lon: f64) -> Option<Address> {
		let place = self.tree.nearest_neighbor([lon, lat])?;
		let (country, continent) = self
			.countries
			.get(&place.country)
			.cloned()
			.unwrap_or_default();
		let region = self
			.regions
			.get(&format!("{}.{}", place.country, place.admin1))
			.cloned();
		let subregion = self
			.subregions
			.get(&format!(
				"{}.{}.{}",
				place.country, place.admin1, place.admin2
			))
			.cloned();
		// Found by position, then checked against the country the town is in: a code is not
		// unique on its own, and the nearest point to a border could belong to the other side.
		let postal_code = self
			.postal
			.as_ref()
			.and_then(|tree| tree.nearest_neighbor([lon, lat]))
			.filter(|found| found.country == place.country)
			.map(|found| found.code.clone());

		Some(Address {
			continent: (!continent.is_empty()).then_some(continent),
			country: (!country.is_empty()).then_some(country),
			country_code: (!place.country.is_empty()).then(|| place.country.clone()),
			region,
			subregion,
			city: (!place.name.is_empty()).then(|| place.name.clone()),
			district: None,
			postal_code,
			// From the polygon the point falls in, not from the nearest town. A settlement a
			// few miles away can sit on the other side of a zone boundary.
			timezone: Some(self.finder.get_tz_name(lon, lat).to_owned()).filter(|t| !t.is_empty()),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn nearness_is_an_ordering_and_not_a_measurement() {
		// Squared degrees, which is not metres and must never be reported as any. It decides
		// which of two candidates is closer and nothing else.
		let place = Place {
			lat: 0.0,
			lon: 0.0,
			name: "origin".into(),
			country: "XX".into(),
			admin1: "01".into(),
			admin2: "001".into(),
		};
		assert!(place.distance_2(&[1.0, 0.0]) < place.distance_2(&[2.0, 0.0]));
		assert_eq!(place.distance_2(&[0.0, 0.0]), 0.0);
	}

	#[test]
	fn a_missing_gazetteer_is_absence_rather_than_failure() {
		// The data is 39MB and lives outside git. Not having fetched it should read the same
		// as a photograph that carried no position: no address, no error.
		assert!(Gazetteer::open(Path::new("/nowhere-at-all")).is_none());
	}

	#[test]
	fn continents_come_from_the_code_the_data_uses() {
		assert_eq!(continent_of("NA"), "North America");
		assert_eq!(continent_of("EU"), "Europe");
		assert_eq!(continent_of("ZZ"), "");
	}
}
