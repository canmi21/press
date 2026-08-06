CREATE TABLE `likes` (
	`ip` text PRIMARY KEY NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `newsletter_subscriptions` (
	`email` text PRIMARY KEY NOT NULL,
	`cancel_token_hash` text NOT NULL,
	`ip` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `newsletter_subscriptions_cancel_token_hash_unique` ON `newsletter_subscriptions` (`cancel_token_hash`);--> statement-breakpoint
CREATE INDEX `newsletter_subscriptions_ip_idx` ON `newsletter_subscriptions` (`ip`);