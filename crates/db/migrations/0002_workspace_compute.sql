-- Where a workspace's computer lives.
--
-- Both columns are null until someone starts the workspace for the first
-- time: a workspace is a record until there is work for it, so an account
-- that signs up and never runs anything provisions nothing and pays for
-- nothing.
--
-- The provider is stored beside its id because the id only means something to
-- the driver that issued it, and a workspace made on one provider must not be
-- looked up on another after a migration.

alter table workspaces
    add column compute_provider text,
    add column compute_id       text,
    -- When the provider was last asked what state it was in, so a stale
    -- answer can be told from a fresh one.
    add column compute_seen_at  timestamptz;

-- Either both or neither: a provider with no id, or an id with no provider,
-- is a row nothing can act on.
alter table workspaces
    add constraint workspaces_compute_complete
    check ((compute_provider is null) = (compute_id is null));

create index workspaces_compute_idx on workspaces (compute_provider, compute_id);
