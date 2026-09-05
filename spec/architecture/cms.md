# The CMS as an application

## The desktop CMS is a resident process, not a viewer

The Tauri client stays running. It exists to do two things the command-line shell structurally
cannot: **run the periodic work on a schedule**, and **be the editor articles are written in**.
Everything else it displays is in service of those two.

That is what makes it resident rather than a window someone opens to look at numbers. A schedule
kept by a process that is only alive while a person is watching is not a schedule, and an editor
that has to be relaunched to save is not an editor. Both requirements land on the same place: the
application layer below the shells has to own long-running work and its record, because a CLI
invocation exits and takes its state with it.

The scriptable shell keeps its own reason to exist: builds and local workflows drive it, and a
scheduled task must remain runnable by hand without the desktop app installed. Neither shell is
the fallback for the other.

An operation that runs on a schedule needs things a one-shot command never did -- what ran, when,
whether it succeeded, what it spent, and what must run before it. Recording that is the
application layer's job under the rule below, not a page's.

#### A page offers only work the task substrate can run

A view that has found outstanding work shows the command that closes it. The command becomes a
button only after the operation has moved below both shells and the task substrate can report its
progress and refuse a second copy; known but unmigrated operations remain text. The Derived page
implements that boundary in [derived.ts](../../apps/cms/client/derived.ts), while the task centre will
eventually provide the complete catalogue and scheduling surface.

The reason is what these operations are. They run for minutes, several of them spend money on a
model, and they are not safe to run twice at once over the same files. A control that cannot be
watched and cannot reject duplication is a worse version of copying the command, because it looks
like it did something. Half of a run mechanism is not a smaller version of one; it is the part that
lies.

An interactive run calls the same in-process application operation as the CLI. The GUI never owns
a second implementation and never turns terminal output into an API. The Tauri adapter is in
[main.rs](../../apps/cms/src-tauri/src/main.rs).

A class that spends money says so wherever it is offered, before anybody reaches for it. A paid
operation does not become a button until that warning is part of the path that starts it.

## The CMS has two shells and one home

`apps/cms` owns both ways a person reaches content management. Its existing Rust binary is the
scriptable shell used by builds and local workflows; the Tauri client is the interactive shell.
They remain in one app because deployment shape does not create a second responsibility. The
Tauri crate is nested at the framework-defined `src-tauri` boundary while the frontend stays a
small Vite entry beside it. Shared CMS operations move behind modules both shells can call when
the interface begins to expose them.

Every CMS capability has one in-process application operation and two optional adapters. The CLI
may expose that operation as a command, and the GUI may expose it through a typed Tauri command,
but neither adapter owns the work. In particular, the GUI never spawns the CLI as a subprocess:
doing so would turn terminal output and exit codes into an accidental internal API, duplicate
process lifecycle concerns, and make the desktop application depend on a separately discoverable
binary. Keeping the operation below both shells gives interactive actions, scripts and scheduled
tasks the same validation, effects and errors. The cost is an explicit library boundary and a
small adapter in each shell; capabilities that exist in only one interface have not yet reached
the shared CMS application surface. An operation that exists for only one provider is still a
shared operation, just not a runner choice -- see [twitter.md](../twitter.md).

The desktop entry starts empty and takes its colours from `@canmi/tokens`; a second design system
does not begin at the window edge. Its native title follows the HTML `<title>` as that value
changes, and the frontend receives only the Tauri permission needed to do that. The active page
owns that title: an outer page starts with its own name alone and may append the detail it opens,
while the shell does not repeat `CMS` as a parent suffix on every page. `app.canmi.cms` is the
application identifier: it follows reverse-domain order for `canmi.app` and does not end in macOS's
`.app` bundle extension. Platform icon variants live at Tauri's `src-tauri/icons` boundary and the
bundle names them explicitly. Both browser interfaces use Tailwind, so the desktop client consumes
the same Tailwind-facing token surface as the site rather than maintaining an adapter of its own.
Tauri's capability schemas under `src-tauri/gen` are generated build output: they stay untracked
and are excluded from repository reference checks.

