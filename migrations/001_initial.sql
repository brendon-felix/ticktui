CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL DEFAULT 'inbox',
    title       TEXT NOT NULL DEFAULT '',
    content     TEXT NOT NULL DEFAULT '',
    due_date    TEXT,
    priority    INTEGER NOT NULL DEFAULT 0,
    repeat_flag TEXT NOT NULL DEFAULT '',
    status      INTEGER NOT NULL DEFAULT 0,
    deleted     INTEGER NOT NULL DEFAULT 0,
    is_all_day  INTEGER NOT NULL DEFAULT 0,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    synced_at   TEXT
);

CREATE TABLE IF NOT EXISTS projects (
    id    TEXT PRIMARY KEY,
    name  TEXT NOT NULL DEFAULT ''
);

INSERT OR IGNORE INTO projects (id, name) VALUES ('inbox', 'Inbox');
