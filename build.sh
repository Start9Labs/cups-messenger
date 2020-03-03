#!/bin/bash

appmgr rm --purge cups
rm cups.s9pk
cp ../cups${1}.tar.gz .
echo "building cups${1}"
rm -rf assets/www
tar -C assets -xvf cups${1}.tar.gz
#docker build --tag start9/cups .
#docker save start9/cups > image.tar
#docker rmi start9/cups
echo "building 2 cups$"
appmgr -vv pack `pwd` -o `pwd`/cups.s9pk
appmgr -vv install cups.s9pk
echo "password: pass" | appmgr configure cups --stdin
appmgr start cups
appmgr tor show cups
#rm ../cups${1}.tar.gz
rm cups${1}.tar.gz
