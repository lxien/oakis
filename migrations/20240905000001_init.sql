CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS posts (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    slug         TEXT NOT NULL UNIQUE,
    summary      TEXT NOT NULL DEFAULT '',
    content_md   TEXT NOT NULL DEFAULT '',
    content_html TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'draft'
                     CHECK (status IN ('draft', 'published')),
    visibility   TEXT NOT NULL DEFAULT 'public'
                     CHECK (visibility IN ('public', 'private')),
    cover_url    TEXT,
    published_at TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_posts_status_published_at
    ON posts (status, published_at DESC);

CREATE INDEX IF NOT EXISTS idx_posts_slug ON posts (slug);

CREATE TABLE IF NOT EXISTS taxonomies (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    scope      TEXT NOT NULL DEFAULT 'post',
    kind       TEXT NOT NULL CHECK (kind IN ('category', 'tag')),
    name       TEXT NOT NULL,
    slug       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_taxonomies_scope_kind_slug
    ON taxonomies (scope, kind, slug);

CREATE INDEX IF NOT EXISTS idx_taxonomies_scope_kind
    ON taxonomies (scope, kind);

CREATE TABLE IF NOT EXISTS post_taxonomies (
    post_id     INTEGER NOT NULL,
    taxonomy_id INTEGER NOT NULL,
    PRIMARY KEY (post_id, taxonomy_id),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (taxonomy_id) REFERENCES taxonomies(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_post_taxonomies_taxonomy
    ON post_taxonomies (taxonomy_id);

CREATE TABLE IF NOT EXISTS post_relations (
    post_id         INTEGER NOT NULL,
    related_post_id INTEGER NOT NULL,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (post_id, related_post_id),
    CHECK (post_id != related_post_id),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (related_post_id) REFERENCES posts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_post_relations_related
    ON post_relations (related_post_id);

CREATE INDEX IF NOT EXISTS idx_post_relations_sort
    ON post_relations (post_id, sort_order, related_post_id);

CREATE TABLE IF NOT EXISTS media (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    url        TEXT NOT NULL UNIQUE,
    mime       TEXT NOT NULL,
    size       INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_media_created_at ON media (created_at DESC);

CREATE TABLE IF NOT EXISTS pages (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    slug         TEXT NOT NULL UNIQUE,
    content_md   TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'draft'
                     CHECK (status IN ('draft', 'published')),
    visibility   TEXT NOT NULL DEFAULT 'public'
                     CHECK (visibility IN ('public', 'private')),
    show_in_nav  INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

INSERT INTO pages (title, slug, content_md, status, visibility, show_in_nav, created_at, updated_at)
VALUES (
    '关于',
    'about',
    '',
    'published',
    'public',
    1,
    datetime('now'),
    datetime('now')
);

CREATE TABLE IF NOT EXISTS sparks (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    content_md   TEXT NOT NULL,
    content_html TEXT NOT NULL DEFAULT '',
    visibility   TEXT NOT NULL DEFAULT 'private'
                     CHECK (visibility IN ('public', 'private')),
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sparks_created ON sparks(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sparks_visibility ON sparks(visibility);

CREATE TABLE IF NOT EXISTS kb_books (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    slug         TEXT NOT NULL UNIQUE,
    summary      TEXT NOT NULL DEFAULT '',
    cover_url    TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'published'
                     CHECK (status IN ('draft', 'published')),
    visibility   TEXT NOT NULL DEFAULT 'public'
                     CHECK (visibility IN ('public', 'private')),
    sort_order   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_kb_books_sort ON kb_books (sort_order ASC, id ASC);

CREATE TABLE IF NOT EXISTS kb_nodes (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id      INTEGER NOT NULL,
    parent_id    INTEGER,
    node_type    TEXT NOT NULL DEFAULT 'doc'
                     CHECK (node_type IN ('folder', 'doc')),
    title        TEXT NOT NULL,
    slug         TEXT NOT NULL,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    content_md   TEXT NOT NULL DEFAULT '',
    content_html TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'draft'
                     CHECK (status IN ('draft', 'published')),
    visibility   TEXT NOT NULL DEFAULT 'public'
                     CHECK (visibility IN ('public', 'private')),
    published_at TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    FOREIGN KEY (book_id) REFERENCES kb_books(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES kb_nodes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_kb_nodes_book_parent
    ON kb_nodes (book_id, parent_id, sort_order, id);

CREATE INDEX IF NOT EXISTS idx_kb_nodes_book_type
    ON kb_nodes (book_id, node_type);

CREATE UNIQUE INDEX IF NOT EXISTS idx_kb_nodes_book_slug
    ON kb_nodes (book_id, slug);
