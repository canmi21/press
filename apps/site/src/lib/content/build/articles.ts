import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { URLS } from '@canmi/urls';
import { parse as parseYaml } from 'yaml';
import { createAssetResolver, type AssetManifest, type MediaManifest } from './assets.ts';
import { assemble, type SegmentLayout, type TranslationSidecar } from './assemble.ts';
import { articleFrontmatter, compile, compilePage } from './compile.ts';
import { indexingMetadata } from './indexing.ts';

/**
 * One interface string the compiler emits into markup rather than a component rendering it.
 *
 * Read out of the message files directly. Paraglide compiles its messages through a Vite plugin,
 * and this module is imported by vite.config.ts before that plugin has run -- so a build-time
 * caller reads the same JSON the plugin does rather than the code it will generate. A key that
 * disappears fails loudly here rather than emitting an empty parenthesis into every article.
 */
async function newTabNotes(messages: string): Promise<Record<LocaleCode, string>> {
	const notes = {} as Record<LocaleCode, string>;
	for (const code of LOCALE_CODES) {
		const catalog = JSON.parse(await readFile(join(messages, `${code}.json`), 'utf8'));
		const note = catalog['support.new-tab'];
		if (typeof note !== 'string') {
			throw new Error(`${code}.json: support.new-tab is missing`);
		}
		notes[code] = note;
	}
	return notes;
}
import type {
	Article,
	ArticleReference,
	ArticleSummary,
	ArticleView,
	CompiledPage,
	CrateRecord,
	Page,
	PageView,
	RepoRecord,
	TweetRecord,
} from '../types.ts';
import { languageTag, LOCALE_CODES, PUBLIC_LANGUAGE, type LocaleCode } from '../../locale/index.ts';
import { highlight } from './highlight.ts';
import { buildPreviews } from './placeholder.ts';

const SEGMENT_LAYOUT_VERSION = 3;

type SummarySidecar = { summary?: Record<string, { text?: string; provider?: string }> };

/**
 * The locale an article's own language names, as the records are keyed.
 *
 * Frontmatter writes the short form; every locale-addressed record here uses the public tag.
 * Traditional Chinese is matched by script before the language falls through -- the same rule
 * `cms summary` applies on the other side of the file. See spec/i18n.md.
 */
function sourceLocale(lang: string): string {
	const [primary = lang, ...rest] = lang.toLowerCase().split('-');
	if (primary === 'zh') {
		const traditional = rest.some((part) => part === 'hant' || ['tw', 'hk', 'mo'].includes(part));
		return traditional ? 'zh-TW' : 'zh-CN';
	}
	const code = (Object.keys(PUBLIC_LANGUAGE) as Exclude<LocaleCode, 'mw'>[]).find(
		(candidate) => PUBLIC_LANGUAGE[candidate].toLowerCase().split('-')[0] === primary,
	);
	return code ? PUBLIC_LANGUAGE[code] : 'en-US';
}

/** The summary sidecar, which is absent until `cms summary` has been run for that article. */
async function readSummaries(file: string): Promise<Record<string, ArticleSummary>> {
	try {
		const text = await readFile(file.replace(/\.md$/, '.summary.yaml'), 'utf8');
		const parsed = parseYaml(text) as SummarySidecar;
		const summaries: Record<string, ArticleSummary> = {};
		for (const [locale, entry] of Object.entries(parsed.summary ?? {})) {
			const summary = entry?.text?.trim();
			if (!summary) continue;
			summaries[locale] = {
				text: summary,
				provider: entry?.provider?.trim() ?? '',
			};
		}
		return summaries;
	} catch {
		return {};
	}
}

/** Prefer the requested summary, then the one fallback language every view can name. */
export function summaryFor(
	summaries: Readonly<Record<string, ArticleSummary>>,
	locale: string,
): ArticleSummary | undefined {
	return summaries[locale] ?? summaries['en-US'];
}

type BuildPaths = {
	contents: string;
	/** Which CDN the built markup names; see createAssetResolver. */
	cdnUrl: string;
	/** Where the Paraglide message catalogues live, read directly; see newTabNotes. */
	messages: string;
	assets: string;
	media: string;
	segments: string;
	crates: string;
	repos: string;
	tweets: string;
};

async function articleFiles(contents: string): Promise<string[]> {
	const files: string[] = [];
	for (const category of await readdir(contents, { withFileTypes: true })) {
		if (!category.isDirectory()) continue;
		const directory = join(contents, category.name);
		for (const entry of await readdir(directory, { withFileTypes: true })) {
			if (entry.isFile() && entry.name.endsWith('.md')) files.push(join(directory, entry.name));
		}
	}
	return files.toSorted();
}

async function pageFiles(contents: string): Promise<string[]> {
	return (await readdir(contents, { withFileTypes: true }))
		.filter((entry) => entry.isFile() && entry.name.endsWith('.md'))
		.map((entry) => join(contents, entry.name))
		.toSorted();
}

function articlePath(contents: string, file: string): string {
	return file.slice(contents.length + 1).replace(/\.md$/, '');
}

