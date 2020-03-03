use ed25519_dalek::PublicKey;
use failure::Error;
use rusqlite::params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use uuid::Uuid;
use failure::ResultExt;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OpenFlags;

use crate::message::{NewInboundMessage, NewOutboundMessage};
use crate::query::BeforeAfter;
use crate::query::Limits;

lazy_static::lazy_static! {
    pub static ref POOL: Pool<SqliteConnectionManager> = {
        let mut flags = OpenFlags::empty();
        flags.insert(OpenFlags::SQLITE_OPEN_READ_WRITE);
        flags.insert(OpenFlags::SQLITE_OPEN_CREATE);
        flags.insert(OpenFlags::SQLITE_OPEN_FULL_MUTEX);
        flags.insert(OpenFlags::SQLITE_OPEN_PRIVATE_CACHE);
        Pool::new(SqliteConnectionManager::file("messages.db").with_flags(flags).with_init(|c| c.execute_batch("PRAGMA busy_timeout = 10000;"))).expect("sqlite connection")
    };
}

pub fn cached_exec<P>(conn: &Connection, q: &str, params: P) -> Result<(), Error>
where
    P: IntoIterator,
    P::Item: rusqlite::ToSql,
{
    let mut stmt = conn.prepare_cached(q).with_context(|e| format!("{}: {}", q, e))?;
    stmt.execute(params).with_context(|e| format!("{}: {}", q, e))?;
    Ok(())
}

pub fn cached_query_row<P, F, T>(conn: & Connection, q: &str, params: P, f: F) -> Result<Option<T>, Error>
where
    P: IntoIterator,
    P::Item: rusqlite::ToSql,
    F: FnMut(&rusqlite::Row) -> Result<T, rusqlite::Error>
{
    let mut stmt = conn.prepare_cached(q).with_context(|e| format!("{}: {}", q, e))?;
    let res = stmt.query_row(params, f).optional().with_context(|e| format!("{}: {}", q, e))?;
    Ok(res)
}

pub fn cached_query_map<P, F, T>(conn: &Connection, q: &str, params: P, f: F) -> Result<Vec<T>, Error>
where
    P: IntoIterator,
    P::Item: rusqlite::ToSql,
    F: FnMut(&rusqlite::Row) -> Result<T, rusqlite::Error>
{
    let mut stmt = conn.prepare_cached(q).with_context(|e| format!("{}: {}", q, e))?;
    let res = stmt.query_map(params, f).with_context(|e| format!("{}: {}", q, e))?;
    res.map(|r| r.with_context(|e| format!("{}: {}", q, e)).map_err(From::from)).collect()
}

pub async fn save_in_message(message: NewInboundMessage) -> Result<(), Error> {
    tokio::task::spawn_blocking(move || {
        let conn = POOL.get()?;
        cached_exec(
            &*conn, 
            "INSERT INTO messages (user_id, inbound, time, content) VALUES (?1, true, ?2, ?3)",
            params![
                &message.from.as_bytes()[..],
                message.time,
                message.content
            ],
        )?;
        Ok::<_, Error>(())
    })
    .await??;
    Ok(())
}

pub async fn save_out_message(message: NewOutboundMessage) -> Result<(), Error> {
    tokio::task::spawn_blocking(move || {
        let conn = POOL.get()?;
        cached_exec(
            &*conn, 
            "INSERT INTO messages (tracking_id, user_id, inbound, time, content, read) VALUES (?1, ?2, false, ?3, ?4, true)",
            params![message.tracking_id, &message.to.as_bytes()[..], message.time, message.content],
        )?;
        Ok::<_, Error>(())
    })
    .await??;
    Ok(())
}

pub async fn save_user(pubkey: PublicKey, name: String) -> Result<(), Error> {
    tokio::task::spawn_blocking(move || {
        let conn = POOL.get()?;
        cached_exec(
            &*conn,
            "INSERT INTO users (id, name) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET name = excluded.name",
            params![&pubkey.as_bytes()[..], name],
        )?;
        Ok::<_, Error>(())
    })
    .await??;
    Ok(())
}

pub async fn del_user(pubkey: PublicKey) -> Result<(), Error> {
    let res = tokio::task::spawn_blocking(move || {
        let conn = POOL.get()?;
        cached_exec(
            &*conn,
            "DELETE FROM users WHERE id = ?1",
            params![&pubkey.as_bytes()[..]],
        )?;
        Ok::<_, Error>(())
    })
    .await??;
    Ok(res)
}

