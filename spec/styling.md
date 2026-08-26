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

## A summary provider mark follows the last letter, not the punctuation

The provider mark at the end of an article summary is visually anchored to the text line above
it rather than to the paragraph edge. When the summary's final line has room for the mark, the
mark remains on that line and its right edge aligns with the final letter on the preceding line.
When the final line has no room, the mark moves to the following line and aligns with the final
letter on the summary's final text line instead.

Punctuation does not supply that anchor. A line ending in `block，` aligns the mark with the right
edge of `k`, and a Chinese sentence ending in `。` aligns it with the preceding Han character.
This keeps the mark tied to the last piece of ink that carries the sentence rather than to the
variable optical width of its closing punctuation. Because both the line break and the anchor
depend on the rendered font and available width,
[article.svelte](../apps/site/src/lib/article/article.svelte) measures them from the same browser
font metrics and recalculates them when the paragraph resizes.

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

## A number's treatment follows the role it plays

The `value` cells in [app.css](../apps/site/src/styles/app.css) give a number its own boxes and
a monospace face. They were built for the subscriber count, and what earns them is that the
number is **live and answers to the reader**: it changed because somebody joined, and it may
change again while the page is open.

A number that is simply true of the site does not earn boxes merely by being numeric. In running
prose it takes the surrounding text's own figures. A repeated trailing metric column has a
different job: the licence directories end each row or section with a package count, and
monospace tabular figures let those narrow values scan as one column. Soft ink and the absence of
boxes keep them subordinate to the names they quantify; this is directory structure, not a live
state.

Counts below one thousand stay as whole numbers. At one thousand and above, compact indicators
use the shared `compactCount` notation: lowercase `k`, uppercase `M`, and a decimal only while it
carries useful precision (`1k`, `1.5k`, `16k`, `2.3M`). This applies to the licence metric columns
as well as stat rows and chart axes. A count written into prose remains complete and
locale-formatted through `Intl.NumberFormat`; compact notation is for a bounded indicator, not a
sentence.

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

## A code language keeps its own name

The language label on a fenced article code block uses the canonical display name supplied by
the syntax grammar catalog: `HTML` remains an initialism, while names such as `TypeScript`,
`JavaScript`, `Objective-C`, and `C++` keep their established casing and punctuation. A fence
alias resolves to the same display name as its language id, so `ts` and `typescript` do not create
two visual names. Plain-text fences keep the label hidden. If a fence names a language outside the
catalog, its authored spelling is preserved rather than uppercased or guessed. Resolution happens
while content is compiled so the browser does not download the grammar catalog merely to print a
short label.

That same top-right position is the code block's copy control. At rest it retains the language
label, or an empty but focusable hit area for plain text. Pointer hover and keyboard focus keep that
label in place while a copy icon enters on its right: `motion` slides the right-anchored inner pair
through a clipping boundary, so the label yields left without a sudden replacement. The transparent
interaction area already has the final revealed width and never changes during the spring; an
animated hit boundary would repeatedly enter and leave a slowly approaching pointer. Reversing the
interaction continues from the live position. Without a language label there is nothing to yield:
the hit area keeps the same geometry and `motion` reveals the icon in place through opacity and
scale, without a lateral entrance or layout movement.

Activation copies the original source rather than reading highlighted HTML, then changes the icon
to a check or cross. Feedback remains for as long as pointer hover or keyboard focus remains. On
leave it starts a short delay before returning to rest, regardless of how long the result was
already visible. Returning before that delay finishes cancels the reset and preserves the result;
the next leave starts a fresh full delay. Its accessible name and live feedback come from the UI
message table. Closing hides the current check or cross before resetting to the copy state, so no
resting icon flashes through the exit. Reduced-motion readers receive each state without the mask
or icon transition.

### A titled code block is one framed disclosure

A fenced code block may carry `title`, `collapsible`, and `default` presentation metadata. A title
creates a header that remains visible in both states. It is collapsible unless explicitly fixed
open with `collapsible="false"`; its initial state is expanded unless `default="collapsed"` is
written. `default` accepts only `expanded` and `collapsed`. Collapse metadata without a title, an
unknown value, or a fixed-open block that asks to start collapsed is an authoring error rather than
a state the component guesses how to repair.

