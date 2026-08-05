# Styling

## Browser lengths are authored in rem

The default authoring ratio is `16 CSS pixels = 1rem`, matching the site's root size on the
author's device. When the user describes a browser length in pixels without explicitly requiring
the `px` unit, treat the number as a design measurement and store its rem conversion. This covers
hairline borders and CSS written through JavaScript as well as ordinary layout declarations.

Geometry read from the DOM is reported in CSS pixels. Calculations may stay numeric in that
coordinate system, but any value written back to a style is divided by the live root font size and
serialized as rem. The helpers in [units.ts](../apps/site/src/lib/client/units.ts) keep authored
measurements and live DOM measurements distinct.

Pixel quantities intrinsic to non-browser-length coordinate systems do not convert: raster asset
dimensions, codec limits, favicon selection and fixed-size image or canvas composition remain
pixels. SVG view-box coordinates remain unitless. An external browser API that only accepts pixels
may keep them when no equivalent percentage is available; that constraint is documented beside the
call rather than being generalized into a styling exception.

## Keyboard focus follows the visible control

Keyboard focus uses a real two-pixel outline in the accessibility accent colour. The outline is
flush with the control rather than floating outside it: the visible edge is the location being
identified, and a second page-coloured moat makes compact controls look larger than they are. A
real outline also remains available to forced-colours mode; a `box-shadow` is not a substitute.

The focusable DOM box does not always represent the control. A padded row whose identity is an
icon puts the outline on that icon; a focusable code child puts it on the surrounding code frame.
The shared focus utilities in
[utilities.css](../apps/site/src/styles/utilities.css) cover direct, inner-child and containing-frame
placement so components do not redraw the same geometry locally. Controls with a visible border
may recolour that border instead when adding an outline would duplicate the edge.

Text links are a separate visual category from buttons and cards. Their outline follows the text
line height and a tight corner, even when an outer button has padding to make its hit target larger.
Inline icon-and-label links use that same height. Padding belongs to interaction geometry and must
not silently turn a text link into a tall focus badge.

Text inputs distinguish pointer focus from keyboard navigation. Pointer focus strengthens the
existing field border; keyboard focus adds the accessibility outline. Unlike buttons and links,
text inputs commonly match `:focus-visible` after a click because the caret itself must stay
visible, so the input pattern tracks the most recent input source rather than relying on that
pseudo-class alone.

Roving-focus menu items and SVG data marks keep their component-native highlighted surface or
stroke. Those states already identify the current keyboard target and forcing a rectangular ring
around them would describe the wrong shape.
