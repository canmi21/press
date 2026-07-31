<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet
	version="1.0"
	xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
	xmlns:sitemap="http://www.sitemaps.org/schemas/sitemap/0.9"
>
	<xsl:output method="html" encoding="UTF-8" indent="yes" />

	<xsl:template match="/">
		<html lang="en">
			<head>
				<meta charset="UTF-8" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />
				<title>Sitemap</title>
				<style>
					:root {
						color-scheme: light dark;
					}
					body {
						font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
						max-width: 48rem;
						margin: 3rem auto;
						padding: 0 1.5rem;
						line-height: 1.5;
					}
					h1 {
						font-size: 1.5rem;
						margin: 0 0 0.25rem;
					}
					p.meta {
						color: light-dark(#666, #999);
						margin: 0 0 2rem;
						font-size: 0.875rem;
					}
					table {
						width: 100%;
						border-collapse: collapse;
						font-size: 0.9rem;
					}
					th,
					td {
						text-align: left;
						padding: 0.5rem 0.75rem;
						border-bottom: 1px solid light-dark(#eee, #2a2a2a);
					}
					th {
						font-weight: 500;
						color: light-dark(#666, #aaa);
					}
					a {
						color: light-dark(#0066cc, #6ea8ff);
						text-decoration: none;
						word-break: break-all;
					}
					a:hover {
						text-decoration: underline;
					}
				</style>
			</head>
			<body>
				<h1>Sitemap</h1>
				<p class="meta">
					<xsl:value-of select="count(sitemap:urlset/sitemap:url)" /> URLs
				</p>
				<table>
					<thead>
						<tr>
							<th>URL</th>
							<th>Priority</th>
							<th>Change Frequency</th>
						</tr>
					</thead>
					<tbody>
						<xsl:for-each select="sitemap:urlset/sitemap:url">
							<tr>
								<td>
									<a href="{sitemap:loc}">
										<xsl:value-of select="sitemap:loc" />
									</a>
								</td>
								<td><xsl:value-of select="sitemap:priority" /></td>
								<td><xsl:value-of select="sitemap:changefreq" /></td>
							</tr>
						</xsl:for-each>
					</tbody>
				</table>
			</body>
		</html>
	</xsl:template>
</xsl:stylesheet>
