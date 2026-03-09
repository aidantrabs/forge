---
title: "designing a feature flag control plane"
description: "the research and architecture behind building a self-hosted feature flag system from scratch"
date: 2026-03-09
tags: [java, system-design]
draft: true
---

i've been thinking about feature flags a lot lately. not the "just use an if statement" kind - the kind where you need to roll out a payment flow to 5% of users in canada on the premium plan, watch it for a week, then crank it to 50% without touching a deploy pipeline. the kind where someone on your team can flip a kill switch at 2am when something goes sideways.

so i'm building [switchboard](https://github.com/aidantrabs/switchboard) - a feature flag control plane. this post is the research and design thinking before i write a single line of code.

## why build one

the obvious question. launchdarkly exists. unleash exists. flagsmith exists. why build another one?

partly because i want to understand the problem space deeply - the same way you don't really understand databases until you've tried to write one. partly because most feature flag systems are either too simple (a json file you check into your repo and pray) or too complex (a whole platform with pricing tiers and a sales team). i want to find the middle ground: something an engineering team could self-host, understand completely, and extend when they need to.

the target is internal platform teams. the kind of team that runs a handful of microservices and wants centralized flag management without sending their evaluation data to a third-party saas. the kind of team where "we need to be able to run this air-gapped" is a real requirement and not just a checkbox on a compliance form.

## the evaluation problem

i started by researching how flag evaluation actually works under the hood. it sounds simple - "is this flag on?" - until you start layering requirements.

here's the evaluation order i've landed on after reading through how launchdarkly, unleash, and openfeature approach it:

1. if the flag is globally disabled, return the default variant. easy.
2. evaluate targeting rules in priority order. each rule has conditions (AND logic) and a served variant. first match wins.
3. if no rules match but there's a rollout percentage, hash the user into a deterministic bucket and check if they're under the threshold.
4. if nothing hits, return the default variant.

the rollout hashing is the part that tripped me up. you can't use `Math.random()` - the same user needs to get the same result every time for the same flag. otherwise you get someone who sees the new checkout flow, refreshes the page, and gets the old one. that's worse than not having flags at all.

the standard approach is consistent hashing. take the flag key + user id, run it through something like murmurhash3, mod 100, check if the result lands under your rollout percentage:

```
hash("new-checkout" + "user-123") mod 100 = 37
rollout = 50%
37 < 50 → user gets the "on" variant
```

same inputs, same output, every time. and here's the property that took me a minute to appreciate: if you change the rollout from 50% to 60%, user-123 still gets "on" - you're only *adding* new users, never removing existing ones. that monotonicity matters a lot more than i initially realized.

## hexagonal architecture (or: am i overengineering this?)

i'm going with spring boot for the server, but i want to try something i've been reading about for a while - hexagonal architecture. the idea is that your business logic lives in a core that knows nothing about the outside world. no spring annotations, no jpa, no kafka. just plain java.

```
domain/          ← pure java. no framework imports. ever.
application/
  port/
    input/       ← interfaces: what the system can do
    output/      ← interfaces: what the system needs
  service/       ← use case implementations
adapter/
  input/rest/    ← spring controllers
  output/
    persistence/ ← jpa (implements output ports)
    messaging/   ← kafka (implements output ports)
    cache/       ← redis (implements output ports)
```

the domain says "i need to save a flag" by defining an interface. a jpa adapter implements that interface. the domain never knows jpa exists. want to swap postgres for something else? write a new adapter, domain doesn't change.

i'll be honest - part of me thinks this is overkill for a project i'm building from scratch. "just put `@Entity` on your domain class, it's fine." but i keep reading post-mortems from teams that started that way and regretted it two years later when their domain was welded to hibernate. i'd rather pay the cost of indirection now while the codebase is small and i can actually understand the boundaries.

the plan is to enforce this with archunit tests - if someone (me, inevitably) accidentally imports a spring annotation in the domain layer, the build fails. trust but verify, especially when you don't trust yourself.

## real-time updates: kafka or bust (or maybe not)

when someone toggles a flag, every service consuming that flag needs to know. the naive approach is polling - every sdk hits the server every few seconds asking "anything change?" this works, it's simple, but it's wasteful and adds latency.

after looking at how other systems handle this, i'm planning to use kafka:

1. flag gets toggled → database updated
2. application service publishes a change event
3. kafka carries it to a topic per project/environment
4. sdks consuming that topic update their local cache immediately

but here's my concern: kafka is heavy. for a small team just trying out feature flags, "also run kafka" is a tough ask. so the sdk needs a fallback - polling on a configurable interval if kafka isn't available. and if the server itself is unreachable, use the last known cached state. graceful degradation at every level.