#[derive(Clone, Debug)]
pub struct UserInfo {
    pub pubkey: PublicKey,
    pub name: Option<String>,
    pub unreads: i64,
}

pub async fn get_user_info() -> Result<Vec<UserInfo>, Error> {
    let res = tokio::task::spawn_blocking(move || {
        let conn = POOL.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT
                messages.user_id,
                users.name,
                SUM(CASE WHEN messages.read THEN 0 ELSE 1 END)
            FROM messages
            LEFT JOIN users
            ON messages.user_id = users.id
            GROUP BY messages.user_id, users.name
            UNION ALL
            SELECT
                users.id,
                users.name,
                count(messages.id)
            FROM users
            LEFT JOIN messages
            ON messages.user_id = users.id
            WHERE messages.user_id IS NULL
            GROUP BY users.id, users.name",
        )?;
        let res = stmt
            .query_map(params![], |row| {
                let uid: Vec<u8> = row.get(0)?;
                Ok(UserInfo {
                    pubkey: PublicKey::from_bytes(&uid).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        )
                    })?,
                    name: row.get(1)?,
                    unreads: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok::<_, Error>(res)
    })
    .await??;
    Ok(res)
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: i64,
    pub tracking_id: Option<Uuid>,
    pub time: i64,
    pub inbound: bool,
    pub content: String,
}

pub async fn get_messages(
    pubkey: PublicKey,
    limits: Limits,
    mark_as_read: bool,
) -> Result<Vec<Message>, Error> {
    let res = tokio::task::spawn_blocking(move || {
        let mut gconn = POOL.get()?;
        let conn = gconn.transaction()?;
        if mark_as_read {
            match (&limits.before_after, &limits.limit) {
                (Some(BeforeAfter::Before(before)), None) => cached_exec(
                    &*conn, 
                    "UPDATE messages SET read = true WHERE user_id = ?1 AND id IN (SELECT id FROM messages WHERE user_id = ?1 AND id < ?2 ORDER BY id DESC)",
                    params![&pubkey.as_bytes()[..], before],
                )?,
                (Some(BeforeAfter::Before(before)), Some(limit)) => cached_exec(
                    &*conn, 
                    "UPDATE messages SET read = true WHERE user_id = ?1 AND id IN (SELECT id FROM messages WHERE user_id = ?1 AND id < ?2 ORDER BY id DESC LIMIT ?3)",
                    params![&pubkey.as_bytes()[..], before, *limit as i64],
                )?,
                (Some(BeforeAfter::After(after)), None) => cached_exec(
                    &*conn,
                    "UPDATE messages SET read = true WHERE user_id = ?1 AND id IN (SELECT id FROM messages WHERE user_id = ?1 AND id > ?2 ORDER BY id ASC)",
                    params![&pubkey.as_bytes()[..], after],
                )?,
                (Some(BeforeAfter::After(after)), Some(limit)) => cached_exec(
                    &*conn,
                    "UPDATE messages SET read = true WHERE user_id = ?1 AND id IN (SELECT id FROM messages WHERE user_id = ?1 AND id > ?2 ORDER BY id ASC LIMIT ?3)",
                    params![&pubkey.as_bytes()[..], after, *limit as i64],
                )?,
                (None, None) => cached_exec(
                    &*conn,
                    "UPDATE messages SET read = true WHERE user_id = ?1 AND id IN (SELECT id FROM messages WHERE user_id = ?1 ORDER BY id DESC)",
                    params![&pubkey.as_bytes()[..]],
                )?,
                (None, Some(limit)) => cached_exec(
                    &*conn,
                    "UPDATE messages SET read = true WHERE user_id = ?1 AND id IN (SELECT id FROM messages WHERE user_id = ?1 ORDER BY id DESC LIMIT ?2)",
                    params![&pubkey.as_bytes()[..], *limit as i64],
                )?,
            };
        }
        let mapper = |row: &rusqlite::Row| {
            Ok(Message {
                id: row.get(0)?,
                tracking_id: row.get(1)?,
                time: row.get(2)?,
                inbound: row.get(3)?,
                content: row.get(4)?,
            })
        };
        let res = match (&limits.before_after, &limits.limit) {
            (Some(BeforeAfter::Before(before)), None) => cached_query_map(
                &*conn, 
                "SELECT id, tracking_id, time, inbound, content FROM messages WHERE user_id = ?1 AND id < ?2 ORDER BY id DESC",
                params![&pubkey.as_bytes()[..], before],
                mapper,
            )?,
            (Some(BeforeAfter::Before(before)), Some(limit)) => cached_query_map(
                &*conn, 
                "SELECT id, tracking_id, time, inbound, content FROM messages WHERE user_id = ?1 AND id < ?2 ORDER BY id DESC LIMIT ?3",
                params![&pubkey.as_bytes()[..], before, *limit as i64],
                mapper,
            )?,
            (Some(BeforeAfter::After(after)), None) => cached_query_map(
                &*conn,
                "SELECT id, tracking_id, time, inbound, content FROM messages WHERE user_id = ?1 AND id > ?2 ORDER BY id ASC",
                params![&pubkey.as_bytes()[..], after],
                mapper,
            )?,
            (Some(BeforeAfter::After(after)), Some(limit)) => cached_query_map(
                &*conn,
                "SELECT id, tracking_id, time, inbound, content FROM messages WHERE user_id = ?1 AND id > ?2 ORDER BY id ASC LIMIT ?3",
                params![&pubkey.as_bytes()[..], after, *limit as i64],
                mapper,
            )?,
            (None, None) => cached_query_map(
                &*conn,
                "SELECT id, tracking_id, time, inbound, content FROM messages WHERE user_id = ?1 ORDER BY id DESC",
                params![&pubkey.as_bytes()[..]],
                mapper,
            )?,
            (None, Some(limit)) => cached_query_map(
                &*conn,
                "SELECT id, tracking_id, time, inbound, content FROM messages WHERE user_id = ?1 ORDER BY id DESC LIMIT ?2",
                params![&pubkey.as_bytes()[..], *limit as i64],
                mapper,
            )?,
        };
        conn.commit()?;
        Ok::<_, Error>(res)
    })
    .await??;
    Ok(res)
}

