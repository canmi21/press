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

## A decoration painted on a box needs the box to hug the text

The spring underline is a background pinned to the bottom of its element, because a background
can grow from zero width and `text-decoration` cannot. That buys the animation and takes on one
liability: a background knows where the box is and nothing about where the baseline is.

Flex and grid stretch their items by default, so a link inside either gets whatever height the
row grew to, and paints its stroke at the bottom of that. Measured on a licence page, the same
class of link sat 27.5px under its glyphs in one row and 2.5px in the next -- the difference was
a neighbouring cell being tall, not anything about the link. `align-self: baseline` on the class
is the fix, declared once rather than at each call site: the failure is invisible until some
unrelated cell in the same row happens to grow, which is exactly the kind of thing nobody
remembers to guard at the point of use. It is ignored outside a flex or grid container.

## A label column is measured, never guessed

A two-column definition list whose label column is a fixed width is a bet that no translation
is wider than the number. The licence pages lost that bet in five of nine locales at
`6.5rem`: `Documentación` needs 111px against 104px and had nowhere to go, while
`Archivos de licencia` and `ライセンスファイル` wrapped to a second line beside a single-line
value. Both are the same fault wearing two faces, one for a word that cannot break and one for
a phrase that can, which is why a wider number only moves the boundary.

So the column is intrinsic -- `auto` -- and shared across the sections that have to line up,
through `grid-template-columns: subgrid`. Sizing each list separately would also never overflow
and would leave two lists on one page disagreeing about where their values begin, by 45px in
Japanese. Intrinsic sizing answers the translation, subgrid answers the alignment, and neither
answer is a measurement anybody has to maintain.

## An in-page jump scrolls without becoming an address

Navigation within one page -- an article's table of contents, the licence page's list of
licences -- moves the reader and leaves the URL alone. These are a way around a long document
rather than addresses worth collecting: a reader walking six sections would otherwise leave six
history entries behind and have to press Back six times to get out of a page they never left.

**Arriving with a hash still works, and is the browser's job.** A fresh navigation to
`/licenses#mpl-2-0` jumps natively on load. A reload of that same URL restores the position the
reader had scrolled to rather than jumping again, which is what a browser already does and what
somebody reloading halfway down a page wants. Nothing here re-implements either.

The distinction is `PerformanceNavigationTiming.type`: code that does take over the initial
jump -- the article ToC, which needs its own offset and a smooth landing -- acts only on
`navigate` and stands aside on `reload`. A fresh article navigation suppresses the native
fragment jump before the body is parsed, begins at the article top, then restores the hash
without moving and scrolls smoothly to it after hydration. Handling reload the same way would
throw away the reader's place.

The control stays an `<a href="#id">` and the handler cancels the default. Without JavaScript
the native jump happens instead, hash and all, which is worse than the scripted behaviour and
much better than a dead control. A modified click -- meta, control, shift, alt, or any button
but the first -- is the reader asking for a new tab, so it is left to the browser untouched.

Smooth scrolling is skipped under `prefers-reduced-motion`.

### A jump lands below the top edge, not against it

**A section reached by a jump keeps roughly a tenth to a fifth of the viewport above it.** A
heading flush with the top edge reads as the end of what came before rather than the start of
what follows, and it puts the reader's eye at the one place on screen it does not naturally
rest. Holding a margin above the target lands the section in the band people actually read
from, and keeps the last line of the previous section visible, so the jump is legible as a move
through one document rather than as a page being replaced.

The reserved space is **a share of the viewport rather than a fixed length**, because what is
being reserved is a share of what the reader can see. A constant that reads as a tenth of a
laptop window is a fifteenth of a tall monitor and a third of a phone held sideways. The default
answer is the `jump-target` utility in
[utilities.css](../apps/site/src/styles/utilities.css).

**The offset belongs to the target, as `scroll-margin-top`.** A native hash jump, a scripted
`scrollIntoView` and anything else that moves to the same element then land in the same place
without having to agree on a number, and nothing that jumps has to know the offset exists.
Arithmetic on the caller's side is the version that drifts: only one caller gets corrected when
the value changes.

The article ToC keeps its own offset rather than this one. It was measured against its own
indicator, which tracks the heading it points at, and a share of the viewport is not the
geometry that was tuned. An exception with a reason is not a second rule.

## Article home navigation yields to the table of contents

On a wide article viewport, the return control occupies the empty interval between the top of
the viewport and the rendered top of the table of contents. Its icon aligns vertically with the
article title in the collapsed default, giving that interval a deliberate upper bias rather than
an arbitrary fixed offset. Expansion keeps that alignment while there is room. Only when the ToC
would cross the corresponding midpoint does the control rise to the live midpoint between the
viewport and the ToC, splitting the remaining space evenly without moving the ToC itself.
Following that boundary must not add a second layout loop to the ToC animation. The return control
calculates its collapsed and expanded endpoints before the state changes, then runs its own spring
between them with a compositor transform. It does not sample the ToC's intermediate geometry or
inherit its trajectory, but the spring is tuned so both controls visually arrive and settle
together. A reversal starts from the control's current position, and reduced-motion preference
changes snap to the corresponding endpoint.

