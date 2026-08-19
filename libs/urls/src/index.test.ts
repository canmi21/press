import { describe, expect, it } from 'vitest';
import {
	DEVELOPMENT_PORTS,
	SLOT_STRIDE,
	developmentUrl,
	developmentUrls,
	isDevHost,
	isDevOrigin,
	loopbackUrl,
	pickUrls,
	slotPort,
	URLS,
} from './index';

describe('pickUrls', () => {
	it('returns development app URLs when isDev=true', () => {
		expect(pickUrls(true)).toEqual(URLS.apps.development);
	});

	it('returns production app URLs when isDev=false', () => {
		expect(pickUrls(false)).toEqual(URLS.apps.production);
	});
});

describe('URLS', () => {
	it('keeps apps, internal domains, and external endpoints separate', () => {
		expect(URLS.apps.production).toHaveProperty('site');
		expect(URLS.apps.production).toHaveProperty('api');
		expect(URLS.apps.production).toHaveProperty('cdn');
		expect(URLS.internal).toHaveProperty('app');
		expect(URLS.internal).toHaveProperty('infra');
		expect(URLS.internal).toHaveProperty('link');
		expect(URLS.external.github).toHaveProperty('cdn');
		expect(URLS.external.google).toHaveProperty('sourcePreferences');
		expect(URLS.external.social).toHaveProperty('telegram');
	});

	// Not under `external`, which is for services somebody else runs. This one is ours, and
	// the licence routes publish it as the answer to where the code is.
	it('names the repository the code is published from', () => {
		expect(URLS.source).toMatch(/^https:\/\/github\.com\/\S+\/\S+$/);
	});

	it('does not keep discarded app slots', () => {
		expect('res' in URLS.apps.production).toBe(false);
		expect('home' in URLS.apps.production).toBe(false);
		expect('web' in URLS.apps.production).toBe(false);
	});

	it('does not keep retired domains', () => {
		// canmi.dev is not being renewed, and `prod` was renamed to `infra` because it read
		// as a sibling of apps.production while meaning something unrelated.
		expect('dev' in URLS.internal).toBe(false);
		expect('prod' in URLS.internal).toBe(false);
	});
});

describe('isDevHost', () => {
	it('matches localhost', () => {
		expect(isDevHost('localhost')).toBe(true);
	});

	it('matches 127.0.0.1', () => {
		expect(isDevHost('127.0.0.1')).toBe(true);
	});

	it('rejects production hosts', () => {
		expect(isDevHost(hostname(URLS.apps.production.site))).toBe(false);
		expect(isDevHost(hostname(URLS.apps.production.api))).toBe(false);
		expect(isDevHost(hostname(URLS.apps.production.cdn))).toBe(false);
	});

	it('rejects empty and arbitrary strings', () => {
		expect(isDevHost('')).toBe(false);
		expect(isDevHost('localhost.evil.com')).toBe(false);
	});
});

const bound = (slot: number) =>
	Object.values(DEVELOPMENT_PORTS).map((port) => port + slot * SLOT_STRIDE);
const inspectors = (slot: number) => [slotPort('api', slot) + 1, slotPort('cdn', slot) + 1];

describe('workspace slots', () => {
	it('binds the base workspace to the base ports', () => {
		expect(slotPort('site', 0)).toBe(DEVELOPMENT_PORTS.site);
		expect(developmentUrls(0)).toEqual({
			site: 'http://localhost:26511',
			api: 'http://localhost:26512',
			cdn: 'http://localhost:26516',
		});
	});

	it('shifts an overlay by a whole stride per slot, so every address follows from the number', () => {
		expect(slotPort('api', 1)).toBe(DEVELOPMENT_PORTS.api + SLOT_STRIDE);
		expect(developmentUrl('cdn', 2)).toBe(
			`http://localhost:${DEVELOPMENT_PORTS.cdn + 2 * SLOT_STRIDE}`,
		);
	});

	// wrangler takes port + 1 for its inspector, so the two workers' inspectors must land on
	// nothing else in the same slot, one slot must end before the next begins, and the CMS port
	// must fall in no slot at all: it is a singleton and never shifts.
	it('keeps every slot clear of the inspector ports and of the CMS port', () => {
		const cms = Number(process.env.CMS_PORT ?? 26521);
		for (let slot = 0; slot < 10; slot++) {
			const taken = [...bound(slot), ...inspectors(slot)];
			expect(new Set(taken).size).toBe(taken.length);
			expect(taken).not.toContain(cms);
			expect(Math.max(...taken)).toBeLessThan(Math.min(...bound(slot + 1)));
		}
	});

	it('refuses a slot that is not a whole number of workspaces', () => {
		expect(() => slotPort('site', -1)).toThrow();
		expect(() => slotPort('site', 1.5)).toThrow();
	});

	// Nothing defines the override in a test, so the map is the base map -- and the Rust
	// mirror, which walks this same object under bare node, renders the same.
	it('serves the base addresses where no dev server injected an override', () => {
		expect(URLS.apps.development).toEqual(developmentUrls(0));
	});
});

describe('isDevOrigin', () => {
	it('accepts a loopback origin on any port', () => {
		expect(isDevOrigin('http://localhost:26511')).toBe(true);
		expect(isDevOrigin('http://localhost:26611')).toBe(true);
		expect(isDevOrigin('http://127.0.0.1:1')).toBe(true);
	});

	it('rejects production origins and non-URLs', () => {
		expect(isDevOrigin(URLS.apps.production.site)).toBe(false);
		expect(isDevOrigin('null')).toBe(false);
		expect(isDevOrigin('')).toBe(false);
	});
});

describe('loopbackUrl', () => {
	it('puts a local tool on its assigned port', () => {
		expect(loopbackUrl(26521)).toBe('http://127.0.0.1:26521');
	});
});

function hostname(url: string): string {
	return new URL(url).hostname;
}
