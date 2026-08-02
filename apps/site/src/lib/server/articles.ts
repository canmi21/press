import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { URLS } from '@canmi/urls';
import { parse as parseYaml } from 'yaml';
import { createAssetResolver, type AssetManifest, type MediaManifest } from '../assets.ts';
import { assemble, type SegmentLayout, type TranslationSidecar } from '../content/assemble.ts';
import { compile } from '../content/compile.ts';
import { indexingMetadata } from '../content/indexing.ts';
import type { Article, ArticleView } from '../content/types.ts';
import { languageTag, LOCALE_CODES, PUBLIC_LANGUAGE, type LocaleCode } from '../locale.ts';
import { highlight } from './highlight.ts';
import { buildPreviews } from './placeholder.ts';

const SEGMENT_LAYOUT_VERSION = 3;

type BuildPaths = { contents: string; assets: string; media: string; segments: string };

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

function articlePath(contents: string, file: string): string {
	return file.slice(contents.length + 1).replace(/\.md$/, '');
}

function translatedRaws(
	file: string,
	article: string,
	raw: string,
	sidecar: TranslationSidecar,
	layout: SegmentLayout,
): Record<LocaleCode, string> {
	const spans = layout.articles[article];
	if (!spans) throw new Error(`${file}: missing from data/build/segments.json`);
	const views = { mw: raw } as Record<LocaleCode, string>;
	for (const [code, locale] of Object.entries(PUBLIC_LANGUAGE) as [
		Exclude<LocaleCode, 'mw'>,
		(typeof PUBLIC_LANGUAGE)[Exclude<LocaleCode, 'mw'>],
	][]) {
		const assembled = assemble(raw, spans, sidecar, locale, file);
		if (assembled.missing.length > 0) {
			throw new Error(`${file}: ${locale} is missing ${assembled.missing.length} live segments`);
		}
		views[code] = assembled.raw;
	}
	return views;
}

export async function buildArticles(
	paths: BuildPaths,
): Promise<{ articles: Article[]; files: string[] }> {
	const files = await articleFiles(paths.contents);
	const assets = JSON.parse(await readFile(paths.assets, 'utf8')) as AssetManifest;
	const media = (parseYaml(await readFile(paths.media, 'utf8')) ?? { media: {} }) as MediaManifest;
	const layout = JSON.parse(await readFile(paths.segments, 'utf8')) as SegmentLayout;
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
		const path = articlePath(paths.contents, file);
		const url = `${URLS.apps.production.site}/${path}`;
		const source = await compile(raw, url, {
			resolveAsset: createAssetResolver(assets, media, previews, 'en-US'),
			highlight,
			sourceFile: file,
		});
		const raws = translatedRaws(file, `${path}.md`, raw, sidecar, layout);
		const compiled = {
			mw: source,
			...Object.fromEntries(
				await Promise.all(
					(Object.keys(PUBLIC_LANGUAGE) as Exclude<LocaleCode, 'mw'>[]).map(async (code) => [
						code,
						await compile(raws[code], url, {
							resolveAsset: createAssetResolver(assets, media, previews, PUBLIC_LANGUAGE[code]),
							highlight,
						}),
					]),
				),
			),
		} as Record<LocaleCode, Awaited<ReturnType<typeof compile>>>;

		const sourceLanguage = compiled.mw.meta.lang;
		const { canonical, alternates } = indexingMetadata(url, sourceLanguage, raws);
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
					} satisfies ArticleView,
				];
			}),
		) as Record<LocaleCode, ArticleView>;
		articles.push({ ...compiled.mw, path, url, views, alternates });
	}

	return {
		articles: articles.toSorted((a, b) => Date.parse(b.meta.created) - Date.parse(a.meta.created)),
		files: [
			...files,
			...files.map((file) => file.replace(/\.md$/, '.i18n.yaml')),
			paths.assets,
			paths.media,
			paths.segments,
		],
	};
}
