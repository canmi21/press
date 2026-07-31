//! Choosing which sizes an image is rendered at.
//!
//! Pure arithmetic, kept apart from the codecs so the rules can be read and tested without
//! decoding anything.

/// Widths offered when an image is large enough to fill them.
pub const TIERS: [u32; 3] = [640, 1280, 1920];

/// Below this, lossy coding is the wrong tool: see `is_flat` in the encoder module.
pub const SMALL_EDGE: u32 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
	pub width: u32,
	pub height: u32,
}

impl Size {
	pub fn new(width: u32, height: u32) -> Self {
		Self { width, height }
	}

	/// The larger dimension. Every decision here is made against this rather than width,
	/// because a portrait photo is not a small image -- it is a tall one, and matching it on
	/// width alone would hand it a tier meant for something a quarter of its size.
	pub fn long_edge(self) -> u32 {
		self.width.max(self.height)
	}

	/// This size scaled so its long edge is `target`, preserving the ratio.
	///
	/// Orientation is never touched. A portrait image stays portrait; the tier only decides
	/// how many pixels it gets, not which way up it is.
	pub fn scaled_to_long_edge(self, target: u32) -> Self {
		let current = self.long_edge();
		if current == 0 || target >= current {
			return self;
		}
		let factor = f64::from(target) / f64::from(current);
		Self::new(
			scale(self.width, factor).max(1),
			scale(self.height, factor).max(1),
		)
	}
}

fn scale(value: u32, factor: f64) -> u32 {
	(f64::from(value) * factor).round() as u32
}

/// The sizes to produce for an image, largest last.
///
/// Contains every tier the original can fill without being enlarged, plus the original size
/// itself as the top rung. Upscaling is never done: it invents detail and costs bytes to
/// carry it.
///
/// An image smaller than the lowest tier yields exactly one size -- its own -- so it is
/// re-encoded rather than resized. There is nothing to gain from a 200px image at 640px.
pub fn ladder(original: Size) -> Vec<Size> {
	let long = original.long_edge();
	let mut sizes: Vec<Size> = TIERS
		.into_iter()
		.filter(|&tier| tier < long)
		.map(|tier| original.scaled_to_long_edge(tier))
		.collect();
	sizes.push(original);
	sizes
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn measures_against_the_long_edge_whichever_it_is() {
		assert_eq!(Size::new(1600, 900).long_edge(), 1600);
		assert_eq!(Size::new(900, 1600).long_edge(), 1600);
	}

	#[test]
	fn keeps_the_ratio_when_scaling() {
		let portrait = Size::new(1000, 2000).scaled_to_long_edge(640);
		assert_eq!(portrait, Size::new(320, 640));

		let landscape = Size::new(2000, 1000).scaled_to_long_edge(640);
		assert_eq!(landscape, Size::new(640, 320));
	}

	#[test]
	fn never_reorients() {
		// A portrait image scaled to a tier stays taller than it is wide. Matching on the long
		// edge is what makes this hold; matching on width would not.
		let out = Size::new(800, 1600).scaled_to_long_edge(640);
		assert!(out.height > out.width);
	}

	#[test]
	fn refuses_to_enlarge() {
		let small = Size::new(300, 200);
		assert_eq!(small.scaled_to_long_edge(1920), small);
	}

	#[test]
	fn offers_every_tier_the_original_can_fill() {
		let widths: Vec<u32> = ladder(Size::new(2400, 1600))
			.into_iter()
			.map(|s| s.long_edge())
			.collect();
		assert_eq!(widths, vec![640, 1280, 1920, 2400]);
	}

	#[test]
	fn stops_below_the_original_and_ends_at_it() {
		let widths: Vec<u32> = ladder(Size::new(1500, 1000))
			.into_iter()
			.map(|s| s.long_edge())
			.collect();
		assert_eq!(widths, vec![640, 1280, 1500]);
	}

	#[test]
	fn a_portrait_ladder_is_measured_on_height() {
		let sizes = ladder(Size::new(1000, 2000));
		assert_eq!(sizes.first().copied(), Some(Size::new(320, 640)));
		assert_eq!(sizes.last().copied(), Some(Size::new(1000, 2000)));
	}

	#[test]
	fn an_image_below_the_lowest_tier_yields_only_itself() {
		// Re-encoded, not resized. Offering it at 640 would upscale a 200px image.
		assert_eq!(ladder(Size::new(200, 150)), vec![Size::new(200, 150)]);
	}

	#[test]
	fn an_image_exactly_on_a_tier_is_not_duplicated() {
		let widths: Vec<u32> = ladder(Size::new(1280, 720))
			.into_iter()
			.map(|s| s.long_edge())
			.collect();
		assert_eq!(widths, vec![640, 1280]);
	}
}
