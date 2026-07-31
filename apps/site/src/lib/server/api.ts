import { Hono } from 'hono';

export const app = new Hono().basePath('/api');

app.get('/', (c) => c.text('hello from hono'));
