/// <reference types="@cloudflare/workers-types" />

import type { LocaleCode } from '$lib/locale';

declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			locale?: { code: LocaleCode; languageTag: string };
		}
		// interface PageData {}
		// interface PageState {}
		interface Platform {
			env: Record<string, unknown>;
			context: ExecutionContext;
			caches: CacheStorage;
			cf?: IncomingRequestCfProperties;
		}
	}

	interface ImportMetaEnv {
		// URLs are imported from @canmi/urls rather than injected, so there is one place to
		// read them from and no second spelling to keep in step. What remains here are values
		// that only exist at build time and have no other source.
		/** TODO: captured for a footer that is not built yet; see vite.config.ts. */
		readonly VITE_COMMIT_HASH: string;
		readonly VITE_BUILD_TIME: string;
	}

	interface ImportMeta {
		readonly env: ImportMetaEnv;
	}

	interface Window {
		canmiArticleInitialHash?: string;
	}
}

export {};
