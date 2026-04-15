CREATE OR REPLACE FUNCTION init_user_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS users (
        id SERIAL PRIMARY KEY,
        name TEXT NOT NULL,
        pass VARCHAR(255) NOT NULL,
        last_visit TIMESTAMP NOT NULL,
        roles TEXT,
        avatar TEXT,
        tags TEXT,
    );
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_posts_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS posts (
        id SERIAL PRIMARY KEY,
        title TEXT,
        content TEXT NOT NULL,
        files TEXT,
        author_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        time TIMESTAMP NOT NULL,
        tags TEXT,
    );
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_chats_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS chats (
        id SERIAL PRIMARY KEY,
        title TEXT,
        content TEXT NOT NULL,
        images TEXT,
        author_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        time TIMESTAMP NOT NULL,
    );
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_cross_cher_members_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS cross_chat_members (
        member_id INTEGER REFERENCES posts(id) ON DELETE CASCADE,
        chat_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
        PRIMARY KEY (member_id, chat_id)
    );
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION init_msg_table()
RETURNS void AS $$
BEGIN
    CREATE TABLE IF NOT EXISTS msg (
        id SERIAL PRIMARY KEY,
        content TEXT NOT NULL,
        files TEXT,
        author_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        chat_id INTEGER NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
        time TIMESTAMP NOT NULL,
    );
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
