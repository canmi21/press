//! What the camera recorded, as far as it is worth keeping.
//!
//! Read from the original and stored in the record; the published variants carry none of it,
//! so a reader downloads pixels and nothing else. Extraction happens once, at import, because
//! the original may not always be on hand later. See spec/architecture.md.
//!
//! Nothing here is trusted about the *file*. EXIF describes what the sensor did, and the two
//! disagree the moment an image is cropped -- one sample reports 4032x3024 in
//! `PixelXDimension` for a frame that is 4032x2268 on disk. Dimensions, ratio and byte count
//! come from decoding. Orientation is the exception and must be read, or every derived image
//! comes out turned.

use serde::{Deserialize, Serialize};

/// Everything kept from one original.
///
/// Absent fields are omitted rather than written as null. The three states a null would have
/// to carry -- never looked, looked and found nothing, not applicable -- are different, and
/// JSON cannot tell them apart. Instead the *container* records the answer: no `metadata` key
/// means extraction never ran, an empty one means it ran and the file had nothing. The same
/// trick as a favicon directory existing to prove a domain was checked.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
	/// `DateTimeOriginal` with `OffsetTimeOriginal` and sub-second precision folded in.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub captured: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub camera: Option<Camera>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub lens: Option<Lens>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub exposure: Option<Exposure>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub location: Option<Location>,
	/// Derived from `location` against the offline gazetteer, not read from the file.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub address: Option<Address>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub software: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub color_space: Option<String>,
	/// Read and honoured. Ignoring this turns every derived image.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub orientation: Option<u16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Camera {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub model: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub manufacturer: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Lens {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub model: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub manufacturer: Option<String>,
	/// Millimetres, to two places. EXIF stores a rational, and converting it to a float
	/// produces `6.764999865652793` -- true to the fraction and useless to read.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub focal_length: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub focal_length_35mm: Option<u32>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub f_number: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Exposure {
	/// Kept as the fraction the camera reported. `1/121` is how a shutter speed is read and
	/// said; `0.008264462809917356` is the same number and answers a different question.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub time: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub iso: Option<u32>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub bias_ev: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub mode: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub program: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metering: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub white_balance: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub flash: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Location {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub latitude: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub longitude: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub altitude: Option<f64>,
	/// `GPSHPositioningError`, in metres.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub accuracy: Option<f64>,
	/// `GPSImgDirection`, degrees true.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub direction: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Address {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub continent: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub country: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub country_code: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub region: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub subregion: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub city: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub district: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub postal_code: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub timezone: Option<String>,
}

impl Metadata {
	/// Whether anything at all was found. Used by the tests, and by a caller that wants to
	/// tell "the file had nothing" from "there is nothing here yet".
	#[allow(dead_code)]
	pub fn is_empty(&self) -> bool {
		*self == Self::default()
	}
}

/// Round to two places, which is the precision any of these are read at.
fn two(value: f64) -> f64 {
	(value * 100.0).round() / 100.0
}

/// Read what is worth keeping from an original.
///
/// Returns `Some(empty)` for a file with no EXIF at all, because "looked and found nothing" is
/// a different answer from "never looked", and the caller stores the difference.
pub fn read(bytes: &[u8]) -> Option<Metadata> {
	let mut cursor = std::io::Cursor::new(bytes);
	let Ok(exif) = exif::Reader::new().read_from_container(&mut cursor) else {
		return Some(Metadata::default());
	};

	let text = |tag: exif::Tag| -> Option<String> {
		let field = exif.get_field(tag, exif::In::PRIMARY)?;
		let raw = field.display_value().to_string();
		let trimmed = raw.trim().trim_matches('"').trim().to_owned();
		(!trimmed.is_empty()).then_some(trimmed)
	};
	let number = |tag: exif::Tag| -> Option<f64> {
		let field = exif.get_field(tag, exif::In::PRIMARY)?;
		match field.value {
			exif::Value::Rational(ref r) if !r.is_empty() => Some(r[0].to_f64()),
			exif::Value::SRational(ref r) if !r.is_empty() => Some(r[0].to_f64()),
			exif::Value::Short(ref v) if !v.is_empty() => Some(f64::from(v[0])),
			exif::Value::Long(ref v) if !v.is_empty() => Some(f64::from(v[0])),
			_ => None,
		}
	};

	let camera = Camera {
		model: text(exif::Tag::Model),
		manufacturer: text(exif::Tag::Make),
	};
	let lens = Lens {
		model: text(exif::Tag::LensModel),
		manufacturer: text(exif::Tag::LensMake),
		focal_length: number(exif::Tag::FocalLength).map(two),
		focal_length_35mm: number(exif::Tag::FocalLengthIn35mmFilm).map(|v| v as u32),
		f_number: number(exif::Tag::FNumber).map(two),
	};
	let exposure = Exposure {
		// Taken as displayed, which keeps the fraction the camera reported.
		time: text(exif::Tag::ExposureTime).map(|v| v.trim_end_matches(" s").to_owned()),
		iso: number(exif::Tag::PhotographicSensitivity).map(|v| v as u32),
		bias_ev: number(exif::Tag::ExposureBiasValue).map(two),
		mode: text(exif::Tag::ExposureMode),
		program: text(exif::Tag::ExposureProgram),
		metering: text(exif::Tag::MeteringMode),
		white_balance: text(exif::Tag::WhiteBalance),
		flash: text(exif::Tag::Flash).map(|v| !v.starts_with("not fired")),
	};
	let location = Location {
		latitude: degrees(&exif, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef),
		longitude: degrees(&exif, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef),
		altitude: number(exif::Tag::GPSAltitude).map(two),
		accuracy: number(exif::Tag::GPSHPositioningError).map(two),
		direction: number(exif::Tag::GPSImgDirection).map(two),
	};

	Some(Metadata {
		captured: captured(&exif),
		camera: (camera != Camera::default()).then_some(camera),
		lens: (lens != Lens::default()).then_some(lens),
		exposure: (exposure != Exposure::default()).then_some(exposure),
		location: (location != Location::default()).then_some(location),
		address: None,
		software: text(exif::Tag::Software),
		color_space: text(exif::Tag::ColorSpace),
		orientation: number(exif::Tag::Orientation).map(|v| v as u16),
		..Metadata::default()
	})
}

