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
    --env TOR_ADDRESS=$(trim /var/lib/tor/cups_service/hostname) \
    --env TOR_KEY=$(tail -c 64 /var/lib/tor/cups_service/hs_ed25519_secret_key | base32) \
    --net bridge
```
  - Start the Docker Container
    - `docker start cups`
  - Get IP address of the Docker Container
    - `docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}'`
  - Update hidden service configuration with the container IP
    - `vim /etc/tor/torrc`
    - Change `127.0.0.1` in the HiddenServicePort config to the result of the previous step
  - Restart Tor
    - `systemctl restart tor`
  - Your cups tor address can be found by running `cat /var/lib/tor/cups_service/hostname`
