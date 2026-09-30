# 0024: WebSocket on 443 with yamux inside, and how a workspace proves who it is

- **Date:** 2026-09-30
- **Status:** Accepted

## Context

`docs/architecture.md` marked the transport **\[proposed\]**: "TLS on port 443 with a stream multiplexer (HTTP/2 or yamux)". Everything in Croncave rides on that choice — sessions, scheduled runs, logs, files and previews all travel over the one connection a workspace opens outward — so it had to be settled before any of them are built.

It also had to answer a question the architecture describes but does not specify: how a workspace proves which workspace it is, given that the thing asking is code we do not control, running in a container anyone's code may be running in.

## Decision

- **A WebSocket on 443, with yamux multiplexing inside it.** The agent dials out; the relay never dials in. WebSocket crosses proxies and corporate networks that only expect HTTP, terminates in axum beside the browser-facing routes, and needs no special network setup. yamux inside gives the independent streams the architecture lists, so a large file transfer cannot block a control message, and a terminal in R2 is a new stream kind rather than a new connection.
- **Authorisation happens before the multiplexer exists.** The hello exchange is plain WebSocket messages. Only once a credential is accepted does yamux start, so a connection that cannot prove who it is never gets streams at all.
- **A one-time bootstrap token, traded for a credential.** The orchestrator mints a bootstrap token on **every** start and puts it in the workspace's environment; the agent trades it on first connect. Both are stored only as hashes, like every other secret we issue. Trading is a single conditional `UPDATE`, so two agents racing the same token cannot both win.
- **The relay owns no database.** Who a credential belongs to is asked through an `Authoriser` the control plane implements, so the relay stays a library that can become its own process without dragging the schema along.
- **A protocol version, refused on mismatch.** An agent is baked into an image and may be months older than the relay it meets. Adding a field is safe, adding a variant is safe, changing a meaning is not — the same expand-then-contract rule the migrations follow.
- **Control messages are JSON.** The volume is tiny and being able to read a connection in a log is worth more than the bytes. Output is raw on its own stream, so the choice costs nothing where volume actually is.

## Alternatives considered

- **gRPC over HTTP/2 (tonic):** bidirectional streams and generated types for free, and HTTP/2 multiplexes natively with no yamux layer. Rejected because it shapes the protocol around gRPC's request-response model, and previews — arbitrary web traffic to a port inside a workspace — fit that model badly.
- **Raw TLS with yamux:** least overhead and total control of the wire, at the cost of WebSocket's ability to cross proxies that only speak HTTP, and of terminating TLS separately from the app.
- **One shared relay secret:** quickest to a working connection, and it means any workspace can impersonate any other. Exactly the kind of temporary path that is still there at launch.
- **A long-lived credential baked into the image:** no bootstrap exchange, and every copy of the image would hold the same key.
- **Keeping the credential across restarts:** fewer tokens minted. Rejected because a fresh start should get a fresh identity, so a container that comes back after being replaced cannot rejoin.

## Consequences

- **The `[proposed]` marker in `architecture.md` is now settled and wants updating upstream**: WebSocket with yamux inside, not "HTTP/2 or yamux".
- Adding a stream kind — files, previews, a terminal — is a variant in the protocol and a handler at each end. `StreamKind` already names the ones that are coming, and an agent refuses a kind it cannot serve rather than mishandling it.
- Credentials do not renew yet. A connection lasting longer than a day would be refused on reconnect; that matters first for step 4's long sessions, which is where renewal belongs.
- The relay holds connections in memory, so it is one process today. The architecture calls for stateless relays with a routing table, which is a change to where the registry lives rather than to the protocol.
- **Running the relay locally means binding more than loopback**, because workspaces are containers. Decision 0021's loopback default stands; local development opts out explicitly, and `.env.example` says why.
