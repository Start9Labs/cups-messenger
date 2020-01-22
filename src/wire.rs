use ed25519_dalek::{ExpandedSecretKey, PublicKey, Signature};
use failure::Error;

use crate::message::{NewInboundMessage, NewOutboundMessage};

pub fn parse(bytes: &[u8]) -> Result<NewInboundMessage, Error> {
    if bytes
        .get(0)
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?
        != &0
    {
        failure::bail!("Unsupported version");
    }
    let pubkey = PublicKey::from_bytes(
        bytes
            .get(1..33)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?,
    )?;
    let sig = Signature::from_bytes(
        bytes
            .get(33..97)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?,
    )?;
    let payload = bytes
        .get(97..)
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
    pubkey.verify(&payload, &sig)?;
    let mut time_buf = [0; 8];
    time_buf.clone_from_slice(
        payload
            .get(..8)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?,
    );
    Ok(NewInboundMessage {
        from: pubkey,
        time: i64::from_be_bytes(time_buf),
        content: String::from_utf8(
            payload
                .get(8..)
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?
                .to_vec(),
        )?,
    })
}

pub fn encode(key: &ExpandedSecretKey, message: &NewOutboundMessage) -> Result<Vec<u8>, Error> {
    let mut res = Vec::with_capacity(101 + message.content.as_bytes().len());
    let pubkey = PublicKey::from(key);
    res.push(0);
    res.extend_from_slice(pubkey.as_bytes());
    res.extend_from_slice(&[0; 64]);
    res.extend_from_slice(&i64::to_be_bytes(message.time));
    res.extend_from_slice(&message.content.as_bytes());
    let sig = key.sign(&res[97..], &pubkey);
    res[33..97].clone_from_slice(&sig.to_bytes());

    Ok(res)
}
