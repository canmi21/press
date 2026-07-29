import { pickUrls, URLS } from '@canmi/urls';

const GITHUB_AVATAR_SCHEME = 'github:avatar:';
const GITHUB_SCHEME = 'github:';

export type Options = {
	cdnUrl?: string;
	/**
	 * Selects the development CDN. Defaults to false, so server and browser always agree.
	 *
	 * Do not infer this from `globalThis.location`: that global is absent during SSR and
	 * present in the browser, so the same image would resolve to the production CDN on the
	 * server and the local one on the client -- a silent hydration mismatch that only appears
	 * in development. The caller knows the answer, so the caller passes it. In SvelteKit that
	 * is `dev` from `$app/environment`.
	 */
	isDev?: boolean;
};

export function imgsrc(input: string, opts: Options = {}): string {
	const cdnUrl = opts.cdnUrl ?? pickUrls(opts.isDev ?? false).cdn;

	if (input.startsWith('data:')) return input;
	if (input.startsWith(GITHUB_AVATAR_SCHEME)) return resolveGithubAvatar(input, cdnUrl);
	if (input.startsWith(GITHUB_SCHEME)) return resolveGithubScheme(input);
	if (hasWebScheme(input)) return rewriteIfKnown(input, cdnUrl);
	return `${cdnUrl}/image/${input}`;
}

// Match on hostname rather than origin: the input is whatever a user pasted, and an
// `http://` GitHub link points at the same host as its `https://` form. Comparing origins
// would silently pass those through unrewritten.
function hostOf(url: string): string {
	return new URL(url).hostname;
}

function hasWebScheme(input: string): boolean {
	const schemeEnd = input.indexOf(':');
	if (schemeEnd < 0) return false;
	const scheme = input.slice(0, schemeEnd).toLowerCase();
	return scheme === 'http' || scheme === 'https';
}

function resolveGithubAvatar(input: string, cdnUrl: string): string {
	const rest = input.slice(GITHUB_AVATAR_SCHEME.length);
	const parsed = parseAvatarRef(rest);
	if (!parsed) return input;
	const query = parsed.size ? `?width=${parsed.size}` : '';
	return `${cdnUrl}/github/avatar/${parsed.idOrName}${query}`;
}

function parseAvatarRef(rest: string): { idOrName: string; size: string | null } | null {
	const lastAt = rest.lastIndexOf('@');
	if (lastAt < 0) {
		return rest ? { idOrName: rest, size: null } : null;
	}
	if (lastAt === 0) {
		const name = rest.slice(1);
		return name ? { idOrName: name, size: null } : null;
	}
	const before = rest.slice(0, lastAt);
	const after = rest.slice(lastAt + 1);
	if (!/^\d+$/.test(after)) return null;
	const idOrName = before.startsWith('@') ? before.slice(1) : before;
	return idOrName ? { idOrName, size: after } : null;
}

function resolveGithubScheme(input: string): string {
	const rest = input.slice(GITHUB_SCHEME.length);
	const atIdx = rest.lastIndexOf('@');
	const pathPart = atIdx >= 0 ? rest.slice(0, atIdx) : rest;
	const ref = atIdx >= 0 ? rest.slice(atIdx + 1) : null;
	const parts = pathPart.split('/');
	if (parts.length < 3) return input;
	const [owner, repo, ...pathBits] = parts;
	return toGithubCdn(owner, repo, ref, pathBits);
}

function rewriteIfKnown(input: string, cdnUrl: string): string {
	let url: URL;
	try {
		url = new URL(input);
	} catch {
		return input;
	}

	if (url.hostname === hostOf(URLS.external.github.avatars)) {
		const match = url.pathname.match(/^\/u\/(\d+)/);
		if (match) {
			const size = url.searchParams.get('s');
			const query = size ? `?width=${size}` : '';
			return `${cdnUrl}/github/avatar/${match[1]}${query}`;
		}
	}

	if (url.hostname === hostOf(URLS.external.github.raw)) {
		const parts = url.pathname.split('/').filter(Boolean);
		if (parts.length >= 4) {
			const [owner, repo, ref, ...pathBits] = parts;
			return toGithubCdn(owner, repo, ref, pathBits);
		}
	}

	if (url.hostname === hostOf(URLS.external.github.web)) {
		const parts = url.pathname.split('/').filter(Boolean);
		if (parts.length >= 5 && (parts[2] === 'raw' || parts[2] === 'blob')) {
			const [owner, repo, , ref, ...pathBits] = parts;
			return toGithubCdn(owner, repo, ref, pathBits);
		}
	}

	return input;
}

function toGithubCdn(
	owner: string,
	repo: string,
	ref: string | null,
	pathBits: readonly string[],
): string {
	const refSuffix = ref ? `@${ref}` : '';
	return `${URLS.external.github.cdn}/${owner}/${repo}${refSuffix}/${pathBits.join('/')}`;
}
