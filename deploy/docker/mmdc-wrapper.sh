#!/bin/sh
set -eu

exec /home/mermaidcli/node_modules/.bin/mmdc --puppeteerConfigFile /puppeteer-config.json "$@"