The titled form is one rounded rectangle. Its title surface owns the rounded top corners, the code
surface below has square top corners, and one shared outer border encloses both; nesting a second
rounded frame would make the join look like two cards stacked together. The title is a native
button only when the block can collapse, with `aria-expanded` and `aria-controls` naming the code
panel. A collapsed panel is inert as well as visually clipped, so Shiki's focusable `pre` cannot
receive keyboard focus while hidden. The panel uses `motion` to spring between its measured current
height and its content height, including when a reader reverses direction mid-animation. The title
separator remains until a collapse settles, so the moving surfaces never expose a transient seam.
Its border colour remains assigned while its zero-width collapsed edge is dormant; otherwise the
header's colour transition reveals a frame of text-coloured border when that edge returns.
Once expanded, the panel returns to natural height rather than retaining a stale measurement;
reduced-motion readers receive the state change without animation.

## A Mermaid fence becomes a diagram after hydration

Mermaid keeps its standard fenced-code authoring form. The language label is the switch: the site
compiles a `mermaid` fence as a diagram block instead of sending it to syntax highlighting, while
feeds and Markdown targets retain readable source. Keeping the standard form means the CMS's code
block schema preserves it without another custom Markdown node, and an editor can eventually put a
preview beside the same source rather than migrating articles to a repository-only syntax.

The public page renders the diagram in the browser. Only an article that contains one pays for the
Mermaid runtime, and the bordered paper frame is server-rendered first so the late SVG replaces a
deliberate loading surface rather than an empty hole. That first server-rendered frame names its
state with a quiet, centred `Loading diagram…` label as well as an abstract placeholder, so the
reader does not have to infer whether an unfinished graphic is decorative or still working. The
optional fence metadata `ratio="2.77366"` records the rendered SVG's width-to-height ratio as a
positive decimal. When present, the loading surface uses that ratio with the same `30rem` minimum
content width as the eventual result, reserving its responsive height before Mermaid loads. It is
authored geometry, not a heuristic. A missing ratio retains the `13rem` fallback; a known ratio
uses an `8rem` floor, so a short horizontal flow is not padded out to fallback height while taller
diagrams remain governed by their content. Malformed values fail content compilation. The frame
follows the ordinary code-block language without
copying its nested surfaces: one thin outer border
contains one uninterrupted paper background, matching the ordinary code surface. Diagram nodes use
the adjacent hover-paper step so they lift out of that deeper field without another component
frame. An inset border or contrasting padding band makes a diagram look heavier than the prose and
is not used. The stage centres every result vertically within its reserved height; Mermaid already
centres the SVG horizontally. A short horizontal flow therefore does not cling to the top of the
fallback-height frame, and the loading and final compositions share the same centre. Horizontal
overflow remains scrollable. A failed render leaves the authored source readable inside that
surface. Reduced-motion readers receive the final states without the loading pulse or reveal. The
boundary is implemented in [mermaid.svelte](../apps/site/src/lib/blocks/mermaid/mermaid.svelte).

Mermaid's theme engine accepts hex colours while the site palette is authored in OKLCH. It does not
justify changing the shared palette or scattering overrides across generated SVG selectors. A
component-only [palette](../apps/site/src/lib/blocks/mermaid/palette.css) therefore mirrors the
interface colours in hex for this adapter alone, with every light and dark value kept together.
Mermaid receives those values through its supported theme configuration; article-authored config
cannot replace the site's security, type, or palette decisions. The duplication is accepted and
local: changing a shared colour may require changing its Mermaid mirror, while every other consumer
continues to have one OKLCH source.

## A quadrant groups claims without inventing scores

A categorical comparison uses a `:::quadrant` container with `::quadrant-item` children. The
container names all four axis directions and gives the figure an accessible title; each item names
one of the four regions and may add one short note. A region may hold no items or several. This is
separate from Mermaid's numeric `quadrantChart`: when an article can defend only relative direction,
placing labels at exact coordinates would manufacture precision that the argument does not contain.

Visible copy is deliberately compressed because position carries the comparison. A title names the
decision in a few words, each axis end uses one short term, and a box normally contains only its
subject. An item note remains available for a distinction that position cannot encode, but it is
not a restatement of either axis. The longer explanation belongs in the figure description and in
the readable non-visual fallbacks. This keeps nuance without making every visual reader parse the
same relationship twice.

The container's optional `description` attribute is the author's place to explain the comparison's
context in Markdown. It is not required for accessibility: the component always generates an English
structural description from the horizontal and vertical axis endpoints and every item-region pairing,
with an explicit empty-state sentence when there are no items. When authored copy exists it precedes
that structural fallback rather than replacing it. The template connective language is deliberately
English-only; author-provided labels remain in their source language, matching the code-like directive
translation boundary.

