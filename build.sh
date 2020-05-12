#!/bin/bash

appmgr rm cups
rm cups.s9pk
docker build --tag start9/cups .
rm image.tar
docker save start9/cups > image.tar
docker rmi start9/cups
appmgr -vv pack `pwd` -o `pwd`/cups.s9pk
appmgr -vv install cups.s9pk