On macOS the WebView extends through the title-bar area. The native title and title-bar surface are
hidden, while the native traffic lights remain independently visible over the interface. Keeping
the decorated window with an overlay preserves those platform controls; removing decorations would
remove them as well and turn their behaviour into application code.

The window and sidebar are unpainted, revealing macOS's semantic Sidebar material, while the main
content is one opaque Web surface inset from the top, bottom and right edges. That inset matches the
native traffic lights' distance from the window edge instead of introducing an unrelated frame.
The sidebar begins below a dead zone containing that inset, the controls' height and the same inset
again, so navigation never competes with window chrome. That dead zone extends across the full
window as a fixed, topmost transparent hit surface, so content paint order cannot intermittently
take the drag gesture; double-clicking it retains the platform title bar's maximise behaviour. Native
chrome colours remain a small light-and-dark token group in `@canmi/tokens`: surface, divider, hover
and selection. Selection is deliberately stronger than hover because a persistent location must
remain identifiable without pointer movement. Their alpha is part of each colour rather than an
element-wide `opacity`, because chrome may be translucent without fading its text and icons. The
transparent WebView support this requires macOS private API and therefore trades away Mac App Store
eligibility; the CMS is a local workspace tool, so the native material is the chosen side of that
trade.

The sidebar reads the site's name from `site.config.yaml` rather than carrying a second identity.
It sits in a row of its own immediately below the drag region. The row is two and a half times the
text's line height and centres the line vertically while keeping its own left inset, so title
geometry is independent of both the window controls above and the navigation below.

The shared visual language extends beyond the palette. The CMS uses the site's quiet text
hierarchy, generous content spacing, hairline borders, paper only for contained surfaces and
restrained line icons. Task pages do not acquire branded tiles or ornamental status chrome merely
because the CMS is an operations tool. The opaque main pane is already the content surface, so
Overview does not subdivide it into a dashboard of cards. Metrics and sections sit directly on that
surface and use spacing and hairline dividers for grouping; even an empty health state remains text
rather than acquiring another inset box.

Overview is a workspace brief, not an inventory dashboard. Its one headline says whether anything
needs attention, and real check findings become the body when the answer is yes. Article and media
counts are a quiet metadata sentence beneath that state instead of four equally weighted metrics.
Distribution charts are absent: they describe the corpus without giving the writer an object to act
on. Recently modified article titles and subtitles supply those objects, ordered by authored
`lastmod` with `created` as the first modification. The brief follows the public site's article
column width so one subject owns the reading path; wider inventory views keep their own geometry.
Its top inset is the same responsive length as its horizontal inset, making the brief one balanced
sheet within the main pane. Compact marks identify the live workspace state and actionable
attention; they sit immediately after their labels without entering the text flow, while ordinary
section labels remain text-only. Recently updated rows reuse the public homepage's article preview:
row geometry, paper thumbnail, title, dotted leader, date and subtitle are one
`@canmi/primitives` surface. The site keeps its link, focus and content-derived line motion; the
CMS keeps a read-only static rendering. Those are consumer behaviours rather than two visual
definitions. Hover was counted among them until the CMS wanted it as well, and the row it got
stayed flat where the site's lit up -- one row, two answers, which is the thing the shared package
exists to prevent. It is a property of the surface rather than of what a consumer does with it, so
it moved into the package and the site's own copy of the rule came out. Labels and article copy retain one uninterrupted left edge, and individual facts stay
unbadged so the icons establish hierarchy without turning every piece of content back into
interface chrome.

