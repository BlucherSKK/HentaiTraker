-- ============================================================
-- Part 1 — Table init functions (idempotent, CREATE IF NOT EXISTS)
-- ============================================================

CREATE OR REPLACE FUNCTION init_user_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS users (
        id         SERIAL PRIMARY KEY,
        name       TEXT NOT NULL,
        pass       VARCHAR(255) NOT NULL,
        last_visit TIMESTAMP NOT NULL,
        roles      TEXT,
        avatar     TEXT,
        tags       TEXT
    );
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_posts_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS posts (
        id        SERIAL PRIMARY KEY,
        title     TEXT,
        content   TEXT NOT NULL,
        files     TEXT,
        author_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        time      TIMESTAMP NOT NULL,
        tags      TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_posts_author_time ON posts (author_id, time DESC);
    CREATE INDEX IF NOT EXISTS idx_posts_time        ON posts (time DESC);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_chats_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS chats (
        id        SERIAL PRIMARY KEY,
        title     TEXT,
        content   TEXT NOT NULL,
        images    TEXT,
        author_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        time      TIMESTAMP NOT NULL
    );
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_cross_cher_members_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS cross_chat_members (
        member_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
        chat_id   INTEGER REFERENCES chats(id) ON DELETE CASCADE,
        PRIMARY KEY (member_id, chat_id)
    );
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_msg_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS msg (
        id        SERIAL PRIMARY KEY,
        content   TEXT NOT NULL,
        files     TEXT,
        author_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        chat_id   INTEGER NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
        time      TIMESTAMP NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_msg_chat_time ON msg (chat_id, time DESC);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_db_schema()
RETURNS void AS $$
BEGIN
    PERFORM init_user_table();
    PERFORM init_posts_table();
    PERFORM init_chats_table();
    PERFORM init_cross_cher_members_table();
    PERFORM init_msg_table();
END;
$$ LANGUAGE plpgsql;

-- ============================================================
-- Part 2 — Create schema (idempotent on every startup)
-- ============================================================

SELECT init_db_schema();

-- ============================================================
-- Part 3 — Named query functions
--   Called from Rust as: SELECT * FROM db_<name>($1, ...)
--   Read functions are STABLE (same args → same result within a tx).
--   Write functions default to VOLATILE.
-- ============================================================

-- ── Users ────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION db_get_user_by_id(p_id INT)
RETURNS SETOF users LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY SELECT * FROM users WHERE id = p_id;
END;
$$;

CREATE OR REPLACE FUNCTION db_insert_user(p_name TEXT, p_pass TEXT, p_roles TEXT)
RETURNS SETOF users LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        INSERT INTO users (name, pass, last_visit, roles)
        VALUES (p_name, p_pass, NOW(), p_roles)
        RETURNING *;
END;
$$;

-- ── Posts ────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION db_get_posts_by_author(p_author_id INT, p_limit INT)
RETURNS SETOF posts LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY
        SELECT * FROM posts
        WHERE  author_id = p_author_id
        ORDER  BY time DESC
        LIMIT  p_limit;
END;
$$;

CREATE OR REPLACE FUNCTION db_get_latest_post_before(p_time TIMESTAMP)
RETURNS SETOF posts LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY
        SELECT * FROM posts
        WHERE  time < p_time
        ORDER  BY time DESC
        LIMIT  1;
END;
$$;

CREATE OR REPLACE FUNCTION db_get_latest_post_now()
RETURNS SETOF posts LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY
        SELECT * FROM posts
        WHERE  time < NOW()
        ORDER  BY time DESC
        LIMIT  1;
END;
$$;

CREATE OR REPLACE FUNCTION db_insert_post(p_author_id INT, p_title TEXT, p_content TEXT)
RETURNS SETOF posts LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        INSERT INTO posts (title, content, author_id, time)
        VALUES (p_title, p_content, p_author_id, NOW())
        RETURNING *;
END;
$$;

-- ── Chats ────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION db_create_chat(p_author_id INT, p_title TEXT, p_content TEXT)
RETURNS SETOF chats LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        INSERT INTO chats (title, content, author_id, time)
        VALUES (p_title, p_content, p_author_id, NOW())
        RETURNING *;
END;
$$;

CREATE OR REPLACE FUNCTION db_add_chat_member(p_chat_id INT, p_member_id INT)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO cross_chat_members (chat_id, member_id)
    VALUES (p_chat_id, p_member_id)
    ON CONFLICT DO NOTHING;
END;
$$;

-- ── Messages ─────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION db_send_message(p_chat_id INT, p_author_id INT, p_content TEXT)
RETURNS SETOF msg LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        INSERT INTO msg (content, author_id, chat_id, time)
        VALUES (p_content, p_author_id, p_chat_id, NOW())
        RETURNING *;
END;
$$;

CREATE OR REPLACE FUNCTION db_get_chat_messages(p_chat_id INT, p_limit INT)
RETURNS SETOF msg LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY
        SELECT * FROM msg
        WHERE  chat_id = p_chat_id
        ORDER  BY time DESC
        LIMIT  p_limit;
END;
$$;
