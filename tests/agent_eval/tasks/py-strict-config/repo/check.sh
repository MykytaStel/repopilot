#!/bin/sh
set -e
mypy
python3 -m unittest -q
