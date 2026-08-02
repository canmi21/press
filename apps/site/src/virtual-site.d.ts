declare module 'virtual:site' {
	export const site: {
		name: string;
		tagline: string;
		author: { name: string; email: string; telegram: string; x?: string };
		feed: { id: string; followDescription: string };
	};
}