The text begins on the same vertical line as the ToC labels and bars. The return icon sits beyond
that line, making direction peripheral while the words preserve the rail's alignment.

The initial document already contains the collapsed ToC and the return control in their resting
positions. Heading identity and order are compile-time article structure, so withholding them
until the browser scans rendered headings only creates a late structural insertion. Browser-side
measurement progressively replaces the ToC's equal placeholder bars with widths derived from the
rendered labels; it does not create the navigation itself.

The control is absent with the ToC rail on narrow viewports. Moving it into the article column
there would turn a desktop spatial aid into another piece of article content and compete with
the title for the first line of attention.

At the other end of the article, its bottom edge becomes the side rail's lower boundary. The
resting layout does not move until the ToC would cross that edge. After contact, the ToC follows
the article end upward as the reader scrolls; expanding it keeps its bottom pinned and grows only
upward. The boundary is the end of `<article>`, before the blank interval and Newsletter divider,
so article navigation does not continue into the page's next region.

The return control applies the midpoint rule once to each resting ToC state, fixing the distance
between them until viewport resize or rail geometry changes. When the article end moves the rail,
the same lower-bound offset is added to both controls instead of dividing their new gap again.
They therefore leave the viewport as one spatial group. The return spring still interpolates
between its own collapsed and expanded endpoints, so a hover transition keeps an independent
trajectory even while both endpoints share the scroll displacement.

Scroll handling uses a cached document-space article end and observed box sizes: a scroll frame
performs arithmetic and compositor writes, not fresh layout reads. A pre-hydration frame applies
the collapsed endpoints after browser scroll restoration, before the component observers take
over, so reloading at the article end does not leave the rail centered until hydration.

## Boxed digits mark a number that moved; a standing fact is plain

The `value` cells in [app.css](../apps/site/src/styles/app.css) give a number its own boxes and
a monospace face. They were built for the subscriber count, and what earns them is that the
number is **live and answers to the reader**: it changed because somebody joined, and it may
change again while the page is open.

A number that is simply true of the site takes the surrounding text's own figures. The licence
page counts the packages it lists, which is a fact about the dependency tree rather than an event, and
setting it in cells claimed a significance it does not have -- the ink says "watch this", and
there is nothing to watch. Reserving the cells for the first kind is what keeps them meaning
anything.

Both remain locale-formatted through `Intl.NumberFormat`. The cells draw their own grouping
because no locale supplies a separator character to a box; a plain figure gets the reader's.

## Latin inside CJK is spaced with a real space

A Latin word set directly against Chinese or Korean needs air on both sides, or
`来自crates.io和npm` reads as one unbroken run. The space is a real one, the same character a
person typing that sentence would use.

Authored copy carries them already, in every locale and in the articles, and they are never to
be stripped. What needed solving is text this site _assembles_: `Intl.ListFormat` joins two
registry names with a bare `和`, and no author was there to type anything.
[spacing.ts](../apps/site/src/lib/locale/spacing.ts) inserts one at each boundary, and the
component keeps it outside the anchor, or the link's underline is drawn under the gap.

**Only script letters count, never punctuation.** A full-width `，`, `。` or `、` already carries
its space inside the glyph, so `npm，` stays tight and Japanese lists, which join with `、`,
gain nothing. Matching on Unicode script properties rather than a block range is what draws
that line.

`text-autospace: normal` was tried first and removed. It does work -- measured, it applies, and
it applies across element boundaries -- but Chrome implements the property's eighth of an em,
which came out at 2px against the 4.4px of a real space, and no other engine ships it. A rule
that lands on one browser and is invisible when it does is not worth the line it takes.

### A name that is two words is held together

A space inside a product name is a break opportunity, and in a line that is otherwise CJK the
break lands there: `均以 MIT` at the edge, `License 发布` starting the next. Interface copy
writes a non-breaking space inside such a name, as the JSON escape `\u00a0` rather than a
literal, so the next person to edit the file sees the character instead of deleting it by
accident.

This is for names a reader knows as one thing. Ordinary prose wraps where it likes.

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

### Quiet metadata controls share one surface

Compact icon-and-label controls in metadata rows use the shared `quiet-control` class. At rest
they are soft text with no surface. Hover and keyboard focus strengthen the text **and** add the
`paper-hover` background; changing only the ink leaves too little feedback for a padded button,
while a permanent surface would make secondary actions compete with the content. The article
summary disclosure is the reference control, and language selection and licence-page actions use
the same geometry and states rather than copying its utility list.

The visible focus outline stays on a `focus-link-inner` child, matching the text-and-icon shape
inside the padded hit area. A Lucide icon in this row is `0.875rem`. Mingcute icons use a
`1rem` height and automatic width, while a Lucide icon occupying that same language-marker slot is
`0.8125rem`; these are optical calibrations for their different view boxes, not interchangeable
box sizes.

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
