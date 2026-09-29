# 0022: Sign in with a link, and no passwords anywhere

- **Date:** 2026-09-29
- **Status:** Accepted

## Context

`docs/product-definition.md` promises "sign up and sign in with email, GitHub or Google" as a Must for R1. The design system settles the shape: the finished sign-in screen has one email field, a **Send sign-in link** button, the line "We'll email you a sign-in link. No password to remember," and its README says plainly that there is no Google or GitHub sign-in.

The reason to reach for passwords instead was that magic links appear to need an email provider, and `docs/architecture.md` still lists that provider as undecided. That turns out not to be true: which provider carries the message is a seam, not a prerequisite.

## Decision

- **Magic links, and no password field anywhere.** Nothing to hash, nothing to reset, nothing to leak, and no rework of two finished screens.
- **A `Mailer` trait decides nothing about the vendor.** `LogMailer` writes the link through `tracing` locally, `TestMailer` captures it, and `UnconfiguredMailer` — what a deployment gets until a provider is chosen — **refuses** rather than logging a live credential where anyone with log access could use it. Choosing a provider is one implementation.
- **The token is 32 random bytes from the operating system, and only its SHA-256 is stored.** SHA-256 rather than Argon2 on purpose: slow hashes exist to make *low-entropy* secrets expensive to guess, and there is nothing to guess in 256 random bits. A slow hash would only make every request slower.
- **A link works exactly once**, enforced by a single conditional `UPDATE … WHERE consumed_at IS NULL AND expires_at > now() RETURNING email`. Two browsers racing the same link cannot both match, so single use survives concurrency rather than depending on a read-then-write.
- **Fifteen minutes to use a link.** Long enough to switch to an email client, short enough that one left sitting in an inbox is not a standing key.
- **Asking for a link never reveals whether an account exists.** The status is identical either way, both cases send a message, and even the rate limit answers with the same 202 — because "too many requests" would itself confirm the address is in use.
- **Five links per address per fifteen minutes**, so the endpoint cannot be used to flood someone's inbox.
- **Sessions are rows, not signed cookies.** Opaque 32-byte secret, stored hashed, thirty days, `HttpOnly`, `SameSite=Lax`, `Secure` outside local development. `Lax` and not `Strict` because arriving from an email client is a cross-site navigation and `Strict` would drop the cookie exactly when it is needed.
- **An account and its personal team are created in one transaction** the first time a link is followed, so a half-made account cannot exist.

## Alternatives considered

- **Email and password first, magic links later:** what was asked for before the design was read. It would have meant a password field the design system has no component for, two finished screens redone, and a reset flow that needs email anyway.
- **Argon2 on the token:** defensible reflex, wrong tool. The input is already high-entropy.
- **Signed cookies (JWT):** no database read per request, but no revocation, and "sign out" would stop meaning anything.
- **Reading, checking, then consuming the link in separate statements:** the obvious shape, and it loses the race. Two tabs opened together would both succeed.
- **Answering "too many requests" when rate-limited:** honest to the user, and a way to enumerate accounts.
- **GitHub or Google sign-in:** ruled out by the design system, which says every user creates a Croncave account.

## Consequences

- **The email provider is still undecided and nothing is blocked by it.** Whoever it is, they implement one trait with one method.
- **A deployment without a provider cannot sign anyone in** — `UnconfiguredMailer` refuses — which is the right failure. Staging needs a real mailer before anyone can use it.
- **Invite-only access is not enforced yet.** Following a link creates an account, and the product says alpha access is by invitation. An allowlist check belongs at `request_link`, and is hardening's to add before the alpha opens.
- **Rate limiting lives in Postgres**, counted from `login_tokens`, which is fine at alpha size and will want moving if sign-in traffic ever grows.
- Sessions are checked against the database on every request. That is what makes revocation immediate; if it ever costs too much, a short-lived cache is the answer, not a signed cookie.
