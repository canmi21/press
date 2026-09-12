# Styling

## Browser lengths are authored in rem

The default authoring ratio is `16 CSS pixels = 1rem`, matching the site's root size on the
author's device. When the user describes a browser length in pixels without explicitly requiring
the `px` unit, treat the number as a design measurement and store its rem conversion. This covers
hairline borders and CSS written through JavaScript as well as ordinary layout declarations.

Geometry read from the DOM is reported in CSS pixels. Calculations may stay numeric in that
coordinate system, but any value written back to a style is divided by the live root font size and
serialized as rem. The helpers in [@canmi/units](../libs/units/src/index.ts) keep authored
measurements and live DOM measurements distinct; they moved out of the site when the CMS began
animating lengths of its own, and [units.ts](../apps/site/src/lib/client/units.ts) re-exports them
so the ten components importing that path did not have to change.

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

A note's marker and the way back from a note are on this too, and are the reason the modifier
test and the reduced-motion check live in [jump.ts](../apps/site/src/lib/client/jump.ts) rather
than inside one component: a marker in prose arrives as compiled HTML with no component to hang a
handler on, so the article root listens for all of them at once. Both ends are jump targets, so
returning to a marker keeps the same band above it that arriving at a section does.

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

## The article is centred; the rail adapts to the region beside it

The table of contents and the return control are one rail down the left of an article, and they
move as one: the same box, the same left edge, written once and consumed by both. They differ
vertically and nowhere else.

**The article never moves.** It is centred in the window at every width, so the region to its
right is empty and exactly as wide as the region on its left. That left region is all the rail
has, and the rail adapting to it is never allowed to shift the article -- a column of text that
slides sideways as a window is dragged is a worse fault than any arrangement of the furniture
beside it. Below the width where the article can hold its own size, the article is what gives,
which is a matter this rule stays out of.

The region runs from the window edge to the article's **first letter**, and the rail's box is
centred in it. To the letter rather than to the column's frame, because the frame is not something
anybody sees: measured against it the rail sat 35px from the window and 60px from the text on an
iPad, and the 25px of column padding in between has nothing the eye can weigh it against, so the
rail reads as pushed left. The page gutter belongs to the region on this side exactly as it does
on the other. Three stages follow:

1. Too narrow: no rail. The article alone, centred, as on any other page.
2. Just wide enough: the rail appears and the spare room is thin, so the left margin takes two
   thirds of it and the gap to the article takes one. An even split here would be even between the
   wrong two things: the rail's leftmost ink is not its text but the return control's glyph, which
   hangs `--rail-icon-overhang` further out, and on an iPad mini an even split left that glyph 27px
   from the window while the entries had 47. Two thirds gives it 43, which is what the next stage
   gives it anyway.
3. Roomier: the margin holds flat at `--rail-hold` while the halves catch up.
4. Wide: the even split, the rail's centre line on the region's centre line, so its margin from the
   window edge and its gap to the article are equal.
5. Past `--rail-left-max`: the left margin holds still, and every further pixel goes into the gap
   between rail and article.

**Stage 5 caps the margin, and it used to cap the centre line.** That was right while the box was
`fit-content` and its width was the browser's answer rather than a number this file held. The box
is `--rail-width` now, so the two say the same thing and the margin is the one the eye reads --
and reading it is what showed the cap was set too far in. At 14rem of centre the rail sat 156px
from the window on a 1600px screen, which is a rail drifted halfway to the text rather than one
beside the window, and a rail is read from the corner of the eye.

It stops at 6rem, the same length the article column's top padding stops at. Past the width where
both are capped the page has one outer breathing room and spends it twice, down the side and above
the title. The whole sequence, in pixels:

| window | region | left margin | gap to text | branch |
| ------ | ------ | ----------- | ----------- | ------ |
| 1088   | 208    | 48          | 24          | two-thirds |
| 1120   | 224    | 59          | 29          | two-thirds |
| 1200   | 264    | 64          | 64          | hold |
| 1280   | 304    | 84          | 84          | even |
| 1360   | 344    | 96          | 112         | capped |
| 1600   | 464    | 96          | 232         | capped |
| 2560   | 944    | 96          | 712         | capped |

Monotonic and continuous throughout, and every branch of it is one CSS expression.

**Stages 2 through 4 are one expression, and stage 3 is why.** Two thirds of the spare and half of
it are two lines that meet only at zero, so switching between them at a width steps the rail
sideways -- 21px, at the width that switched. A flat hold between them joins the branches where
`2/3 s` reaches the hold and again where `s/2` does, which makes the margin continuous at every
width and puts no second breakpoint in a file that already warns about the one it has. Measured:

| window | spare | left margin | gap to text | glyph from window |
| ------ | ----- | ----------- | ----------- | ----------------- |
| iPad mini, 1133px  | 95px  | 63px | 32px | 43px |
| 11-inch iPad, 1210px | 133px | 67px | 66px | 47px |

The mini takes the two-thirds branch and the 11-inch the even one, which is the pair `--rail-hold`
was chosen against.

Stage 3 exists because a rail is read from the corner of the eye. Left centred forever it drifts
inward as the window grows, and on a wide monitor a rail halfway to the text is neither beside the
article nor at the edge of anything.

### The rail's box is one declared width

It is one box in the DOM, holding both the table of contents and the return control, rather than
two elements agreeing on a number. `translate: -50%` centres it. **The width is declared --
8.5rem -- and is the same on every article, in every language, and in the first frame the server
sends.** Nothing measures anything to arrive at it.

Getting here took two wrong answers, and both are worth keeping because each looks correct until
it is running.

**A measured width published as a custom property.** No server can know it, so the page painted
at one position and jumped to another the moment it hydrated.

**A box sized to its entries.** `width: fit-content` reads as obviously right -- the entries are
the only thing a reader sees, so why centre anything else -- and it is stable within one view: a
web font swapping in re-sizes and re-centres it with no listener to forget. What it is not stable
across is _content_. The box moves its own centre whenever its entries change width, and the
return control is centred on that box, so the control tracks the length of the longest heading.
Switching one article between languages took the widest entry from 48px of Korean to 104px of
German and slid `Back` 28px across the page. Measured across the corpus, the source views alone
spread the box from 3.25rem to 8.34rem.

That is the fault: **the return control is a fixed part of the page and has no business tracking a
heading.** A reader switching languages is comparing two views of one article, which is exactly
when a control moving 28px is most visible and least explicable.

**8.5rem is where the source headings stop.** The widest source view reaches 8.34rem; the rest
are well under. A translation longer than that wraps, which is what the second line is for -- and
in the article whose headings are longest, five of thirteen Spanish entries take it. Wrapping is
a legible outcome and a moving control is not, so the trade is made in that direction.

The cost is accepted rather than hidden: an article with short headings no longer fills its box,
so its entries sit left of centre with space to their right. That space buys a control that does
not move.

**A wrapped entry gets two comparable lines**, through `text-wrap: balance` on the label. Left to
fill and spill, the break lands wherever the width runs out -- `Independencia de la` over `UI` put
nineteen characters above two, which reads as a mistake rather than as a wrapped label. Balance is
built for exactly this shape of text, short and headline-like, and it evens out one label's own
lines without looking at its neighbours. Where it is unsupported the text fills as before.

**Balance evens the lines; it does not choose where the break may land, and for Han that is the
part that matters.** The label also carries `overflow-wrap: anywhere`, which lets a break fall
between any two characters. For a run of Han that is usually right, and next to a space the author
wrote it is not: `不使用 JS 运行时的代价` came out as `不使用 JS 运` over `行时的代价`, splitting a
word to fill three more characters when the boundary had already been written as a space. Balance
left it there, because two lines of eleven and five characters are comparable -- the fault was in
which breaks were allowed, not in how the lines were evened.

So a script whose spaces are boundaries prefers them: `word-break: keep-all` for `:lang(zh)` and
`:lang(ko)`, which turns that entry into `不使用 JS` over `运行时的代价`. `overflow-wrap: anywhere`
stays underneath as the floor, so a Han run with no space in it still breaks wherever it must,
exactly as it did before.

**Japanese is excluded, and the measurement is the argument.** Its spaces are not boundaries in
the same sense, so `keep-all` there only removes the opportunities the script does have:
`Web フレームワークだけではない` came apart into four lines, one of them a single kana. Korean is
included on the script's terms rather than on a case observed here -- no entry in the corpus wraps
in Korean yet, and the first one that does would otherwise split mid-eojeol, which is the same fact
the article prose takes `keep-all` on.

Latin was measured too and left alone. `keep-all` moves two of its breaks, one for the better --
French stops splitting `sans-runtime` -- and one for the worse, German stranding an opening quote
at the end of a line. Nothing there was asking to be fixed.

Collapsing changes nothing: the bars occupy less of the box, and the box, the centring and the hit
area stay where they were.

**Nothing inside can widen the box.** The return control is taken out of its flow and the active
indicator is absolutely positioned, which is what lets the indicator and the return icon hang
outside its left edge as ornaments. Both are tuned to that edge: the icon is translated left of it
so the word the control carries lines up with the entries, and a box drawn around either would
push every entry right by the width of a decoration.

