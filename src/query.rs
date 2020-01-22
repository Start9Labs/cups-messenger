use ed25519_dalek::PublicKey;
use failure::Error;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Query {
    Users,
    Messages {
        pubkey: String,
        limit: Option<usize>,
    },
}

pub async fn handle(q: Query) -> Result<Vec<u8>, Error> {
    match q {
        Query::Users => get_user_info().await,
        Query::Messages { pubkey, limit } => {
            get_messages(
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

pub async fn get_user_info() -> Result<Vec<u8>, Error> {
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
    }
    Ok(res)
}

pub async fn get_messages(pubkey: PublicKey, limit: Option<usize>) -> Result<Vec<u8>, Error> {
    let dbmsgs = crate::db::get_messages(pubkey, limit, true).await?;
    let mut res = Vec::new();
    for msg in dbmsgs {
        if msg.inbound {
            res.push(1);
        } else {
            res.push(0);
        }
        res.extend_from_slice(&i64::to_be_bytes(msg.time));
        res.extend_from_slice(&u64::to_be_bytes(msg.content.as_bytes().len() as u64));
        res.extend_from_slice(msg.content.as_bytes());
    }
    Ok(res)
}
