# Building and running on a device other than a Start9 Embassy
This guide assumes you are running a debian-based operating system and are logged in as root.

  - Clone the repo
    - `git clone https://github.com/Start9Labs/cups-messenger.git`
    - This document assumes you have cloned to `~/cups-messenger`
    - `cd ~/cups-messenger`
    - `git submodule update --init`
  - Install Tor
    - `apt install tor`
  - Set up Tor Hidden Service
    - `vim /etc/tor/torrc`
    - Add the following lines:
```
SOCKSPort 0.0.0.0:9050 # This makes your Tor proxy available to your network. If your server is not behind a NAT that you control, make sure to set a SOCKS policy, or bind to the host ip on the docker0 interface

HiddenServiceDir /var/lib/tor/cups_service
HiddenServicePort 80 127.0.0.1:80
HiddenServicePort 59001 127.0.0.1:59001
HiddenServiceVersion 3
```
  - Restart Tor
    - `systemctl restart tor`
  - Create a mount point for the container
    - `mkdir -p /var/opt/cups/start9`
  - Write config file
    - `vim /var/opt/cups/start9/config.yaml`
    - Add `password: <your password>` with the password you want to use
  - Build Cups UI
    - `cd cups-messenger-ui`
    - Build
      - `npm i`
      - `npm run build-prod`
    - Copy over result
      - `cp -r www ../assets/www`
    - Return to Cups
      - `cd ..`
  - Copy over assets
    - `cp -r assets/* /var/opt/cups`
  - Build cups server
    - Make sure you have the [Rust toolchain](https://rustup.rs)
    - `cargo build --release`
    - NOTE: the docker image is designed for musl. If you are not on a musl platform, you must cross compile.
      - [Here](https://github.com/messense/rust-musl-cross) is a useful tool for cross compiling to musl.
      - You must also replace `target/release` with `target/<your platform>/release` everwhere in this guide, as well as in nonembassy.Dockerfile
    - (Optional) `strip target/release/cups`
  - Install Docker
    - `apt install docker.io`
  - Build the Docker Image
    - `docker build . -f nonembassy.Dockerfile -t start9/cups`
  - Create the Docker Container
```bash
docker create \
    --restart unless-stopped \
    --name cups \
    --mount type=bind,src=/var/opt/cups,dst=/root \
    --env TOR_ADDRESS=$(cat /var/lib/tor/cups_service/hostname | sed 's/\n*$//g') \
    --env TOR_KEY=$(tail -c 64 /var/lib/tor/cups_service/hs_ed25519_secret_key | base32 -w0 | sed 's/\n*$//g') \
    --net bridge \
    start9/cups
```
  - Start the Docker Container
    - `docker start cups`
  - Get IP address of the Docker Container
    - `docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' cups`
  - Update hidden service configuration with the container IP
    - `vim /etc/tor/torrc`
    - Change `127.0.0.1` in the HiddenServicePort config to the result of the previous step
  - Restart Tor
    - `systemctl restart tor`
  - Your cups tor address can be found by running `cat /var/lib/tor/cups_service/hostname`