The breakpoint that decides whether the rail appears at all is derived from this width by hand --
`8.5rem + 2 * 1.5rem` of clearance beside each side of a 45rem article -- because a media query
cannot read a custom property. It is no longer a test of whether the widest possible rail would
fit; with one declared width it is the rail. Change the width and change that number with it.

### Collapsed, the bars are a thumbnail of the list

The collapsed rail reads as the table of contents seen from too far away to make out words: how
long each entry is, and where the list rises and falls. That is all anyone reads off a column of
bars, and it is the whole design brief -- **a thumbnail, not a chart**. Exact widths were tried
first and said less than they cost; below a tenth of the longest heading the differences are
noise dressed as precision.

**An entry that wraps contributes half its width per line.** Measured flat, a heading that will
occupy two lines still reported one long line -- and being the longest, it set the scale every
other bar was divided by. The rail's longest bar then belonged to the one entry that is not a long
line at all, and everything else was flattened underneath it. The width is measured in the
label's own font rather than the heading's, because the wrap happens in the rail at the rail's
size and the two fonts are not proportional to each other.

**Ten steps of the longest heading**, so a bar is the fraction of the longest heading that this
one is -- which is what it looks like it means. Scaling between the shortest and the longest
instead would spend the whole range on whatever spread the article happens to have, drawing two
headings of six and seven characters a third of the rail apart.

Three rules then shape the column, and each exists because the step alone produced something
that read badly.

**No entry stands more than three steps above a neighbour, and the outlier comes down.** Raising
everything around it satisfies the same constraint and is not the same thing: one entry towering
over its neighbours is what reads badly, so the fix is to pull _it_ back rather than stretch the
rest of the column toward it, which would spend the top of the scale on an article that has
nothing that long in it. **The tenth step is therefore not something every article reaches** --
it is there for a heading long enough to earn it against the company it keeps, and an outlier, by
being an outlier, does not.

**Two neighbours on the same step are separated by one step**, toward whichever side they were
already nearer. One step rather than more: these two headings are genuinely the same length, and
a bigger push would say they are not. Two steps was tried and is too much -- with evenly sized
headings it leaves only differences of two and three, so the column can do nothing but alternate,
and equal headings get drawn three steps apart. **A tie is broken by the heading's own text**, so
the same heading falls the same way on every render while two different ones in the same position
do not; anything derived from the index would make every article break its ties identically,
which is a pattern rather than a choice.

**A column where nothing reaches the low end slides down until its shortest entry rests on the
third step.** An article whose headings are all long has no short bar to anchor it and reads as
uniformly heavy, with the bottom of the scale unused. The third step rather than the first,
because the shortest entry in such an article is still a long heading. This is **a shift, never a
rescale**: every difference above it was chosen by the two rules, and rescaling would quietly
undo them, where moving all of the steps by one amount changes none of them.

So neither end of the scale is guaranteed. The first step is reached by an article that really
has a two-character heading among longer ones; the last by one that has a heading long enough to
earn it. The middle is where every column lives.

**A bar may never be wider than the widest label.** The box is `fit-content` around the entries
at full expansion, which holds only while the text is the widest thing in it -- and in an
article whose headings are all short it is not. Two of them had a longest label of 52px against
a 64px bar, so the bars sized the box, and hydrating them from their served width to their real
one widened it under a control centred on the same box: the rail sat still while `Back` slid 6px
left, over the whole length of the bar animation. Capping the bars at the widest label as drawn
restores the invariant rather than patching the symptom -- a thumbnail should not be wider than
what it is a thumbnail of.

Every bar is served at the fifth step and animates to its own, so the column resolves outward
from the middle rather than growing from nothing.

### Absent rather than squeezed

The rail appears only where the region holds it with `--rail-edge` clear on both sides. An earlier
rule gave it whatever the region had, so between the old breakpoint and the width it actually
needed it was drawn narrow and pressed against the window edge, where entries wrap to three lines
and the column reads as something that fell off the page. A control that cannot be shown properly
is better not shown: the headings are still in the document, and the article is what the reader
came for.

The test is made against `--rail-width` rather than the measured box, because a media query
can read neither. So it asks whether the widest rail this site can draw would fit, and an article
with short headings is shown no earlier than one without -- the alternative is a breakpoint that
moves per article, which is a worse thing to explain than a conservative one.

`--rail-edge` decides both when the rail appears and how much air it has when it does, and the two
cannot be separated: centred in the region, its margin and its gap are the same length. Raising it
buys a rail that never looks cramped at the cost of a band of window widths that show none.

**The breakpoint is now three rem conservative, deliberately left so.** The region grew by the page
gutter when it was redefined to reach the text, so the clearance test it encodes is met at 65rem
rather than the 68rem the media query still holds. Moving it would make the rail appear on windows
that have never shown one, which is a change to what the page is rather than to how it is spaced,
and the spacing fix did not need it. The number stays where it is until somebody decides that
question on its own terms.

### Rejected: centring the rail and the article together

Treating rail, gap and article as one block centred in the window balances the page at every width
and was built to see. It moves the article -- right by half the rail as the rail appears, and off
the window's centre from then on. The article holding still is worth more than the balance.

The lengths this is computed from sit together on `:root` in
[utilities.css](../apps/site/src/styles/utilities.css) so they can be argued with in one place. One
is repeated by hand: the width at which the rail appears is written as a number in the media query
and has to be kept in step with the ones it is derived from.

## The space above the title is the space beside it

The article column's top padding is not a number chosen per width. It is the distance from the
window's edge to the first letter, measured on the side and applied to the top, so the text sits
the same distance from the edge above it as from the edge beside it.

Below the column's cap that distance is the page gutter and nothing else: a phone gets 1.5rem, the
same length `px-6` spends on each side, and the heading sits an even margin from three edges. Past
the cap the column stops growing and every further pixel of window becomes margin, so the distance
grows and the heading is pushed down by exactly what has opened up beside it. One expression,
continuous through every width, with no breakpoint and no script in it:

```css
padding-top: clamp(var(--page-gutter), var(--page-gutter) + (100% - var(--rail-column)) / 2, 6rem);
```

**The cap is what makes the two stages meet without a step.** 6rem is what the top was before any
of this and what the bottom still is, and the growing gutter reaches it at 54rem -- fourteen rem
before the rail appears at 68. So by the time the page grows a table of contents the heading has
already settled where it used to be, and the rail's arrival moves nothing vertically. Reverse that
ordering and the breakpoint becomes a jump. Measured:

| window | side gutter | top padding | rail |
| ------ | ----------- | ----------- | ---- |
| 390px  | 24px        | 24px        | no   |
| 744px  | 36px        | 36px        | no   |
| 1133px | 231px       | 96px        | yes  |

The bottom keeps 6rem at every width and is not part of this. The space under the footer competes
with nothing, so there is nothing for it to yield to on a phone -- which is the asymmetry's whole
justification: the top is expensive because it stands between the reader and the first word, and
the bottom is not.

`--page-gutter` exists so the length the sides spend and the length the top spends are one
declaration. Changing it, or `--rail-column`, moves where the cap is reached; check it still lands
below 68rem.

## A subsection is nearer, and for that reason unlisted

An article may carry a second heading level. It renders at the same size, weight and colour as
a section, and differs only in sitting closer to what precedes it -- `mt-8` where a section
takes `mt-12`.

**Type size cannot carry this distinction, because it is already spent.** The scale runs 16px
title, 15px heading, 14px prose: one pixel apart, separated by weight rather than size. A third
level below that lands on the prose size with only weight left to spend, and weight is what
divides a heading from prose in the first place. Enlarging the section to make room would undo
the restraint the whole scale is built on. Space is the remaining signal, and the honest one:
sitting nearer says _this belongs to what is above it_, which is exactly the relation.

**Only sections are listed in the table of contents.** The rail is 192px wide and collapses to
a column of bars -- a way to reach a section rather than an outline of the article. A subsection
is reached by arriving at its parent and reading on.

The two rules hold each other up, and neither works alone. Filtering the rail while the two
levels look identical would make the listing look incomplete: a reader who cannot see that a
heading is a subsection can only conclude the table of contents lost it. Made visibly nearer,
the same listing reads as complete, and needs no explanation. It also keeps the levels honest
for a screen reader, which is told the nesting either way.

**What is filtered is the listing, not the address.** Both levels are anchored, both resolve,
and a link to a subsection works exactly as before. Demoting a heading is therefore reversible
in the reader's terms even though it changes the segment id -- see [i18n.md](i18n.md) on
migrating the translations rather than rebuying them.

Which headings may be demoted is a question about the article, not about the rail: a subsection
is one its parent can stand for. Demoting a heading because it is short, or to tidy the rail,
trades a reader's ability to reach it for an appearance, and that is the wrong way round.

## The return control rests level with the title

The control at the top left of an article sits on the same line as the article's heading. The two
are the first things on the page, and one floating above the other reads as attached to nothing.

