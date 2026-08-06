import type { Bindings as StoreBindings } from '@canmi/store';

// Env is generated from Wrangler. The store package imports its runtime types so it can coexist
// with DOM types; replace just those two structurally equivalent generated fields at that edge.
export type Bindings = Omit<Env, 'ASSETS' | 'PUBLIC'> & StoreBindings;
