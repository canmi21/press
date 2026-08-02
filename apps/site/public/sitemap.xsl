<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
	<xsl:output method="html" indent="no" encoding="UTF-8" doctype-system="about:legacy-compat"/>
	<xsl:strip-space elements="*"/>

	<xsl:template match="/">
		<html lang="en">
			<head>
				<meta charset="utf-8"/>
				<meta name="viewport" content="width=device-width, initial-scale=1"/>
				<title>
					<xsl:choose>
						<xsl:when test="name(*) = 'urlset'">Sitemap</xsl:when>
						<xsl:when test="name(*) = 'feed'">Feed</xsl:when>
						<xsl:otherwise><xsl:value-of select="name(*)"/></xsl:otherwise>
					</xsl:choose>
				</title>
				<style>
					:root {
						color-scheme: light dark;
						--bg: #ffffff;
						--fg: #303942;
						--tag: #881280;
						--attr: #994500;
						--value: #1a1aa6;
						--punct: #303942;
					}
					@media (prefers-color-scheme: dark) {
						:root:not(.light) {
							--bg: #171717;
							--fg: #d4d4d4;
							--tag: #c792ea;
							--attr: #f07178;
							--value: #c3e88d;
							--punct: #89ddff;
						}
					}
					.dark {
						--bg: #171717;
						--fg: #d4d4d4;
						--tag: #c792ea;
						--attr: #f07178;
						--value: #c3e88d;
						--punct: #89ddff;
					}
					html, body {
						margin: 0;
						padding: 0;
						background: var(--bg);
						color: var(--fg);
					}
					.xml-code {
						margin: 0;
						padding: 0.75rem 1.5rem 1.5rem 0.75rem;
						font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
						font-size: 0.8125rem;
						line-height: 1.5;
						white-space: pre-wrap;
						word-break: break-all;
						tab-size: 2;
						-moz-tab-size: 2;
					}
					.t { color: var(--tag); }
					.a { color: var(--attr); }
					.v { color: var(--value); }
					.p { color: var(--punct); }
				</style>
				<script>
					(function () {
						var m = document.cookie.match(/(?:^|; )theme=(light|dark)/);
						var t = m ? m[1] : (window.matchMedia("(prefers-color-scheme:dark)").matches ? "dark" : "light");
						if (t === "dark") document.documentElement.classList.add("dark");
					})();
				</script>
			</head>
			<body>
				<pre class="xml-code"><span class="p">&lt;?</span><span class="t">xml</span><xsl:text> </xsl:text><span class="a">version</span><span class="p">="</span><span class="v">1.0</span><span class="p">"</span><xsl:text> </xsl:text><span class="a">encoding</span><span class="p">="</span><span class="v">UTF-8</span><span class="p">"?&gt;</span><xsl:text>&#10;</xsl:text><span class="p">&lt;?</span><span class="t">xml-stylesheet</span><xsl:text> </xsl:text><span class="a">type</span><span class="p">="</span><span class="v">text/xsl</span><span class="p">"</span><xsl:text> </xsl:text><span class="a">href</span><span class="p">="</span><span class="v">/xml.xsl</span><span class="p">"?&gt;</span><xsl:text>&#10;</xsl:text><xsl:apply-templates select="*"/></pre>
			</body>
		</html>
	</xsl:template>

	<xsl:template match="*">
		<xsl:param name="depth" select="0"/>
		<xsl:call-template name="indent">
			<xsl:with-param name="depth" select="$depth"/>
		</xsl:call-template>
		<span class="p">&lt;</span><span class="t"><xsl:value-of select="name()"/></span>
		<xsl:if test="$depth = 0">
			<xsl:for-each select="namespace::*[name() != 'xml']">
				<xsl:text> </xsl:text>
				<span class="a">
					<xsl:choose>
						<xsl:when test="name() = ''">xmlns</xsl:when>
						<xsl:otherwise>xmlns:<xsl:value-of select="name()"/></xsl:otherwise>
					</xsl:choose>
				</span>
				<span class="p">="</span>
				<span class="v"><xsl:value-of select="."/></span>
				<span class="p">"</span>
			</xsl:for-each>
		</xsl:if>
		<xsl:for-each select="@*">
			<xsl:text> </xsl:text>
			<span class="a"><xsl:value-of select="name()"/></span>
			<span class="p">="</span>
			<span class="v"><xsl:value-of select="."/></span>
			<span class="p">"</span>
		</xsl:for-each>
		<xsl:choose>
			<xsl:when test="count(*) = 0 and normalize-space(.) = ''">
				<span class="p">/&gt;</span>
				<xsl:text>&#10;</xsl:text>
			</xsl:when>
			<xsl:when test="count(*) = 0">
				<span class="p">&gt;</span><xsl:value-of select="."/><span class="p">&lt;/</span><span class="t"><xsl:value-of select="name()"/></span><span class="p">&gt;</span>
				<xsl:text>&#10;</xsl:text>
			</xsl:when>
			<xsl:otherwise>
				<span class="p">&gt;</span>
				<xsl:text>&#10;</xsl:text>
				<xsl:apply-templates select="*">
					<xsl:with-param name="depth" select="$depth + 1"/>
				</xsl:apply-templates>
				<xsl:call-template name="indent">
					<xsl:with-param name="depth" select="$depth"/>
				</xsl:call-template>
				<span class="p">&lt;/</span><span class="t"><xsl:value-of select="name()"/></span><span class="p">&gt;</span>
				<xsl:text>&#10;</xsl:text>
			</xsl:otherwise>
		</xsl:choose>
	</xsl:template>

	<xsl:template name="indent">
		<xsl:param name="depth"/>
		<xsl:if test="$depth &gt; 0">
			<xsl:text>&#9;</xsl:text>
			<xsl:call-template name="indent">
				<xsl:with-param name="depth" select="$depth - 1"/>
			</xsl:call-template>
		</xsl:if>
	</xsl:template>
</xsl:stylesheet>
