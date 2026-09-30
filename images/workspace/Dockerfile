# The image a workspace runs.
#
# Built from the repository root:
#     docker build -f images/workspace/Dockerfile -t croncave/workspace:dev .
#
# The agent is compiled against musl and linked statically, so the final
# image needs no runtime beyond a shell. That also means the same binary
# works in any base image, which matters because workspace images are plain
# OCI images that must run on any provider.

FROM rust:1-alpine AS build

# musl-dev for the C runtime cargo links against; nothing here needs OpenSSL
# because every TLS path in Croncave uses rustls.
RUN apk add --no-cache musl-dev

WORKDIR /src

# The whole workspace, because the agent shares crates with the services.
COPY Cargo.toml Cargo.lock ./
COPY crates crates

RUN cargo build --release --locked -p croncave-agent

FROM alpine:3.21

# A shell and the basics, because commands run in here.
RUN apk add --no-cache ca-certificates

COPY --from=build /src/target/release/croncave-agent /usr/local/bin/croncave-agent

# Work happens here, and this is what survives while a workspace sleeps.
WORKDIR /workspace

# Nothing is EXPOSEd, and nothing here listens. A workspace only ever dials
# out: see AGENTS.md, "Workspaces never listen."
ENTRYPOINT ["/usr/local/bin/croncave-agent"]
