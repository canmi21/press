//! Choosing which sizes an image is rendered at.
//!
//! Pure arithmetic, kept apart from the codecs so the rules can be read and tested without
//! decoding anything.

/// Widths offered when an image is large enough to fill them.
pub const TIERS: [u32; 3] = [640, 1280, 1920];

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
/// Every tier the original can fill without being enlarged. Upscaling is never done: it
/// invents detail and costs bytes to carry it.
///
/// An image smaller than the lowest tier yields exactly one size -- its own -- so it is
/// re-encoded rather than resized. There is nothing to gain from a 200px image at 640px.
///
/// An image that outgrows the largest tier is capped there, because no layout on the site
/// asks for more and the pixels above the cap are paid for by every reader. `keep_original`
/// overrides that for the images where the detail is the point -- a photograph rather than a
/// screenshot of some text. It adds one more rung at the original resolution, still AVIF and
/// still lossy, so "original" here means the full frame rather than the original file.
pub fn ladder(original: Size, keep_original: bool) -> Vec<Size> {
	let long = original.long_edge();
	let mut sizes: Vec<Size> = TIERS
		.into_iter()
		.filter(|&tier| tier < long)
		.map(|tier| original.scaled_to_long_edge(tier))
		.collect();

	// Below the cap the original is the top rung, and there is no separate full-resolution
	// variant to ask for -- the largest tier already is it.
	if long <= CAP || keep_original {
		sizes.push(original);
	}
	sizes
}

/// The largest tier. An original above this is only kept when asked for by name.
const CAP: u32 = TIERS[TIERS.len() - 1];

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

	fn rungs(size: Size, keep_original: bool) -> Vec<u32> {
		ladder(size, keep_original)
			.into_iter()
			.map(|s| s.long_edge())
			.collect()
	}

	#[test]
	fn caps_an_oversized_original_at_the_largest_tier() {
		// Nothing on the site renders wider than 1920, so the rest is weight every reader pays
		// for and nobody sees.
		assert_eq!(rungs(Size::new(2400, 1600), false), vec![640, 1280, 1920]);
	}

	#[test]
	fn keeps_the_full_frame_only_when_asked() {
		assert_eq!(
			rungs(Size::new(2400, 1600), true),
			vec![640, 1280, 1920, 2400]
		);
	}

	#[test]
	fn adds_no_rung_for_an_original_already_under_the_cap() {
		// The top rung is the original either way, so the flag has nothing to add and must not
		// produce the same size twice.
		assert_eq!(rungs(Size::new(1500, 1000), false), vec![640, 1280, 1500]);
		assert_eq!(rungs(Size::new(1500, 1000), true), vec![640, 1280, 1500]);
	}

	#[test]
	fn a_portrait_ladder_is_measured_on_height() {
		// The cap applies to the long edge, so a tall image is capped on its height and stays
		// narrower than the tier number suggests.
		let sizes = ladder(Size::new(1000, 2000), false);
		assert_eq!(sizes.first().copied(), Some(Size::new(320, 640)));
		assert_eq!(sizes.last().copied(), Some(Size::new(960, 1920)));

		let kept = ladder(Size::new(1000, 2000), true);
		assert_eq!(kept.last().copied(), Some(Size::new(1000, 2000)));
	}

	#[test]
	fn an_image_below_the_lowest_tier_yields_only_itself() {
		// Re-encoded, not resized. Offering it at 640 would upscale a 200px image.
		assert_eq!(
			ladder(Size::new(200, 150), false),
			vec![Size::new(200, 150)]
		);
	}

	#[test]
	fn an_image_exactly_on_a_tier_is_not_duplicated() {
		assert_eq!(rungs(Size::new(1280, 720), false), vec![640, 1280]);
		assert_eq!(rungs(Size::new(1920, 1080), false), vec![640, 1280, 1920]);
	}
}