How many of those rows appear is measured against the window rather than chosen. A fixed count is
a short list on a tall window or a scrollbar on a short one, and which of the two it is depends on
the machine rather than on anything the design decided -- so every article the snapshot carries is
laid out and the ones that overflow are taken back, leaving the brief exactly full. One row always
survives that trimming: a window too short for a single row is one nothing can be fitted to, and an
empty section under its heading reads as "no articles" rather than as "no room", so the page
scrolls instead. Beneath the last row that fits is the way out to the library, placed before the
trimming because it occupies the room the final row would otherwise be measured into and hidden
only when the rows on screen are already the whole corpus. The snapshot itself still carries a
ceiling, since it must stay a fixed size as the corpus grows; it is set past what the tallest
window can draw, so it bounds the payload without ever deciding the page.

Articles is a ledger, and the one page here that is not built from the reading primitive. Overview
keeps it, because Overview is a brief and its recent list exists to be read; the library exists to
be worked, and the same row served both badly. Reused there it made the tool look like the site it
manages, promised a click it did not answer, and buried the only actionable fact -- a locale short
of segments, an absent summary, segments an edit left stale -- as a third line under a subtitle
nobody needed in order to identify an article they wrote.

So a row is one line: title, section, segment count, what is outstanding, authored date, in fixed
columns that the group's own header names, so a value and its label cannot drift apart.
Identification is the row's job and reading is not, so the subtitle, the path and the per-locale
standing move into a panel the row opens.

A record's own facts sit one step above the column names and one below its title, which is a tier
the palette did not have: three loud tiers within 0.08 of each other and then a gap of 0.32 down to
soft. `--color-text-muted` fills it. `--color-ink` was the near miss -- it sits in that range by
number, but the site spends it as a background fill, and one name meaning both a surface and a
text weight is how a palette stops being readable.

The rail that marked a row needing work is gone with it. The Todo band already says so, and a
second statement of the same fact was competing with the tick for the same edge. Colour on this
page is now spent on exactly one thing: blue, on the line of an article a run is touching right
now, which nothing else says.

A stale segment is not missing work. Editing a paragraph changes its id, so the old id keeps the
translations it had -- one per locale, with the provider and token count that paid for them --
while the new id gets its own. Nothing reader-facing is wrong: every live paragraph is translated.
What is left is a sidecar growing by eight entries per typo. They are kept rather than dropped on
sight because a corrected typo leaves a translation still almost right; that is the argument for a
sweep somebody asks for, and against one a translation run performs on its way past.

**Rows are grouped by what they need, not by where they live.** Todo, In Progress, Complete -- and
a group with nothing in it is not drawn. A group opens with a band across the full width carrying
its name and a count, and the rows beneath it carry no rule between them: the band already says
where a group starts and ends, so a line per row was a third kind of divider doing the same job
twice. The band is the only place a heading is drawn, and it draws no glyph -- a coloured mark per
heading put three accents on a page whose one meaningful mark is the rail saying a row needs work.

The columns are the article, what it is short of, when it changed, and what to do about it.
Section and segment count came out: a section is a grouping rather than a per-row fact, and the
length belongs in the sentence that says what is wrong with it. Section grouping survives as the
other choice
rather than the arrangement: four sections over six articles spent a screen imposing a taxonomy
unrelated to the work. Which grouping is in force is a tab under the title; the filter and the sort
are menus at the right of that same row, and each states its current value rather than a generic
verb, because a control that reads `Filter` when a filter is applied hides the fact that the list
is short for a reason.

**In progress is derived from the live run registry.** A run records its task id and not the items
it is working on, so an article cannot be matched to a run exactly -- what is knowable is that
while `locale` runs, the articles short of translations are the ones being worked on. The mapping
from task to the findings it closes is one table in
[articles.ts](../../apps/cms/client/articles.ts), so a task that becomes runnable adds a line
there rather than a branch. This is the page's only claim about live state, and it is the reason
the library polls the same registry the Derived page does.

**How long a move takes is derived from how far it goes, and not in a straight line.** Three
arrangements were measured on this page, and the first two were both wrong in opposite directions.

