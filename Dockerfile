FROM alpine

EXPOSE 59001

ADD ./target/armv7-unknown-linux-musleabihf/release/cups /usr/local/bin/cups
ADD ./docker_entrypoint.sh /usr/local/bin/docker_entrypoint.sh
RUN chmod a+x /usr/local/bin/docker_entrypoint.sh

ENTRYPOINT ["/usr/local/bin/docker_entrypoint.sh"]
