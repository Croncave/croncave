# Croncave: pricing

Tiers, usage billing, caps and the guardrails on the free tier. **This file is
the source of truth**, consolidated here on 2026-09-30.

## Summary

Croncave charges a monthly subscription in one of three tiers (Free, Plus and Pro), plus usage. Tiers decide what you're allowed to do. Usage is what you actually consumed, billed at our providers' own rates. This doc proposes the tiers, the usage rates and the guardrails. Every number marked \[proposed\] is a starting point to test with the alpha founders, not a final price. It builds on the [product definition](product.md) and the [technical architecture](architecture.md).

**Principles**

1. **Subscriptions pay for Croncave. Usage pays for providers.** Our margin lives mainly in the subscription, so heavy users never subsidise light ones.
2. **AI usage at exactly cost.** Built-in AI features are billed at exactly what the model provider charges us, with no markup.
3. **Everything else at cost plus a small percentage,** such as compute and storage, where the provider's terms allow it.
4. **No surprise bills.** Every account has a spending cap, set before anything runs, and usage is visible live.
5. **Plain units.** Users see "hours awake" and "GB stored," not vCPU-seconds.

## What we pay for

An awake workspace costs from about 3 cents an hour for a light machine to about 34 cents an hour for a powerful one. A sleeping workspace costs only its disk, at 15 cents per GB a month. Built-in AI features cost $1–$10 per million tokens depending on the Claude model.

