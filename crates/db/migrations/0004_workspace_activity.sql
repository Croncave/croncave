-- When something last happened in a workspace.
--
-- "Sleep by default" needs an answer to "is anything going on?", and until
-- now nothing could give one. This is that answer: the moment of the last
-- thing that counts as the workspace being in use.
--
-- Sessions, previews and a browser terminal each add their own signal as
-- they arrive; they update this column rather than inventing their own, so
-- the idle rule stays in one place.

alter table workspaces
    add column last_active_at timestamptz;

-- The sweeper looks for awake workspaces that have been quiet, so it reads
-- by state and time together.
create index workspaces_idle_idx on workspaces (state, last_active_at);
