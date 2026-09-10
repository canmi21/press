import { describe, expect, it } from 'vitest';
import { inlineScriptString } from './inline-script';
import { applyTheme, currentTheme, followSystemTheme, themeCookie, themeScript } from './index';

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

	it('writes the same cookie the control writes', () => {
		// The script settles the first visit and the control settles every one after it. A
		// difference in path or lifetime here is a preference that expires on one of the two
		// paths and not the other, which nothing would report.
		const [, attributes = ''] = /document\.cookie="theme="\+m\+"([^"]*)"/.exec(themeScript) ?? [];
		expect(attributes).not.toBe('');
		expect(themeCookie('dark')).toBe(`theme=dark${attributes}`);
		expect(themeCookie('light')).toBe(`theme=light${attributes}`);
	});
});

describe('reading and painting a theme', () => {
	function root(dark: boolean) {
		const classes = new Set(dark ? ['dark'] : []);
		return {
			classList: {
				contains: (name: string) => classes.has(name),
				toggle: (name: string, on: boolean) => (on ? classes.add(name) : classes.delete(name)),
			},
		} as unknown as HTMLElement;
	}

	it('reads what is painted rather than what was stored', () => {
		// On a first visit there is no cookie yet and the class is already correct, which is
		// exactly when the two would disagree.
		expect(currentTheme(root(true))).toBe('dark');
		expect(currentTheme(root(false))).toBe('light');
	});

	it('paints either theme, from either theme', () => {
		const element = root(false);
		applyTheme('dark', element);
		expect(currentTheme(element)).toBe('dark');
		applyTheme('dark', element);
		expect(currentTheme(element)).toBe('dark');
		applyTheme('light', element);
		expect(currentTheme(element)).toBe('light');
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