A fixed spring settles in about the same time whatever distance it covers, so a short move is a
slow one: a group of four folded 203px in 512ms while one article's panel opened 86px in 466ms,
0.40 pixels a millisecond against 0.19. Dividing by a constant speed corrects that and overshoots
-- the same panel then finished in a tenth of a second, which reads as a flash rather than a
movement. Neither is how the eye reads travel.

So the time grows with the **square root** of the distance, anchored on the fold that already felt
right. A move four times as long takes twice as long: short ones get proportionally more time than
their size and long ones less. At the anchor, against the constant speed it replaced, 86px goes
from 108ms to 154ms and 600px from 750ms to 408ms.

**And it is a tween, not a spring, because a spring has no end.** It approaches its target
asymptotically and stops when the library decides it is close enough -- for `motion` that is a
`restDelta` of 0.01, a hundredth of a pixel, which is the right default for an opacity travelling
0 to 1 and the wrong one for a height in pixels. Measured folding a group: 182ms of a 468ms move,
39% of the time, spent covering the last three and a half pixels. Nothing visible happens during
it and it reads as the panel being slow to let go -- not as dropped frames, which were measured
and were zero. Correcting the rest thresholds recovered 74ms; the remainder is the curve itself.

**The curve is chosen on what its worst frame costs.** That is what smoothness is at 60fps: not
the average rate but the largest jump. Over the reference fold, against the 13.7px a linear ramp
would use per frame, the ease-out quintic first tried put **56.8px in its opening frame** -- a
third of the distance in 16ms, which is what "there is an animation but it is not smooth" was
describing, since no frame was dropped. CSS's own `ease` peaks at 30.6px and opens with 11.1, so
it is 46% flatter at the top and still moves visibly on the first frame after the press. Curves
that start from rest are flatter again and were rejected for it: two frames of stillness after a
click reads as the control not having heard it.

**A page measures nothing while it is hidden.** The window opens on the Overview, so the library's
first draw happens with its tabs reporting zero width -- and an indicator placed there is pinned
to nothing and, worse, marked as placed, so its real placement then animates in from the left edge
as though it were a loading effect. Both halves are needed: the placement declines to run without
geometry, and being shown re-runs it. The Overview already re-fits its recent list for this exact
reason; the rule is the page's, not that one list's.

**A move is only played where it can be seen.** A panel taller than the window spends most of a
fold below the fold, and nothing on screen changes while it does. Measured folding a 3474px list in
a 536px window: 300ms of a 500ms move passed before anything in view shifted, so the press read as
ignored for that whole time and then everything happened at once.

So the travel is capped at what is visible -- from the panel's top edge to the bottom of whatever
scrolls it, and never more than that scroller's own height. The rest of the distance is taken
instantly, where nobody is looking. The same fold now moves in view on the first frame, and takes
334ms rather than 500 because the distance it is scaled from is the distance somebody watches. A
panel that already fits is untouched: the cap does not bind, and it plays in full.

A panel entirely below the fold animates not at all. There is no honest animation of something
nobody can see, and playing one is only delay.

**Anything that opens or closes animates its own height, on the one spring.** A group folding, an
article's panel, and whatever grows a disclosure next: if a press changes the shape of a box, the
box travels between the two shapes rather than snapping. The gesture is `animateHeight` in
[motion.ts](../../apps/cms/client/motion.ts) and the spring is `@canmi/motion`, shared with the
site, so there is one way a thing opens here and no second set of numbers to keep in step.

Two consequences follow and both are load-bearing. **A panel is built whether or not it is open**,
folded to nothing when closed -- one that exists only while open cannot be animated, because the
element the motion drives would be created and destroyed by the very press that should move it.
And **the press must not redraw the list**, since a rebuild mid-flight destroys the element the
animation is holding. Those are the two interactions on this page that update in place, and this
rule is why.

