# Old browsers, and the two floors that are not the same floor

A browser can fail this site in two unrelated ways, and conflating them is what made the first
attempt at this wrong. Keep them apart:

| | What decides it | Failure looks like |
| ---------------- | ------------------------------ | -------------------------------- |
| **Syntax floor** | `build.target`, i.e. esbuild | The bundle does not parse. Nothing runs, including any code meant to help. |
| **API floor** | Which built-ins the code calls | The bundle runs and throws when it reaches the missing method. |

**A build target lowers syntax and supplies no runtime built-ins.** That is why a target of
`chrome111` does not mean "runs on Chrome 111" -- it means "the syntax parses there". Nothing in
the build has an opinion about whether `Array.prototype.toSorted` exists, and nothing can: the
call site is indistinguishable from any other method call.

This was learned the expensive way. `toSorted` reached production, threw for a real reader, and
arrived as a Sentry issue. No configuration would have predicted it.

## The API floor: one canary, and all of core-js behind it

[compatibility.ts](../apps/site/src/lib/client/compatibility.ts) checks for
`Array.prototype.toSorted` and, if it is absent, dynamically imports `core-js/stable` before
hydration.

**There is no list of modules any more, and that is the point.** A hand-written list has no
knowable correct length: an entry missing from it is a crash in somebody's browser, discovered
the way the first one was. Loading the whole stable set removes the question instead of
answering it.

Being generous is free here because the import is dynamic. Measured on the built client:

| | |
| ------------------------------------ | ----------------------------------------- |
| Check compiled into the eager entry | 174 bytes, dynamic import included |
| core-js in the entry's static closure | none. 38 eager chunks, 1187.5 KB, and core-js in none of them |
| What a current browser fetches | nothing |
| What a browser below the line fetches | 87.1 KB gzipped, once, before hydration |

The closure is the measurement that matters and text search cannot give it: `manifest.json`'s
`imports` are the static edges, `dynamicImports` are not, so grepping chunks for a core-js marker
counts the lazy ones too and reads as though the bundle were bloated.

### Why `toSorted`, and why only one check

It is the one that actually broke. Chrome 110, Firefox 115 and Safari 16.0 shipped it, and a
reader below that line reached production.

One check rather than a set, because browser support is strongly ordered: a browser new enough to
have this has the decade of features before it, and a browser without it needs everything anyway.
The canary is not a claim about which API the next crash involves -- it is a cheap proxy for
"this browser is old", and the whole of core-js is what answers the crash.

**The canary is where the API floor is declared.** Not in a config file, not in a table. Moving
that line is editing this one condition.

`stable` rather than `es`, which omits `URL` and `structuredClone`, or `actual`, which adds
proposals nothing here writes.

## The syntax floor is set to the same line, deliberately

`browserslist` in [apps/site/package.json](../apps/site/package.json) is the only place the
syntax floor is written, and [vite.config.ts](../apps/site/vite.config.ts) derives esbuild's
`build.target` from it.

It names Chrome 110, Firefox 115 and Safari 16.0 -- the canary's line. **A rescue only happens if
the browser could parse the code doing the rescuing.** A target above that line hands exactly the
readers this mechanism exists for a bundle that dies before the check runs, and core-js sitting in
a chunk they never reach helps nobody. The two floors agree by construction rather than by
somebody remembering to keep them in step.

Stated rather than left to Vite's default, which is a baseline of somebody else's choosing and can
move under a major -- it was `chrome111, edge111, firefox114, safari16.4` when this was written,
which is above the canary for Firefox and would have had precisely the effect described above.

**Floors, never a relative query.** `> 0.5%` or `last 2 versions` is resolved against
`caniuse-lite`, so the compiled output would change on an unrelated dependency update and
rebuilding one commit twice would not produce the same bytes.

Only the site declares one. `api` and `cdn` run on workerd, and the CMS runs in a webview shipped
with the client; none of them meets an arbitrary browser.

### Tailwind's floor is higher and is not this one

Tailwind 4 requires Chrome 111, Safari 16.4 and **Firefox 128** -- it compiles to `@property` and
`color-mix()`, which is a hard requirement rather than a degradation.

That floor is deliberately not adopted here. A reader on Firefox 120 gets broken styling either
way; the choice is whether they also get broken scripts. Serving them working JavaScript on a
badly styled page is the better half of a bad situation, so the CSS floor stays where Tailwind
puts it and the JavaScript floor stays where the canary puts it.

## What was tried first, and why it is gone

A `mise run compat` task ran `babel-plugin-polyfill-corejs3` over the built client to report which
core-js modules the code needed, so the hand-written list could be kept honest. It worked, and it
answered a question worth abandoning rather than automating: once every stable polyfill loads
behind one check, there is no list for a report to be about.

Two findings from it are worth keeping, because both are traps to fall into again.

**Asking `core-js-compat` what a baseline lacks is the wrong question.** It answers with everything
the baseline lacks -- around seventy stable modules -- rather than what this code reaches for.
Computing the intersection needs a tool that reads call sites, and `usage-global` has no type
information, so a bare `.map()` is charged to `Iterator` as well as to `Array`.

**core-js's version numbers do not mean what they appear to mean.** They record the version from
which core-js considers a native implementation *fully spec-correct*, not the version that first
shipped the feature. Measured against features whose age is not in doubt:

```
es.json.stringify   {chrome: 114, firefox: 135, safari: 18.4}   ES5, 2009
es.array.push       {chrome: 122, firefox: 55,  safari: 16.0}   ES1
es.array.includes   {chrome: 53,  firefox: 102, safari: 27.0}   ES2016
```

Read as support data, that says `JSON.stringify` needs Chrome 114. It does not. Any floor derived
by taking a maximum over these numbers is meaningless, and one was derived that way before the
check above was run -- it claimed the bundle required Chrome 145.