The rendered figure uses a centred Cartesian cross. Its intersection stays at the exact centre of the
outer frame. The four regions first take their intrinsic item sizes, then the largest region defines
four equal-width and equal-height corner tracks. Content is not centred within those tracks. Every
non-empty region anchors its first authored item by the card corner nearest the cross, using the same
inline and block gap in all four directions; further items flow away from the cross. The nearest card
in a sparse region therefore aligns with the nearest card in a denser region opposite it, while an
empty region draws nothing and cannot pull another region towards the centre. The layout is tuned for
the common case of one to three items in a region; further independent items wrap outward instead of
being merged or stretching an axis indefinitely. A small minimum keeps sparse figures legible, while
maximum inline and block sizes preserve breathing room around dense ones.

Both lines span the full item area. Only after that boundary does the positive end add its arrow and
then its axis label; negative labels sit beyond the opposite boundary without an arrow. The result is
four content corners with a short axis extension at the centre of each outer edge, rather than labels
stealing length from the cross. The vertical line carries an arrow only at its top end and the
horizontal line only at its right end, so the positive directions remain explicit without decorating
all four endpoints. A region accepts zero or more independent items. Each item becomes its own
content-width bordered paper-hover label; siblings are centred together and wrap as a group instead
of being concatenated into an invented combined object, and no axis line crosses one. An empty region
is whitespace, not a dashed placeholder: absence already carries meaning here, while an outlined empty
object would imply missing or loading data. Numeric ticks remain absent.

The authored title is an accessible name and a non-visual fallback, not a visible title bar. A hidden
`figcaption` gives the title and generated description separate HTML nodes; the figure's image role
references them with `aria-labelledby` and `aria-describedby` instead of flattening everything into
one oversized accessible name. The visual stage remains `aria-hidden`, so a screen reader receives
the semantic summary once rather than traversing decorative axis and card markup.

The outer frame therefore contains only the visible comparison. It matches a code block or Mermaid
diagram and uses only shared interface tokens; it has no data-visualisation palette of its own. The
page receives static HTML and CSS; the figure adds no client-side renderer or component-local runtime.
Feed, Markdown and plain-text targets lower the figure to a readable list of axis-region labels and
items instead of dropping its meaning. Directive attributes remain structural and therefore follow
the existing non-translatable directive rule in [i18n.md](i18n.md). The boundary is implemented in
[quadrant.svelte](../apps/site/src/lib/blocks/quadrant.svelte).

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

## An article rule is a pause, not a wall

A Markdown thematic break inside article prose renders as five short strokes in the strong border
colour. Together they occupy roughly three sixteenths of the text measure and stay centred, because
the mark separates thoughts rather than dividing the page into structural regions. The strokes are
two pixels thick: enough to remain deliberate at that short length without becoming a structural
rule. Generous vertical space supplies the pause the compact mark implies.

## A quotation borrows a quiet surface

A source quotation in article prose stays a native Markdown blockquote. It does not become a custom
note directive: quotation is its meaning, while a note would describe an aside written by the
article's author and would erase that distinction from feeds and plain-text consumers. The rendered
quote uses the quiet hover paper as a slip behind the prose, with a two-pixel strong-border rail at
the start edge and rounded corners only where the slip is free. This is enough separation to make a
quoted instruction scannable without giving it the visual weight of an interactive card or a
warning. Multiple paragraphs keep a small internal gap so the slip remains one quotation.

## The subscription surface closes both reading paths

The same Newsletter component appears on the homepage and after the body of every article.
The homepage reaches somebody browsing the site; the article tail reaches somebody who has
finished reading. These are two entrances to one subscription, so they share copy, state and
presentation rather than growing page-specific variants that can drift apart. On the homepage,
Newsletter precedes Support so the larger subscription invitation remains part of the reading flow
and the smaller actions finish the page.

An article separates the invitation from its authored body with the same quiet one-pixel rule
used by the homepage's structural surfaces. The rule belongs to that placement, not to the
Newsletter default, because the homepage already arrives at it across a section boundary.
The invitation sits after the semantic `<article>`, not inside it: the table of contents scans
that boundary, so only headings authored as article content can enter its navigation.

Homepage-only interaction stays outside it. Support actions describe the site as a whole and
would turn every article ending into a second homepage footer; an article page ends after its
subscription invitation instead.

## Compact action rails reveal detail on demand

The homepage Support surface holds Like, Google source preference and Sponsor. These are reader
actions and read as one small section; revision and Follow stay off the page until they have a
quieter placement of their own. Visitor, uptime, word-count, update-age and license rows do not
appear on the homepage.

Each Support action presents an icon and its shortest useful identity at rest, while pointer hover
and keyboard focus reveal the full localized instruction in place.