**A group folds from its middle.** The band's name and count are the collapse control, which is
why the mark on its left and the dots on its right sit outside that button -- pressing either of
those would otherwise fold the group as a side effect. The panel's height is animated on the same
spring as everything else and the motion is interruptible by construction: a running animation is
stopped before another starts and the new one departs from wherever the old had reached, so a
double press reverses rather than queueing. Folding is the one interaction that does not redraw
the list, because a rebuild mid-flight would destroy the element the animation is holding.

A row's grid sets `column-gap` rather than `gap`. With both, the folded panel underneath still cost
a row-gap's band beneath the title, so the hover rectangle stood 7px taller than its content and
the title sat above its centre -- a gap around a zero-height thing is still a gap.

**Rows carry ticks, and the actions read what the ticks mean.** A row's tick sits directly under
its group's mark, which is why the band, the column names and the rows share one leading column --
and why they also share its horizontal padding. Without it the rows began at the panel edge while
the band began inside its own, so the tick sat a padding's width left of the mark it belongs under
and the attention rail fell outside the row box entirely, where it fought the hover surface for the
same edge. The tick is centred in that column rather than started at its edge, since the mark fills
the column and a tick is narrower.

The column names carry a tick of their own, which takes the whole group. Anything short of
everything means the press is asking for everything, and only a full box clears -- that is what
makes the mixed state actionable rather than a third thing to get out of. It follows the rows
rather than commanding them: ticking one row drops the header to mixed, which is what makes
"take all of them, then drop the two I do not want" work.
Nothing ticked means a group action covers the whole group; ticks mean it covers those, and the
menu entry says which in its own words -- one control with two readings is exactly what an
interface has to state out loud rather than leave to be discovered. Ticking is applied in place
rather than by redrawing, for the same reason folding is.

**Actions are dots, not a word.** There will be more than one of them, and a named button repeated
down a column was one word said six times. They are revealed on hover and kept for the keyboard
and for an open menu.

**A column name is the sort control, and the last one pressed is the one in force.** Pressing a
second column moves the ordering onto it rather than adding a tie-break nobody asked for.

Each heading draws both arrows, grey, and a press walks a ring of three: off, up, down, off. That
ring is why there is no reset control anywhere -- the way back out is one more press of the same
heading, which is where somebody would look for it. Drawing the arrows greyed rather than only on
the active column is what makes the third state discoverable: a heading with nothing beside it
gives no reason to believe pressing it a third time would do anything.

With no column pressed the menu above decides, which is what default means here, and choosing from
that menu takes the ordering back off whichever column was holding it.

**A mutation is not a task, and does not wait for the task substrate.** The catalogue exists for
work that takes minutes, asks a model, or cannot safely run twice at once. Sweeping an article's
stale segments is a YAML rewrite taken under that record's own lock: it has no progress worth
reporting and nothing to refuse a second copy of beyond the lock it already takes. So it is a live
menu entry today, called synchronously, and the listing is read back afterwards because the numbers
it changed are on screen. Drawing a progress bar for it would describe something nobody can watch.

An action that *is* a task stays drawn and visibly inert until the operation moves below both
shells -- the affordance exists so the page has its final shape, and it is unmistakably not
pressable so it cannot be the half of a run mechanism that lies.

**What an action offers has to be something that exists.** The row menu offered `cms locale`
against stale segments, and running it would not have removed one: `store::orphans` was called in
two places and both were counting. The number was a notice the CLI printed, and the interface read
it as a queue. Sweeping them is now a real operation, so the entry is real; the group's own entry
covers the whole group, or the ticked rows, and says which in its title.

**Selection moves rather than redraws.** A tab strip is about place, so one bar travels between
the tabs instead of each tab lighting its own underline, and the rail it rides on is not drawn at
all -- the bar is what says where you are, and a visible track is a line the eye has to discount.
The first placement is silent: there is no previous tab to travel from, and a bar sweeping in on
load reads as a loading animation.

