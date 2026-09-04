// The plugin ships no types. What is declared is only what this repository uses: a Babel plugin
// factory, handed to `plugins` and never called here. Describing it as `unknown` does not
// type-check, because Babel's `PluginItem` will not take it -- so the shape has to be at least
// this specific, and being more specific would be describing an API nothing reads.
declare module 'babel-plugin-polyfill-corejs3' {
	import type { PluginObj } from '@babel/core';

	const plugin: (api: unknown, options: object) => PluginObj;
	export default plugin;
}
