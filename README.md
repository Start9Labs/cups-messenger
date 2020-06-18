# cups

## API

### Authorization

Cups uses Basic Auth

- The username is always `me`.
- The password is defined in `./start9/config.yaml`

### Send Message

#### Request

`POST` with body `0x00 <Tracking ID (UUID BE)> <ED25519 PubKey of Recipient (32 bytes)> <UTF-8 Encoded Message>`

### Name User

#### Request

`POST` with body `0x01 <ED25519 PubKey of User> <UTF-8 Encoded Name>`

### Get Contact Book

#### Request

`GET` with query `?type=users`

#### Response

`<User Info>*` where `<User Info>` = `<ED25519 PubKey of User> <Unreads Count (u64 BE)> <Length of Name (1 byte)> <UTF-8 Encoded Name>`

### Get Messages

#### Request

`GET` with query `?type=messages&pubkey=<RFC4648 Base32 encoded ED25519 PubKey of User>&limit=<Maximum number of messages to return>`

#### Response

`<Message>*` in reverse chronological order where `<Message>` = `<0x00 for Inbound / 0x01 for Outbound> <ID (i64 BE)> <Tracking ID (UUID BE)> <Unix Epoch (i64 BE)> <Length of Message (u64 BE)> <UTF-8 Encoded Message>`

### Get Version

#### Request

Unauthenticated `GET` with no query

#### Response

`<Major Version (u64 BE)> <Minor Version (u64 BE)> <Patch Version (u64 BE)>` 

## Building s9pk for Embassy

from cups dir on x86
```bash
rust-musl-builder cargo +beta build --release
rust-musl-builder musl-strip ./target/armv7-unknown-linux-musleabihf/release/cups
scp ./target/armv7-unknown-linux-musleabihf/release/cups <EMBASSY>:<path/to/cups>/target/armv7-unknown-linux-musleabihf/release/cups
cd cups-messenger-ui
npm i
npm run build-prod
ssh <EMBASSY> "rm -rf <path/to/cups>/assets/www"
scp -r www <EMBASSY>:<path/to/cups>/assets
```

from cups dir on EMBASSY
```bash
sudo appmgr rm cups
docker build --tag start9/cups .
docker save start9/cups > image.tar
docker rmi start9/cups
sudo appmgr pack $(pwd) -o cups.s9pk
```

## Building for Non-Embassy devices

See NONEMBASSY.md

## Publishing a new version
  - Update semver in:
    - src/main.rs
    - Cargo.toml
    - cups-messenger-ui/package.json
    - assets/httpd.conf
    - manifest.yaml (release notes too)
  - [Build s9pk for Embassy](https://github.com/Start9Labs/cups-messenger/blob/master/README.md#building-s9pk-for-embassy)
  - [Publish](https://github.com/Start9Labs/operations/blob/master/PUBLISHING.md)