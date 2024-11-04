#!/bin/bash
echo "############################" 
echo 'Build bitcoin base image...'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:base ./version/bitcoin-core/base


echo "############################" 
echo 'Build bitcoin base-22.04 image...'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:base-22.04 ./version/bitcoin-core/base-22.04

echo
echo "############################" 
echo 'Build bitcoin v27.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v27.1 ./version/bitcoin-core/v27.1

echo
echo "############################" 
echo 'Build bitcoin v26.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v26.0 ./version/bitcoin-core/v26.0

echo
echo "############################" 
echo 'Build bitcoin v25.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v25.0 ./version/bitcoin-core/v25.0

echo
echo "############################" 
echo 'Build bitcoin v24.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v24.1 ./version/bitcoin-core/v24.1

echo
echo "############################" 
echo 'Build bitcoin v23.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v23.0 ./version/bitcoin-core/v23.0

echo
echo "############################" 
echo 'Build bitcoin v22.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v22.0 ./version/bitcoin-core/v22.0

echo
echo "############################" 
echo 'Build bitcoin v0.21.2 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.21.2 ./version/bitcoin-core/v0.21.2

echo
echo "############################" 
echo 'Build bitcoin v0.21.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.21.1 ./version/bitcoin-core/v0.21.1

echo
echo "############################" 
echo 'Build bitcoin v0.21.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.21.0 ./version/bitcoin-core/v0.21.0

echo
echo "############################" 
echo 'Build bitcoin v0.20.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.20.1 ./version/bitcoin-core/v0.20.1

echo
echo "############################" 
echo 'Build bitcoin v0.20.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.20.0 ./version/bitcoin-core/v0.20.0

echo
echo "############################" 
echo 'Build bitcoin v0.19.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.19.1 ./version/bitcoin-core/v0.19.1

echo
echo "############################" 
echo 'Build bitcoin v0.19.0.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.19.0.1 ./version/bitcoin-core/v0.19.0.1

echo
echo "############################" 
echo 'Build bitcoin v0.19.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.19.0 ./version/bitcoin-core/v0.19.0

echo
echo "############################" 
echo 'Build bitcoin v0.18.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.18.1 ./version/bitcoin-core/v0.18.1

echo
echo "############################" 
echo 'Build bitcoin v0.18.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.18.0 ./version/bitcoin-core/v0.18.0

echo
echo "############################" 
echo 'Build bitcoin v0.17.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.17.1 ./version/bitcoin-core/v0.17.1

echo
echo "############################" 
echo 'Build bitcoin v0.17.0.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.17.0.1 ./version/bitcoin-core/v0.17.0.1

echo
echo "############################" 
echo 'Build bitcoin v0.17.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.17.0 ./version/bitcoin-core/v0.17.0

echo
echo "############################" 
echo 'Build bitcoin v0.16.3 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.16.3 ./version/bitcoin-core/v0.16.3

echo
echo "############################" 
echo 'Build bitcoin v0.16.2 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.16.2 ./version/bitcoin-core/v0.16.2

echo
echo "############################" 
echo 'Build bitcoin v0.16.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.16.1 ./version/bitcoin-core/v0.16.1

echo
echo "############################" 
echo 'Build bitcoin v0.16.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.16.0 ./version/bitcoin-core/v0.16.0

echo
echo "############################" 
echo 'Build bitcoin v0.15.2 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.15.2 ./version/bitcoin-core/v0.15.2

echo
echo "############################" 
echo 'Build bitcoin v0.15.1 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.15.1 ./version/bitcoin-core/v0.15.1

echo
echo "############################" 
echo 'Build bitcoin v0.15.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v0.15.0 ./version/bitcoin-core/v0.15.0

echo
echo "############################" 
echo 'Build bitcoin knots v23.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v23.0-knots ./version/bitcoin-knots/v23.0-knots

echo
echo "############################" 
echo 'Build bitcoin knots v22.0 image'
echo "############################" 
echo
docker build -t ubuntu-bitcoin:v22.0-knots ./version/bitcoin-knots/v22.0-knots

echo "############################" 
echo 'Build btcd base image...'
echo "############################" 
echo
docker build -t ubuntu-btcd:base ./version/btcd/base

echo
echo "############################" 
echo 'Build btcd v0.23.1 image'
echo "############################" 
echo
docker build -t ubuntu-btcd:v0.23.1 ./version/btcd/v0.23.1

echo
echo "############################" 
echo 'Build btcd v0.23.0 image'
echo "############################" 
echo
docker build -t ubuntu-btcd:v0.23.0 ./version/btcd/v0.23.0

echo
echo "############################" 
echo 'Build btcd v0.22.1 image'
echo "############################" 
echo
docker build -t ubuntu-btcd:v0.22.1 ./version/btcd/v0.22.1

echo "############################" 
echo 'Build bcoin base image...'
echo "############################" 
echo
docker build -t ubuntu-bcoin:base ./version/bcoin/base

echo
echo "############################" 
echo 'Build bcoin v2.2.0 image'
echo "############################" 
echo
docker build -t ubuntu-bcoin:v2.2.0 ./version/bcoin/v2.2.0

echo
echo "############################" 
echo 'Build bcoin v2.1.0 image'
echo "############################" 
echo
docker build -t ubuntu-bcoin:v2.1.0 ./version/bcoin/v2.1.0

echo
echo "############################" 
echo 'Build bcoin v2.0.0 image'
echo "############################" 
echo
docker build -t ubuntu-bcoin:v2.0.0 ./version/bcoin/v2.0.0

echo "############################" 
echo 'Build validation image...'
echo "############################" 
echo
docker build -t ubuntu-validation ./validation

echo
echo "############################" 
echo 'Try remove <none> images...'
echo "############################" 
echo
docker rmi $(docker images -f "dangling=true" -q)

echo
echo "############################" 
echo 'Build finished'
echo "############################" 
echo