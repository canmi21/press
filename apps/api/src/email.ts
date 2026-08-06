const ATOM = String.raw`[a-z0-9!#$%&'*\-/=?^_\x60{|}~]+`;
const LOCAL_PART = new RegExp(`^${ATOM}(?:\\.${ATOM})*$`);
const DOMAIN_LABEL = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;

/** Canonicalize the product's deliberately narrow, bare ASCII email identity. */
export function canonicalEmail(input: unknown): string | undefined {
	if (typeof input !== 'string') return undefined;

	const lowered = input.trim().toLowerCase();
	if (lowered.length > 254) return undefined;

	const parts = lowered.split('@');
	if (parts.length !== 2) return undefined;
	const [taggedLocal, domain] = parts;
	if (!taggedLocal || !domain) return undefined;

	const local = taggedLocal.split('+', 1)[0];
	if (!local || local.length > 64 || !LOCAL_PART.test(local)) return undefined;

	const labels = domain.split('.');
	if (labels.length < 2 || labels.some((label) => !DOMAIN_LABEL.test(label))) return undefined;

	return `${local}@${domain}`;
}
