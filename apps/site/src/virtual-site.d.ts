declare module 'virtual:site' {
	export const site: {
		name: string;
		tagline: string;
		author: { name: string; email: string };
		feed: { id: string; followDescription: string };
	};
}
