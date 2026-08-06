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
