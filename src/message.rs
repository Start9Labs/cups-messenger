use ed25519_dalek::PublicKey;
use failure::Error;

pub struct NewInboundMessage {
    pub from: PublicKey,
    pub time: i64,
    pub content: String,
}

pub struct NewOutboundMessage {
    pub to: PublicKey,
    pub time: i64,
    pub content: String,
}

lazy_static::lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::builder().proxy(crate::PROXY.clone()).build().expect("CLIENT");
}

pub async fn send(msg: NewOutboundMessage) -> Result<(), Error> {
    use sha3::{Digest, Sha3_256};

    let mut hasher = Sha3_256::new();
    hasher.input(b".onion checksum");
    hasher.input(msg.to.as_bytes());
    hasher.input(&[3]);
    let mut onion = Vec::with_capacity(35);
    onion.extend_from_slice(msg.to.as_bytes());
    onion.extend_from_slice(&hasher.result()[..2]);
    onion.push(3);
    CLIENT
        .post(&format!(
            "http://{}.onion",
            base32::encode(base32::Alphabet::RFC4648 { padding: true }, &onion)
        ))
        .body(crate::wire::encode(&*crate::SECKEY, &msg)?)
        .send()
        .await?;
    crate::db::save_out_message(msg).await?;
    Ok(())
}

pub async fn receive(msg: &[u8]) -> Result<(), Error> {
    let msg = crate::wire::parse(msg)?;
    crate::db::save_in_message(msg).await
}
