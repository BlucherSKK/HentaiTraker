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
        avatar     TEXT,
        tags       TEXT,
        settings   TEXT
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

CREATE OR REPLACE FUNCTION init_roles_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS roles (
        id          SERIAL PRIMARY KEY,
        name        TEXT NOT NULL UNIQUE,
        permissions INTEGER[] NOT NULL DEFAULT '{}'
    );
    INSERT INTO roles (name, permissions)
    VALUES ('admin', ARRAY[-1])
    ON CONFLICT (name) DO NOTHING;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_user_roles_cross_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS user_roles (
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
        PRIMARY KEY (user_id, role_id)
    );
    CREATE INDEX IF NOT EXISTS idx_user_roles_user ON user_roles (user_id);
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
    PERFORM init_roles_table();
    PERFORM init_user_roles_cross_table();
END;
$$ LANGUAGE plpgsql;

-- ============================================================
-- Part 2 — Create schema (idempotent on every startup)
-- ============================================================

SELECT init_db_schema();

-- ============================================================
-- Part 3 — Named query functions
-- ============================================================

-- ----- users -----

CREATE OR REPLACE FUNCTION db_get_user_by_id(p_id INT)
RETURNS SETOF users LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY SELECT * FROM users WHERE id = p_id;
END;
$$;

CREATE OR REPLACE FUNCTION db_get_user_by_name(p_name TEXT)
RETURNS SETOF users LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY SELECT * FROM users WHERE name = p_name;
END;
$$;

CREATE OR REPLACE FUNCTION db_insert_user(p_name TEXT, p_pass TEXT)
RETURNS SETOF users LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        INSERT INTO users (name, pass, last_visit)
        VALUES (p_name, p_pass, NOW())
        RETURNING *;
END;
$$;

-- ----- settings -----

CREATE OR REPLACE FUNCTION db_get_settings(p_user_id INT)
RETURNS TEXT LANGUAGE plpgsql STABLE AS $$
DECLARE
    v_settings TEXT;
BEGIN
    SELECT settings INTO v_settings FROM users WHERE id = p_user_id;
    RETURN v_settings;
END;
$$;

CREATE OR REPLACE FUNCTION db_set_settings(p_user_id INT, p_settings TEXT)
RETURNS SETOF users LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        UPDATE users SET settings = p_settings
        WHERE id = p_user_id
        RETURNING *;
END;
$$;

-- ----- profile -----

CREATE OR REPLACE FUNCTION db_update_user(
    p_target_id   INT,
    p_modifier_id INT,
    p_name        TEXT,
    p_pass        TEXT,
    p_avatar      TEXT,
    p_tags        TEXT
) RETURNS SETOF users LANGUAGE plpgsql AS $$
BEGIN
    IF p_target_id = p_modifier_id THEN
        RETURN QUERY
            UPDATE users SET
                name   = COALESCE(p_name,   name),
                pass   = COALESCE(p_pass,   pass),
                avatar = COALESCE(p_avatar, avatar),
                tags   = COALESCE(p_tags,   tags)
            WHERE id = p_target_id
            RETURNING *;
    ELSE
        IF EXISTS (
            SELECT 1 FROM roles r
            JOIN user_roles ur ON ur.role_id = r.id
            WHERE ur.user_id = p_modifier_id AND r.name = 'admin'
        ) THEN
            RETURN QUERY
                UPDATE users SET
                    name   = COALESCE(p_name,   name),
                    pass   = COALESCE(p_pass,   pass),
                    avatar = COALESCE(p_avatar, avatar),
                    tags   = COALESCE(p_tags,   tags)
                WHERE id = p_target_id
                RETURNING *;
        END IF;
    END IF;
END;
$$;

-- ----- posts -----

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

CREATE OR REPLACE FUNCTION db_get_latest_posts(p_limit BIGINT)
RETURNS SETOF posts LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY
        SELECT * FROM posts
        ORDER BY time DESC
        LIMIT p_limit;
END;
$$;

