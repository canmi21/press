# Reaching a reader

## Formats are produced here, not at the edge

Cloudflare's image transformations cannot read AVIF below an Enterprise plan, and even there
the source is capped at 1200px while these variants go to 1920. The format chosen for storage
is the one format that pipeline cannot open. Measured: an AVIF source returns
`ERROR 9520: Original image has unsupported format` where the identical request against a PNG
source succeeds.

So the CDN decodes and re-encodes in the worker, using WASM codecs. That removes the plan
tier, the monthly quota and the dimension ceiling together, and the cost is bounded because
the extension is the entire request -- there is no size parameter to vary, so a caller cannot
invent work. Results are held in the edge cache, so the decode is paid once per colo rather
than once per reader.

Only the decoders for what is stored and the encoders for what is asked for. The AVIF
_encoder_ is deliberately absent: 1.1MB compressed against 332KB for the decoder, and
`cms image` already produces AVIF locally where the time costs nothing.

## The extension asks for a format

Only AVIF is stored. `/image/{cid}.avif` is served straight from the bucket; any other
extension is a request to convert that same object, which the worker satisfies through
Cloudflare's image transformations.

Cloudflare counts a conversion once per image regardless of how many formats it ends up
serving, so the whole fallback chain costs one transformation rather than a second and third
copy of the library. Storage would be nearly free either way -- what a stored fallback really
costs is the sync, the derive time, and a second thing to keep consistent.

No `?format=` parameter, because the extension already says which format is wanted and two
spellings of one request fragment the cache key. It also caps the exposure: only a size that
was derived exists as an object, so nobody can burn the monthly transformation quota by
asking for arbitrary dimensions.

The failure mode to remember is that exceeding the quota does not degrade -- new conversions
return an error while already-cached ones keep serving. That is why the request path a browser
takes by default is the stored AVIF, and conversion is only ever the fallback.

## Caching is the worker's job now

The old CDN served these files through a static-assets binding and set their cache policy in
a `_headers` file: `/fonts/*` for one year, `immutable`. That file has no equivalent once a
worker reads from R2, so the policy has to be reasserted in worker code or it is silently lost
-- the assets keep working while being re-fetched on every visit.

The trap inside the old policy is worth keeping in view. Latin font filenames carry no content
hash: `IoskeleyMono-Regular-latin.woff2` is a stable name. Declaring it `immutable` for a year
promises that the bytes at that name never change, so re-subsetting the font requires a new
filename. CJK chunks already carry content hashes and need no such promise. Whatever replaces
`_headers` has to preserve both cases rather than pretending all font names have one shape.
