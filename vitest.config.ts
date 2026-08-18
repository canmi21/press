import { defineConfig } from 'vitest/config';

/**
 * The one thing the default configuration cannot resolve.
 *
 * `apps/cdn` imports its codecs' `.wasm` files directly, because wrangler substitutes a compiled
 * `WebAssembly.Module` for each import at bundle time. Node has no such substitution, so any test
 * that reaches one of those modules fails to load before a single assertion runs -- which left the
 * route module untestable and its behaviour unheld.
 *
 * The stub is an empty object, and that is safe for exactly the reason it is narrow: a codec is
 * initialised lazily, on the first call that needs it, so a path that does not encode or decode
 * never touches what this replaces.
 *
 * **The limit, stated because it is invisible otherwise:** a test that does exercise a transcode
 * will not fail honestly here -- it will fail inside a codec initialised from nothing. Testing
 * that path needs a real Worker runtime (`@cloudflare/vitest-pool-workers`), not a wider stub.
 */
export default defineConfig({
	plugins: [
		{
			name: 'stub-wasm',
			// Ahead of Vite's own resolver, which otherwise hands the bytes to the JS loader and
			// fails while reading them as source.
			enforce: 'pre' as const,
			resolveId(id) {
				return id.endsWith('.wasm') ? '\0stub-wasm' : null;
			},
			load(id) {
				return id === '\0stub-wasm' ? 'export default {};' : null;
			},
		},
	],
});
