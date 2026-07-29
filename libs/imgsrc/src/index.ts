import { URLS } from '@canmi/urls';

const IMAGE_CDN_PREFIX = 'https://cdn.canmi.net/image';
const JSDELIVR_PREFIX = 'https://cdn.jsdelivr.net/gh';
const GITHUB_AVATAR_SCHEME = 'github:avatar:';
const GITHUB_SCHEME = 'github:';

export type Options = {
	resUrl?: string;
};

export function imgsrc(input: string, opts: Options = {}): string {
	const resUrl = opts.resUrl ?? URLS.production.res;

	if (input.startsWith('data:')) return input;
	if (input.startsWith(GITHUB_AVATAR_SCHEME)) return resolveGithubAvatar(input, resUrl);
	if (input.startsWith(GITHUB_SCHEME)) return resolveGithubScheme(input);
	if (input.startsWith('http://') || input.startsWith('https://')) {
		return rewriteIfKnown(input, resUrl);
	}
	return `${IMAGE_CDN_PREFIX}/${input}`;
}

function resolveGithubAvatar(input: string, resUrl: string): string {
	const rest = input.slice(GITHUB_AVATAR_SCHEME.length);
	const parsed = parseAvatarRef(rest);
	if (!parsed) return input;
	const query = parsed.size ? `?width=${parsed.size}` : '';
	return `${resUrl}/github/avatar/${parsed.idOrName}${query}`;
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
	const refSuffix = ref ? `@${ref}` : '';
	return `${JSDELIVR_PREFIX}/${owner}/${repo}${refSuffix}/${pathBits.join('/')}`;
}

function rewriteIfKnown(input: string, resUrl: string): string {
	let url: URL;
	try {
		url = new URL(input);
	} catch {
		return input;
	}

	if (url.hostname === 'avatars.githubusercontent.com') {
		const match = url.pathname.match(/^\/u\/(\d+)/);
		if (match) {
			const size = url.searchParams.get('s');
			const query = size ? `?width=${size}` : '';
			return `${resUrl}/github/avatar/${match[1]}${query}`;
		}
	}

	if (url.hostname === 'raw.githubusercontent.com') {
		const parts = url.pathname.split('/').filter(Boolean);
		if (parts.length >= 4) {
			const [owner, repo, ref, ...pathBits] = parts;
			return `${JSDELIVR_PREFIX}/${owner}/${repo}@${ref}/${pathBits.join('/')}`;
		}
	}

	if (url.hostname === 'github.com') {
		const parts = url.pathname.split('/').filter(Boolean);
		if (parts.length >= 5 && (parts[2] === 'raw' || parts[2] === 'blob')) {
			const [owner, repo, , ref, ...pathBits] = parts;
			return `${JSDELIVR_PREFIX}/${owner}/${repo}@${ref}/${pathBits.join('/')}`;
		}
	}

	return input;
}
