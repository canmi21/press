declare module 'virtual:redirects' {
	// Merged 301 map (built-in + site.config.yaml), baked at build and consumed by
	// the prerendered [...path] route.
	export const redirects: Record<string, string>;
}
