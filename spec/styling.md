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

Keyboard focus uses a real `0.125rem` outline in the accessibility accent colour. The outline is
flush with the control rather than floating outside it: the visible edge is the location being
identified, and a second page-coloured moat makes compact controls look larger than they are. A
real outline also remains available to forced-colours mode; a `box-shadow` is not a substitute.

The focusable DOM box does not always represent the control. A padded row whose identity is an
icon puts the outline on that icon; a focusable code child puts it on the surrounding code frame.
The shared focus utilities in
[utilities.css](../apps/site/src/styles/utilities.css) cover direct, inner-child and containing-frame
placement so components do not redraw the same geometry locally. Controls with a visible border
may recolour that border instead when adding an outline would duplicate the edge.

No control shows the browser's own focus indicator. Chrome draws that as a two-tone ring, a light
contrast edge paired with its blue, which reads as a stray white border against these surfaces, and
it reaches anything that takes focus without opting into one of the utilities above -- a menu panel
that focuses itself as it opens is the case that surfaced it. A base-layer rule replaces it with the
same accent outline rather than removing it, so a control that was never given a focus utility stays
visible to the keyboard rather than going silent. Suppressing focus outright belongs only where
something else already marks the position, as with a parent that hands its outline to a child.

Text links are a separate visual category from buttons and cards. Their outline follows the text
line height and a tight corner, even when an outer button has padding to make its hit target larger.
Inline icon-and-label links use that same height. Padding belongs to interaction geometry and must
not silently turn a text link into a tall focus badge.

That corner is applied only while the outline is drawn. A radius also clips the element's own
background, and one sized to round a focus outline is several times the height of a stroke painted
along the bottom of the same box, so a resting radius shortens that stroke's lower edge without
touching its upper one and bows a straight line into a lens. On a text link the radius has no work
to do outside focus, so it belongs to the focus state rather than the base rule.

Article prose links carry a thin, rounded underline in the strong border colour at rest, then draw
another in the article metadata text colour from left to right on hover or keyboard focus. Each
stroke sits one step below the previous one on the neutral ramp, which runs strong text, text, soft
text, strong border, border. Holding both strokes under the prose they mark keeps the affordance
subordinate until interaction. The second stroke uses the same sampled non-linear spring as the
translation notice link. It is a layered background rather than `text-decoration`, because the latter cannot
animate its width; the resting layer remains visible throughout, so the animation reinforces an
affordance instead of being the only indication that the text is a link.

## `:focus-visible` is the browser's guess, and the site keeps its own answer

The pseudo-class is a heuristic, and it is not ours. Where it is least reliable is focus a script
moved: a menu handing focus back to its trigger as it closes is the case that matters here, and
engines disagree about whether that counts. Guess wrong on a phone and a keyboard affordance is
drawn for somebody who has no keyboard.

So the document records what the last input actually was -- `keydown` of a navigation key marks
`kbd`, `pointerdown` marks `pointer`, and touch arrives as a pointer like any other. **A
positively known pointer takes the outline away**; the pseudo-class still decides everything else.

**It is written as a suppression, never as a keyboard requirement**, and that asymmetry is the
point. With the attribute absent -- no input yet, or the tracker never installed -- the rule does
not apply and plain `:focus-visible` stands. It can therefore show a ring once too often, and it
can never leave a keyboard user with no indicator at all. The opposite spelling fails silently in
exactly the direction that matters.

Text inputs are the older, narrower case of the same idea. Pointer focus strengthens the existing
field border; keyboard focus adds the accessibility outline. They needed it first because a text
input commonly matches `:focus-visible` after a click, the caret having to stay visible, so the
pseudo-class alone was never enough there.

Roving-focus menu items and SVG data marks keep their component-native highlighted surface or
stroke. Those states already identify the current keyboard target and forcing a rectangular ring
around them would describe the wrong shape.
