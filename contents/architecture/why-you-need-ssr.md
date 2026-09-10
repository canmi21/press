---
title: Untitled
subtitle: A placeholder, kept only until the real piece is written
description: A scaffold article with nothing to say yet. It exists so the shape of a new
  entry under architecture can be seen rendering before any of its words are decided.
lang: en
draft: true
created: 2026-09-09T22:00:34Z
lastmod: 2026-09-09T22:00:34Z
---

## The Scope

When I first encountered SSR I took it for a kind of novelty. But as I kept digging into its internal data flow and logic, I realized there was a problem that most modern frameworks, and most people using them, overlook. That problem is the subject of this post, and so is the answer I want to propose for it: CTR, compile-time rendering.

I am not arguing that SSR is bad. It solves real problems, and for those problems it is the right answer. But do you really need it?

This post has two goals. First, I want to show that CTR is workable rather than merely appealing. Second, I want to establish where the boundary falls, which parts of a page genuinely require arbitrary code execution at request time, and which parts only appear to. 

I care far more about the second.

## The Detour

**Why the detour happens?**

SSR is usually discussed as a single feature, but it bundles three separable capabilities: the page contains content when it arrives, the data in it is fresh for this request, and the server can run arbitrary code to decide what the page looks like.

Most applications want the first two. The third is what you pay for, and for most pages it is not what you came for. It took me about half a year to see this. I came from backend and low-level work, so performance is where my attention goes by default, and after a couple of months in frontend I kept returning to the same question: why is SSR this slow? 

For a long time I looked for ways to make it faster. Eventually I noticed something about the question itself. I wanted SSR to be faster only because I was using two of its three capabilities. I had been paying, on every request, for the ability to run arbitrary code on the server — and I almost never used it. 

The reason nearly everyone ends up paying for it anyway is that no other option provided the first two together.