**It travels on its own gesture, not the one panels open with.** A box growing and an object
crossing a strip are different things. What the bar has is a **centre** that moves and a **width**
that adapts, and those are separate facts: the centre is where it is, the width is how much of the
label beneath it is covered. Driving offset and width together conflates them into a rectangle
redrawn at successive positions -- correct, and inert.

So the centre is animated on one curve and the width on another, over a single duration, starting
and landing together; left and width are derived from that pair each frame and neither is animated
directly. The centre's curve leaves decisively and settles, because the movement is the gesture.
The width's is the flatter of the two -- a resize that raced the movement would look like the bar
snapping to its new size before arriving, and one that lagged would leave it the wrong length at
rest for a frame.

Measured across this page's own tabs, the centre crosses 27.5 to 104 while the width goes 55 to 62
and stays strictly between them: it grows or shrinks, and never overshoots. An earlier version
drove the two edges instead, which stretched the bar past both widths mid-flight; that reads as a
bar being thrown rather than one moving. Travel is also slower for its distance than a panel
opening, because a tab strip's hops are short enough that the panel curve would be over before the
movement could be read as one.

The spring is `@canmi/motion`, shared with the site's disclosures, which is the pair that moved it
out of the site. The gesture itself is in [motion.ts](../../apps/cms/client/motion.ts) rather than
in the page, because every page that grows a tab strip wants the same one.

Marks appear on the tabs and on the group bands, and the type on both rises to the size the band's
own name is set in. The restraint this file asks for is about colour, and applying it to size and
contrast as well had left the controls quieter than the rows they act on.

**A control that shows a state offers the way out of it.** When a column heading takes the
ordering, the button above stops naming a menu choice and says the ordering came from elsewhere --
which leaves no route back except pressing that heading twice more, a route somebody has to
already know. Under the pointer it becomes the reset instead, and the press then *is* the reset:
opening the menu underneath would answer a different question from the one the button is currently
asking.

**State goes on the attribute, never on a property that may not reflect one.** Two faults on this
control came from the same habit within a day. `dataset.x = undefined` writes the string
`"undefined"`, so `[data-x]` kept matching and the button sat in its reset styling for good. And
`hidden` lives on `HTMLElement`'s prototype and not on `SVGElement`'s, so assigning it to an icon
set a plain JavaScript property that reflects nowhere -- verified in the window: after the
assignment the attribute was still absent and the element still `display: block`, while
`toggleAttribute` set it and hid it. A cast to `HTMLElement` is what let that compile, so the
marks are typed `Element` now, where the wrong form is not available to write.

`toggleAttribute(name, on)` is the form for both. It is also the one that reads the same as the
question being asked.

**A control resizing because its content changed does it in motion.** Swapping a label changes a
button's width, and a snap between two sizes reads as the interface being retyped. The width is
measured before and after the change, pinned, driven, and released back to `auto` -- pinning it
permanently would stop it following its own text at another zoom. The gesture is a spring here
where a panel gets a tween, and the overshoot is the reason rather than an oversight: a control
pushed out or pulled in wants a little give at the end. That shape came from the site's support
actions, which had it before there was anywhere to keep it; what they add on top, revealing masked
copy in step with the width, stays theirs.

Nothing in CSS may transition that width. Two things driving one length is how a control ends up
fighting itself.

**A control is a surface, not a word.** The page is the ground; anything pressable sits on paper
with a hairline around it, the same pairing the derived cards and the run panel already use. The
restraint this file asks for is about colour carrying meaning, not about removing the contrast that
tells a button from a label -- taken absolutely it left the controls indistinguishable from text.

**A row in the library is a press to the page that reads the article.** It used to open into a
panel; the panel listed every segment, then only the stale ones with a control to the rest, and
once it held one control the disclosure was a press that bought nothing. So the row goes straight
there. Dropping a stale segment on its own moved with the reading: it is offered in the study of
that segment, where the reader can see what they are deleting.

