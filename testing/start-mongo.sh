#!/bin/bash
path=$(dirname $(realpath "$0"))
mongod --dbpath="$path/data" --bind_ip="127.0.0.1" --port=27017
