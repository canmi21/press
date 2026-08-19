import { parseSlot, slotPort, type AppName } from '../src/index.ts';

/**
 * The `wrangler dev` port flags for one worker in the current workspace slot.
 *
 * wrangler.jsonc pins each worker's base port, which is what a bare `pnpm run dev` gets. The
 * mise dev tasks append these flags so an overlay workspace binds its own slot instead of
 * colliding with the base -- one place computes the number, from the same table the site and
 * the URL map read. The inspector rides one above the port, as the pinned pair does.
 * See spec/toolchain.md.
 */
const WORKERS: readonly AppName[] = ['api', 'cdn'];

const app = process.argv[2];
if (!app || !WORKERS.includes(app as AppName)) {
	console.error(`usage: wrangler-ports.ts <${WORKERS.join('|')}>`);
	process.exit(2);
}
const port = slotPort(app as AppName, parseSlot(process.env.WORKSPACE_SLOT));
console.log(`--port ${port} --inspector-port ${port + 1}`);
