use std::convert::Infallible;
use std::net::SocketAddr;

use ed25519_dalek::PublicKey;
use failure::Error;
use hyper::body::HttpBody;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use uuid::Uuid;

mod db;
mod delete;
mod message;
mod migrations;
mod query;
mod util;
mod wire;

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub password: String,
    pub address_private_key: String,
}

lazy_static::lazy_static! {
    pub static ref MAJOR: [u8; 8] = u64::to_be_bytes(env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap());
    pub static ref MINOR: [u8; 8] = u64::to_be_bytes(env!("CARGO_PKG_VERSION_MINOR").parse().unwrap());
    pub static ref PATCH: [u8; 8] = u64::to_be_bytes(env!("CARGO_PKG_VERSION_PATCH").parse().unwrap());
    pub static ref VERSION: [u8; 24] = {
        let mut version = [0; 24];
        version[..8].clone_from_slice(&*MAJOR);
        version[8..16].clone_from_slice(&*MINOR);
        version[16..].clone_from_slice(&*PATCH);
        version
    };
    pub static ref CONFIG: Config = serde_yaml::from_reader(std::fs::File::open("./start9/config.yaml").expect("./start9/config.yaml")).expect("./start9/config.yaml");
    pub static ref PROXY: reqwest::Proxy = reqwest::Proxy::http("socks5h://embassy:9050").expect("PROXY");
    pub static ref SECKEY: ed25519_dalek::ExpandedSecretKey =
        ed25519_dalek::ExpandedSecretKey::from_bytes(
            &base32::decode(
                base32::Alphabet::RFC4648 { padding: false },
                &CONFIG.address_private_key,
            ).expect("TOR_KEY"),
        ).expect("TOR_KEY");
}

async fn get_bytes(body: &mut Body) -> Result<Vec<u8>, Error> {
    let mut res = Vec::new();
    while {
        if let Some(chunk) = body.data().await {
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
        Ok(_) => {
            eprintln!("OK");
            res
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
            Response::builder()
                .status(500)
                .body(format!("{}", e).into())
                .map_err(From::from)
        }
    }
}

async fn handler(mut req: Request<Body>) -> Result<Response<Body>, Error> {
    match req.method() {
        &Method::POST => match req.headers().get("Authorization") {
            Some(auth) => {
                if auth
                    == &format!(
                        "Basic {}",
                        base64::encode(&format!("me:{}", &*CONFIG.password))
                    )
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
                                tracking_id: Some(Uuid::from_slice(&req_data[1..17])?)
                                    .filter(|a| !a.is_nil()),
                                to: PublicKey::from_bytes(&req_data[17..49])?,
                                time: std::time::UNIX_EPOCH
                                    .elapsed()
                                    .map(|a| a.as_secs() as i64)
                                    .unwrap_or_else(|a| a.duration().as_secs() as i64 * -1),
                                content: String::from_utf8(req_data[49..].to_vec())?,
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
                } else {
                    Response::builder()
                        .status(401)
                        .body(Body::empty())
                        .map_err(From::from)
                }
            }
            _ => crate::message::receive(&get_bytes(req.body_mut()).await?)
                .await
                .map(|_| Body::empty())
                .map(Response::new),
        },
        &Method::GET => match (req.headers().get("Authorization"), req.uri().query()) {
            (Some(auth), Some(query))
                if auth
                    == &format!(
                        "Basic {}",
                        base64::encode(&format!("me:{}", &*CONFIG.password))
                    ) =>
            {
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
            (_, None) => Ok(Response::new(Body::from(&VERSION[..]))),
            _ => Response::builder()
                .status(401)
                .body(Body::empty())
                .map_err(From::from),
        },
        &Method::DELETE => match (req.headers().get("Authorization"), req.uri().query()) {
            (Some(auth), Some(query))
                if auth
                    == &format!(
                        "Basic {}",
                        base64::encode(&format!("me:{}", &*CONFIG.password))
                    ) =>
            {
                match serde_urlencoded::from_str(query) {
                    Ok(q) => crate::delete::handle(q)
                        .await
                        .map(|_| Body::empty())
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

#[derive(Clone, Debug, serde::Serialize)]
pub struct Metrics<'a> {
    version: u8,
    data: Data<'a>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Data<'a> {
    password: Metric<'a>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Metric<'a> {
    #[serde(rename = "type")]
    value_type: &'static str,
    value: &'a str,
    description: Option<&'static str>,
    copyable: bool,
    qr: bool,
    masked: bool,
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    println!("USING PROXY: {:?}", &*PROXY);
    &*CONFIG;
    let data = Data {
        password: Metric {
            value_type: "string",
            value: &CONFIG.password,
            description: Some("Password for authenticating to your Cups service. This password can be updated in the Cups config page."),
            copyable: true,
            qr: false,
            masked: true,
        }
    };
    serde_yaml::to_writer(
        std::fs::File::create("/root/start9/.stats.yaml.tmp").unwrap(),
        &Metrics { version: 2, data },
    )
    .unwrap();
    std::fs::rename("./start9/.stats.yaml.tmp", "./start9/stats.yaml").unwrap();

    let mig = crate::migrations::migrate();
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
