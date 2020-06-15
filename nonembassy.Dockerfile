FROM alpine

EXPOSE 59001 80

RUN apk add lighttpd

ADD ./target/release/cups /usr/local/bin/cups
RUN chmod a+x /usr/local/bin/cups
ADD ./docker_entrypoint.sh /usr/local/bin/docker_entrypoint.sh
RUN chmod a+x /usr/local/bin/docker_entrypoint.sh

ENTRYPOINT ["/usr/local/bin/docker_entrypoint.sh"]
