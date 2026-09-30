-- How a workspace proves which workspace it is.
--
-- Two kinds, one table. A bootstrap token is put inside a workspace when it
-- starts and works exactly once; the agent trades it for a credential it
-- uses for the life of that connection.
--
-- Neither is stored: only its SHA-256, the same rule as sign-in links and
-- session cookies. Reading this table therefore lets nobody impersonate a
-- workspace.

create table workspace_tokens (
    id           uuid        primary key,
    workspace_id uuid        not null references workspaces (id) on delete cascade,
    kind         text        not null check (kind in ('bootstrap', 'credential')),
    token_hash   bytea       not null unique,
    expires_at   timestamptz not null,
    -- Set the moment a bootstrap token is traded in, so it works once.
    consumed_at  timestamptz,
    created_at   timestamptz not null default now()
);

create index workspace_tokens_workspace_idx on workspace_tokens (workspace_id);
create index workspace_tokens_expires_at_idx on workspace_tokens (expires_at);