**Segments is a workbench, not a document.** Two panes under a header that stays put: the roster
on the left is every segment of the article in its own order, one line each, and the study on the
right is the one under the cursor -- its paragraph, then each translation of it. They scroll
apart, because moving to the ninetieth segment must not lose the roster's place and reading a
long translation must not scroll the roster away. Up and down walk the roster. A stale segment is
listed first and marked with the one word that says why it is there; its study offers the drop.

The page chooses its own article. A menu at the top lists every one; the library's rows send one
here as a shortcut past it; and with nothing chosen it opens the most recently written article
rather than explaining that it is empty. A second menu narrows the study to one language against
the original, which is what reviewing a single locale looks like.

**Which segments are listed is a tab, the way the library's grouping is.** All, Stale and
Untranslated sit at the left of the action row under the same travelling bar, because they are
one page shown three ways and that is what the library already says a tab means. They were the
roster's own counts for a day, pressable, and that made a second kind of tab on a page that had
the first kind a foot above it. The roster keeps a caption of what the view listed. Untranslated
means short of the chosen language, or of any language while every one is shown, so the tab
means something whichever menu is in force. A row keeps its number under any view: the twelfth
paragraph is still the twelfth when it is the only one listed. The study lists every language the
corpus carries and says "Not translated yet" where one is absent, because an absent pane and one
still loading look the same. A view that lists nothing says what that means once, in the middle
of the study, where the reader is looking and there is room for the sentence; the roster's
caption already carries the zero, and a line under it was the same fact twice.

**A segment is drawn as the markdown it is.** Shown as plain text it reads as punctuation --
backticks around code, asterisks around emphasis, hashes where a heading was -- and somebody
comparing nine renderings of one paragraph is reading the prose, not the marks. The chain is the
site's own: remark, GFM, mdast to hast, hast to HTML. A smaller renderer here would be a second
answer to what a paragraph looks like, and this is the copy nobody would check.

Its output is inserted rather than escaped, and the licence for that is narrower than "the corpus
is trusted". Measured on this chain: `<script>`, an `onerror` attribute and an `onclick` one are
all dropped, because `mdast-util-to-hast` ignores HTML nodes unless asked not to. What does come
through is a `javascript:` URL written as an ordinary markdown link, so links are checked against
a scheme list and lose their `href` when they fail it. One hole, closed where it is, rather than
a claim about what the corpus contains.

**A language is named the way it names itself.** `ko-KR` tells a writer nothing they were asking;
한국어 tells them immediately. The names are `@canmi/locales`, shared with the site's language
picker, which is what moved them out of the site: one table keyed by the tag the corpus stores,
read there through the short code its URLs use. Only the pair that needs telling apart carries a
qualifier -- there is one English and two Chinese, each written in its own script.

The two groups are drawn differently because the data differs, not because the design chose to. A
live segment has its paragraph, so the row shows that and the translations are what it became. A
stale one has none: the paragraph was edited away, and the translation is the only text of it that
exists anywhere. **No before-and-after is possible and none is offered** -- an interface showing
one would be inventing the before, and reaching for git to recover it fails the moment articles
stop living there.

That is also what the page is *for* when nothing is stale: it is where a segment's full content
can be found. It is the only surface that can answer "what does this article actually say in
Korean", and it does.

**Bodies are fetched when they are opened, in two steps.** The outline carries what a row needs --
id, one line of text, how many locales hold it -- and a segment's renderings are fetched when that
segment is opened. The largest sidecar here is 609 KB across 1128 translations, so sending it whole
to draw a list of a hundred and forty rows would spend the entire file rendering each row's first
line.

**A stale segment can be dropped on its own.** The article-level sweep remains, and the panel adds
the row-level one: tick the ones to go, or drop the group. The command takes the ids it is given
rather than re-deriving them, because the caller has already decided one row at a time and a second
opinion here would delete what it thinks is stale instead of what was ticked.

