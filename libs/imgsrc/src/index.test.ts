import { describe, expect, it } from 'vitest';
import { imgsrc } from './index';

describe('imgsrc', () => {
	describe('hash filename', () => {
		it('rewrites bare filename to cdn.canmi.net/image', () => {
			expect(imgsrc('abc123.png')).toBe('https://cdn.canmi.net/image/abc123.png');
		});

		it('rewrites a 64-char hex hash', () => {
			const hash = '3f7dcc6f50caafa3667d680a0a5592ae6ca14440216c72b138606ae5465eac17';
			expect(imgsrc(`${hash}.png`)).toBe(`https://cdn.canmi.net/image/${hash}.png`);
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
			expect(imgsrc('https://example.com/x.png')).toBe('https://example.com/x.png');
		});

		it('unknown http host', () => {
			expect(imgsrc('http://example.com/x.png')).toBe('http://example.com/x.png');
		});

		it('malformed url passthrough', () => {
			expect(imgsrc('https://not a url')).toBe('https://not a url');
		});
	});

	describe('github: scheme → jsdelivr', () => {
		it('without @ref uses default branch', () => {
			expect(imgsrc('github:innei/shiro/apps/web/public/innei-dark.svg')).toBe(
				'https://cdn.jsdelivr.net/gh/innei/shiro/apps/web/public/innei-dark.svg',
			);
		});

		it('with @ref pins the commit', () => {
			expect(imgsrc('github:innei/shiro/apps/web/public/innei-dark.svg@90ef7b8')).toBe(
				'https://cdn.jsdelivr.net/gh/innei/shiro@90ef7b8/apps/web/public/innei-dark.svg',
			);
		});

		it('with @ref accepts branch names', () => {
			expect(imgsrc('github:innei/shiro/icon.svg@main')).toBe(
				'https://cdn.jsdelivr.net/gh/innei/shiro@main/icon.svg',
			);
		});

		it('with too-few segments passthrough', () => {
			expect(imgsrc('github:owner')).toBe('github:owner');
			expect(imgsrc('github:owner/repo')).toBe('github:owner/repo');
		});
	});

	describe('github:avatar: scheme', () => {
		it('numeric id without size', () => {
			expect(imgsrc('github:avatar:72544151')).toBe('https://cdn.ffoni.com/github/avatar/72544151');
		});

		it('numeric id with @size', () => {
			expect(imgsrc('github:avatar:72544151@192')).toBe(
				'https://cdn.ffoni.com/github/avatar/72544151?width=192',
			);
		});

		it('username via @ prefix', () => {
			expect(imgsrc('github:avatar:@canmi21')).toBe('https://cdn.ffoni.com/github/avatar/canmi21');
		});

		it('username with @size', () => {
			expect(imgsrc('github:avatar:@canmi21@192')).toBe(
				'https://cdn.ffoni.com/github/avatar/canmi21?width=192',
			);
		});

		it('respects custom resUrl', () => {
			expect(imgsrc('github:avatar:72544151@192', { resUrl: 'http://localhost:26516' })).toBe(
				'http://localhost:26516/github/avatar/72544151?width=192',
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
		it('avatars.githubusercontent.com → res /github/avatar/<id>', () => {
			expect(imgsrc('https://avatars.githubusercontent.com/u/72544151?v=4')).toBe(
				'https://cdn.ffoni.com/github/avatar/72544151',
			);
		});

		it('avatars URL respects custom resUrl', () => {
			expect(
				imgsrc('https://avatars.githubusercontent.com/u/72544151', {
					resUrl: 'http://localhost:26516',
				}),
			).toBe('http://localhost:26516/github/avatar/72544151');
		});

		it('avatars URL preserves ?s=N as ?width=N', () => {
			expect(imgsrc('https://avatars.githubusercontent.com/u/72544151?s=192')).toBe(
				'https://cdn.ffoni.com/github/avatar/72544151?width=192',
			);
		});

		it('avatars URL drops other query params but keeps ?s=N', () => {
			expect(imgsrc('https://avatars.githubusercontent.com/u/72544151?v=4&s=64')).toBe(
				'https://cdn.ffoni.com/github/avatar/72544151?width=64',
			);
		});

		it('raw.githubusercontent.com → jsdelivr', () => {
			expect(
				imgsrc(
					'https://raw.githubusercontent.com/innei/shiro/90ef7b8/apps/web/public/innei-dark.svg',
				),
			).toBe('https://cdn.jsdelivr.net/gh/innei/shiro@90ef7b8/apps/web/public/innei-dark.svg');
		});

		it('github.com /raw/ link → jsdelivr', () => {
			expect(imgsrc('https://github.com/innei/shiro/raw/main/apps/web/public/innei-dark.svg')).toBe(
				'https://cdn.jsdelivr.net/gh/innei/shiro@main/apps/web/public/innei-dark.svg',
			);
		});

		it('github.com /blob/ link → jsdelivr', () => {
			expect(
				imgsrc('https://github.com/innei/shiro/blob/main/apps/web/public/innei-dark.svg'),
			).toBe('https://cdn.jsdelivr.net/gh/innei/shiro@main/apps/web/public/innei-dark.svg');
		});

		it('jsdelivr URL passthrough (already canonical)', () => {
			const u = 'https://cdn.jsdelivr.net/gh/innei/shiro@main/apps/web/public/innei-dark.svg';
			expect(imgsrc(u)).toBe(u);
		});
	});
});
