#!/bin/bash

appmgr rm cups
rm cups.s9pk
cp ../cups${1}.tar.gz .
echo "building cups${1}"
rm -rf assets/www
tar -C assets -xvf cups${1}.tar.gz
appmgr -vv pack `pwd` -o `pwd`/cups.s9pk
appmgr install cups.s9pk
#rm ../cups${1}.tar.gz
rm cups${1}.tar.gz