It leaves that line only when something is in its way, which is the table of contents: opened
under the cursor, or simply tall with entries in a short window. What counts as "in its way" is
half the control plus the gap it keeps from the first entry, measured rather than assumed so a
larger root size or a second line of text moves the threshold with it. Nothing here knows whether
the entries are open or merely numerous -- both are a taller rail, and the same rule answers both.

Below twice that clearance there is no room left to keep, and the control takes the middle of the
band that remains rather than being pushed off the top edge. The two expressions are equal exactly
where they meet, so the control slides between them as a window is resized instead of jumping.

The rule before this one put the control in the middle of the band above the entries whether or
not the band had room to spare. On an ordinary article that sat it 12px above the title -- close
enough to look like a mistake rather than a decision, which is what it was.

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

## An author's note is written where it is meant, and numbered where it lands

`:fn{is="..."}` marks the word it follows and sends what it says to the end of the article, where
the notes are collected in order with a way back to each marker. It is the numbered note of a
book, not the popover a translator leaves -- that is [`:tn`](i18n.md), which explains a phrase in
place and is written by a machine rather than by hand.

**The note travels with the text it explains, not with a label.** GFM's footnote is the other
shape -- `[^seam]` in the prose, `[^seam]: ...` at the bottom -- and it was rejected for two
reasons, in this order:

- **The editor cannot keep it.** The CMS is where articles are written, and its markdown is
  CommonMark with hand-written node schemas for the constructs this repository added. Round-tripped
  through it, `[^seam]` comes back `\[^seam]` -- escaped, silently, in both the marker and the
  definition. Teaching it otherwise means two more ProseMirror schemas, one of them a block
  container, and an answer for what a definition looks like while it is being edited.
- **A label is a second thing to keep true.** Writing the note where the marker is removes the
  pairing entirely: nothing to match, nothing to renumber, nothing that can be deleted at one end
  and left dangling at the other.

Numbers are the machine's. They run across the whole article in the order the notes are written,
so inserting one renumbers the rest without anybody touching them. **Two notes that say the same
thing are two notes** -- there is no label to merge them by, and a reader who met the explanation
twice was given it twice on purpose.

### What the shape costs

The note lives in a directive attribute, and that has two consequences worth knowing before
writing one.

**It cannot contain a straight quote.** The syntax has no escape for one, so the parser drops the
whole attribute and leaves a marker that says nothing -- silent, and indistinguishable from a typo.
The build refuses it, naming the article, and `validate.rs` refuses the same shape coming back from
a translation. Curly quotes are fine, as are braces, backticks and asterisks; all arrive as
literal text.

**It is flat text.** No links, no code spans, no emphasis inside a note. If one ever needs them,
the escape hatch is a container directive -- and that brings the pairing problem back with it, so
it is not worth reaching for until a note actually needs it.

### The collected notes are set like the article

The section at the end is small and quiet, at roughly three quarters the prose's size and the
soft text colour. That is what a note at the foot of a page is: something to step over on the way
past and come back to deliberately, not something competing with the article for the same
attention. Set at the article's own size and colour it did compete, which is how the ratio was
arrived at rather than guessed.

**One bright thing per note, and it is the number.** The whole line is soft, the quoted phrase
included -- weight alone marks the phrase, which is enough to find it by without spending colour
on it. The colour goes to the superscript, because that is the part a reader is actually looking
for: it says which of the markers above this note answers.

An earlier attempt at quiet was uniformly grey and small with nothing to catch on at all, and read
as somebody else's apparatus rather than the writer speaking under their breath. Grey was not the
mistake; grey with nothing in it was.

The heading above the section is the exception in the other direction: it renders at the same
size and colour as the article title and the newsletter heading, none of which sets a size of
its own -- three section names on one page, matched by sharing the inheritance chain rather
than by copying a number. Only the notes under it stay small.

**Closing the fold carries the page with it.** Closing shortens the document, so a reader near
the end is left above a bottom that no longer exists and the browser pulls them up to the new
one. It is right to, and it arrives at the worst moment: the pull begins partway through the
animation, the instant the document becomes shorter than the current scroll position, and lands
as up to 50px in a single frame after a stretch of no movement at all. That discontinuity is what
reads as a lurch -- the movement itself is unavoidable, since the reader is looking at the very
notes being folded away.

So the movement is taken over and spent on the same spring as the height, with progress read from
the distance the panel has covered rather than from a clock, which keeps the two on one curve
instead of on two that agree by luck. Opening needs none of it: a longer document never forces
the page to move. And the reader outranks it -- scrolling during the animation leaves the
position away from where the carry last put it, which is taken as steering, and it stands down
for the rest of the move.

**A wrapped note is balanced, and that was measured rather than reasoned.** By category it is
the wrong answer: a note is a sentence, and the prose elsewhere on this site uses `text-wrap:
pretty` wherever the language breaks between words, which leaves lines full and only refuses to
end on a word alone -- see "Where a line ends is declared per language" below. Measured on the Spanish
view of the article with thirty-three notes, `pretty` changed nothing at all -- every line came
out identical to plain filling, because it intervenes only when the last line is down to about a
word, and these end on a quarter of a line. `balance` closed all four of the short endings. The
same property is on the table of contents labels for the ordinary reason, that they are titles.

Neither does anything for the Chinese views. Chinese breaks between almost any two characters, so
filling already reaches the end of the line and there is nothing left to even out -- all three
modes render identically. This class of property is for the languages that break between words.

**The walk back from a note lights the words it lands on.** Arriving in the right scroll band
is not the same as knowing which words were left: the noted words may sit anywhere in their
line and appear more than once in the paragraph. So the return fills the words with the
selection colour the instant the scroll settles -- the reader's own "I marked this" ink,
themed for both modes, worn tailored where a drag is raw: rounded, hugging the glyphs, whole
on its first frame because the instant of arrival is the message -- and fades it slowly, the
marker's number lit beside it and letting go on the same clock.

The light is painted above the text as its own translucent layer, never as a background on
it. A background sits under the element's children, and any child that brings its own -- an
inline code span is the ordinary case -- swallows the light exactly where it lands; a layer
on top cannot be covered by anything the content grows later, and the selection colour's own
alpha keeps the words readable through it. The layer's boxes are read off the text's rendered
line fragments at the moment of arrival, one per fragment, so the wrap semantics below are
geometry the layer copies rather than CSS it relies on. A dotted underline was tried
under the fill and cut: two inks were saying one thing, and the fill already says it in the
reader's own colour.

The walk down gets the same light, on the note's whole line: the phrase and the explanation
compose one sentence, so the fill covers both rather than picking a half. Across a wrap the
line's box is sliced where the prose words' is cloned, and the asymmetry is the meaning: the
square edges at a break say the sentence continues, where two finished pills would say two
things -- a real drag-selection breaks the same way, and this is its ink. The marked words in
prose are short and each fragment is wholly "the words", so their boxes close.
`:target` cannot carry this, deliberately: the move never touches the URL (see above), so a
class set by the jump does, and the compiled prose wraps the noted words in a span that exists
only to receive it. Accent is not spent here -- appearing and disappearing is what catches the
eye, and the colour that means focus should not also mean "you came from over there". Under
reduced motion the highlight appears and leaves without animating; the information is kept.

**The notes render after the article, not at the end of it.** The rule above the newsletter had
always been the article's ending boundary, and the notes are apparatus about the article rather
than part of it -- so they belong below that boundary, not above it inside the body. Moving them
out is also what keeps the rail and the table of contents honest without either changing: both
measure the article, and the notes were the one thing inside it that was not article. The
boundary rule is worn by whichever section comes first -- the notes when the article carries
any, the newsletter otherwise -- and sits in the same place either way.

**The boundary hairline is dashed; the one between notes and newsletter is plain.** What follows
the last paragraph is offered rather than fenced off -- the notes are the article's own words
stepped down into, and the newsletter is an invitation -- so a solid rule there would read as
closing a door the page wants open. Between the notes and the newsletter, though, a rule only
separates two offerings from each other, and that is ordinary chrome: plain, like the divider
inside a translator's note popover. The site draws dashed elsewhere on the boundary reasoning:
the leader on an article card, joining a title to its date rather than dividing them, and the
leaders on the licence pages.

The notes are spaced tightly, closer than the paragraphs above them. At this size they are a
block to be scanned rather than paragraphs to be read apart, and spacing carried over from larger
text made two notes read as two unrelated things.

The marker's size is a ratio rather than a length, so one rule serves both places it appears:
beside prose it lands where a fixed size used to, and among the smaller notes it shrinks with
them. The same for the arrow that ends a note.

**A note names its words first, then its number, then what it says.** The quoted phrase is what
lets the section be read on its own, in the order a reader needs it: what this is about, which
mark it was, what it means.

**The number is the same superscript that marked it in the prose.** It was a right-aligned counter
column first, which drew a table: a list of arrows down one edge and a rail of digits down the other,
neither of which the reader had met before. Reusing the marker means the number in the note and
the number in the prose are one thing seen twice, and the section stops looking like an apparatus
bolted underneath. It sits between the phrase and the note, the one place it needs air on the side
facing the note -- in the prose it follows the word it belongs to and must not be spaced off it.