The rail measures each localized short and long label, then springs the button between those live
widths with `motion`. This is computed geometry rather than a fixed hover target: locale, font and
the Like count all change the answer. When the short label is a substring of the instruction, that
shared text stays as one DOM segment. Prefix and suffix segments sit in zero-width masks driven by
the same spring as the pill: a suffix is uncovered after a stationary label, while a prefix pushes
the shared label right as it is uncovered. This makes the copy read as material revealed by the
pill rather than one string replacing another. Every shipped locale preserves that substring for
all three Support actions, with a message contract test guarding the relationship. The component
keeps a crossfade only as a defensive fallback; these actions must not rely on it. Translations
choose an idiomatic local short label first rather than forcing an English noun into every locale.

Like keeps its remembered state legible without making the whole rail permanently heavy. A click
fills the heart and updates the count; leaving returns the button to the ordinary paper surface.
Hovering or focusing a remembered Like inverts it to the ink surface. The same state changes must
remain understandable through `aria-pressed`, and reduced-motion users get the final labels without
the width transition.

Sponsor is deliberately unavailable while U.S. F-1 immigration restrictions apply. Activating it
opens a modal notice instead of navigating away. The rest of the page blurs behind the modal, and
either the close control or any point on that background dismisses it.

That notice is interface copy, so it resolves through the UI message table at the page's own
locale like every other string around it -- heading, sentence and the close control's label
alike. It names the restriction plainly in all nine views rather than softening to a generic
"unavailable": the reader is being told why an offered action does not work, and a reason that
survives translation is the only version of that sentence worth having.

Its heading is visible rather than announced to assistive technology alone. A modal carrying one
sentence and a bare close control reads as a fragment of the page rather than a surface of its
own, so the notice opens with the icon of the action that summoned it beside a heading weighted
like the page's other section headings, with the sentence below in the metadata text colour. The
icon and the close control are each centred on one line box, so a heading that wraps in a longer
locale moves the text without dragging them out of line with its first line.

Data palettes belong to the visualisation that gives them meaning, not to the site theme. The
Cargo palette lives in a component-only stylesheet scoped below `.cargo-widget`; it stays vivid
in both page themes and never becomes a token available to unrelated interface chrome.

## Motion runs at runtime only when the value is not known in advance

`motion` is a dependency, and reaching for `animate()` is the wrong default. It earns its place
where the target is computed -- the article list measures the corpus before it knows what widths
to animate to, and no stylesheet can hold a number that does not exist until the page has read
its own content. When a hover, open or state flip has targets written in the source, running it
through a library puts a per-frame JavaScript cost on an animation CSS was going to composite
anyway.

Wanting spring physics is not a reason to cross that line. A spring is a curve, and a curve can
be sampled once and written as a CSS `linear()` easing -- which is what the library itself emits
when it hands an animation to the browser. Sample it from `motion`'s own generator so the
physics are not reimplemented by hand, then paste the result. The repo keeps the real curve and
spends nothing at runtime.

Sampled once means stored once. The curve lives in `--ease-spring` and every consumer reads it
from there; a second copy of those points is how two animations meant to feel identical begin to
drift apart.

One trap worth stating, because it is invisible until someone wonders why the bounce never
shows: an overshoot has to have somewhere to go. A spring driving `background-size` or a colour
is clipped at its limit, so the overshoot is spent on nothing and the curve should simply be
damped out. A transform or an unconstrained layout dimension such as width has room to show it.

## A block card is a box, and its arrow says whether the destination leaves the site

Everything a directive drops into an article body -- `::github`, `::linkcard`, `::article` --
wears the same box: a one-pixel border on paper, a 0.75rem radius, and a corner arrow that
appears on hover. That shell is what separates a card from the prose around it, so a card
without one reads as whatever it happens to sit next to. `::article` first shipped as the
homepage's row lifted whole and read as a stray list item, which is what the rule came out of.

**A card sits on the column's left edge.** Prose is left-aligned, so a centred card is the one
element in the reading column that does not start where every line above it starts, and a lone
one reads as a divider rather than as something quoted. `::github` still centres by default,
which is its `align` attribute's to change; a card with no such attribute is left and stays left.

**The arrow's direction is not decoration.** `↗` says the link leaves the site and pairs with
the `target="_blank"` those cards carry; `→` says it stays. Getting this backwards is the easy
mistake, because a new card is written by copying the nearest existing one and both of the
first two point outward. A reader who has learned the pair reads the arrow before the URL.

What a card holds is the subject's, not the box's: [media.md](architecture/media.md) for a
cover, [workspace.md](architecture/workspace.md) for why a card pointing inside the corpus
carries no copy of its own.
