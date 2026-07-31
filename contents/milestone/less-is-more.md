---
title: Less is more
subtitle: 从 Next.js 13 鸽到 Next.js 16
description:
  笔者用两年把博客拖成反向代理、自研框架与 vfs 的副产物，在 AI 焦虑里学会放手、All in Cloudflare
  上线了第一篇。一段从 Next.js 13 鸽到 16 的过度工程自白。
lang: zh
created: 2026-03-24T08:49:57.449Z
lastmod: 2026-04-01T10:36:51.601Z
views: 2540
---

这个博客，我想了两年

说"想"其实不太准确，更像是一个执念。在早期我还不会写前后端，不会写嵌入式代码的时候就有了，当时还在玩电子，偶然间看到了 [@ushio](https://github.com/sakura-ushio) 的 B站视频 和 网站，窝觉得很好玩我也要。后来花了小半年时间，从零学会了写 C 和一点点 JS CSS 用 Hexo 搭了一个静态博客部署在 Github Pages，也算是有了自己的小天地。

::linkcard{src="5a1baa3c26908b684f8d356a6d352f3c5c4650f9fe03957913f65ab61f1f5226.png" url="https://hexo.io" title="Hexo: A fast, simple & powerful blog framework."}

::linkcard{src="3f7dcc6f50caafa3667d680a0a5592ae6ca14440216c72b138606ae5465eac17.png" url="https://sakura-ushio.icu" title="ushio" tone="dark" favicon="https://avatars.githubusercontent.com/u/33977278?v=4"}

再后来，看到了 [@Innei](https://twitter.com/__oQuery) 的小站，设计很精致、功能也很全，当时的我看了一眼我就想要，甚至还赞助支持了，虽然后面没用上但是这钱也花的不怨 因为我摸到了 React 源码，就和我最初看到 C 一样，不过就是代码嘛，有什么大不了的，总有一天我要自己写一个。

::linkcard{src="ac50a78a098e6cea66e73c23bf15e1425680749cd19c1cdcfa97fc15a8e790b7.png" url="https://innei.in" title="静かな森" favicon="https://raw.githubusercontent.com/innei/shiro/90ef7b8576449e7fcd4dafd733395091b55dc357/apps/web/public/innei-dark.svg"}

那是 2024 年，~~然后 24, 25, 26年 到了现在~~

从那之后就停不下来了

## 失控 {#out-of-control}

一开始只是想给 [@colmugx](https://x.com/colmugx) Hexo 主题 [Nlvi](https://github.com/colmugx/hexo-theme-nlvi) 加个深浅色切换，用 Dark Reader 的 JS 糊了一个，丑，不满意。然后学 CSS @media，再到 JS SPA 切换，TailwindCSS .dark class, 再到 SSR next-theme，最后手搓 Cookie + Head Sync Inline 脚本解决 FOUC 闪烁，我折腾了四五种方案。

::linkcard{src="05284704146b27f0483f1b9bc3324db1666ddad3ce1d39448e5e368e90d06289.png" url="https://github.com/colmugx/hexo-theme-nlvi" title="🎨 A simple theme for hexo." tone="dark"}

但这只是冰山一角。看到 React 之后，我从石器时代的 Hexo 和 Vanilla JS 一路走到了 Next.js 13 Page Router，又拐去看 Vue、研究 SPA 路由、理解 index.html fallback，项目新建了无数次。买了服务器，折腾 Docker、Nginx、NAS，嫌 1Panel 丑手写了 Canopy 面板 研究 NAS 存储阵列写了 RFS，搞 SSL 被国内环境折磨的 acme.sh 抽风逼到用 Rust 写了个反向代理：Vane & Lazyacme，本来只是一个晚上三小时写出来的八百行垃圾，结果群友一怂恿，写了四个月，HTTP/3、Zero-Copy、Layer 模型、Flow 引擎全往里塞。后面还有 Seam，一个全栈框架，也是天坑，以后再说吧。

## 过度工程 {#over-engineering}

然后 AI 时代来了，焦虑又翻了一倍

最讽刺的是，这些东西本来都应该是有了博客之后，边记录边做的。结果博客鸽了又鸽，想法越来越多，却没有一个地方能记录它们。一边是内心在谴责；说好了 Vane 要做完，就不能，停另一边是越做越远的无力感。我一直都记得我要做什么，但就是做不出来。仔细想想我其实已经很久都没有坐下来写过什么东西了，现在回头看，这一切都指向同一个东西——过度工程。

我当时其实并不不觉得自己在跑偏。Vane？反正博客以后也要部署，总得有个反向代理吧。Seam？反正迟早要写全栈，不如先把我理想中的框架搞好。每一步都有理由，每一步都是"以后用得上"。但问题是，"以后"永远不会来，因为我从未开始...

## 失败 {#failure}

做博客实际上不需要一个反向代理，不需要一个自研框架。而当时的我把它们当成了前置条件，作为了那个永远 Blocking 但是却合理的理由，但它们从来都不是。真正的前置条件只有一个：坐下来，把第一篇文章写出来。

其实博客不是没动过，恰恰相反，我做了三次

::linkcard{src="4230097ab8d4c19e4f46ae6fd03c7f45e502bf7c30b696c2e459a91f107760a2.png" url="https://vercel.com" title="Vercel"}

第一次还是 Next.js 13 Page Router 那会。那时候我蒙懂 SPA 是什么，SSR、SSG、ISR 这些概念一个都不清楚，但看别人说 SSR 好，那我也要用。结果写着写着发现自己根本不理解这些架构在解决什么问题，做不下去了。后来呢，转 Vue 2 纯 SPA，为什么选 Vue？说来惭愧，觉得名字好听，反正也新，试试呗。也没撑多久。第三次回到 React，从单 JSX 开始，学了 TypeScript，会了 TSX，理解了 CJS 和 ESM 的区别，爱上了 Reactive，用上了 Lucide、Framer Motion、Radix UI 这些日后离不开的库——这次终于像点样子了。但我还是失败了，因为本质上我只是在模仿，别人用什么我就拿来用，看起来像那么回事，实际上根本不理解为什么要这样做。再加上 AI 的幻觉加持，很多东西都能跑起来一个 MVP，但 Scale Up 的时候全是坑。

::linkcard{src="746a62eba40455fb59823c70dbc55bcd3ae8e43bc29151a2ceb2a384dd8defc4.png" url="https://remix.run" title="Remix" tone="dark"}

后来又学了 React(~~Remix~~) Router Loader、Next.js 15 App Router、RSC，但这时候日常已经很忙了，没什么时间写代码，倒也没机会第四次失败。现在想起来，我的网站从 Next.js 13 写到了 Next.js 15 ，前脚刚迁移完成，还没上线，Next.js 16 beta-1 就出了。

## 放手吧 {#letting-go}

说"想开了"其实不准确，是我累了。

![](8531b6a61c000d330539e0dc488cc158a3b37bab2f8b31c9b6d242d6717f4bb1.png)

AI 时代的节奏太快了。模型一个接一个迭代，新技术一轮接一轮冒出来，我越来越焦虑，好像停下来就要被超越。最疯的时候一天写 16个 小时代码，Token 一天就能烧 $1.9k USD ~~(大概2月的时候吧)~~，人快废了。但让这一切停下来的不是什么顿悟，就是单纯的累了。在越来越紧的节奏下，最后一根稻草压下来，我反而释然了，想开了，摆烂了；这样，也挺好的。

我开始放手。放弃曾经执着的东西，学会点到为止，明天的事明天再说。Vane 没做完？先放着。Seam 还差很远？以后再来。博客不完美？先上线。开始完全 Vibe Coding，没问题就不管，有问题再说。

奇怪的是，放手之后质量并没有变差。我想这大概是因为之前那些"古法"编程的日子没有白费；早在 GPT-2 时代我就开始用 AI 了，但那时候我还在老老实实地写每一行代码、理解每一个概念。那段经历给了我底子，与其说后来 Vibe Coding 不如说是 Context Coding，该有的判断力还在，并没有因为放手就出什么低级错误或者说是事故。

是之前所有的折腾，给了我现在放手的资本

## 原点 {#back-to-origin}

最终的方案反而很简单：All in Cloudflare。R2 对象存储、D1 Edge 数据库、KV 做缓存、Worker 跑 SSR，全部 Serverless，没有服务器。

::linkcard{src="cda5a4010f101ebeecbf8283756e844964b5554908f0dcab91f1dcb3157324dc.png" url="https://workers.cloudflare.com" title="Cloudflare Worker" tone="light"}

这个答案讽刺吗？太讽刺了。因为在到达这个"简单"的终点之前，我走了一整圈弯路。曾经计划自建，所以要管服务器、折腾 Docker、管容器，于是写了 Canopy 做管理面板，写了 Lazycert 管证书，写了 Vane 做反向代理，写了 Twig 做资源监控埋点。一堆项目，就为了伺候一台垃圾服务器。

存储也是一样。我嫌对象存储贵，想用自己服务器的硬盘，于是写了 RFS 一种 vfs 为了原子去重合并单文件，甚至复活了 80-90年代 的注册表理念，最后还通过 FUSE 挂载起来了。结果呢，花了更多时间造了一个不那么好的轮子，服务器还空跑了一年，完全就是彻底的亏本生意。

::linkcard{src="48fdfa653b7df54e55225d9d9eaa4d25903b0a05a6cc617128e1f456e56fa316.png" url="https://vercel.com" title="Vercel"}

最后绕了一大圈，还是回到了 Serverless，Vercel 的概念好，但我不喜欢 Next.js；而且 Vercel 的魔法太多了，还有就是那个中间件设计的提前编译跑 Edge 上，最后还是割裂，甚至出了几个 CVE，还不如要么全部源站一起。而 Cloudflare Worker 就很好，达到了另外一种极端，小于 10MB 的产物编译成 WASM，Edge 上能跑真正的业务逻辑，这才是我想要的。除了费钱以外没什么缺点——但说起来也不算缺点，毕竟省下的时间就是钱。

## Less is more {#less-is-more}

这句话我听过无数遍，但花了两年才真正理解它的含义。

回归工程的本质：「Make it function, make it correct, then make it exceptional.」我终于走上了正轨，让它 functional 了。现在的博客简陋吗？简陋。不完整吗？确实不完整。但总比没有好。如果你连上线都没有，一直在做那个"也许需要"的东西，一直在追求看不见的完美，那最后什么都拿不出来。这不是妥协，这是一种工程上的取舍，反正我是想开了。所以我想开了。不再为一个细节执着，反而走出了门。手机相册里已经很久没有户外的照片了，随手拍了一下路边的——已经很久没出门了嘛 (

![](b0e8e8202342dc9e84ca97035ede29b36dd268520dca3dff1e764df87d20ebac.png)

回头看之前走的弯路，后悔吗？不后悔。有些事你不自己试试，也许永远想不明白。更何况在 AI 时代，想走我当年的老路已经基本没有机会了，太多工作流已经被改变。另外这个项目是 [Taki](https://github.com/canmi21/taki)，100% AI Coding，但这不意味着不能长期维护，不意味着没有人味。其实有没有 AI 不重要，再说了，我喜欢的也不是写代码本身，而是 Building 一个东西...
