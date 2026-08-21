export async function prepareBrowserRuntime(): Promise<void> {
	if (typeof Array.prototype.toSorted === 'function') return;
	await import('core-js/modules/es.array.to-sorted.js');
}
