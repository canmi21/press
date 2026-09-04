# The browser baseline, and what it costs

## One baseline, declared as floors

`browserslist` in [apps/site/package.json](../apps/site/package.json) is the only place this
site says which browsers it is for. Two things read it and nothing else may state it a second
time: [vite.config.ts](../apps/site/vite.config.ts) turns it into esbuild's `build.target`, and
`mise run compat` hands it to core-js to ask what built-ins those browsers lack.

Only the site declares one. `api` and `cdn` run on workerd, and the CMS runs in a webview whose
engine ships with the client -- none of them meet an arbitrary browser, so none of them has this
question.

**The entries are floors -- `chrome >= 111` -- never a relative query like `> 0.5%` or
`last 2 versions`.** A relative query is resolved against `caniuse-lite`, so the compiled output
would change when an unrelated dependency updated, and rebuilding one commit twice would not
produce the same bytes. Moving a floor stays an edit somebody made on purpose, which is what
every other pin here is.

`build.target` is stated rather than left to Vite's default for the same reason it is stated
rather than guessed: the default is a baseline of somebody else's choosing and can move under a
major. It happened to be `chrome111, edge111, firefox114, safari16.4` when this was written,
which is not the same as the floor below.

### The floor comes from the CSS, because that is the higher one

Tailwind 4 needs Chrome 111, Safari 16.4 and **Firefox 128** -- it compiles to `@property` and
`color-mix()`, which is a hard requirement rather than a degradation. Vite's own JavaScript
baseline allowed Firefox 114. A reader on Firefox 120 would therefore have been served working
scripts and broken styling, while the build carried a polyfill for their benefit.

So the baseline is the stricter of the two constraints, and stating it once is what stopped the
two from disagreeing silently. When a floor moves, it moves here.

## What `mise run compat` answers, and what it cannot

It reports which core-js modules the built client bundle reaches for that the baseline does not
supply. It reads `.svelte-kit/output/client`, so it wants a build rather than a checkout, and it
is deliberately outside `verify`: paying for a full build on every commit buys nothing, and this
drifts on the timescale of browser releases rather than commits.

**It reports and never fails.** That is not politeness, it is accuracy -- see below.

### Asking core-js alone is the wrong question

`core-js-compat` answers "what does this baseline lack", which is around seventy stable modules
nobody here calls. The useful set is the intersection with real code, and computing that half
needs a tool that reads call sites. `babel-plugin-polyfill-corejs3` in `usage-global` mode is
that tool: it decides where an import would be injected, and this task collects the decisions
instead of writing them out.

**Babel is not in the build and must not enter it.** Vite compiles with esbuild; this reads what
Vite produced. The analysis runs over built chunks rather than sources for two reasons, and the
second is the one that matters: Babel cannot parse `.svelte`, and the built bundle includes the
dependencies, which is where an unexpected modern API is most likely to arrive.

### The absolute list is noise; the delta is the signal

`usage-global` has no type information. A bare `x.map(...)` is attributed to `Array` and to
`Iterator` alike, so iterator-helper modules appear for code that never touched an iterator.
core-js compounds it by patching spec corners in methods that have existed for a decade, so
`es.array.includes` and `es.json.stringify` show up as well. Measured here: 42 modules reported,
of which the honest count of real ones is close to zero.

A wall of forty entries is not a finding, and a check that prints one every time gets skipped
rather than read. So the reviewed set is committed to
[compat-snapshot.json](../apps/site/scripts/compat-snapshot.json) and the task reports the
**difference** against it. One new entry since the last review is worth a look. `--update`
accepts the current set, which is a deliberate act that shows up in a diff.

### The direction that actually pays

Three questions are asked, and the third is why the task exists:

- **appeared** -- in the bundle, not in the snapshot. Review it.
- **gone** -- in the snapshot, no longer reached. The code moved on.
- **surplus** -- loaded by `compatibility.ts` and not needed. **Delete it.**

Only the third is reliable, because it is measured against a list somebody wrote on purpose
rather than against Babel's guess. It is also the only one with an ongoing cost: a polyfill for
a browser nobody supports any more is bytes every reader downloads forever, and nothing else in
this repository would ever mention it again. The first run found one --
`es.array.to-sorted`, which Firefox has had since 115.

### What keeps it current is core-js, not caniuse-lite

The obvious guess is that `caniuse-lite` is the package whose updates keep the answer fresh. It
is not, once the baseline is floors rather than relative queries: nothing here asks browserslist
to resolve market share. The compatibility data is core-js's own, shipped in `core-js-compat`
and updated with each core-js release -- so the dependency that has to move is the one already
carried for the polyfills themselves.

The drift is one-directional and quiet. Floors only rise, so the gap only shrinks: over time the
task stops asking for polyfills and starts naming ones that can go.

## Polyfills are feature-detected, never imported outright

[compatibility.ts](../apps/site/src/lib/client/compatibility.ts) checks for the method and
dynamically imports the module only when it is absent, from `hooks.client.ts`'s `init`. A
current browser pays nothing: the branch is not taken and the chunk is never fetched.

Importing core-js wholesale is the alternative and is rejected on measurement -- `core-js/actual`
is 250.7 KB minified and 93.0 KB gzipped, for a site whose whole point is being small. The
per-module form costs one line and one round trip, and only for the readers who need it.

**core-js cannot answer a DOM question, and no amount of it ever will.** Its entire `web.*`
surface is 26 modules: `atob`/`btoa`, DOM collection iteration, `DOMException`, the timers,
`queueMicrotask`, `structuredClone`, `URL` and `URLSearchParams`. Event objects are not in it.
A `KeyboardEvent` arriving without a `key` is guarded at the call site, because there is nowhere
else to guard it.