The list stays an ordered list for what a screen reader is told, with nothing of a list drawn --
so the visible superscript is hidden from it, or the ordinal would be announced twice. The way
back trails the note's last word rather than sitting at the right edge: it belongs to the sentence
just read, and a column of arrows is more furniture.

#### Past five, the rest is folded away

An article can carry more notes than the article has room to end on -- one here carries
thirty-three -- and a page whose last screen is apparatus reads as though the apparatus were
the point. So five stand outside the fold and the rest sit behind it, opened by a control
that says how many are there.

**Five by count, not by height.** A height would cut a note mid-sentence at a boundary nobody
chose; five is enough to show that this is a list and how dense it is.

The fold is left one line tall rather than closed to nothing, and that line fades out. A hard
cut says the list ends there, which is the one thing it must not say; text dissolving mid-line
says it continues and something is holding it back, and the control's count then says how
much. The fade is a mask rather than a gradient painted in the paper's colour -- a painted one
would be a second place the background is written down, wrong the moment either changes.

**A fold that clips has to be positioned.** `overflow: hidden` does not clip an absolutely
positioned descendant whose containing block lies further up, and every note carries one: the
way back's purpose is written for a screen reader, and the utility that hides it visually takes
it out of flow. Twenty-eight of those escaped a static fold, stood at their unclipped positions,
and added a screen of empty document below the page with a scrollbar to match. The fold's own
height had nothing to do with it -- setting it to zero changed nothing -- which is what made the
symptom read as unexplainable. Anything here that clips is `position: relative` for that reason,
not for stacking.

**The fold opens before the scroll, not during it.** A marker in the prose may point at a note
behind the fold, and the walk down must still land on it. `scrollIntoView` resolves its
destination the moment it is called, so a fold opening afterwards pushes the note below the
position the scroll is already travelling to and the reader lands short. Opening first is also
what keeps it unseen: the section is still below the fold, so the height changes where nobody
is looking and the reader arrives at a section that was simply already open. Animating that
opening would be the visible version of the same thing, and slower than the scroll it races.

The marker cannot ask the section whether a note is folded -- it is compiled HTML with no
component of its own, delegated at the article root -- so the section leaves one revealer
behind a module for as long as it is mounted. One slot rather than a registry: a page has one
collected-notes section or none, and a second subscriber's question, which section did the
reader mean, has no answer.

