/**
 * Naming a stored object, and naming a response about it.
 *
 * Pure string handling, kept apart from the route so it can be read and tested without
 * loading a codec. The route pulls in several megabytes of WebAssembly; none of that is
 * needed to know where an object lives.
 */

/** Content ids are BLAKE3 truncated to 128 bits, hex encoded. */
const CID = /^[0-9a-f]{32}$/;

/** `image/{ab}/{cd}/{cid}.{ext}`, matching the layout apps/cms writes. */
export function keyFor(cid: string, extension: string): string {
	return `image/${cid.slice(0, 2)}/${cid.slice(2, 4)}/${cid}.${extension}`;
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
