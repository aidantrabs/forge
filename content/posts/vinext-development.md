---
title: "hunting concurrency bugs in vinext"
description: "contributing to an open-source vite plugin that reimplements next.js - finding and fixing real async isolation bugs in the pages router"
date: 2026-03-13
tags: ["open-source", "typescript", "vite", "next.js", "asynclocalstorage"]
draft: false
---

i wanted to contribute to something bigger than my own projects. vinext caught my eye - it's cloudflare's experiment in reimplementing the entire next.js api surface on top of vite. almost every line was written by ai, and they're very open about that. the codebase is genuinely interesting to read through.

the issue i picked up was [#478](https://github.com/cloudflare/vinext/issues/478) - "phase 2/2: finalize als rollout with parity tests, cleanup, and docs". the previous pr (#450) had just landed a massive refactor that consolidated 5-6 separate `AsyncLocalStorage` instances into a single unified request context. my job was to prove it actually works under concurrent load, document the architecture, and clean up the leftovers.

## what is asynclocalstorage and why does it matter here

quick context if you haven't worked with this before. `AsyncLocalStorage` (als) is a node.js api that lets you store data that follows an async call chain - think of it like thread-local storage but for javascript's single-threaded async model. when request a comes in and kicks off some async work, and request b arrives before a finishes, als makes sure each request sees its own data.

in a framework like next.js (or vinext), every request needs its own headers, cookies, navigation state, router context, cache tags, etc. without als, concurrent requests on something like cloudflare workers would stomp on each other's state. request a's `<Head>` title shows up in request b's response. that kind of thing.

## the setup

vinext is a pnpm monorepo. the key commands:

```bash
pnpm test tests/some-file.test.ts   # targeted tests (always do this, not the full suite)
pnpm run typecheck                    # tsgo
pnpm run lint                         # oxlint
pnpm run fmt:check                    # oxfmt
```

they use conventional commits (`fix:`, `feat:`, `test:`, `docs:`), and every pr gets reviewed by an ai agent called bigbonk (claude opus with max thinking). their bias is towards merging, which is refreshing.

## writing the concurrency tests

the first thing i needed was fixture pages that expose request-scoped state in the html output. i created two pages in the test fixture:

**`concurrent-head.tsx`** - takes a `?id=N` query param via `getServerSideProps` and sets a `<title>` and `<meta>` tag with that id:

````tsx
export default function ConcurrentHeadPage({ reqId }: Props) {
  return (
    <div>
      <Head>
        <title>{`req-${reqId}`}</title>
        <meta name="req-id" content={reqId} />
      </Head>
      <h1 data-testid="req-id">{reqId}</h1>
    </div>
  );
}
````

**`concurrent-router.tsx`** - echoes back the ssr pathname and query from `getServerSideProps` plus `useRouter()`.

the test fires 15 concurrent requests at each page and verifies every response contains only its own data. if head state leaks between requests, request 0's response would have request 14's title.

small gotcha i hit along the way - `<title>req-{reqId}</title>` in jsx produces children as an array `["req-", "42"]`, not a single string. vinext's head shim only serializes string children for title tags, so the title rendered empty. switching to `<title>{`req-${reqId}`}</title>` produces a single string child and works fine. not a bug i introduced - it's pre-existing in the head shim - but it tripped me up for a bit.

## finding a real bug

the router isolation test passed immediately. the head isolation test did not.

```
expected 'req-13' to be 'req-0'
```

the body content was correct (request 0 showed `req-id=0`), but the `<title>` showed `req-13` - another request's head state had leaked into this response. this is exactly the kind of bug the issue asked me to verify.

## the root cause

this one took some digging. the architecture has a registration pattern - `head.ts` (the `next/head` shim) has module-level defaults for collecting ssr head children:

```typescript
let _ssrHeadChildren: React.ReactNode[] = [];
let _getSSRHeadChildren = (): React.ReactNode[] => _ssrHeadChildren;
```

and `head-state.ts` registers als-backed replacements:

```typescript
_registerHeadStateAccessors({
  getSSRHeadChildren(): React.ReactNode[] {
    return _getState().ssrHeadChildren; // reads from per-request ALS scope
  },
  // ...
});
```

the problem: vite's dev server has **separate module graphs** for different environments. the dev-server imports `head-state.ts` as a static import (node context), which registers the als accessors on the node context's copy of `head.ts`. but the `Head` react component runs during ssr rendering in **vite's ssr module graph** - a completely different module instance. that ssr instance of `head.ts` never had `_registerHeadStateAccessors` called on it, so it was still using the shared module-level `_ssrHeadChildren` array.

every concurrent request's `Head` component was pushing elements into the same array.

this is the kind of thing that doesn't surface in serial tests. you need real concurrent load to catch it.

## the fix

two lines:

```typescript
await server.ssrLoadModule("vinext/head-state");
await server.ssrLoadModule("vinext/router-state");
```

added right after the unified request context is created, before any rendering happens. this loads the state modules in vite's ssr module graph, which triggers the accessor registration on the correct module instance. the same pattern was already used for `vinext/i18n-state` - just nobody had done it for head and router state.

the prod server doesn't have this problem because everything gets compiled into one bundle where the imports resolve to the same module instance.

i also checked the other server files for parity (agents.md is very clear about this - if you touch one server file, check all four). the app router rsc entry doesn't use pages router head/router state, and the generated prod entry already has the correct imports. the bug was dev-only.

## prod concurrency tests

the prod build is a different beast. in dev, vite has separate module graphs for different environments (node vs ssr). in prod, everything gets compiled into a single bundle - so the module-level singleton problem that caused the head leak in dev doesn't exist.

but we still need to prove that `getServerSideProps` data is isolated between concurrent requests. the test builds the fixture to a temp directory, starts the prod server on a random port, and fires the same 15 concurrent requests.

setting this up was its own adventure. the `pages-basic` fixture has an `alias-test.tsx` page with a `@/components/heavy` import that breaks when building outside the original directory structure. filtered it out during the temp dir copy - it's not relevant to what we're testing.

interesting discovery: the prod server has some known limitations compared to dev. `<Head>` component children don't get injected into the html `<head>` section, and `useRouter().pathname` returns `/` instead of the actual route during ssr. these aren't concurrency bugs - they're consistent behavior regardless of load. so the prod tests focus on what matters: verifying `getServerSideProps` data and ssr props don't leak between requests.

all four tests pass - head isolation in dev, router isolation in dev, data isolation in prod (both pages).

## things i learned

- vite's multi-environment module graphs mean you can have the same module loaded multiple times with completely different state. this is by design for rsc/ssr/client separation, but it creates subtle bugs when server-side code assumes module singletons.
- `AsyncLocalStorage` works great for request isolation, but only if the als-backed accessors are registered in every module instance that needs them.
- jsx `<title>text-{variable}</title>` produces an array of children, not a string. `<title>{`text-${variable}`}</title>` produces a single string. matters when the consumer only handles string children.
- writing concurrency tests that actually catch isolation bugs requires real parallel `Promise.all` with enough requests to trigger interleaving. serial tests will never catch these.
