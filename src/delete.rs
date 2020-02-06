use ed25519_dalek::PublicKey;
use failure::Error;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Query {
    User { pubkey: String },
}

pub async fn handle(q: Query) -> Result<(), Error> {
    match q {
        Query::User { pubkey } => {
            crate::db::del_user(PublicKey::from_bytes(
                &base32::decode(base32::Alphabet::RFC4648 { padding: false }, &pubkey)
                    .ok_or_else(|| failure::format_err!("invalid pubkey"))?,
            )?)
            .await
        }
    }
}