this is the part of the design i'm least confident about. distributed cache invalidation is one of those problems that sounds straightforward and then eats your weekend. but the alternative - a network round-trip for every flag evaluation in a hot code path - isn't acceptable.

## the sdk layer cake

i want the sdk to work in three modes, layered on top of each other:

**pure java sdk**: zero spring dependencies. construct a client with a builder, pass in your api url, call `isEnabled("flag-key", context)`. works in any jvm application - spring, dropwizard, plain old `public static void main`.

**spring boot starter**: wraps the sdk with auto-configuration. one dependency in your `build.gradle.kts`, two lines in `application.yml`, and you get a wired-up client bean, health indicators, metrics, the works. zero boilerplate.

**openfeature provider**: for teams that don't want to couple to a proprietary api. [openfeature](https://openfeature.dev/) is an emerging standard for feature flag evaluation - you code against the standard interface, swap providers behind it. switchboard becomes just another provider you can plug in or rip out.

and then there's local mode. this one i feel strongly about. for development and testing, the sdk should load flags from a json file:

```json
{
    "flags": {
        "new-checkout": { "enabled": true, "variant": "on" },
        "dark-mode": { "enabled": false, "variant": "off" }
    }
}
```

check it into your repo, use it in tests, run in ci with no external dependencies. if your feature flag system requires a running server to run unit tests, something has gone wrong.

## the data model question

this took me a few iterations on paper. the key realization: a flag's *definition* is project-scoped, but its *state* is per-environment.

"new-checkout" exists once as a concept - it has a key, a name, some variants. but it can be enabled in dev, 50% rolled out in staging, and disabled in production, each with completely different targeting rules. this is how teams actually work. you don't want a flag that's either globally on or globally off everywhere.

so the model splits into `FeatureFlag` (the definition) and `FlagEnvironmentConfig` (the per-environment state). targeting rules hang off the environment config, not the flag itself. this felt weird at first but the more i thought about it the more it made sense - you target differently in dev vs production.

i'm also planning for four flag types: **release** (ship a feature incrementally), **experiment** (a/b testing), **operational** (circuit breakers, maintenance mode), and **permission** (entitlement gating). they all evaluate the same way mechanically, but the type gives you metadata for lifecycle management - release flags should eventually be cleaned up, operational flags might live forever.

## four interfaces, one source of truth

the system needs to be operable through:

- **rest api**: the source of truth. write endpoints for management, read endpoints for sdks. separated so they could theoretically scale independently.
- **java sdk**: how services consume flags. evaluates locally from cache, syncs in the background.
- **cli**: for terminal-first workflows. `switchboard flags toggle new-checkout --env production`. json output for scripting.
- **dashboard**: a react spa for visual management. tanstack router, tanstack query, tanstack table, tailwind. it's a pure client-side app that talks to the rest api.

the dashboard being optional is deliberate. the api and cli are primary. if your team lives in the terminal, you never need to open a browser. the dashboard is there for the people who want to see a rollout slider and a toggle switch.

for the dashboard stack specifically - i'm going heavy on tanstack. router gives type-safe routing with inferred params (no manual type assertions). query handles all the server state with caching and background refetching. table gives headless primitives for the flag lists and audit logs. it's a lot of one ecosystem but they're designed to work together and it avoids the usual glue code.

## things i haven't figured out yet

**stale flag detection.** flags accumulate. teams create them for a release, ship it, forget to clean up. i want to surface warnings when flags haven't been modified in a while, but the ux of "hey this flag might be dead" without being annoying is an unsolved problem in my head.

**audit log growth.** every write operation should produce an audit entry with before/after state as json. great for debugging, but the table grows unboundedly. partitioning by time and project is the obvious answer but i haven't thought through the query patterns enough yet.

**the kafka question.** i keep going back and forth. kafka gives me real-time propagation but it's a heavy dependency. server-sent events would be simpler for small deployments. maybe i support both and let teams pick. or maybe i start with polling and add kafka later. this is the kind of decision that's hard to reverse so i want to get it right.

**how small can each commit actually be?** i'm planning to build this in tiny increments - domain model first, then ports, then adapters, then sdk, then cli, then dashboard. each step should be a handful of files. i've never actually tried to be this disciplined about it on a project this size. we'll see if i can stick to it.

## the end goal

clone the repo, run `docker compose up`, and have a fully working feature flag platform - server, dashboard, database, cache, message broker, and a demo service showing flags being evaluated in real time. under two minutes from git clone to toggling your first flag.

that's the bar. if it takes longer than that to evaluate switchboard, the developer experience has failed.

the source code is at [github.com/aidantrabs/switchboard](https://github.com/aidantrabs/switchboard).
