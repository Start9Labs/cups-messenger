use ed25519_dalek::PublicKey;
use failure::Error;
use uuid::Uuid;

const fn const_true() -> bool {
    true
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Query {
    #[serde(rename_all = "camelCase")]
    Users {
        #[serde(default)]
        #[serde(deserialize_with = "crate::util::deser_parse")]
        include_recent_messages: u8,
    },
    Login,
    #[serde(rename_all = "camelCase")]
    Messages {
        pubkey: String,
        #[serde(flatten)]
        limits: Limits,
        #[serde(default = "const_true")]
        mark_as_read: bool,
    },
    #[serde(rename_all = "camelCase")]
    New {
        pubkey: String,
        #[serde(deserialize_with = "crate::util::deser_parse_opt")]
        limit: Option<usize>,
    },
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Limits {
    #[serde(deserialize_with = "crate::util::deser_parse_opt")]
    pub limit: Option<usize>,
    #[serde(flatten)]
    pub before_after: Option<BeforeAfter>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeforeAfter {
    Before(#[serde(deserialize_with = "crate::util::deser_parse")] i64),
    After(#[serde(deserialize_with = "crate::util::deser_parse")] i64),
}

pub async fn handle(q: Query) -> Result<Vec<u8>, Error> {
    match q {
        Query::Users {
            include_recent_messages,
        } => get_user_info(include_recent_messages).await,
        Query::Login => Ok(Vec::new()),
        Query::Messages {
            pubkey,
            limits,
            mark_as_read,
        } => get_messages(
            PublicKey::from_bytes(
                &base32::decode(base32::Alphabet::RFC4648 { padding: false }, &pubkey)
                    .ok_or_else(|| failure::format_err!("invalid pubkey"))?,
            )?,
            limits,
            mark_as_read,
        )
        .await
        .map(|(_, a)| a),
        Query::New { pubkey, limit } => {
            get_new(
                PublicKey::from_bytes(
                    &base32::decode(base32::Alphabet::RFC4648 { padding: false }, &pubkey)
                        .ok_or_else(|| failure::format_err!("invalid pubkey"))?,
                )?,
                limit,
            )
            .await
        }
    }
}

pub async fn get_user_info(include_recent_messages: u8) -> Result<Vec<u8>, Error> {
    let dbinfo = crate::db::get_user_info().await?;
    let mut res = Vec::new();
    for info in dbinfo {
        res.extend_from_slice(info.pubkey.as_bytes());
        res.extend_from_slice(&u64::to_be_bytes(info.unreads as u64));
        if let Some(name) = info.name {
            res.push(name.as_bytes().len() as u8);
            res.extend_from_slice(name.as_bytes());
        } else {
            res.push(0);
        }
        if include_recent_messages > 0 {
            println!("including {} recent messages", include_recent_messages);
            let (count, messages) = get_messages(
                info.pubkey,
                Limits {
                    before_after: None,
                    limit: Some(include_recent_messages as usize),
                },
                false,
            )
            .await?;
            res.push(count as u8);
            res.extend(messages);
        }
    }
    Ok(res)
}

pub async fn get_messages(
    pubkey: PublicKey,
    limits: Limits,
    mark_as_read: bool,
) -> Result<(usize, Vec<u8>), Error> {
    let dbmsgs = crate::db::get_messages(pubkey, limits, mark_as_read).await?;
    let count = dbmsgs.len();
    let mut res = Vec::new();
    for msg in dbmsgs {
        if msg.inbound {
            res.push(1);
        } else {
            res.push(0);
        }
        res.extend_from_slice(&i64::to_be_bytes(msg.id));
        res.extend_from_slice(&msg.tracking_id.unwrap_or_else(Uuid::nil).as_bytes()[..]);
        res.extend_from_slice(&i64::to_be_bytes(msg.time));
        res.extend_from_slice(&u64::to_be_bytes(msg.content.as_bytes().len() as u64));
        res.extend_from_slice(msg.content.as_bytes());
    }
    Ok((count, res))
}

pub async fn get_new(pubkey: PublicKey, limit: Option<usize>) -> Result<Vec<u8>, Error> {
    let dbmsgs = crate::db::get_new_messages(pubkey, limit, true).await?;
    let mut res = Vec::new();
    for msg in dbmsgs {
        if msg.inbound {
            res.push(1);
        } else {
            res.push(0);
        }
        res.extend_from_slice(&i64::to_be_bytes(msg.id));
        res.extend_from_slice(&msg.tracking_id.unwrap_or_else(Uuid::nil).as_bytes()[..]);
        res.extend_from_slice(&i64::to_be_bytes(msg.time));
        res.extend_from_slice(&u64::to_be_bytes(msg.content.as_bytes().len() as u64));
        res.extend_from_slice(msg.content.as_bytes());
    }
    Ok(res)
}
