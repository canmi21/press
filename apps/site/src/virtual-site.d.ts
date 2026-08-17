declare module 'virtual:site' {
	export const site: {
		name: string;
		tagline: string;
		author: {
			name: string;
			fullName: string;
			role: string;
			email: string;
			telegram: string;
			x?: string;
			github: string;
			githubId: number;
			fediverse: string;
			bluesky: string;
		};
		feed: { id: string; followDescription: string };
	};
}
