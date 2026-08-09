/**
 * Naming a stored object, and naming a response about it.
 *
 * Pure string handling, kept apart from the route so it can be read and tested without
 * loading a codec. The route pulls in several megabytes of WebAssembly; none of that is
 * needed to know where an object lives.
 */

/** Content ids are BLAKE3 truncated to 128 bits, hex encoded. */
const CID = /^[0-9a-f]{32}$/;

/**
 * `{prefix}/{ab}/{cd}/{cid}.{ext}`, matching the layout apps/cms writes.
 *
 * The fanout exists for a filesystem mirror, which has a directory that overflows; R2 has no
 * directories at all. It is therefore a storage detail and never appears in a URL -- a caller
 * asks for `{cid}.{ext}` and the prefix and the split are put back on here. Publishing it
 * would make the bucket's layout an interface nobody could change afterwards.
 */
function fanned(prefix: string, cid: string, extension: string): string {
	return `${prefix}/${cid.slice(0, 2)}/${cid.slice(2, 4)}/${cid}.${extension}`;
}

export function keyFor(cid: string, extension: string): string {
	return fanned('image', cid, extension);
}

/** Licence texts are stored the same way, and are always plain text. */
export function licenseKeyFor(cid: string): string {
	return fanned('license', cid, 'txt');
}

/** Split `{cid}.{ext}`, or null if it is not that shape. */
export function parseName(name: string): { cid: string; extension: string } | null {
	const dot = name.lastIndexOf('.');
	if (dot <= 0) return null;
	const cid = name.slice(0, dot).toLowerCase();
	const extension = name.slice(dot + 1).toLowerCase();
	return CID.test(cid) ? { cid, extension } : null;
}

/** One id serves several formats, so the format is part of what the tag identifies. */
export function validatorFor(cid: string, extension: string): string {
	return `"${cid}.${extension}"`;
}
