import { index, integer, sqliteTable, text } from 'drizzle-orm/sqlite-core';

export const newsletterSubscriptions = sqliteTable(
	'newsletter_subscriptions',
	{
		email: text('email').primaryKey(),
		cancelTokenHash: text('cancel_token_hash').notNull().unique(),
		ip: text('ip').notNull(),
		createdAt: integer('created_at').notNull(),
	},
	(table) => [index('newsletter_subscriptions_ip_idx').on(table.ip)],
);

export const likes = sqliteTable('likes', {
	ip: text('ip').primaryKey(),
	createdAt: integer('created_at').notNull(),
});

// One row per article, created on its first read. The slug is the site's own article path,
// so a count follows the article rather than any one of its nine language views. Nothing here
// identifies a reader: this is a counter, and the row it lands in is the whole record.
export const articleReads = sqliteTable('article_reads', {
	slug: text('slug').primaryKey(),
	count: integer('count').notNull(),
});
