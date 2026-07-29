import { URLS } from '@canmi/urls';
import { describe, expect, it } from 'vitest';
import { imgsrc } from './index';

const prodCdn = URLS.apps.production.cdn;
const devCdn = URLS.apps.development.cdn;
const github = URLS.external.github;

describe('imgsrc', () => {
	describe('hash filename', () => {
		it('rewrites bare filename to the production CDN image path', () => {
			expect(imgsrc('abc123.png')).toBe(`${prodCdn}/image/abc123.png`);
		});

		it('rewrites a 64-char hex hash', () => {
			const hash = '3f7dcc6f50caafa3667d680a0a5592ae6ca14440216c72b138606ae5465eac17';
			expect(imgsrc(`${hash}.png`)).toBe(`${prodCdn}/image/${hash}.png`);
		});

		it('uses the development CDN when requested', () => {
			expect(imgsrc('abc123.png', { isDev: true })).toBe(`${devCdn}/image/abc123.png`);
		});
	});

	describe('data url', () => {
		it('passthrough', () => {
			const data =
				'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=';
			expect(imgsrc(data)).toBe(data);
		});
	});

	describe('plain url passthrough', () => {
		it('unknown https host', () => {
			const input = `${URLS.internal.app}/x.png`;
			expect(imgsrc(input)).toBe(input);
		});

		it('unknown http host', () => {
			const input = `${URLS.apps.development.api}/x.png`;
			expect(imgsrc(input)).toBe(input);
		});

		it('malformed url passthrough', () => {
			const input = `${URLS.internal.app} not a url`;
			expect(imgsrc(input)).toBe(input);
		});
	});

	describe('github: scheme to GitHub CDN', () => {
		it('without @ref uses default branch', () => {
			expect(imgsrc('github:innei/shiro/apps/web/public/innei-dark.svg')).toBe(
				`${github.cdn}/innei/shiro/apps/web/public/innei-dark.svg`,
			);
		});

		it('with @ref pins the commit', () => {
			expect(imgsrc('github:innei/shiro/apps/web/public/innei-dark.svg@90ef7b8')).toBe(
				`${github.cdn}/innei/shiro@90ef7b8/apps/web/public/innei-dark.svg`,
			);
		});

		it('with @ref accepts branch names', () => {
			expect(imgsrc('github:innei/shiro/icon.svg@main')).toBe(
				`${github.cdn}/innei/shiro@main/icon.svg`,
			);
		});

		it('with too-few segments passthrough', () => {
			expect(imgsrc('github:owner')).toBe('github:owner');
			expect(imgsrc('github:owner/repo')).toBe('github:owner/repo');
		});
	});

	describe('github:avatar: scheme', () => {
		it('numeric id without size', () => {
			expect(imgsrc('github:avatar:72544151')).toBe(`${prodCdn}/github/avatar/72544151`);
		});

		it('numeric id with @size', () => {
			expect(imgsrc('github:avatar:72544151@192')).toBe(
				`${prodCdn}/github/avatar/72544151?width=192`,
			);
		});

		it('username via @ prefix', () => {
			expect(imgsrc('github:avatar:@canmi21')).toBe(`${prodCdn}/github/avatar/canmi21`);
		});

		it('username with @size', () => {
			expect(imgsrc('github:avatar:@canmi21@192')).toBe(
				`${prodCdn}/github/avatar/canmi21?width=192`,
			);
		});

		it('respects custom cdnUrl', () => {
			expect(imgsrc('github:avatar:72544151@192', { cdnUrl: devCdn })).toBe(
				`${devCdn}/github/avatar/72544151?width=192`,
			);
		});

		it('empty body passthrough', () => {
			expect(imgsrc('github:avatar:')).toBe('github:avatar:');
		});

		it('only @ passthrough', () => {
			expect(imgsrc('github:avatar:@')).toBe('github:avatar:@');
		});

		it('non-numeric after final @ passthrough (malformed)', () => {
			expect(imgsrc('github:avatar:canmi21@abc')).toBe('github:avatar:canmi21@abc');
		});
	});

	describe('plain url rewrite for known github surfaces', () => {
		it('avatars URL rewrites to CDN avatar route', () => {
			expect(imgsrc(`${github.avatars}/u/72544151?v=4`)).toBe(`${prodCdn}/github/avatar/72544151`);
		});

		it('avatars URL respects custom cdnUrl', () => {
			expect(
				imgsrc(`${github.avatars}/u/72544151`, {
					cdnUrl: devCdn,
				}),
			).toBe(`${devCdn}/github/avatar/72544151`);
		});

		it('avatars URL preserves ?s=N as ?width=N', () => {
			expect(imgsrc(`${github.avatars}/u/72544151?s=192`)).toBe(
				`${prodCdn}/github/avatar/72544151?width=192`,
			);
		});

		it('avatars URL drops other query params but keeps ?s=N', () => {
			expect(imgsrc(`${github.avatars}/u/72544151?v=4&s=64`)).toBe(
				`${prodCdn}/github/avatar/72544151?width=64`,
			);
		});

		it('raw GitHub content URL rewrites to GitHub CDN', () => {
			expect(imgsrc(`${github.raw}/innei/shiro/90ef7b8/apps/web/public/innei-dark.svg`)).toBe(
				`${github.cdn}/innei/shiro@90ef7b8/apps/web/public/innei-dark.svg`,
			);
		});

		it('rewrites an http GitHub URL, not only https', () => {
			// Matching on url.origin instead of url.hostname silently passed these through,
			// because the origin carries the scheme and the URL map stores the https form.
			const insecure = github.raw.replace('https://', 'http://');
			expect(imgsrc(`${insecure}/innei/shiro/90ef7b8/apps/web/public/innei-dark.svg`)).toBe(
				`${github.cdn}/innei/shiro@90ef7b8/apps/web/public/innei-dark.svg`,
			);
		});

		it('GitHub raw link rewrites to GitHub CDN', () => {
			expect(imgsrc(`${github.web}/innei/shiro/raw/main/apps/web/public/innei-dark.svg`)).toBe(
				`${github.cdn}/innei/shiro@main/apps/web/public/innei-dark.svg`,
			);
		});

		it('GitHub blob link rewrites to GitHub CDN', () => {
			expect(imgsrc(`${github.web}/innei/shiro/blob/main/apps/web/public/innei-dark.svg`)).toBe(
				`${github.cdn}/innei/shiro@main/apps/web/public/innei-dark.svg`,
			);
		});

		it('GitHub CDN URL passthrough when already canonical', () => {
			const input = `${github.cdn}/innei/shiro@main/apps/web/public/innei-dark.svg`;
			expect(imgsrc(input)).toBe(input);
		});
	});
});
