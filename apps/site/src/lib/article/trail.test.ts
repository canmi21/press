import { expect, it } from 'vitest';
import { advance, backTarget, readTrail, type Trail } from './trail';

const at = (path: string, ...paths: string[]): Trail => ({ at: path, paths });
const storage = (value: string) => ({ getItem: () => value });

it('remembers the page a reader came from', () => {
	const trail = advance(at('/'), '/architecture/one', '/');

	expect(trail).toEqual(at('/architecture/one', '/'));
	expect(backTarget(trail)).toBe('/');
});

it('stacks a second article on top of the first', () => {
	const first = advance(undefined, '/architecture/one', '/');
	const second = advance(first, '/architecture/two', '/architecture/one');

	expect(backTarget(second)).toBe('/architecture/one');
	expect(second.paths).toEqual(['/', '/architecture/one']);
});

// Whether the reader used the Back control, the browser's button or a link pointing back, the
// arrival looks the same and has to be treated the same -- otherwise A -> B -> A -> B grows.
it('cuts back to a page already on the trail however it was reached', () => {
	const deep = at('/architecture/two', '/', '/architecture/one');
	const back = advance(deep, '/architecture/one', '/architecture/two');

	expect(back).toEqual(at('/architecture/one', '/'));
	expect(advance(back, '/', '/architecture/one')).toEqual(at('/'));
});

// A reload is indistinguishable from a fresh visit except by what the record claims, which is
// the whole reason it carries the page it belongs to.
it('survives a reload of the page it was recorded for', () => {
	const trail = at('/architecture/two', '/', '/architecture/one');

	expect(advance(trail, '/architecture/two')).toEqual(trail);
});

it('discards a trail that belongs to another page', () => {
	const trail = at('/architecture/two', '/', '/architecture/one');

	expect(advance(trail, '/architecture/three')).toEqual(at('/architecture/three'));
	expect(advance(trail, '/architecture/three', '/mirror/elsewhere')).toEqual(
		at('/architecture/three', '/mirror/elsewhere'),
	);
});

it('sends Back home when nothing has been walked', () => {
	expect(backTarget(undefined)).toBe('/');
	expect(backTarget(at('/architecture/one'))).toBe('/');
});

it('keeps the trail bounded rather than growing with every step', () => {
	let trail = advance(undefined, '/a0', '/');
	for (let step = 1; step < 20; step++) {
		trail = advance(trail, `/a${step}`, `/a${step - 1}`);
	}

	expect(trail.paths).toHaveLength(8);
	expect(backTarget(trail)).toBe('/a18');
});

it('ignores a stored value that is not a trail', () => {
	expect(readTrail(storage('not json'))).toBeUndefined();
	expect(readTrail(storage('{"at":"/","paths":[1,2]}'))).toBeUndefined();
	expect(readTrail(storage('{"at":"/","paths":["/x"]}'))).toEqual(at('/', '/x'));
});
