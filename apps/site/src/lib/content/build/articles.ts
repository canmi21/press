import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { URLS } from '@canmi/urls';
import { parse as parseYaml } from 'yaml';
import { createAssetResolver, type AssetManifest, type MediaManifest } from './assets.ts';
import { assemble, type SegmentLayout, type TranslationSidecar } from './assemble.ts';
import { compile, compilePage } from './compile.ts';
import { indexingMetadata } from './indexing.ts';
import type {
	Article,
	ArticleView,
	CompiledPage,
	CrateRecord,
	Page,
	PageView,
	RepoRecord,
} from '../types.ts';
import { languageTag, LOCALE_CODES, PUBLIC_LANGUAGE, type LocaleCode } from '../../locale/index.ts';
import { highlight } from './highlight.ts';
import { buildPreviews } from './placeholder.ts';

const SEGMENT_LAYOUT_VERSION = 3;

type SummarySidecar = { summary?: Record<string, { text?: string }> };

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
async function readSummaries(file: string): Promise<Record<string, string>> {
	try {
		const text = await readFile(file.replace(/\.md$/, '.summary.yaml'), 'utf8');
		const parsed = parseYaml(text) as SummarySidecar;
		return Object.fromEntries(
			Object.entries(parsed.summary ?? {})
				.map(([locale, entry]) => [locale, entry?.text?.trim() ?? ''])
				.filter(([, value]) => value.length > 0),
		);
	} catch {
		return {};
	}
}

type BuildPaths = {
	contents: string;
	assets: string;
	media: string;
	segments: string;
	crates: string;
	repos: string;
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

function translatedRaws(
	file: string,
	article: string,
	raw: string,
	sidecar: TranslationSidecar,
	layout: SegmentLayout,
): { raws: Record<LocaleCode, string>; translatable: Record<LocaleCode, string> } {
	const spans = layout.articles[article];
	if (!spans) throw new Error(`${file}: missing from data/build/segments.json`);
	const raws = { mw: raw } as Record<LocaleCode, string>;
	const translatable = {} as Record<LocaleCode, string>;
	for (const [code, locale] of Object.entries(PUBLIC_LANGUAGE) as [
		Exclude<LocaleCode, 'mw'>,
		(typeof PUBLIC_LANGUAGE)[Exclude<LocaleCode, 'mw'>],
	][]) {
		const assembled = assemble(raw, spans, sidecar, locale, file);
		if (assembled.missing.length > 0) {
			throw new Error(`${file}: ${locale} is missing ${assembled.missing.length} live segments`);
		}
		raws[code] = assembled.raw;
		translatable.mw = assembled.translatable.source;
		translatable[code] = assembled.translatable.translated;
	}
	return { raws, translatable };
}

export async function buildArticles(
	paths: BuildPaths,
): Promise<{ articles: Article[]; files: string[] }> {
	const files = await articleFiles(paths.contents);
	const assets = JSON.parse(await readFile(paths.assets, 'utf8')) as AssetManifest;
	const media = (parseYaml(await readFile(paths.media, 'utf8')) ?? { media: {} }) as MediaManifest;
	const layout = JSON.parse(await readFile(paths.segments, 'utf8')) as SegmentLayout;
	// Fetched by `cms embed`. Absent is a working state rather than an error: an article whose
	// crate has not been read yet keeps the placeholder it had before, which is what the
	// placeholder is for.
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
	};
	if (layout.version !== SEGMENT_LAYOUT_VERSION) {
		throw new Error(
			`${paths.segments}: expected version ${SEGMENT_LAYOUT_VERSION}, got ${layout.version}`,
		);
	}
	const previews = await buildPreviews(assets);
	const articles: Article[] = [];

	for (const file of files) {
		const sidecarFile = file.replace(/\.md$/, '.i18n.yaml');
		const [raw, sidecarText] = await Promise.all([
			readFile(file, 'utf8'),
			readFile(sidecarFile, 'utf8'),
		]);
		const sidecar = parseYaml(sidecarText) as TranslationSidecar;
		const summaries = await readSummaries(file);
		const path = articlePath(paths.contents, file);
		const url = `${URLS.apps.production.site}/${path}`;
		// The original's own locale, read off the frontmatter before anything is compiled --
		// `compile` reports it, but the resolver below needs it to run at all.
		const originLocale = sourceLocale(/^lang:\s*(\S+)/m.exec(raw)?.[1] ?? 'en-US');
		const source = await compile(raw, url, {
			// Not `en-US`. On the original view an image's alt is read beside prose in the
			// article's own language, and a description generated in English and translated into
			// eight is available in that one too. Reading the original meant hearing the pictures
			// described in a language the article never used.
			resolveAsset: createAssetResolver(assets, media, previews, originLocale),
			highlight,
			sourceFile: file,
			embeds,
		});
		const { raws, translatable } = translatedRaws(file, `${path}.md`, raw, sidecar, layout);
		const compiled = {
			mw: source,
			...Object.fromEntries(
				await Promise.all(
					(Object.keys(PUBLIC_LANGUAGE) as Exclude<LocaleCode, 'mw'>[]).map(async (code) => [
						code,
						await compile(raws[code], url, {
							resolveAsset: createAssetResolver(assets, media, previews, PUBLIC_LANGUAGE[code]),
							highlight,
							sourceFile: file,
							embeds,
						}),
					]),
				),
			),
		} as Record<LocaleCode, Awaited<ReturnType<typeof compile>>>;

		const sourceLanguage = compiled.mw.meta.lang;
		const { canonical, canonicalUrls, alternates } = indexingMetadata(url, translatable);
		const views = Object.fromEntries(
			LOCALE_CODES.map((code) => {
				const view = compiled[code];
				return [
					code,
					{
						meta: view.meta,
						blocks: view.blocks,
						text: view.text,
						feed: view.feed,
						code,
						languageTag: languageTag(code, sourceLanguage),
						canonical: canonical[code],
						// `mw` takes the summary written in the article's own language rather
						// than a translation of it, for the same reason it takes that language's
						// alt text: the original view is the one nothing was done to.
						summary: summaries[code === 'mw' ? originLocale : PUBLIC_LANGUAGE[code]],
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
			paths.assets,
			paths.media,
			paths.segments,
			paths.crates,
			paths.repos,
		],
	};
}

export async function buildPages(
	paths: Pick<BuildPaths, 'contents' | 'segments'>,
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