/// Sexagesimal degrees, minutes and seconds as one signed number.
fn degrees(exif: &exif::Exif, value: exif::Tag, reference: exif::Tag) -> Option<f64> {
	let field = exif.get_field(value, exif::In::PRIMARY)?;
	let exif::Value::Rational(ref parts) = field.value else {
		return None;
	};
	if parts.len() < 3 {
		return None;
	}
	let decimal = parts[0].to_f64() + parts[1].to_f64() / 60.0 + parts[2].to_f64() / 3600.0;
	// South and west are negative. Dropping the reference would put every southern photograph
	// in the wrong hemisphere while looking perfectly plausible.
	let negative = exif
		.get_field(reference, exif::In::PRIMARY)
		.map(|f| f.display_value().to_string())
		.is_some_and(|r| r.starts_with('S') || r.starts_with('W'));
	let signed = if negative { -decimal } else { decimal };
	Some((signed * 10_000.0).round() / 10_000.0)
}

/// `DateTimeOriginal`, with the sub-second and offset the camera also recorded.
///
/// Assembled rather than taken whole because EXIF splits one instant across three tags. An
/// instant missing its offset is not an instant, it is a wall clock reading.
fn captured(exif: &exif::Exif) -> Option<String> {
	let plain = exif
		.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)?
		.display_value()
		.to_string();
	let stamp = plain.trim().replacen(' ', "T", 1);
	let sub = exif
		.get_field(exif::Tag::SubSecTimeOriginal, exif::In::PRIMARY)
		.map(|f| f.display_value().to_string().trim_matches('"').to_owned())
		.filter(|s| !s.is_empty());
	let offset = exif
		.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY)
		.map(|f| f.display_value().to_string().trim_matches('"').to_owned())
		.filter(|s| !s.is_empty());

	let mut out = stamp.replace(':', "-");
	// The date separators became hyphens above; the time ones have to go back.
	if let Some(at) = out.find('T') {
		let (date, time) = out.split_at(at);
		out = format!("{date}{}", time.replace('-', ":"));
	}
	if let Some(sub) = sub {
		out.push('.');
		out.push_str(&sub);
	}
	if let Some(offset) = offset {
		out.push_str(&offset);
	}
	Some(out)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_file_with_no_exif_still_answers() {
		// "Looked and found nothing" is not "never looked", and only the first is a fact about
		// the image. The caller stores the difference by whether the container is present.
		let found = read(b"not an image at all").expect("an answer");
		assert!(found.is_empty());
	}

	#[test]
	fn an_empty_metadata_serialises_to_an_empty_object() {
		// Which is the whole point: a screenshot carries no camera, and writing twenty nulls to
		// say so would treble the record to assert nothing.
		let json = serde_json::to_string(&Metadata::default()).expect("json");
		assert_eq!(json, "{}");
	}

	#[test]
	fn readings_are_rounded_to_what_anyone_would_quote() {
		// EXIF stores rationals; 178/100 becomes 1.7799999713880652 through a float. True to
		// the fraction, and not the number written on the lens.
		assert_eq!(two(1.7799999713880652), 1.78);
		assert_eq!(two(6.764999865652793), 6.76);
	}
}
