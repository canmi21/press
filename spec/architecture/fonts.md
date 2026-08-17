# The font pipeline

## A font pipeline input is disposable

The font pipeline only moves in one direction: a full face under `data/fonts` is input, and web
chunks under `data/public/fonts/{family}` are output. The input is useful only while somebody
may slice that face again. Once the chunks exist it may be deleted, and a family with prebuilt
chunks needs no input of its own. It may still name an input retained by a different family that
owns their shared chunks. Keeping every original forever would turn a temporary build need into
repository policy without buying the browser anything.

The authored [font manifest](../../data/fonts.json) records that distinction. Ioskeley Mono has
eight prebuilt chunks and no retained input; that is a complete family, not a missing source.

That manifest stays in `data/` because it is a curated asset record and the slicing pipeline must
read it without depending on a TypeScript runtime. Consumers do not cross that directory boundary:
the root export of [`@canmi/fonts`](../../libs/fonts/package.json) provides the typed family list and
its package stylesheet paths. This keeps one authored list while making the library the public
answer to what a family is and where its stylesheet lives.

## A runtime font is a separate dependency

An application may independently need a full face at runtime. `cms og` renders arbitrary titles
with LXGW WenKai and therefore loads that one full TTF by path; a web subset cannot answer for a
character it does not contain. That runtime dependency is why the 24MB file stays. No other
published family gets a retained full face merely because this one has two roles.

## Latin and CJK use different slicing strategies

Latin faces are a few hundred kilobytes and are split into the handful of named writing-system
subsets Google Fonts uses -- `latin`, `latin-ext`, and the other groups a face publishes. Their
readable filenames are stable cache interfaces. CJK faces are tens of megabytes, so they are
split by character frequency into hundreds of `unicode-range` chunks: common characters arrive
first, and content hashes name the output because no person benefits from reading those names.

The strategy is explicit in the manifest rather than inferred from glyph coverage. Coverage says
what a face contains, but not whether its existing readable URLs are a compatibility promise;
inferring would let a font update silently change both its publication layout and cache identity.
See the [font runbook](../../libs/fonts/README.md) for the operational side.

A selectable family is the name a person picks, not a set of bytes. Its generic fallback completes
the CSS stack, and its faces say which local or redistributable typefaces may satisfy that choice.
Metric compatibility decides what may substitute; it does not decide which choices are offered.
Two families therefore remain separate entries when their local-first stacks differ, even if they
share the same published chunks. Keeping the choice and its sources together prevents a second
selectable-font list from disagreeing with the published faces.

The stylesheets live in `libs/fonts`, apart from the colour tokens. They are a different kind
of fact -- what a family is and where its files are, rather than what the site looks like --
and the CJK sheet alone is 75KB gzipped, which nothing should import until the site actually
sets that family.

## A hash in the name buys a year

Cache lifetime follows one rule everywhere: **a name carrying a content hash is cached for a
year and marked `immutable`, but only on a 2xx. Anything else is cached for five minutes.**

HTML is the one thing that is not cached at all, because its body varies by the reader's
locale cookie. See [locale.md](../locale.md).

The year is an observation, not a promise. Changing the bytes changes the hash and therefore
the URL, so a hashed name cannot come to mean anything else and nobody has to remember to bust
it. CJK font chunks work that way too. Latin subset names are the deliberate exception: a name
such as `IoskeleyMono-Regular-latin.woff2` is readable and stable, so re-subsetting must publish
a new filename or every reader keeps the old bytes for a year.

Errors get five minutes rather than nothing. A missing favicon is requested on every page
view, and without any caching each one is a full trip to the origin. Five rather than a year
because an error is a statement about right now -- the asset it refers to may be published a
minute later, and a year-long 404 would outlive its own reason.

A route that stores its own response has to stamp the header before storing it, which is
earlier than the middleware runs. So the value is one exported constant that both use, rather
than two spellings that agree until they do not.