function pageDocument(path: string, { meta, body }: Pick<CompiledPage, 'meta' | 'body'>): string {
	const name = path.charAt(0).toUpperCase() + path.slice(1);
	const lines = [`# ${name}`, ''];
	if (meta.summary) lines.push(`> ${meta.summary}`, '');
	if (meta.title) {
		lines.push(`## ${meta.title}`, '');
		if (meta.description) lines.push(meta.description, '');
	}
	if (body) lines.push('## Bio', '', body, '');
	return `${lines.join('\n').trim()}\n`;
}

export function translatedRaws(
	file: string,
	article: string,
	raw: string,
	sidecar: TranslationSidecar,
	layout: SegmentLayout,
): {
	raws: Record<LocaleCode, string>;
	translatable: Record<LocaleCode, string>;
	translationAvailable: Record<LocaleCode, boolean>;
} {
	const spans = layout.articles[article];
	if (!spans) throw new Error(`${file}: missing from data/build/segments.json`);
	const raws = { mw: raw } as Record<LocaleCode, string>;
	const translatable = {} as Record<LocaleCode, string>;
	const translationAvailable = { mw: true } as Record<LocaleCode, boolean>;
	for (const [code, locale] of Object.entries(PUBLIC_LANGUAGE) as [
		Exclude<LocaleCode, 'mw'>,
		(typeof PUBLIC_LANGUAGE)[Exclude<LocaleCode, 'mw'>],
	][]) {
		const assembled = assemble(raw, spans, sidecar, locale, file);
		if (assembled.missing.length > 0) {
			// One missing body block falls back the whole view. Mixing translated and source
			// paragraphs would look complete while changing language halfway through the article.
			raws[code] = raw;
			translatable[code] = assembled.translatable.source;
			translationAvailable[code] = false;
		} else {
			raws[code] = assembled.raw;
			translatable[code] = assembled.translatable.translated;
			translationAvailable[code] = true;
		}
		translatable.mw = assembled.translatable.source;
	}
	return { raws, translatable, translationAvailable };
}

/** One article's assembled views, carried from the frontmatter pass into the compile pass. */
type Prepared = {
	file: string;
	path: string;
	url: string;
	originLocale: string;
	summaries: Record<string, ArticleSummary>;
	raws: Record<LocaleCode, string>;
	translatable: Record<LocaleCode, string>;
	translationAvailable: Record<LocaleCode, boolean>;
};

