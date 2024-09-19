#!/bin/bash

export UID="$(id -u)"
export GID="$(id -g)"

docker compose run --name $1 -u $(id -u) -i $1 bash
