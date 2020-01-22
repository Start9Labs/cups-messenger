use std::convert::Infallible;
use std::net::SocketAddr;

use ed25519_dalek::PublicKey;
use failure::Error;
use futures::StreamExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};

mod db;
mod message;
mod query;
mod wire;

#[derive(serde::Deserialize)]
pub struct Config {
    pub password: String,
}

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = serde_yaml::from_reader(std::fs::File::open("./start9/config.yaml").expect("./start9/config.yaml")).expect("./start9/config.yaml");
    pub static ref PROXY: reqwest::Proxy = reqwest::Proxy::all(&format!("http://{}:9050", std::env::var("HOST_IP").expect("HOST_IP"))).expect("PROXY");
    pub static ref SECKEY: ed25519_dalek::ExpandedSecretKey =
        ed25519_dalek::ExpandedSecretKey::from_bytes(
            &base32::decode(
                base32::Alphabet::RFC4648 { padding: true },
                &std::env::var("TOR_KEY").expect("TOR_KEY"),
            ).expect("TOR_KEY"),
        ).expect("TOR_KEY");
}

async fn get_bytes(body: &mut Body) -> Result<Vec<u8>, Error> {
    let mut res = Vec::new();
    while {
        if let Some(chunk) = body.next().await {
            res.extend_from_slice(&*chunk?);
            true
        } else {
            false
        }
    } {}
    Ok(res)
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Error> {
    let res = handler(req).await;
    match &res {
        Ok(_) => eprintln!("OK"),
        Err(e) => eprintln!("ERROR: {}", e),
    };
    res
}

async fn handler(mut req: Request<Body>) -> Result<Response<Body>, Error> {
    match req.method() {
        &Method::POST => match req.headers().get("Authorization") {
            Some(auth)
                if auth
                    == &format!(
                        "Basic {}",
                        base64::encode(&format!("me:{}", &*CONFIG.password))
                    ) =>
            {
                let req_data = get_bytes(req.body_mut()).await?;
                if req_data.len() < 33 {
                    Response::builder()
                        .status(400)
                        .body(Body::empty())
                        .map_err(From::from)
                } else {
                    match req_data[0] {
                        0 => crate::message::send(crate::message::NewOutboundMessage {
                            to: PublicKey::from_bytes(&req_data[1..33])?,
                            time: std::time::UNIX_EPOCH
                                .elapsed()
                                .map(|a| a.as_secs() as i64)
                                .unwrap_or_else(|a| a.duration().as_secs() as i64 * -1),
                            content: String::from_utf8(req_data[33..].to_vec())?,
                        })
                        .await
                        .map(|_| Body::empty())
                        .map(Response::new),
                        1 => crate::db::save_user(
                            PublicKey::from_bytes(&req_data[1..33])?,
                            String::from_utf8(req_data[33..].to_vec())?,
                        )
                        .await
                        .map(|_| Body::empty())
                        .map(Response::new),
                        _ => Response::builder()
                            .status(400)
                            .body(Body::empty())
                            .map_err(From::from),
                    }
                }
            }
            _ => crate::message::receive(&get_bytes(req.body_mut()).await?)
                .await
                .map(|_| Body::empty())
                .map(Response::new),
        },
        &Method::GET => match (req.headers().get("Authorization"), req.uri().query()) {
            (Some(auth), Some(query)) if auth == &format!("Basic {}", &*CONFIG.password) => {
                match serde_urlencoded::from_str(query) {
                    Ok(q) => crate::query::handle(q)
                        .await
                        .map(Body::from)
                        .map(Response::new),
                    Err(e) => Response::builder()
                        .status(400)
                        .body(Body::from(format!("{}", e)))
                        .map_err(From::from),
                }
            }
            (_, None) => Response::builder()
                .status(400)
                .body(Body::empty())
                .map_err(From::from),
            _ => Response::builder()
                .status(401)
                .body(Body::empty())
                .map_err(From::from),
        },
        _ => Response::builder()
            .status(405)
            .body(Body::empty())
            .map_err(From::from),
    }
}

#[tokio::main]
async fn main() {
    let mig = crate::db::migrate();
    // Construct our SocketAddr to listen on...
    let addr = SocketAddr::from(([0, 0, 0, 0], 59001));

    // And a MakeService to handle each connection...
    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    // Then bind and serve...
    let server = Server::bind(&addr).serve(make_service);

    mig.await.expect("migration");
    // And run forever...
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