pub async fn get_new_messages(
    pubkey: PublicKey,
    limit: Option<usize>,
    mark_as_read: bool,
) -> Result<Vec<Message>, Error> {
    let res = tokio::task::spawn_blocking(move || {
        let mut gconn = POOL.get()?;
        let conn = gconn.transaction()?;
        let id: Option<i64> = cached_query_row(
            &*conn, 
            "SELECT id FROM messages WHERE user_id = ?1 AND read = false ORDER BY id ASC LIMIT 1",
            params![&pubkey.as_bytes()[..]],
            |row| row.get(0),
        )?;
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(Vec::new());
        };
        if mark_as_read {
            if let Some(limit) = limit {
                cached_exec(
                    &*conn,
                    "UPDATE messages SET read = true WHERE user_id = ?1 AND id IN (SELECT id FROM messages WHERE user_id = ?1 AND id >= ?2 ORDER BY id ASC LIMIT ?3)",
                    params![&pubkey.as_bytes()[..], id, limit as i64]
                )?;
            } else {
                cached_exec(
                    &*conn,
                    "UPDATE messages SET read = true WHERE user_id = ?1 AND id IN (SELECT id FROM messages WHERE user_id = ?1 AND id >= ?2 ORDER BY id ASC)",
                    params![&pubkey.as_bytes()[..], id]
                )?;
            }
        }
        let mapper = |row: &rusqlite::Row| {
            Ok(Message {
                id: row.get(0)?,
                tracking_id: row.get(1)?,
                time: row.get(2)?,
                inbound: row.get(3)?,
                content: row.get(4)?,
            })
        };
        let res = if let Some(limit) = limit {
            cached_query_map(
                &*conn,
                "SELECT id, tracking_id, time, inbound, content FROM messages WHERE user_id = ?1 AND id >= ?2 ORDER BY id ASC LIMIT ?3",
                params![&pubkey.as_bytes()[..], id, limit as i64],
                mapper
            )?
        } else {
            cached_query_map(
                &*conn,
                "SELECT id, tracking_id, time, inbound, content FROM messages WHERE user_id = ?1 AND id >= ?2 ORDER BY id ASC",
                params![&pubkey.as_bytes()[..], id],
                mapper
            )?
        };
        conn.commit()?;
        Ok::<_, Error>(res)
    })
    .await??;
    Ok(res)
}
