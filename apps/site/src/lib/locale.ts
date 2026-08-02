export const LOCALE_CODES = ['mw', 'de', 'en', 'es', 'fr', 'ja', 'ko', 'zh', 'tw'] as const;

export type LocaleCode = (typeof LOCALE_CODES)[number];

export const PUBLIC_LANGUAGE = {
	de: 'de-DE',
	en: 'en-US',
	es: 'es-ES',
	fr: 'fr-FR',
	ja: 'ja-JP',
	ko: 'ko-KR',
	zh: 'zh-CN',
	tw: 'zh-TW',
} as const satisfies Record<Exclude<LocaleCode, 'mw'>, string>;

const CODE_SET = new Set<string>(LOCALE_CODES);

export function localeCode(value: string | null | undefined): LocaleCode | undefined {
	return value != null && CODE_SET.has(value) ? (value as LocaleCode) : undefined;
}

/** A public BCP-47 tag. `mw` is the source article, so its tag is article-owned. */
export function languageTag(code: LocaleCode, sourceLanguage: string): string {
	return code === 'mw' ? sourceLanguage : PUBLIC_LANGUAGE[code];
}

function codeForLanguageRange(value: string): Exclude<LocaleCode, 'mw'> | undefined {
	const range = value.toLowerCase();
	const subtags = range.split('-');
	if (subtags[0] === 'zh') {
		return subtags.includes('hant') || ['tw', 'hk', 'mo'].some((tag) => subtags.includes(tag))
			? 'tw'
			: 'zh';
	}
	const base = subtags[0];
	return base === 'de' ||
		base === 'en' ||
		base === 'es' ||
		base === 'fr' ||
		base === 'ja' ||
		base === 'ko'
		? base
		: undefined;
}

export function acceptedLocale(header: string | null | undefined): LocaleCode | undefined {
	if (!header) return undefined;
	const ranges = header
		.split(',')
		.map((part, index) => {
			const [rawRange, ...parameters] = part.trim().split(';');
			if (!rawRange || rawRange === '*') return undefined;
			let quality = 1;
			for (const parameter of parameters) {
				const match = /^\s*q=(0(?:\.\d{0,3})?|1(?:\.0{0,3})?)\s*$/i.exec(parameter);
				if (!match) return undefined;
				quality = Number(match[1]);
			}
			return Number.isFinite(quality) && quality > 0
				? { range: rawRange, quality, index }
				: undefined;
		})
		.filter((range): range is { range: string; quality: number; index: number } => range != null)
		.toSorted((a, b) => b.quality - a.quality || a.index - b.index);

	for (const { range } of ranges) {
		const code = codeForLanguageRange(range);
		if (code) return code;
	}
	return undefined;
}

export type LocaleInputs = {
	query: string | null | undefined;
	cookie: string | null | undefined;
	acceptLanguage: string | null | undefined;
};

/** First valid source wins; the article itself is always the final `mw` view. */
export function resolveLocale(inputs: LocaleInputs): LocaleCode {
	return (
		localeCode(inputs.query) ??
		localeCode(inputs.cookie) ??
		acceptedLocale(inputs.acceptLanguage) ??
		'mw'
	);
}

export function shouldWriteLanguageCookie(
	cookie: string | null | undefined,
	code: LocaleCode,
): boolean {
	return cookie !== code;
}

/** Preserve the order and values of every parameter not owned by locale selection. */
export function withoutLanguageParameter(url: URL): string | undefined {
	if (!url.searchParams.has('lang')) return undefined;
	url.searchParams.delete('lang');
	return `${url.pathname}${url.search}${url.hash}`;
}

/** Cookie-varying HTML must never pass through a shared cache. */
export function privateHtml(response: Response): Response {
	if (!response.headers.get('content-type')?.toLowerCase().startsWith('text/html')) return response;
	const headers = new Headers(response.headers);
	headers.set('Cache-Control', 'private, no-store');
	return new Response(response.body, {
		status: response.status,
		statusText: response.statusText,
		headers,
	});
}
