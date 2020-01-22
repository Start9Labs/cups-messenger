#!/bin/bash

docker build --tag start9/whisper .
docker save start9/whisper > image.tar
docker rmi start9/whisper
appmgr -vv pack `pwd` -o `pwd`/whisper.s9pk
