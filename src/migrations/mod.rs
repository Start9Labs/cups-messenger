use failure::Error;
use failure::ResultExt;
use rusqlite::params;
use rusqlite::OptionalExtension;

pub async fn migrate() -> Result<(), Error> {
    let pool = crate::db::POOL.clone();
    tokio::task::spawn_blocking(move || {
        let mut gconn = pool.get()?;
        let conn = gconn.transaction()?;
        init(&conn)?;
        tracking_ids(&conn)?;
        conn.commit()?;
        Ok::<_, Error>(())
    })
    .await??;
    Ok(())
}

pub fn init(conn: &rusqlite::Transaction) -> Result<(), Error> {
    let q = "SELECT count(name) FROM sqlite_master WHERE type = 'table' AND name = 'migrations'";
    let exists: i64 = conn
        .query_row(q, params![], |row| row.get(0))
        .with_context(|e| format!("{}: {}", q, e))?;
    let q = "SELECT * FROM migrations WHERE name = 'init'";
    if exists == 0
        || conn
            .query_row(q, params![], |_| Ok(()))
            .optional()
            .with_context(|e| format!("{}: {}", q, e))?
            .is_none()
    {
        let q = "CREATE TABLE messages (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        user_id BLOB NOT NULL,
                        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        inbound BOOLEAN NOT NULL,
                        time INTEGER NOT NULL,
                        content TEXT NOT NULL,
                        read BOOLEAN NOT NULL DEFAULT FALSE
                    )";
        conn.execute(q, params![])
            .with_context(|e| format!("{}: {}", q, e))?;
        let q = "CREATE TABLE users (
                        id BLOB PRIMARY KEY,
                        name TEXT NOT NULL
                    )";
        conn.execute(q, params![])
            .with_context(|e| format!("{}: {}", q, e))?;
        let q = "CREATE TABLE migrations (
                        time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        name TEXT
                    )";
        conn.execute(q, params![])
            .with_context(|e| format!("{}: {}", q, e))?;
        let q = "INSERT INTO migrations (name) VALUES ('init')";
        conn.execute(q, params![])
            .with_context(|e| format!("{}: {}", q, e))?;
    }
    Ok(())
}

pub fn tracking_ids(conn: &rusqlite::Transaction) -> Result<(), Error> {
    let q = "SELECT * FROM migrations WHERE name = 'tracking_ids'";
    if conn
        .query_row(q, params![], |_| Ok(()))
        .optional()
        .with_context(|e| format!("{}: {}", q, e))?
        .is_none()
    {
        let q = "ALTER TABLE messages ADD tracking_id BLOB";
        conn.execute(q, params![])
            .with_context(|e| format!("{}: {}", q, e))?;
        let q = "CREATE INDEX messages_user_id_idx ON messages(user_id)";
        conn.execute(q, params![])
            .with_context(|e| format!("{}: {}", q, e))?;
        let q = "CREATE INDEX messages_tracking_id_idx ON messages(tracking_id)";
        conn.execute(q, params![])
            .with_context(|e| format!("{}: {}", q, e))?;
    }
    Ok(())
}
