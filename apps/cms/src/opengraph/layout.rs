//! Placing text on the card.
//!
//! Three bands down the height of the canvas: the site name at the top, the title and its
//! subtitle in the middle, the date and category at the bottom. The bottom band is aligned
//! right on purpose -- X draws the domain over the bottom left of every card it renders, so
//! anything put there is covered by somebody else's chrome.
//!
//! Sizes and opacities are carried over from the previous generator rather than reinvented;
//! only the engine changed. See spec/architecture.md.

use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{Paint, PixmapMut, Rect, Transform};

/// The size every consumer expects, and the one the layout is tuned for.
pub const WIDTH: u32 = 1200;
pub const HEIGHT: u32 = 630;

const PAD_X: f32 = 72.0;
const PAD_Y: f32 = 56.0;

/// The measurements the old card used, kept so the two look like the same site.
const SITE_SIZE: f32 = 44.0;
const TITLE_SIZE: f32 = 96.0;
const SUBTITLE_SIZE: f32 = 38.0;
const CATEGORY_SIZE: f32 = 36.0;
const DATE_SIZE: f32 = 30.0;

const TITLE_LINE: f32 = 1.15;

/// How small the title may be shrunk before wrapping is accepted instead.
///
/// A title reads best on one line, so a long one is stepped down until it fits rather than
/// broken. Past this the type is small enough that two larger lines look better than one
/// cramped one, and the layout stops fighting it.
const TITLE_MIN_SIZE: f32 = 56.0;
const TITLE_STEP: f32 = 4.0;
const SUBTITLE_LINE: f32 = 1.4;

/// Ink, and the opacities each band is knocked back to.
const INK: (u8, u8, u8) = (0x33, 0x33, 0x33);
const PAPER: (u8, u8, u8) = (0xff, 0xff, 0xff);
const SITE_ALPHA: f32 = 0.34;
const SUBTITLE_ALPHA: f32 = 0.48;
const CATEGORY_ALPHA: f32 = 0.34;
const DATE_ALPHA: f32 = 0.24;

/// The middle band is held back from the right edge so a long title breaks rather than
/// running the full width, which reads badly at this size.
const TEXT_WIDTH: f32 = 1000.0;

const GAP: f32 = 16.0;
const BOTTOM_GAP: f32 = 24.0;

pub struct Card<'a> {
	pub site: &'a str,
	pub title: &'a str,
	pub subtitle: Option<&'a str>,
	pub category: Option<&'a str>,
	pub date: Option<&'a str>,
}

fn colour(alpha: f32) -> Color {
	Color::rgba(INK.0, INK.1, INK.2, (alpha * 255.0).round() as u8)
}

/// One run of text, laid out and measured but not yet placed.
struct Line {
	buffer: Buffer,
	width: f32,
	height: f32,
	lines: u32,
}

fn lay(
	fonts: &mut FontSystem,
	text: &str,
	size: f32,
	line_height: f32,
	max_width: f32,
	family: &str,
) -> Line {
	let mut buffer = Buffer::new(fonts, Metrics::new(size, size * line_height));
	buffer.set_size(Some(max_width), Some(HEIGHT as f32));
	buffer.set_text(
		text,
		&Attrs::new().family(Family::Name(family)),
		Shaping::Advanced,
		None,
	);
	buffer.shape_until_scroll(fonts, false);

	// Measured from what was actually laid out rather than from the string: a CJK title wraps
	// at a different place than its character count suggests, and the bands below have to move
	// down by however many lines it really took.
	let mut width: f32 = 0.0;
	let mut lines = 0u32;
	for run in buffer.layout_runs() {
		width = width.max(run.line_w);
		lines += 1;
	}
	let height = lines as f32 * size * line_height;
	Line {
		buffer,
		width,
		height,
		lines,
	}
}

fn paint(
	pixmap: &mut PixmapMut<'_>,
	fonts: &mut FontSystem,
	cache: &mut SwashCache,
	line: &mut Line,
	at: (f32, f32),
	fill: Color,
) {
	line.buffer.draw(fonts, cache, fill, |x, y, w, h, colour| {
		if colour.a() == 0 {
			return;
		}
		let mut paint = Paint::default();
		paint.set_color_rgba8(colour.r(), colour.g(), colour.b(), colour.a());
		paint.anti_alias = false;
		if let Some(rect) = Rect::from_xywh(at.0 + x as f32, at.1 + y as f32, w as f32, h as f32) {
			pixmap.fill_rect(rect, &paint, Transform::identity(), None);
		}
	});
}