The motion is the code block's, by sharing it rather than by copying its numbers. Both are the
same gesture -- a panel answering a press -- and two springs written out separately are two
numbers to keep in step with no way to tell later whether they were meant to be equal. See
[the titled code block](#a-titled-code-block-is-one-framed-disclosure), which was first.

### The marker wraps the words it explains

`:fn[the words]{is="what they mean"}` -- the same shape as a translator's note, and for the same
reason: something has to say which words are being explained. Without them a collected note is a
sentence in mid-air, and a reader who scrolled down to it has to hold the one they left in their
head to make sense of it.

The words render exactly as written and the marker follows them, so the sentence reads as it
would without the note at all.

**A heading is safe because the wrapped words are its own.** A heading is flattened to a string
for its entry and its slug, and flattening keeps a directive's children while dropping its
attributes -- so the words arrive in the table of contents, where they belong, and the note does
not. That is the whole reason the note lives in an attribute rather than beside the words.

That holds for the compiled entry. The rendered heading also carries a real superscript, so the
table of contents strips `.note-marker` before reading a heading back out of the DOM -- without it an
entry gained a stray digit at hydration, which is how the rule was found.

## A spoiler is fog, not redaction

`:spoiler[the words]` keeps its words in the sentence but out of view: fogged by a blur, lifted
while the pointer hovers or the element holds focus, restored the moment the reader moves away.
Telegram's spoiler is the model. No markdown dialect standardises one -- `||text||` is a
convention three platforms happen to share and CommonMark never adopted -- so this is a DLC
directive like `:t` and `:fn`, not a syntax borrowed from anywhere.

The hiding is a display choice and stays in CSS. The words remain real in the compiled HTML --
selectable, translated segment text, read by assistive technology -- because a reader who cannot
hover is owed the content, not the ceremony. For the same reason the reveal listens to `:focus`
rather than `:focus-visible`: a tap's focus is the only ask a touch screen has, and revealing is
this element's whole job, while the focus ring it wears through `.focus-link` stays
keyboard-only. There is no JavaScript to toggle it, so nothing has to hydrate before the fog
lifts.

The markdown target unwraps the directive to its words. That target's readers are models, and
the words are worth more to them than the fact that a page would have hidden them.

## Back is one step up the reading trail {#back-is-one-step-up-the-reading-trail}

The return control at the top left of an article does not mean "the homepage". It means one step
back the way the reader came, which is the homepage only when that is where they came from. A
reader who followed a card from one article into another and is then sent home has lost the
thread they were reading, and the control that did it looked like the way back.

The trail is a list of paths in `sessionStorage["trail"]`, named the way the `localStorage` keys
in [engagement.md](engagement.md) are -- one lowercase noun, no prefix. The storage was chosen
for its lifetime rather than its convenience: one tab, surviving reloads, gone when the tab
closes. Two tabs on one site are two readers here, and they get two trails.

**The browser's own history is not this.** Its previous entry may be an anchor jump inside the
same article, a locale switch, or a page on somebody else's site -- none of them a step in a
reading trail, all of them indistinguishable from one at the moment Back is pressed. Nothing but
the trail records the sequence, which is why it is recorded rather than inferred.

Two consequences worth knowing before changing it. **The record carries the page it belongs to**,
because a reload arrives looking exactly like a fresh visit, and a trail that could not say which
page it was for would offer a way back to somewhere the reader never was. And **arriving at a
page already on the trail cuts back to it** rather than appending, whichever way the reader got
there -- this control, the browser's button, a link that happens to point back. Stated once, it
saves special-casing each of them, and it is what stops two articles that link to each other from
growing a trail between them without end.

Every page records, not only articles, even though only an article shows the control. The step it
has to remember is usually taken somewhere else, and a page that declined to record itself would
be a hole the next article's Back link falls into.

The server cannot see any of this, so the markup ships the homepage -- right for a reader
arriving directly, which is everyone the server can see -- and the destination is corrected after
hydration. Nothing moves when it changes: the label is the same word either way, which is what
makes the correction invisible rather than a flicker.

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

**A live count takes tabular figures wherever it is set in proportional type.** Inter's digits are
not the same width -- `1` is 6.6px against `4`'s 10.5px at the Support rail's size -- so a reader
who gives a like watches the pill resize itself and push the two pills beside it, as the direct
result of the press they just made. Tabular figures give every digit the widest one's advance, so
the width answers only to how many digits there are, and that change has a reason the reader can
see. The feature belongs to the same proportional font and is not a monospace face: only the
digits take the fixed advance, and the word beside them is untouched.

That is the general form of what the `value` cells do by other means. A boxed monospace number
holds still because the face holds every glyph still; a number set in running type needs the
figures asked for by name. Both exist for one reason, that a number which answers to the reader
must not move the page while answering.

Counts below one thousand stay as whole numbers. At one thousand and above, compact indicators
use the shared `compactCount` notation: lowercase `k`, uppercase `M`, and a decimal only while it
carries useful precision (`1k`, `1.5k`, `16k`, `2.3M`). This applies to the licence metric columns
as well as stat rows and chart axes. A count written into prose remains complete and
locale-formatted through `Intl.NumberFormat`; compact notation is for a bounded indicator, not a
sentence.

## Where a line ends is declared per language

Article prose carried no line-breaking policy at all until this was written. `.article-content`
matched no CSS rule anywhere in the repository -- it was a hook for the note handlers and nothing
else -- so every property governing where a line ends was whatever the browser had. Measured on
the Simplified Chinese view before any of this: `text-wrap: wrap`, `hyphens: manual`,
`word-break: normal`, `overflow-wrap: normal`, `line-break: auto`, `text-spacing-trim: normal`,
`text-autospace: no-autospace`. All initial values, none of them chosen.

**The policy is keyed on `:lang()`, never on a locale code.** `<html lang>` already carries the
resolved language, and for the `mw` view that language is the article's own rather than a fixed
one -- see [locale.md](locale.md). A rule written against the language reaches that view without
having to know it exists, and it reaches the bare tags a frontmatter `lang` supplies (`zh`, `en`)
as well as the full ones the eight translations carry (`zh-CN`, `en-US`).

**The shared block pins the defaults, and pinning them changes nothing today.** That is the
point. These are the values a browser is still free to move, and two of them are moving:
`text-spacing-trim` and `text-autospace` decide how a CJK line treats its punctuation and the
seam between Han and Latin, so a default changing under us reshapes every Chinese, Japanese and
Korean paragraph on the site with nothing here edited. `no-autospace` is also the value this
corpus wants rather than the one it happens to have, and the section below says why: the space
between a Latin word and a Han character is a real space somebody typed, so a browser inserting
its own would be spacing that seam twice.

Everything below was measured on `compile-time-rendering`, the longest article in the corpus,
across the forty-odd multi-line paragraphs each view has, at both widths the column is ever drawn
at: 672px, which is where it stops growing, and 354px, which is an iPhone. The narrow half was
measured in Safari on the simulator rather than a desktop browser narrowed to look like one --
that distinction turned out to carry the whole result.

### Two rule sets, because the trade reverses with the column

A language that breaks between words wants two things that fight each other: a tight right edge,
and a final line that is not a stranded word. Which one is worth buying depends on how wide the
column is, so the policy has a narrow half and a wide half and they choose differently.

**Narrow is the base case and it buys the right edge, with `hyphens: auto`.** At 354px an
unhyphenated column is ragged on every screenful, and hyphenating collapses it. Mean gap between
the end of a line and the right edge, as a share of the column, and the count of lines standing
more than an eighth short:

| view    | gap, filled | gap, hyphenated | loose lines, filled | loose lines, hyphenated | of |
| ------- | ----------- | --------------- | ------------------- | ----------------------- | --- |
| German  | 9.7%        | 4.5%            | 215                 | 7                       | 776 |
| English | 7.5%        | 4.7%            | 120                 | 23                      | 624 |
| Spanish | 8.6%        | 4.3%            | 191                 | 10                      | 755 |
| French  | 8.4%        | 5.0%            | 180                 | 37                      | 767 |

The price is a handful of paragraphs whose last line comes out shorter -- English 6 to 10,
Spanish 4 to 7, French 9 to 11, German unchanged at 8. That is paid once per paragraph against a
gain paid once per line, and at this width there are seventeen lines per paragraph.

**Wide buys the final line, with `text-wrap: pretty`, above `--rail-column`.** The breakpoint is
the width at which the column stops growing, so the rule changes exactly when the column becomes
the measure it was designed at rather than whatever the window left it. At 672px the right edge
is already tight without help -- German sits at a 4.6% mean gap -- so hyphenation has little left
to win, and `pretty` has little left to spend: no extra lines at all, and the stranded final
lines go 10 to 2 in German and 9 to 0 in English.

**`text-wrap: pretty` is absent from the narrow half, and finding out why is why the phone was
used.** It was adopted on Chrome's implementation, where it is free. WebKit's is a different
thing wearing the same name. At 354px it adds lines in every language, eleven in German and
forty-two in Japanese, and it roughly doubles the right-hand gap everywhere: German 9.7% to
12.8%, with loose lines going 215 to 400. It still does what it was bought for, but on a narrow
column it charges every other line on the screen for it.

That is also the caution this section exists to carry. A property measured in one engine has been
measured in one engine. `pretty` looked free because Chrome's is; the number that mattered was
only visible in WebKit, and only on a column narrow enough for the cost to show.

**Neither property reaches Chinese, Japanese or Korean, at either width.** That scoping was
written on Chrome evidence, where `pretty` is a no-op for CJK, and WebKit is the reason to keep
it rather than relax it: there `pretty` took Japanese from a 1.6% mean gap to 7.9% and from one
loose line to fifty-eight, in exchange for final lines those scripts barely strand.

`balance` is rejected at both widths and for the same reason each time. It clears final lines
about as well as `pretty` does, but it pays across the whole paragraph rather than at its end: at
672px it took English from two loose lines to eighty-six. It stays on titles, where a block of
even lines is the point -- the table of contents labels and the note list.

**Japanese gets `line-break: strict`.** Japanese typography forbids certain characters at the
head of a line, and `auto` does not enforce it: twelve lines in this article opened on one, eight
of them on the long vowel mark `ー` and the rest on small kana. `strict` removed all twelve and
cost no lines at all, 328 either way. This is the clearest case on the page -- a rule the script
has always had, applied by a value that is free.

It holds on a phone unchanged: at 354px the same article opened fifteen lines on a forbidden
character and `strict` cleared all fifteen for one extra line out of 608.

**Korean gets `word-break: keep-all`.** Korean is written with spaces, but the default treats it
as breakable between any two syllables, so words split mid-eojeol: 110 times across 262 lines
here. `keep-all` removes every one of them for eight extra lines, a three percent taller column.
That is the trade this site takes, because the reader's word staying whole is worth more than
three percent.

On a phone the fault it fixes is worse, not better: at 354px the default split a word 238 times
across 491 lines, nearly every other line, and `keep-all` again removed all of them -- for 15
lines rather than 8, and with nothing overflowing at that width.

The risk `keep-all` introduces is a long unbreakable run overflowing a narrow column, and it was
measured rather than guarded against. Nothing overflows down to a 240px column; the first failure
is at 200px, on a Korean parenthetical glued to a Latin initialism with no space between them.
The article column never gets near that, so no `overflow-wrap` floor is written. If one is ever
needed this is the paragraph that predicted it.

**Chinese gets nothing beyond the shared block, and that was measured too.** `strict` was run
against `auto` on both the Simplified and Traditional views: identical line counts, 223 and 224,
and no line opening on punctuation under either. Chrome already applies the rule for Han, so
there is nothing to buy. Both Chinese views take one configuration, which is also what the corpus
wants -- the two scripts differ in their glyphs, not in where a line may end.

What Chinese must not be given is Korean's rule, and the phone is where that would have been
found out. `keep-all` on a 354px Chinese column overflows thirteen paragraphs outright and takes
the mean right gap to 22.5%, because Han has no spaces for it to keep whole. The `:lang(ko)`
selector is load-bearing rather than tidy.

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

## A menu opens from the edge its trigger is anchored to

A control sitting in a row's flow opens its panel from its left edge, which is where the eye
already is. A control the page has pushed to the article's right frame opens from its right, so the
panel and the thing that summoned it share an edge rather than the panel hanging inward from a
control that is itself against the frame.

The condition is the rail's again, and it is read off whether the rail is rendered -- the computed
`display` of `.article-rail` -- rather than from a width. That keeps the breakpoint the one number
in `utilities.css`, which a script asking `matchMedia` for `68rem` would have quietly copied.

**This one is decided in script, and the rule about CSS choosing does not apply to it.** A panel is
not in the document until it is opened, so there is no server render for the choice to survive and
no first frame to be wrong. What there is instead is a frame to be wrong *after*: the alignment is
settled in the open handler, before the panel mounts, because an effect running after it has
mounted would position it against one edge and then move it in view.

Above the rail's width nothing changes, which is the point -- there the control sits in the row
behind the summary and opening from its left is what it always did.

## A floating surface stops where the page's text stops

A menu, a popover, anything the floating layer places: when a collision pushes it back from the
window's edge, it stops at 1.5rem, which is the article column's own gutter. The library's default
is 8px. That is invisible on a laptop, where nothing opens near an edge, and wrong on every width
below the rail's, where the language switcher sits against the column's right frame and its menu is
wide enough to be pushed back every time -- the panel ends up a hair from the glass while the paragraph beside it
holds a clear margin, which reads as the menu having fallen off rather than opened.

The value is written as a number in [menu-content.svelte](../apps/site/src/lib/components/menu-content.svelte)
because the library measures in pixels and cannot read a custom property, so it agrees with the
column's `px-6` by hand rather than by reference. One number, in the one component every menu on
the site renders through.

It is not a phone rule with a breakpoint. Collision padding does nothing until something collides,
so the same declaration is invisible at every width where there is room and correct at the one
width where there is not -- which is a better shape than a media query that has to name where
phones end.

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

### The ring's colour is declared at rest, or it fades in from the text

An outline has a colour even while `outline-style` is `none` and nothing is drawn, and unless it
is set that colour is `currentColor`. Naming the accent only inside `:focus-visible` therefore
leaves a control whose outline colour _changes_ when it is focused -- which is invisible until
something animates it.

Something does. Tailwind v4 added `outline-color` to `transition-colors`, and nearly every
control here carries that utility for its hover. So the ring faded in from the element's own text
colour over whatever duration the hover happened to use: a pale flash ahead of the blue, worst on
anything light-on-dark, and reported as "a white ring, then the blue one". Ten controls on the
home page alone were doing it, measured as a resting `outline-color` equal to each element's
`color`.

The fix belongs to the utilities and not to the fifteen call sites: the focus classes state
`outline-color: var(--color-accent)` at rest, so focusing a control changes only the outline's
width and there is nothing left to interpolate. A component that reaches for a narrower
`transition-[background-color]` to dodge this is treating the symptom, and the next component
will not know to.

The general rule, which outlives this one property: **a value that only appears under a state
should be declared in the base too, whenever anything transitions it.** A transition interpolates
from the value that was already there, and "there was no value" resolves to something -- here the
text colour -- rather than to nothing.

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
inside the padded hit area. A Lucide icon in this row is `0.875rem`, and a Lucide icon in the
language menu's trailing marker slot is `0.8125rem`. The language marks are not a box size at all;
see below. None of these are interchangeable: they are optical calibrations for different view
boxes.

### An icon set is sized by the ink it carries, not by one class for all of it

A box size is a promise about the space an icon may use, and that is not what the reader sees. The
reader sees the ink. Mingcute's language marks do not fill their view boxes alike, so the one
`h-4` they all carried shipped four different sizes: rasterised at a 16px box and measured by
counting painted pixels, `translate-line` and `translate-2-line` reach 12.00px of ink where
`translate-2-ai-line` and `world-2-line` reach 13.38px, and the first pair is lighter in mass
besides. The English, Spanish and Simplified rows read up to 15.5% smaller than the five beside
them, in a column whose whole job is to be compared by scanning straight down it.

**The figure each mark is normalised on is `sqrt(extent * sqrt(mass))`**: how far the ink reaches,
corrected by how much of it is inside that reach. Reach alone is the wrong thing to equalise --
bringing a narrow mark up to the widest reach scales its strokes with it and it arrives as the
heaviest mark in the menu -- and mass alone under-corrects for the same reason in reverse. Both
terms scale with the box, so the figure does too, and each mark's height is a ratio of measured
numbers rather than a second round of guessing.

**An ornament does not vote on size.** `translate-2-line` and `translate-2-ai-line` are one
drawing: rasterised together at a 16px box they share 53.36px² of ink, the plain one has 0.86px²
of its own from an antialiased edge, and the whole of the other's extra 14.2px² sits in the
top-right corner, which is the sparkle. A sparkle is ink, so it raises its mark's figure and
lowers its scale, while the plain mark's scale goes up -- and the letterform they share then
arrives at two sizes on rows that sit next to each other, which is the one comparison this
correction exists to get right. So the plain mark is sized by its sibling's measurement instead of
its own. It measures smaller on the figure, 9.65 against 10.73, and that is the price of the
drawing matching, which is the thing actually being looked at.

This is an exception and is written as one. Two marks qualify only when they are the same drawing
differing by a decoration; glyphs that merely resemble each other are still measured apart.

**The size they are normalised to is the compass on the closed trigger**, which is itself a
correction: `size-3.75` rather than the row's Lucide `size-3.5`, because a circle that reaches its
box reads smaller than a glyph that only reaches it at the corners. That mark is the one this
control was already right at, so the menu is brought to it. It settles the trigger as well, which
was two sizes rather than one: the compass and the mark that replaces it differed by up to 12%
according to which language was being read.

The heights are written out as literal classes, because Tailwind reads source text and would not
find a height it has to evaluate. A test parses them back and holds each to the ratio its measured
optical size asks for, so the literals cannot drift from the table they stand for.

**The trailing marker is not brought along.** A check is a light statement -- it says only that
this row is the one -- and a check enlarged to match a compass beside it would be a check
insisting. It measures 6.06 against the compass's 9.29 in the same slot, and that difference is
the two marks meaning different things rather than one of them being wrong.

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

## An article block has to be a block to be spaced like one

The article column spaces what it holds with `space-y-4`, which in this version of Tailwind is a
`margin-block-end` on each child but the last. That works on every block and on nothing else: a
non-replaced inline box discards its vertical margins, so an inline child takes the rhythm from
whatever came before it and gives none to whatever comes after.

An embedded picture was that child for as long as the block existed. `picture` is inline in the
browser's own stylesheet, and the `img` inside it being `display: block` does not change what the
wrapper is. So an image sat 16px below the paragraph above -- a gap that belonged to the paragraph
-- and 0px above the paragraph below, while a link card, whose anchor carries `block`, had 16 on
both sides. The asymmetry was visible without being measurable by eye: the two blocks look alike
and only one of them was spaced.

**The failure is silent, which is the part worth writing down.** Nothing is missing from the
markup, nothing overlaps, and the margin is there in the computed style -- it simply has no effect
on that box. A new block type is one `display` value away from the same bug, so the check is one
line: every child of `.article-content` must compute to a block-level display. Measured after the
fix, across every article: no inline children, and every picture sits 16px from its neighbours
except where the next thing is a heading, which brings its own 48.

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

The homepage Support surface holds Like, one favour to ask, and Sponsor. These are reader actions
and read as one small section; revision and Follow stay off the page until they have a quieter
placement of their own. Visitor, uptime, word-count, update-age and license rows do not appear on
the homepage.

**The middle slot moves on once its favour has been done.** Asking the same reader to set the same
source preference on every visit is asking nothing: once it is set there is nothing left to set,
and a control that goes on offering it is furniture. So it offers Google first and a star on the
repository afterwards -- a different favour, in the same slot, rather than a second pill that
would be permanent clutter for the readers who never do either.

Which one is showing is decided from two stores, and each answers a different question about the
same click. `support.preferred` in the reader's state record -- see [engagement.md](engagement.md)
-- says this reader was sent to Google at some point, which is what moves the slot on.
`sessionStorage["support.preferred"]` says it was this tab that did it, which is what stops the
slot moving under them: a reader who clicks and then reloads, or navigates away and comes back,
would otherwise find a different control where they just pressed one, and a page that changes its
mind about what it is asking for reads as a page that lost track. Within the tab that did it, the
pill stays where it was.

The two live apart on purpose. One is a fact about the reader and belongs in the record a later
build will sync between their devices; the other is a fact about a visit and has no business
outliving the tab.

Neither store exists on the server, so the markup carries Google -- right for every first-time
reader, which is everyone the server can see -- and a returning reader's pill changes after
hydration. This is the one place on the page where that is accepted rather than designed around,
and what makes it acceptable is that both resting labels are a six-letter brand name: the row does
not move, one word is replaced by another. Every storage read is wrapped, because a reader in a
private window has no stores and the default is already the right answer for them.

Each Support action presents an icon and its shortest useful identity at rest, while pointer hover
and keyboard focus reveal the full localized instruction in place.

The rail measures each localized short and long label, then springs the button between those live
widths with `motion`. This is computed geometry rather than a fixed hover target: locale, font and
the Like count all change the answer. When the short label is a substring of the instruction, that
shared text stays as one DOM segment. Prefix and suffix segments sit in zero-width masks driven by
the same spring as the pill: a suffix is uncovered after a stationary label, while a prefix pushes
the shared label right as it is uncovered. This makes the copy read as material revealed by the
pill rather than one string replacing another. Every shipped locale preserves that substring for
all four Support labels -- both of the middle slot's favours included -- with a message contract
test guarding the relationship. The component
keeps a crossfade only as a defensive fallback; these actions must not rely on it. Translations
choose an idiomatic local short label first rather than forcing an English noun into every locale.

Like keeps its remembered state legible without making the whole rail permanently heavy. A click
fills the heart and updates the count; leaving returns the button to the ordinary paper surface.
Hovering or focusing a remembered Like inverts it to the ink surface. The same state changes must
remain understandable through `aria-pressed`, and reduced-motion users get the final labels without
the width transition.

**The reveal answers to whether the pointer can hover, not to how wide the window is.** A touch
screen has no hover, but a tap synthesises `mouseenter` -- so the pill would grow under the finger
that meant to press it and then stay grown, with no pointer to leave and take it back. Reading the
instruction costs a press either way; growing first only moves the target.

This was first written as a width, and width is the wrong question. An iPad reports 1133px and
`hover: none`: wider than any breakpoint this site draws, with nothing on it that hovers. The
guard was open on the one device class it existed for, which is the failure a proxy makes and the
capability it stands in for does not. `(hover: hover)` also answers correctly for the case no width
can describe, a laptop whose screen is also a touch screen: it has a pointer that hovers, so it
expands, and it is right that it does.

Only the pointer path is guarded. Keyboard focus is never what a tap produces -- the component
tests `:focus-visible` -- so a tablet with a keyboard still gets the full label on Tab. Collapsing
is not guarded either: whatever opened a pill has to be able to put it back. The query is live
rather than read once, so a tablet that is given a trackpad finds the other answer.

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

## The leader carries the clearance it needs, so it can take it away

An article row is a title, a dotted leader and a date on one line, and the leader is the part that
gives way: its flex basis is zero, so it takes only the space the other two leave and closes to
nothing on a narrow screen. Clearance around it was the row's `gap`, and a gap belongs to the row
rather than to any item in it -- so when the leader closed, its 24px of clearance stayed behind,
holding open a space with nothing in it while the title beside it was cut short for want of two
pixels. On a phone that turned a title that fits into one ending in an ellipsis.

The clearance on the title's side now lives inside the leader, as the margin of a pseudo-element
that draws the dashes. It is part of the leader's own width, so it closes when the leader does,
and while there is room it puts the dashes exactly where the gap used to. The date's side stays a
real margin: it has to survive, because a title that genuinely does not fit still has to be told
apart from the date beside it.

The shape is self-limiting at the end. Once the leader is narrower than the clearance it holds,
the dashes are zero-length and nothing is drawn, so the row never shows a two-dash stub on its way
to showing none.

## An article is offered in one shape, wherever it is offered

`::article` renders the row the homepage lists, unchanged -- the same sheet-of-bars thumbnail,
title, dotted leader and date, from the same component. It does not get the bordered box
`::github` and `::linkcard` wear.

The box is not a house style every card owes; it is what those two need. A repository and an
external page are foreign objects quoted into the page, and the border is what says so. An
article of this site's own is not foreign, and the site already has a way of putting one in
front of a reader. A second one would be a second answer to a question that has an answer, and
the two would drift -- the homepage's row and the in-body card would agree on the day they were
written and not after.

Both alternatives were built before this was settled. A box in `::github`'s shape read as a card
about something external; a box in the tweet card's shape, carrying description and a character
count, read well on its own and still said "this is a different kind of thing than the six rows
on the homepage", which is exactly what it is not.

It opens in place, like every other link here. Opening the in-body one in a new tab was tried
first and is what [the reading trail](#back-is-one-step-up-the-reading-trail) replaced: a new tab
buys the reader their position back by handing them a window to close, and it answers only for
the one link that was built to open it.

What a card holds is the subject's, not the shape's:
[workspace.md](architecture/workspace.md) has why a card pointing inside the corpus carries no
copy of its own.

## The homepage carries the content-language control, below the bio

Content language is a setting for the whole site, and the homepage is where somebody arrives
without having come to read one particular thing. It carries the switcher on its own quiet row.

**Below the bio rather than beside the name.** The bio is identity copy and is rendered from the
source in every view -- see [i18n.md](i18n.md). A switcher placed above it would be a control
whose first use appears to do nothing, which is the worst thing a preference control can look
like. Under the bio it sits exactly where its effect begins. Measured, that costs no findability:
the row lands around 330px on a wide window and 380px on a narrow one, well inside the first
screen either way.

The row has no heading and no rule above it. This is page furniture, and what it writes belongs
to the site rather than to this page; a divider across the column would frame it as a section and
imply the setting stopped there.

### The phone reads a shorter bio, and it is the same bio

The bio is identity copy and is never translated, so a narrow screen cannot be given a different
text without there being two texts to keep in step. There are not two. The markdown carries one
bio and four markers say what a narrow screen does with parts of it.

| Marker    | Compiles to        | What it means                           |
| --------- | ------------------ | --------------------------------------- |
| `wide`    | `hidden sm:inline` | Present only on a wide screen           |
| `narrow`  | `sm:hidden`        | Present only on a narrow one            |
| `ownline` | `max-sm:block`     | Takes a line of its own on a narrow one |
| `apart`   | `max-sm:mt-4`      | And a paragraph's gap above it          |

`narrow` is the only one that adds words rather than removing them, and it earns that before it is
used. Subtracting left the first paragraph ending mid-thought, and the sentence that finishes it
costs the desktop composition a fourth line with an orphan on it -- measured, that paragraph's fill
drops from 84% to 76%. A `wide` and `narrow` pair also carries the second paragraph's two openings,
which are one sentence in two word orders: the phone's avoids two paragraphs in a row beginning
with `I`, and the desktop keeps the one that was written for it.

`ownline` and `apart` are separate because a line of its own and a break before it are two
decisions, and an author may want only the first. `apart` uses the gap the bio already puts
between paragraphs, so the space it opens is the one the page already has rather than a second
number.

Only the page reads them. The markdown and text targets carry no classes, so `/homepage.md` and
anything reading it get the whole sentence; a feed reader has no stylesheet and gets it too.

**That is exact for `wide` and approximate for `narrow`.** A marker that only removes leaves the
other targets reading the full text, which is what they should have. A marker that swaps leaves
them reading both halves in a row -- `/homepage.md` says `I hope somedaySomeday I hope`. It is a
known cost of the pair rather than a defect in either, and the fix, if it is ever worth one, is for
those targets to read the markers too.

**Subtracting on a phone is a layout decision, not an edit** -- which is also why the copy that
disappears is the one sentence that enumerates rather than says anything: on a phone it is the
first thing that reads as a list, and it carries the two unbreakable runs that made the paragraph
rag badly in the first place.

**A link carries its own width, because it is the one run that cannot be wrapped.** `:link`
nested inside `:t` stops being a link, so the four markers above cannot reach it -- and the email
link needs two forms, `drop me an email` on a wide screen and `Email` on a phone, where the
capital is doing the work the dropped words did. So `wide` and `narrow` are read off the `:link`
directive itself and travel with the segment.

They are spelled as variants there, `max-sm:hidden` and `sm:hidden`, where a `:t` run says
`hidden sm:inline`. A span has no display utility to argue with; a link is `inline-flex` for its
icon, and `hidden` is the same kind of declaration at the same level, so which of the two won
would be settled by Tailwind's emission order rather than by anything written here. A variant
sorts after a plain utility and is not that argument.

**Mark the run before a break, never the run after it.** A `:link` rendered inside another
directive stops being a top-level node, and the homepage renders those live so their icons come
from the shared component -- nested, they come back as plain anchors, and the accessible new-tab
note comes back as the source file's absolute path. Wrapping the sentence that ends a line has
the same effect on layout and none of that.

Nothing responsive is written on the container's own wrapping style. The desktop composition was
tuned as it stands, and a `text-wrap` that changed with width would change it; the difference
between the two readings lives entirely in the markdown. The one exception is the container's top
padding, which is halved below `sm`: 6rem is most of a phone screen before anything is read, and
the space under the footer is not competing with anything.

## Phone copy is two lines, and the second is the shorter one

Every translated block of interface prose is written to a shape rather than a length: two lines on
a phone, or one line followed by a shorter one. Never a third line holding a fragment, and never a
second line longer than the first. A trailing three words read as an accident, and an ascending rag
puts the widest line at the bottom of a block the eye is leaving.

The rule is about the rag, so it is checked by measuring, not by counting characters. Each
candidate is rendered at the phone's column width and the rendered lines are read back; the copy is
then written to the measurement. English gained a word to make its first line fill -- `sent
straight to your inbox` rather than `straight to your inbox` -- while German and Spanish each lost
one to come back from three lines to two. Nine locales cannot all be trimmed the same way, so each
is tuned against its own rendering.

**The newsletter pitch swaps two elements; the bio swaps markers inside one string.** The two
mechanisms differ because the bio is identity copy rendered from source in every view, while the
pitch is translated nine times. Markers inside one string work when there is one string; here the
short pitch is its own message key, and the component renders the long one above `sm` and the short
one below it. Both are always in the document and CSS chooses, so the choice survives the server
render.

**The translation notice is written to the same shape, and it is the harder case.** The pitch is
one string; the notice is four -- a translation, a polished source view, a script conversion, and
a language this article has no version of -- and each is written nine times. All thirty-six carry
a `.short` sibling, and the component renders both readings with CSS choosing, which is the
pitch's mechanism rather than a second one.

The target is stated as a shape, not a length: one line filled to at least 85% of the box, or two
lines whose first is at least 88% and whose second falls between 60% and 90% of it. A second line
under half the first is the stub this rule exists to prevent -- the German notice used to end on
one at 45% -- and a second line as long as the first is the ascending rag. Where the copy would
not fit either shape, the sentence loses a clause rather than being allowed a third line: the
short `polished` no longer names the language the article is written in, because the reader of a
source view already chose it.

**The short copy is measured on the narrowest phone, not a convenient one.** The box is 316px on
a 390pt iPhone and 328px on a 402pt one, and twelve pixels is the difference between a Chinese
notice that fills its line and one that spills a single character onto a second. Tuned at 328px,
four of the eight views broke at 316; tuned at 316, all thirty-two combinations land in shape and
the wider phone is merely a little loose -- the German translated notice sits at 52% there rather
than 65%. Loose is the harmless direction, so the narrow phone is the one the copy answers to.

Chinese and Traditional Chinese take one line for all four messages, Japanese one for the short
notice and two for the rest, Korean two for the middle pair, and the four European languages two
throughout with second lines between 63% and 88%.

**A short form ends without its final punctuation where the script allows it.** Chinese and
Japanese do: a line of prose that stops at the edge of a tinted box has already been ended by the
box, and the full stop is a mark the reader does not need twice. The long form keeps it, having
room to be a sentence. Korean and the European languages keep theirs at both lengths, because a
period is doing more work in a script whose sentences are not otherwise visually bounded.

**The register is written, concise and impersonal-leaning, and it is set in each language rather
than translated into it.** The reference sentence is the Chinese one: `该版本的措辞经过细微修改，推荐
阅读原文。` Formal enough to be the software speaking, short enough not to lecture, and carrying none
of the scaffolding a translation leaves behind. Two failure modes sit either side of it. One is
translationese -- `已为你显示`, `以...为准`, `你正在阅读的译本可能带有细微的措辞润色` -- which is
accurate and reads like a dialog box. The other is the overcorrection: hearing "stiff" and writing
`这一版措辞上动过一点，想读原样的话可以看原文`, which is plain speech where written Chinese was wanted
and reads worse to a native reader than the stiffness it replaced.

**None of the nine is a rendering of another.** Each sentence is composed in its own language to
the same register, which is not the same as saying the same words: German reaches for
`Die Formulierung dieser Fassung wurde leicht überarbeitet`, Japanese for
`この版は表現に細かな調整が入っています`, Korean for `이 판은 표현이 조금 다듬어진 것이며`. A
sentence mapped clause-for-clause out of the Chinese would land somewhere between the two failure
modes in every one of them, because what makes a sentence sound composed rather than converted is
different in each language. The shape rule above is the only thing all nine share.

**`{language}` is inside the measured string, so the shape is exact for the corpus and
approximate beyond it.** A notice naming Chinese (Simplified) is twenty-four characters longer in
German than one naming English, and every article today is written in Chinese but one. The copy is
tuned against that, and an article written in a language with a much shorter or longer name will
sit slightly off the shape rather than break it -- the sentence is written so the slack falls on
the second line.

**`mw` takes Chinese here, against the default.** [locale.md](locale.md) says a message added to
`mw` takes the English wording, and that rule is about messages nobody has an opinion about yet.
These four are not: the owner already wrote the long forms in Chinese, and a short form is the
same sentence for a narrower box. Pairing an English short with a Chinese long would swap language
at the breakpoint, which is the one thing the pair must not do. The `mw` view renders no notice at
all -- the component takes every code but that one -- so these strings are the catalogue staying
whole rather than copy anybody reads.

**Nothing sets `text-wrap: pretty` on copy tuned this way.** Chrome ignores the value on these
paragraphs and lays them out exactly as `auto` does, while Safari 26 implements it by reflowing
earlier lines to rescue the last one -- which empties the first line of a two-line block to avoid a
short second, producing the ascending rag this rule exists to prevent. Greedy filling is both what
the measurements are taken against and what the two engines agree on.

The bio carried it anyway until a 390pt phone showed what it cost. `something I made and` ended a
line with room to spare while `thinks,` waited on the next one with the nowrapped clause -- Safari
had pulled a word back to keep the last line from being short, which is the reflow above doing
exactly what it says. Removing the class put `thinks,` back where filling puts it and left the
closing clause alone on its line, which is the shape the copy was written for. A 402pt phone
improved too, to two nearly full lines. The rule was right; it was the markup that had not caught
up with it.

## A phone is shown the title that fits, not the title cut short

The article column gives its title 85% of its width on a phone, which is 300px inside the page
padding on an iPhone 17 Pro. A title past that is replaced by the short form rather than wrapped:
a short title is a phrase written for the room it has, and a wrapped one is a full title that ran
out of room. The card list is the tighter case and is where the short forms are written to -- 186
px beside a leader and a date -- so a short title always clears the article page.

**The choice is made in the build, not in the browser.** Whether a title fits is a property of the
string, so it cannot change between renders; computing it at runtime would mean the first frame
guessing and correcting itself. Both headings are in the document and CSS chooses between them,
which is the same shape the card uses and for the same reason: the choice survives the server
render. `display: none` keeps the unshown one out of the accessibility tree, so exactly one is
announced.

Where the full title fits, the two strings are equal and the markup carries one title twice. That
is the cost of deciding in CSS rather than in a media query the server cannot see, and it is paid
in bytes rather than in a wrong first frame.

**The estimate exists twice, and one corpus holds the two together.** `width::pixels` in the CMS
refuses a translation that will not fit; `width.ts` in the site build chooses which title a phone
sees. Neither can call the other -- one is Rust, and putting the choice in the build artifact
would make that artifact depend on translation state with nothing to detect it going stale. So
both are tested against the same ten strings measured in the rendered page, and a constant edited
on one side turns the other side's tests red.

## The metadata row sheds a control on a phone rather than wrapping raggedly

The row under the title carries a date, a character count, a read count, the summary disclosure
and the language switcher. On a laptop that is one line. On a phone's 354px column it was two,
and which item fell to the second line depended on how long the words came out in that language,
which is the shape a row takes just before it stops looking designed.

**The read count is the one that goes.** It is the only item in the row a reader never acts on --
the date and the count are what the article is, the other two are controls -- so dropping it costs
the least. It is hidden below `sm` rather than removed, because the room exists above that and a
number nobody asked to hide is still worth showing where it fits.

**The summary label gets a short reading, on the mechanism the notice and the pitch already use.**
`article.summary.short` exists in all nine catalogues and the button renders both with CSS
choosing. Eight of them repeat their own word, because `Summary`, `总结`, `要約` and `Résumé` have
nothing shorter to say. German does: `Zusammenfassung` is fifteen characters and the longest label
in the row by half, and `Resümee` is the same word for the same thing at seven. Keeping this as a
message rather than a condition in the component means the next language that finds a shorter word
changes a catalogue, not a component.

**The switcher takes the article's right frame wherever the rail is absent.** The question is not
how wide the window is but whether the table of contents is on screen, and those are different
questions: whether the read count fits is a matter of pixels, where the language control belongs is
a matter of what else the page is already anchoring. With the rail there is a column of navigation
down one side, and a second right-aligned control opposite it is one anchor too many, so the
control rejoins the row's flow behind the summary. Without the rail it is the only thing on the row
a reader reaches for rather than reads, and the frame is where a reader looks for one.

So the rule is paired with the rail's own, in the same `@media` block and against the same number,
rather than given a breakpoint of its own to drift from. An iPad mini shows it both ways within one
device: right-aligned in portrait at 744pt, back in the flow in landscape at 1133pt.

**The switcher drops its region below `sm`, and that is what bought the last line.** Removing the
read count was not enough: German still needed 388px of a 354px column and four of the nine views
wrapped. `(DE)`, `(CN)`, `(ES)` qualify nothing among the eight published views, whose endonyms
already differ from one another -- `简体中文` from `繁體中文` included -- so below `sm` the article's
switcher asks for the name alone. It asks, rather than deciding for itself: `phoneRegion` is a prop
and only the article's row passes it, because only the caller knows what else is in its row. The
original view's `Original (XX)` fallback keeps its region at every width, since there is no endonym
there and the region is the whole identifier.

Both readings are rendered and CSS picks, not a width read in script. The reason is the one the
title and the pitch already give: the choice has to survive the server render, and a control that
corrects its own label on the first frame is worse than one a few pixels wider.

**Then German, then the gap, and the phone that settled it was the narrow one.** A 402pt iPhone
left German 3px, which is not a margin. `Zusammenfassung` becomes `Abriss` rather than `Resümee` --
seven pixels, and a shade of meaning toward the outline it summarises, spent knowingly. That fixed
German and promoted Spanish, whose `Resumen` has nothing shorter behind it, so the row's own gap
goes from 8px to 6px below `sm`. Measured on a 390pt iPhone, where the column is 342px rather than
354 and everything is 12px tighter than the first device suggested:

| view       | slack at 342px | slack at 354px |
| ---------- | -------------- | -------------- |
| Spanish    | ~0             | 9              |
| original   | ~0             | 12             |
| English    | 2              | 14             |
| French     | 6              | 18             |
| German     | 13             | 25             |
| Chinese    | 31             | 43             |
| Japanese   | 46             | 58             |
| Korean     | 54             | 66             |

All nine views are one line on both, which is the shape this row now has everywhere rather than
one it reaches in some languages. Spanish and the original view are the ones with nothing to
spare: a character count that grows a sixth digit takes about eleven pixels and would wrap them
again on the narrow phone. The next pixels available are the gap at 4px, and after that there is
nothing left that is not information.

**The narrow phone is the one to measure on.** The first pass was taken on a 402pt device, found
every view fitting, and was wrong about four of them -- a 390pt iPhone is twelve pixels narrower
and that is most of the margin this row has.

## The theme control is a button, not a menu

It is built and has no home yet: nothing on the site renders it while its placement is being
decided. What follows is the component's own contract, which does not depend on where it lands.

Two states, so the control is the choice rather than a way to reach it. It writes the cookie and
toggles the class, and nothing reloads: every colour on the page is a token under that one class,
which is the whole reason the class exists.

**It reads the class, never the cookie.** The pre-paint script in `app.html` settles the theme
before this component exists, from the cookie if there is one and from the system query if there
is not -- so on a first visit the cookie says nothing and the document already says everything.
The server cannot render the control for the same reason, and it renders neither icon until
mounted rather than guessing one and swapping it a frame later.

Both glyphs occupy one grid cell and the unused one is hidden rather than removed, so the row's
height is the same in both states and the sun and the moon cross without anything below them
moving. The turn is one rotation: the outgoing glyph leaves along the path the incoming one
arrives by, so a press reads as one dial turning rather than two icons trading places.

**It animates only after a press.** Arriving on a page that is already dark is not a change of
theme, and spinning the icon on every load would announce something that did not happen. That is
the same line the newsletter draws in [engagement.md](engagement.md) between what a reader just
did and what they are.

Path and lifetime for the cookie come from `@canmi/theme`, which also builds the pre-paint
script, and a test holds the two to the same string. A control writing a shorter life than the
script would expire a preference on one path and not the other, and nothing would report it.