| Cost | Unit price | Source |
| --- | --- | --- |
| Light workspace (shared-cpu-4x, 2 GB) | $0.0284 per hour awake | [Fly pricing](https://docs.fly.io/about/pricing) |
| Standard workspace (performance-2x, 4 GB) | $0.1721 per hour awake | [Fly pricing](https://docs.fly.io/about/pricing) |
| Power workspace (performance-4x, 8 GB) | $0.3443 per hour awake | [Fly pricing](https://docs.fly.io/about/pricing) |
| Workspace disk (volume) | $0.15 per GB a month, awake or asleep | [Fly pricing](https://docs.fly.io/about/pricing) |
| Stopped machine's system disk | $0.15 per GB per 30 days | [Fly pricing](https://docs.fly.io/about/pricing) |
| Disk snapshots | $0.08 per GB a month, first 10 GB free | [Fly pricing](https://docs.fly.io/about/pricing) |
| Outbound data, North America | $0.02 per GB | [Fly pricing](https://docs.fly.io/about/pricing) |
| Claude Haiku 4.5, for quick built-in features | $1 in, $5 out per million tokens | [BenchLM, Sept 2026](https://benchlm.ai/anthropic/api-pricing) |
| Claude Sonnet 5, for plans and dashboards | $2 in, $10 out per million tokens | [BenchLM, Sept 2026](https://benchlm.ai/anthropic/api-pricing) |
| Card payments | 2.9% + $0.30 per charge | [Flexprice](https://flexprice.io/blog/stripe-pricing-breakdown-2026) |
| Subscription and usage billing | 0.7% of billed volume | [Flexprice](https://flexprice.io/blog/stripe-pricing-breakdown-2026) |

The preset names (Light, Standard, Power) are proposals that map our resource presets onto Fly machine sizes. Shared-CPU machines are throttled under sustained load, which makes Light suit monitors and scripts, and Standard suit agent sessions. Object storage and email costs are small and still to be priced. Fly offers a 40% discount on reserved annual capacity, which we can use once usage is predictable.

## Tiers

Free lets anyone try one real workspace at no cost. Plus is for individuals running work every day. Pro is for heavy users running many workspaces in parallel. Every value in this table is \[proposed\].

|  | Free | Plus | Pro |
| --- | --- | --- | --- |
| Monthly price | $0, no card needed | $20 | $60 |
| Included usage | $3 of usage a month, paid by us | None: usage billed on top | None: usage billed on top |
| Workspaces | 2 | 10 | 50 |
| Awake at the same time | 1 | 3 | 10 |
| Largest workspace size | Light | Standard | Power |
| Disk per workspace | 5 GB | 50 GB | 200 GB |
| Most frequent schedule | Hourly | Every 5 minutes | Every minute |
| Claude Code sessions | Yes, with your key or subscription | Yes | Yes, several in parallel |
| Built-in AI features | Within the included usage | Billed at cost | Billed at cost |
| Templates | Starter set | All | All, plus early access |
| Private previews | Yes | Yes | Yes |
| Keep-awake workspaces (bots) | None | 1 | 5 |
| Fast-wake tier | No | No | Yes |
| Retention | Logs 7 days | Product defaults | Configurable and longer |
| Support | Community | Email | Priority email |

Collaboration will sit in Pro, or in a Team tier, once it ships. The R1 alpha is free for the three founders, with usage covered by us, so we learn real usage before fixing these numbers.

## Usage billing

Usage is metered per second and billed once a month on the same invoice as the subscription.

| What's metered | Shown to users as | Rate |
| --- | --- | --- |
| Awake time, by workspace size | Hours awake | Provider cost plus \[proposed: 10%\] |
| Disk, awake or asleep | GB stored per month | Provider cost plus \[proposed: 10%\] |
| Snapshots and backups | GB stored per month | Provider cost plus \[proposed: 10%\] |
| Outbound data | GB sent | Provider cost plus \[proposed: 10%\] |
| Built-in AI features | AI usage, in dollars, per feature | Exactly provider cost |

**Not billed by us:** Claude Code sessions use the user's own API key or Claude subscription, so Anthropic bills the user directly. Croncave only charges the awake time of the workspace the session runs in.

**Card fees.** Stripe takes about 3.6% of what we bill (2.9% for cards plus 0.7% for usage billing), plus $0.30 per charge. Because AI usage is passed through at exactly cost, we absorb those fees on it. The subscription price covers this, and the markup on other usage covers its own fees. Billing everything on one monthly invoice keeps it to one $0.30 charge.

**Spending caps**

- Every account has a monthly cap, with a default per tier \[proposed: $0 over the included usage on Free, $25 on Plus, $100 on Pro\]. Users can raise or lower it, and set caps per workspace.
- We alert at 50%, 80% and 100% of the cap.
- At the cap, work pauses and workspaces sleep. Nothing is deleted, and one click raises the cap and resumes.
- Usage is visible live, per workspace and per feature.

## Worked examples

With the proposed prices, the three R1 personas pay between $0 and about $54 a month. These use the unit costs above with a 10% markup on compute and storage. All usage figures are assumptions.

| Persona | Tier | Assumptions | Subscription | Usage | Monthly total |
| --- | --- | --- | --- | --- | --- |
| Founder: overnight coding | Plus | Standard workspace awake 140 h (6 h a night for 20 nights, plus 20 h of daytime review), 20 GB disk, $4 of built-in AI on Sonnet 5 | $20.00 | $33.80 | $53.80 |
| Stock monitor, 10-minute idle timeout | Plus | Light workspace checking every 5 minutes, so it never sleeps: 730 h awake, 2 GB disk, $0.50 of built-in AI | $20.00 | $23.64 | $43.64 |
| Stock monitor, 1-minute idle timeout | Plus | Same checks, but it sleeps between them: about 219 h awake (10 s wake, 20 s check, 60 s idle, 288 times a day) | $20.00 | $7.67 | $27.67 |
| Designer: prototypes | Free | Light workspace awake 24 h, 5 GB disk, $1 of built-in AI | $0.00 | $2.57, inside the $3 included | $0.00 |

Claude Code usage is extra for the founder and the designer, billed by Anthropic to their own key or subscription.

**What the monitor rows show.** Frequent schedules keep a workspace awake with the default timeout. Two fixes follow: set a short idle timeout automatically when a task runs more often than every 10 minutes, and bring forward "light checks without waking the workspace," which is currently scheduled for Later.

## Free tier guardrails

Free compute is exactly what crypto miners and spammers look for, so the free tier is generous for trying the product and useless for abuse.

- **Hard stop at the included usage.** Free accounts have no card, so work pauses at $3 of usage a month \[proposed\]. It never becomes a bill.
- **Light workspaces only,** with one awake at a time. Mining on throttled shared CPUs isn't worth the effort.
- **No keep-awake,** and schedules no more often than hourly.
- **Verified sign-up:** a US location check, plus email and phone verification \[proposed\], to stop account farming.
- **Stricter egress** than paid tiers: known mining pools blocked, outbound email ports blocked, and lower bandwidth limits.
- **Idle free workspaces are deleted after 60 days of no use** \[proposed\], with two email warnings first. This keeps storage costs from piling up.
- **Upgrade path:** adding a card moves the account to Plus with nothing lost.

## Decisions

- [ ] **Tier names and prices.** Decided: Free, Plus and Pro. Prices are set later.
- [ ] **Markup on compute and storage.** Is 10% right? Decided after R1, from real usage. On Fly: its standard terms limit use to a customer's "internal use" and forbid use "for the benefit of a third party". But on September 23, 2026 Fly support said building platforms on Fly is allowed, and that the wording will be clarified. Check before launch: get written confirmation from Fly for our case, including the markup.
- [ ] **Free included usage.** Is $3 a month enough to feel real without inviting abuse? Decided after R1, from real usage.
- [ ] **Short idle timeouts for frequent tasks,** and whether "light checks without waking" moves up from Later. Decided: tasks that run more often than every 10 minutes get a short idle timeout automatically. Light checks are reconsidered after R1.
- [ ] **Annual plans,** which could pass on part of Fly's 40% reserved-capacity discount. Out of scope for now.
- [ ] **Team pricing,** once collaboration ships: Decided: per seat, worked out when collaboration ships.
