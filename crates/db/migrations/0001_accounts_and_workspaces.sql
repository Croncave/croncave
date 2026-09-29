-- The first records Croncave needs: who someone is, the team everything
-- belongs to, and the workspaces that team owns.
--
-- Two rules from AGENTS.md shape this schema. Everything belongs to a team,
-- never directly to a user, so a personal team of one becomes a shared team
-- later without a migration. And every action is attributed, so rows record
-- who created them.

-- Users ---------------------------------------------------------------------

create table users (
    id          uuid        primary key,
    -- Stored already lowercased and trimmed, so a plain unique index is
    -- enough and no extension is needed.
    email       text        not null unique,
    name        text,
    created_at  timestamptz not null default now(),
    updated_at  timestamptz not null default now()
);

-- Teams ---------------------------------------------------------------------

create table teams (
    id          uuid        primary key,
    name        text        not null,
    -- 'personal' is the team of one created at sign-up. Text with a check
    -- rather than a Postgres enum: a value can be added in one migration and
    -- removed in another, which enum types make painful.
    kind        text        not null check (kind in ('personal', 'shared')),
    created_at  timestamptz not null default now(),
    updated_at  timestamptz not null default now()
);

create table memberships (
    id          uuid        primary key,
    user_id     uuid        not null references users (id) on delete cascade,
    team_id     uuid        not null references teams (id) on delete cascade,
    role        text        not null check (role in ('owner', 'member')),
    created_at  timestamptz not null default now(),
    unique (user_id, team_id)
);

create index memberships_team_id_idx on memberships (team_id);

-- Workspaces ----------------------------------------------------------------

create table workspaces (
    id          uuid        primary key,
    team_id     uuid        not null references teams (id) on delete cascade,
    name        text        not null,
    -- The lifecycle states from docs/architecture.md. A workspace has no
    -- compute until step 2, so it starts asleep and stays there.
    state       text        not null default 'asleep'
                            check (state in ('asleep', 'waking', 'awake', 'stopping')),
    created_by  uuid        not null references users (id),
    created_at  timestamptz not null default now(),
    updated_at  timestamptz not null default now()
);

create index workspaces_team_id_idx on workspaces (team_id);

-- Sign-in -------------------------------------------------------------------

-- A sign-in link, emailed to one address. The token itself is never stored:
-- only its SHA-256, so reading this table does not let anyone sign in.
create table login_tokens (
    id          uuid        primary key,
    email       text        not null,
    token_hash  bytea       not null unique,
    expires_at  timestamptz not null,
    -- Set the moment the link is used, so a link works exactly once.
    consumed_at timestamptz,
    created_at  timestamptz not null default now()
);

create index login_tokens_email_idx on login_tokens (email);
create index login_tokens_expires_at_idx on login_tokens (expires_at);

-- A signed-in browser. Same rule: the cookie's value is stored only as a
-- hash, and the row is what makes a session revocable.
create table sessions (
    id           uuid        primary key,
    user_id      uuid        not null references users (id) on delete cascade,
    token_hash   bytea       not null unique,
    expires_at   timestamptz not null,
    created_at   timestamptz not null default now(),
    last_seen_at timestamptz not null default now()
);

create index sessions_user_id_idx on sessions (user_id);
create index sessions_expires_at_idx on sessions (expires_at);