export async function buildArticles(
	paths: BuildPaths,
): Promise<{ articles: Article[]; files: string[] }> {
	const files = await articleFiles(paths.contents);
	const notes = await newTabNotes(paths.messages);
	const assets = JSON.parse(await readFile(paths.assets, 'utf8')) as AssetManifest;
	const media = (parseYaml(await readFile(paths.media, 'utf8')) ?? { media: {} }) as MediaManifest;
	const layout = JSON.parse(await readFile(paths.segments, 'utf8')) as SegmentLayout;
	// Captured before the build. An absent record is a working state rather than an error: the
	// article keeps the directive as a placeholder until its external facts have been fetched.
	const embeds = {
		crates:
			(
				JSON.parse(await readFile(paths.crates, 'utf8').catch(() => '{}')) as {
					crates?: Record<string, CrateRecord>;
				}
			).crates ?? {},
		repos:
			(
				JSON.parse(await readFile(paths.repos, 'utf8').catch(() => '{}')) as {
					repos?: Record<string, RepoRecord>;
				}
			).repos ?? {},
		tweets:
			(
				JSON.parse(await readFile(paths.tweets, 'utf8').catch(() => '{}')) as {
					tweets?: Record<string, TweetRecord>;
				}
			).tweets ?? {},
	};
	if (layout.version !== SEGMENT_LAYOUT_VERSION) {
		throw new Error(
			`${paths.segments}: expected version ${SEGMENT_LAYOUT_VERSION}, got ${layout.version}`,
		);
	}
	const previews = await buildPreviews(assets);
	const articles: Article[] = [];

	// Two passes over the corpus, because an `::article` card names an article that nothing has
	// compiled yet. The first assembles every view and reads its frontmatter; the second
	// compiles, by which point every article is nameable in every locale.
	const prepared: Prepared[] = [];
	const references = Object.fromEntries(
		LOCALE_CODES.map((code) => [code, {} as Record<string, ArticleReference>]),
	) as Record<LocaleCode, Record<string, ArticleReference>>;

	for (const file of files) {
		const sidecarFile = file.replace(/\.md$/, '.i18n.yaml');
		const [raw, sidecarText] = await Promise.all([
			readFile(file, 'utf8'),
			// A translation sidecar is generated output. Its absence is a source-only article,
			// not a reason the site cannot render that article at all.
			readFile(sidecarFile, 'utf8').catch(() => ''),
		]);
		const sidecar = (parseYaml(sidecarText) ?? {}) as TranslationSidecar;
		const summaries = await readSummaries(file);
		const path = articlePath(paths.contents, file);
		const url = `${URLS.apps.production.site}/${path}`;
		// The original's own locale, read off the frontmatter before anything is compiled --
		// `compile` reports it, but the resolver below needs it to run at all.
		const originLocale = sourceLocale(/^lang:\s*(\S+)/m.exec(raw)?.[1] ?? 'en-US');
		const { raws, translatable, translationAvailable } = translatedRaws(
			file,
			`${path}.md`,
			raw,
			sidecar,
			layout,
		);
		for (const code of LOCALE_CODES) {
			const { title, subtitle, description, created } = articleFrontmatter(raws[code], file);
			references[code][path] = { title, subtitle, description, created };
		}
		prepared.push({
			file,
			path,
			url,
			originLocale,
			summaries,
			raws,
			translatable,
			translationAvailable,
		});
	}

	for (const {
		file,
		path,
		url,
		originLocale,
		summaries,
		raws,
		translatable,
		translationAvailable,
	} of prepared) {
		const source = await compile(raws.mw, url, {
			newTabNote: notes.mw,
			// Not `en-US`. On the original view an image's alt is read beside prose in the
			// article's own language, and a description generated in English and translated into
			// eight is available in that one too. Reading the original meant hearing the pictures
			// described in a language the article never used.
			resolveAsset: createAssetResolver(assets, media, previews, paths.cdnUrl, originLocale),
			articles: references.mw,
			highlight,
			sourceFile: file,
			embeds,
		});
		const compiled = {
			mw: source,
			...Object.fromEntries(
				await Promise.all(
					(Object.keys(PUBLIC_LANGUAGE) as Exclude<LocaleCode, 'mw'>[]).map(async (code) => [
						code,
						translationAvailable[code]
							? await compile(raws[code], url, {
									newTabNote: notes[code],
									resolveAsset: createAssetResolver(
										assets,
										media,
										previews,
										paths.cdnUrl,
										PUBLIC_LANGUAGE[code],
									),
									articles: references[code],
									highlight,
									sourceFile: file,
									embeds,
								})
							: source,
					]),
				),
			),
		} as Record<LocaleCode, Awaited<ReturnType<typeof compile>>>;

		const sourceLanguage = compiled.mw.meta.lang;
		const { canonical, canonicalUrls, alternates } = indexingMetadata(url, translatable);
		const views = Object.fromEntries(
			LOCALE_CODES.map((code) => {
				const view = compiled[code];
				const summaryLocale = code === 'mw' ? originLocale : PUBLIC_LANGUAGE[code];
				return [
					code,
					{
						meta: view.meta,
						toc: view.toc,
						blocks: view.blocks,
						text: view.text,
						feed: view.feed,
						code,
						languageTag: languageTag(code, sourceLanguage),
						canonical: canonical[code],
						translationAvailable: translationAvailable[code],
						// `mw` takes the summary written in the article's own language rather
						// than a translation of it, for the same reason it takes that language's
						// alt text: the original view is the one nothing was done to.
						summary: summaryFor(summaries, summaryLocale),
					} satisfies ArticleView,
				];
			}),
		) as Record<LocaleCode, ArticleView>;
		articles.push({ ...compiled.mw, path, url, views, canonicalUrls, alternates });
	}

	return {
		articles: articles.toSorted((a, b) => Date.parse(b.meta.created) - Date.parse(a.meta.created)),
		files: [
			...files,
			...files.map((file) => file.replace(/\.md$/, '.i18n.yaml')),
			...files.map((file) => file.replace(/\.md$/, '.summary.yaml')),
			paths.assets,
			paths.media,
			paths.segments,
			paths.crates,
			paths.repos,
			paths.tweets,
		],
	};
}

export async function buildPages(
	paths: Pick<BuildPaths, 'contents' | 'messages' | 'segments'>,
): Promise<{ pages: Page[]; files: string[] }> {
	const files = await pageFiles(paths.contents);
	const layout = JSON.parse(await readFile(paths.segments, 'utf8')) as SegmentLayout;
	if (layout.version !== SEGMENT_LAYOUT_VERSION) {
		throw new Error(
			`${paths.segments}: expected version ${SEGMENT_LAYOUT_VERSION}, got ${layout.version}`,
		);
	}
	const pages: Page[] = [];

	for (const file of files) {
		const raw = await readFile(file, 'utf8');
		const path = articlePath(paths.contents, file);
		const source = compilePage(raw, file);
		// Every view of a page is the source. A page is not an article: the homepage is identity
		// copy, its bio was always rendered from `mw` whatever the view, and the eight
		// translations sitting beside it were never read by anything. Keeping them meant a
		// sidecar the build could not start without, holding text nobody would ever see.
		// See spec/i18n.md.
		const compiled = Object.fromEntries(LOCALE_CODES.map((code) => [code, source])) as Record<
			LocaleCode,
			CompiledPage
		>;
		const views = Object.fromEntries(
			LOCALE_CODES.map((code) => {
				const { meta, blocks } = compiled[code];
				return [code, { meta, blocks } satisfies PageView];
			}),
		) as Record<LocaleCode, PageView>;

		pages.push({ path, markdown: pageDocument(path, source), views });
	}

	return { pages, files: [...files, paths.segments] };
}
