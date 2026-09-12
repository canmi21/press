/**
 * Everything this site remembers about a reader, in one `localStorage` key.
 *
 * `cache` belongs to TanStack Query and `email` to the newsletter; both are somebody else's
 * record with its own lifetime. What was left was the site's own small facts, and the first of
 * them arrived as a loose key of its own. A second would have arrived the same way, and a
 * tenth -- which is how a reader's storage ends up a scatter of names nothing owns and nothing
 * can move together. The persisted query cache showed the shape to take instead: one container,
 * edited in place.
 *
 * Kept deliberately simpler than that cache. There is no eviction, no staleness and no
 * serialisation beyond `JSON`, because none of these facts expire and all of them are small.
 *
 * **Keys are flat and dotted**, the way the message catalogue's are: `support.preferred`, not a
 * `support` object with a `preferred` field inside it. Nesting buys grouping that the dot already
 * expresses, and costs every reader and writer a walk down a path that may not exist yet. A
 * component simple enough to hold one fact names it after the component and stops.
 *
 * **The store is passed in**, the way `readTrail` takes one: the browser's is the only one in
 * production and a test has no business installing a global to reach this. See spec/styling.md.
 *
 * **The version is an integer and only ever goes up.** It is here from the first write rather
 * than added when it is first needed, because a record without one cannot be migrated later: the
 * code that would migrate it has no way to know what it is looking at. Three hundred versions
 * from now it is still an integer, and the cost of starting today is this paragraph.
 */

const KEY = 'state';

/** The shape `write` produces. Raise it in the same commit that adds the migration to reach it. */
export const VERSION = 1;

export type State = { version: number; [key: string]: unknown };

/** As much of `Storage` as this needs, so a test can hand over a plain object. */
export type Store = Pick<Storage, 'getItem' | 'setItem'>;

/**
 * One step per version, in order: `MIGRATIONS[0]` takes a record at version 1 to version 2.
 *
 * A step edits the record in place and may assume every earlier step has run. It may not fail:
 * there is nowhere to report to and nothing a reader could do, so a step that cannot make sense
 * of what it finds deletes it and lets the default stand.
 *
 * Empty today, which is the point at which the mechanism is cheapest to introduce.
 */
const MIGRATIONS: ((state: State) => void)[] = [];

function fresh(): State {
	return { version: VERSION };
}

/**
 * The stored record, migrated up to `VERSION`.
 *
 * A record from a *newer* version is returned untouched rather than reset. That case is a reader
 * whose other device runs a later build -- which is the case cloud sync will make ordinary -- and
 * the keys this build understands are still readable inside it. Discarding it would throw away
 * facts this build simply has no opinion about.
 */
function read(storage: Store): State {
	try {
		const raw = storage.getItem(KEY);
		if (raw === null) return fresh();
		const parsed: unknown = JSON.parse(raw);
		if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return fresh();
		const state = parsed as State;
		if (!Number.isInteger(state.version) || state.version < 1) return fresh();
		while (state.version < VERSION) {
			MIGRATIONS[state.version - 1]?.(state);
			state.version += 1;
		}
		return state;
	} catch {
		// No storage at all, or a record that is not JSON. Either way the defaults stand.
		return fresh();
	}
}

function save(storage: Store, state: State): void {
	try {
		storage.setItem(KEY, JSON.stringify(state));
	} catch {
		// Private browsing, or storage the reader has turned off. Nothing to record into.
	}
}

/** What is stored under `key`, or `fallback` where nothing is, or the type is not what was asked. */
export function recall<T>(storage: Store, key: string, fallback: T): T {
	const value = read(storage)[key];
	return typeof value === typeof fallback ? (value as T) : fallback;
}

/** Store `value` under `key`, migrating whatever is already there on the way past. */
export function remember(storage: Store, key: string, value: unknown): void {
	const state = read(storage);
	state[key] = value;
	save(storage, state);
}

/** Forget one key, leaving the rest of the record and its version alone. */
export function forget(storage: Store, key: string): void {
	const state = read(storage);
	if (!(key in state)) return;
	delete state[key];
	save(storage, state);
}
