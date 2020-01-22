#!/bin/bash

docker build --tag start9/cups .
docker save start9/cups > image.tar
docker rmi start9/cups
appmgr -vv pack `pwd` -o `pwd`/cups.s9pk
