/**
 * Reaching into markup this file did not write.
 *
 * The desktop shell renders its shape in index.html and fills it from TypeScript, so every module
 * here starts by finding the nodes it owns. A selector that matches nothing is a mismatch between
 * the markup and the code, which is a bug in this repository rather than a state to handle -- so
 * it throws, naming the selector, instead of returning null for each caller to re-decide.
 */
export function requiredElement<T extends Element>(root: ParentNode, selector: string): T {
	const element = root.querySelector<T>(selector);
	if (element === null) throw new Error(`required element is missing: ${selector}`);
	return element;
}
