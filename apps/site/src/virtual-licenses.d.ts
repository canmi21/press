declare module 'virtual:licenses' {
	import type { LicenseRecord } from '$lib/licenses/record';

	/**
	 * Every dependency the deployables ship, keyed by purl. Written by `cms licenses` into
	 * data/build/licenses.json and embedded here by Vite; the licence texts it points at are
	 * published objects rather than part of this.
	 */
	export const licenses: LicenseRecord;
}