/// Lay the title out as large as it can be while still occupying one line.
///
/// Measured rather than estimated: where a CJK title breaks has no relation to its character
/// count, so the only way to know whether a size fits is to shape it and look.
fn fit_title(fonts: &mut FontSystem, text: &str, family: &str) -> Line {
	let mut size = TITLE_SIZE;
	loop {
		let laid = lay(fonts, text, size, TITLE_LINE, TEXT_WIDTH, family);
		if laid.lines <= 1 || size <= TITLE_MIN_SIZE {
			return laid;
		}
		size -= TITLE_STEP;
	}
}

/// Render a card to raw RGBA pixels.
pub fn render(fonts: &mut FontSystem, family: &str, card: &Card<'_>) -> Vec<u8> {
	let mut pixels = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
	let mut pixmap = PixmapMut::from_bytes(&mut pixels, WIDTH, HEIGHT).expect("canvas");
	pixmap.fill(tiny_skia::Color::from_rgba8(PAPER.0, PAPER.1, PAPER.2, 255));

	let mut cache = SwashCache::new();

	let mut site = lay(fonts, card.site, SITE_SIZE, 1.2, TEXT_WIDTH, family);
	paint(
		&mut pixmap,
		fonts,
		&mut cache,
		&mut site,
		(PAD_X, PAD_Y),
		colour(SITE_ALPHA),
	);

	let mut title = fit_title(fonts, card.title, family);
	let mut subtitle = card.subtitle.filter(|text| !text.is_empty()).map(|text| {
		lay(
			fonts,
			text,
			SUBTITLE_SIZE,
			SUBTITLE_LINE,
			TEXT_WIDTH,
			family,
		)
	});

	// The middle band is centred on the canvas rather than pinned, so a one-line title and a
	// three-line one both sit in the optical middle instead of drifting downward.
	let middle = title.height + subtitle.as_ref().map_or(0.0, |s| s.height + GAP);
	let mut y = (HEIGHT as f32 - middle) / 2.0;

	paint(
		&mut pixmap,
		fonts,
		&mut cache,
		&mut title,
		(PAD_X, y),
		colour(1.0),
	);
	y += title.height + GAP;
	if let Some(subtitle) = &mut subtitle {
		paint(
			&mut pixmap,
			fonts,
			&mut cache,
			subtitle,
			(PAD_X, y),
			colour(SUBTITLE_ALPHA),
		);
	}

	// Bottom band, right-aligned and laid out from the right edge inward.
	let mut right = WIDTH as f32 - PAD_X;
	if let Some(category) = card.category.filter(|c| !c.is_empty()) {
		let mut laid = lay(fonts, category, CATEGORY_SIZE, 1.2, TEXT_WIDTH, family);
		right -= laid.width;
		paint(
			&mut pixmap,
			fonts,
			&mut cache,
			&mut laid,
			(right, HEIGHT as f32 - PAD_Y - CATEGORY_SIZE * 1.2),
			colour(CATEGORY_ALPHA),
		);
		right -= BOTTOM_GAP;
	}
	if let Some(date) = card.date.filter(|d| !d.is_empty()) {
		let mut laid = lay(fonts, date, DATE_SIZE, 1.2, TEXT_WIDTH, family);
		right -= laid.width;
		// Sitting on the same baseline as the category, which is a larger size, so it is
		// pushed down by the difference rather than aligned on its own box.
		paint(
			&mut pixmap,
			fonts,
			&mut cache,
			&mut laid,
			(
				right,
				HEIGHT as f32 - PAD_Y - CATEGORY_SIZE * 1.2 + (CATEGORY_SIZE - DATE_SIZE) * 0.8,
			),
			colour(DATE_ALPHA),
		);
	}

	pixels
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_canvas_is_what_every_consumer_expects() {
		// 1200x630 is not a preference. Cards are cropped to it by the platforms that read
		// them, so producing anything else means letting somebody else choose the crop.
		assert_eq!(WIDTH, 1200);
		assert_eq!(HEIGHT, 630);
	}

	#[test]
	fn the_bottom_left_is_left_empty() {
		// X draws the domain over that corner. Everything in the bottom band is placed from
		// the right edge inward, so nothing can drift into it.
		assert!(PAD_X > 0.0);
		assert!(TEXT_WIDTH < WIDTH as f32 - PAD_X);
	}
}
