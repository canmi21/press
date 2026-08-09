import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { URLS } from '@canmi/urls';
import { describe, expect, it } from 'vitest';

// The config itself, read from disk rather than through `virtual:site`: the claim below is
// about what the file says, and the virtual module is the thing that would hide a change to it.
const CONFIG = fileURLToPath(new URL('../../site.config.yaml', import.meta.url));
const config = readFileSync(CONFIG, 'utf8');

function scalar(key: string): string | undefined {
	return new RegExp(`^${key}:\\s*(\\S.*?)\\s*$`, 'm').exec(config)?.[1];
}

describe('site config', () => {
	/**
	 * `domain` is a label drawn on the OpenGraph card, not an address anything resolves, which
	 * is why it may sit outside libs/urls at all. That exemption only holds while the two agree
	 * -- a card advertising a host the site no longer answers on is worse than no card -- and
	 * nothing structural can enforce it, because one is read by Rust and the other by the
	 * bundler. So it is enforced here.
	 */
	it('draws the same host on a card that libs/urls resolves', () => {
		const domain = scalar('domain');
		expect(domain).toBeDefined();
		expect(domain).toBe(new URL(URLS.apps.production.site).hostname);
	});

	// The home card repeats these, rendered by a separate program. An empty one would render a
	// card with a gap where the author should be, and nothing else would notice.
	it('names the author the home card introduces', () => {
		expect(scalar('  fullName')).toBeTruthy();
		expect(scalar('  role')).toBeTruthy();
	});
});
