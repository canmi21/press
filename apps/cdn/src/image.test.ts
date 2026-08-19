import { imageKey } from '@canmi/store';
import { describe, expect, it } from 'vitest';
import image from './image';

const CID = '44b6081deaf0242ca3bf83d62a3b6c95';

/**
 * A bucket holding exactly the keys named and nothing else.
 *
 * `get` is all the paths under test reach: deciding whether to redirect asks only whether the id
 * resolves to a stored object, never what is in it.
 */
function bucketWith(keys: string[]) {
	return {
		PUBLIC: {
			get: async (key: string) =>
				keys.includes(key)
					? { body: new Response('stored').body, httpMetadata: {}, httpEtag: '"e"' }
					: null,
		},
	} as never;
}

describe('a request spelling jpeg as jpg', () => {
	it('is redirected permanently to the canonical spelling', async () => {
		const response = await image.request(`/${CID}.jpg`, {}, bucketWith([imageKey(CID, 'avif')]));

		expect(response.status).toBe(301);
		expect(response.headers.get('Location')).toBe(`/image/${CID}.jpeg`);
	});

	/**
	 * Why the id is looked up before the redirect is issued rather than after. Sending a client to
	 * an address that will answer 404 makes it spend two round trips to learn nothing is there.
	 */
	it('is answered 404 directly when nothing is stored under the id', async () => {
		const response = await image.request(`/${CID}.jpg`, {}, bucketWith([]));

		expect(response.status).toBe(404);
		expect(response.headers.get('Location')).toBeNull();
	});

	/** A flat-colour asset is stored as PNG, so the lookup has to try every stored format. */
	it('finds the id whichever format it was published as', async () => {
		const response = await image.request(`/${CID}.jpg`, {}, bucketWith([imageKey(CID, 'png')]));

		expect(response.status).toBe(301);
	});
});

describe('a request naming something that is not an id', () => {
	it('is rejected before the bucket is touched', async () => {
		const response = await image.request('/not-a-content-id.jpg', {}, bucketWith([]));

		expect(response.status).toBe(400);
	});
});