**The work surfaces run close to the pane's edge; the Overview keeps its frame.** A library and a
workbench are wide things, and the margin that framed the Overview's cards was empty ground on
either side of a table. So the pane's inset is narrow, and the Overview alone restores the wider
one -- it was composed against that margin, and moving the edge in is compensated there rather
than seen. Controls on a page sit at the right of its action row, the library's and the
workbench's alike; the left is for tabs, and a page with none leaves it empty.

**The top right of the title row is held open and stays empty.** Search, a primary create action
and the signed-in identity belong to the window rather than to a page. Reserving the corner now
costs nothing and stops those controls from being invented per page and then having to move.

**Completion is still the resting state.** A healthy article draws no tick, no badge and no locale
strip; the rows carrying work are marked by a single hairline rail at the left edge, which is the
only pigment on the page. Marking the exception scales with a corpus and marking the norm does not:
eight complete locales repeated down every row is a screen of green saying nothing. The count of
segments is stated because it is the unit the work is measured in -- two stale segments out of a
hundred and forty-one is a different job from six out of sixty-nine -- and the panel names the
command that closes it as text, under the rule above.

Controls here are view state only: a filter between everything and what needs a pass, and the
disclosure. Neither runs anything, so neither waits on the task substrate. The ledger stays on the
main pane and gains no outer box -- the opaque pane is already the surface, and a second one inside
it is the dashboard chrome this deliberately is not.

The first window is a centred 1280 by 720 logical pixels, a 16:9 default rather than a minimum or
a fixed canvas. After that first launch, geometry belongs to the native shell: Tauri's window-state
plugin saves size, position and maximised state in the application's config directory and restores
them before showing the next window. `localStorage` holds page state, not coordinates whose meaning
depends on monitors and their scale factors. The configured window begins hidden so restoration
does not flash the default rectangle before moving to the saved one.

Theme behaviour is shared separately from its colour values. `@canmi/tokens` remains the palette;
`@canmi/theme` owns the system dark-mode query and the site's pre-paint bootstrap. The desktop shell
follows that system query live, while the public site can still honour its explicit `theme` cookie.

The WebView is one application shell with a persistent left sidebar. Its top-level destinations are
Overview, Articles, Media, Automations and Activity: content and resources are things to manage,
while scheduled work and its history are separate views of what the CMS does to them. Individual
CLI commands do not become navigation destinations. They become tasks inside Automations, with
their runs reported by Activity, so adding another operation does not make the application's
information architecture wider.

The CMS interface is `en-US` only. It is an authoring and operations tool for the local workspace,
not a reader-facing surface, so it does not carry a locale selector, message catalog or translated
UI copy. Internationalisation belongs to interfaces the site's readers use; the CMS manages that
content without localising itself.

`dev-cms` enables the MCP bridge as an optional Cargo feature and exposes Tauri's JavaScript global
only through its runtime development config. The bridge is additionally gated by Rust debug
assertions and binds to loopback, so an agent can inspect and evaluate the native WebView without
putting a debugging server in a release client or on the local network.

The two package managers disagree about strictness, and the layout has to respect that.

**pnpm globs.** `pnpm-workspace.yaml` uses `libs/*` and `apps/*`. pnpm only picks up
directories containing a `package.json`; Rust-only directories are invisible to it. Adding a
Rust library requires no pnpm change.

**Cargo does not glob.** `Cargo.toml` lists members by hand. Cargo errors on any
glob-matched directory that has no `Cargo.toml`, and that error breaks _every_ cargo command
in the repo, not just the one crate. Verified: with `members = ["libs/*"]` plus an `exclude`
list, adding one TypeScript library and forgetting to exclude it takes the whole workspace
down. With explicit members, new TypeScript libraries have no effect at all.

The cost is one line in `Cargo.toml` per Rust crate. The failure it buys off is a hard stop
triggered by the most routine action in the repo.
