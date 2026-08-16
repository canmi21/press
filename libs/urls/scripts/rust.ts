import { writeFileSync } from 'node:fs';
import { rustUrlMap } from '../src/rust.ts';

writeFileSync(new URL('../../../apps/cms/src/urls.rs', import.meta.url), rustUrlMap());
