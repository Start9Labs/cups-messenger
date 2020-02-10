#!/bin/bash

wget https://cups-ui.s3.amazonaws.com/cups-ui-0.1.1.tar.gz
tar -C assets/ -xvf cups-ui-0.1.1.tar.gz
docker build --tag start9/cups .
docker save start9/cups > image.tar
docker rmi start9/cups
appmgr -vv pack `pwd` -o `pwd`/cups.s9pk
