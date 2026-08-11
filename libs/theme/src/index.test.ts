import { describe, expect, it } from 'vitest';
import { inlineScriptString } from './inline-script';
import { followSystemTheme, themeScript } from './index';

describe('inlineScriptString', () => {
	it('keeps serialized values inside the inline script element', () => {
		expect(inlineScriptString('</script>\u2028\u2029')).toBe(
			'"\\u003C/script\\u003E\\u2028\\u2029"',
		);
	});
});

describe('themeScript', () => {
	it('keeps the site theme cookie contract', () => {
		expect(themeScript).toContain('\\btheme=(light|dark)\\b');
		expect(themeScript).toContain('document.cookie="theme="+m+');
	});
});

describe('followSystemTheme', () => {
	it('applies the current mode and follows changes', () => {
		const modes: boolean[] = [];
		let listener: (() => void) | undefined;
		let removed: (() => void) | undefined;
		const root = {
			classList: { toggle: (_name: string, enabled: boolean) => modes.push(enabled) },
		} as unknown as HTMLElement;
		const media = {
			matches: false,
			addEventListener: (_type: string, callback: () => void) => {
				listener = callback;
			},
			removeEventListener: (_type: string, callback: () => void) => {
				removed = callback;
			},
		} as unknown as MediaQueryList;

		const stop = followSystemTheme(root, media);
		(media as unknown as { matches: boolean }).matches = true;
		listener?.();
		stop();

		expect(modes).toEqual([false, true]);
		expect(removed).toBe(listener);
	});
});