CREATE OR REPLACE FUNCTION db_create_post(p_author_id INT, p_title TEXT, p_content TEXT, p_files TEXT, p_tags TEXT)
RETURNS SETOF posts LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        INSERT INTO posts (title, content, files, author_id, time, tags)
        VALUES (p_title, p_content, p_files, p_author_id, NOW(), p_tags)
        RETURNING *;
END;
$$;

-- ----- chats -----

CREATE OR REPLACE FUNCTION db_get_user_chats(p_member_id INT)
RETURNS SETOF chats LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY
        SELECT c.* FROM chats c
        JOIN cross_chat_members ccm ON ccm.chat_id = c.id
        WHERE ccm.member_id = p_member_id
        ORDER BY c.time DESC;
END;
$$;

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
    INSERT INTO cross_chat_members (member_id, chat_id)
    VALUES (p_member_id, p_chat_id)
    ON CONFLICT DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION db_is_chat_member(p_chat_id INT, p_member_id INT)
RETURNS BOOLEAN LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN EXISTS (
        SELECT 1 FROM cross_chat_members
        WHERE chat_id = p_chat_id AND member_id = p_member_id
    );
END;
$$;

-- ----- messages -----

CREATE OR REPLACE FUNCTION db_send_message(p_chat_id INT, p_author_id INT, p_content TEXT, p_files TEXT)
RETURNS SETOF msg LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        INSERT INTO msg (content, files, author_id, chat_id, time)
        VALUES (p_content, p_files, p_author_id, p_chat_id, NOW())
        RETURNING *;
END;
$$;

CREATE OR REPLACE FUNCTION db_get_chat_messages(p_chat_id INT, p_limit BIGINT)
RETURNS SETOF msg LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY
        SELECT * FROM msg
        WHERE  chat_id = p_chat_id
        ORDER  BY time DESC
        LIMIT  p_limit;
END;
$$;

-- ----- roles -----

CREATE OR REPLACE FUNCTION db_set_user_roles(p_user_id INT, p_role_ids INTEGER[])
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM user_roles WHERE user_id = p_user_id;
    INSERT INTO user_roles (user_id, role_id)
        SELECT p_user_id, unnest(p_role_ids)
        ON CONFLICT DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION db_get_roles()
RETURNS SETOF roles LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY SELECT * FROM roles;
END;
$$;

CREATE OR REPLACE FUNCTION db_create_role(p_name TEXT, p_permissions INTEGER[])
RETURNS SETOF roles LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
        INSERT INTO roles (name, permissions)
        VALUES (p_name, p_permissions)
        RETURNING *;
END;
$$;

CREATE OR REPLACE FUNCTION db_get_user_roles(p_user_id INT)
RETURNS SETOF roles LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN QUERY
        SELECT r.* FROM roles r
        JOIN user_roles ur ON ur.role_id = r.id
        WHERE ur.user_id = p_user_id;
END;
$$;

CREATE OR REPLACE FUNCTION db_assign_role(p_user_id INT, p_role_id INT)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO user_roles (user_id, role_id)
    VALUES (p_user_id, p_role_id)
    ON CONFLICT DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION db_revoke_role(p_user_id INT, p_role_id INT)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM user_roles WHERE user_id = p_user_id AND role_id = p_role_id;
END;
$$;

CREATE OR REPLACE FUNCTION db_has_role(p_user_id INT, p_role_id INT)
RETURNS BOOLEAN LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN EXISTS (
        SELECT 1 FROM user_roles WHERE user_id = p_user_id AND role_id = p_role_id
    );
END;
$$;

CREATE OR REPLACE FUNCTION db_user_has_permission(p_user_id INT, p_permission INT)
RETURNS BOOLEAN LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN EXISTS (
        SELECT 1 FROM roles r
        JOIN user_roles ur ON ur.role_id = r.id
        WHERE ur.user_id = p_user_id
          AND p_permission = ANY(r.permissions)
    );
END;
$$;
