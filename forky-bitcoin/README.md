# forky-bitcoin
Fuzzing Bitcoin Core to check consensus inconsistency caused by version update using differential fuzzing.

## Install Docker
Our environment is Ubuntu18.04 (x86-64)  

    # Install Docker
    sudo apt-get install apt-transport-https ca-certificates curl software-properties-common
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
    sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu bionic stable"
    sudo apt-get update
    apt-cache policy docker-ce
    sudo apt-get install docker-ce

    # Give permission to user
    sudo usermod -aG docker $USER
    newgrp docker
    sudo service docker restart
    
## Install Docker Compose
We use docker-compose v2.1.0 to manage all contatiner 

    # Install Docker Compose
    sudo curl -L https://github.com/docker/compose/releases/download/v2.1.0/docker-compose-`uname -s`-`uname -m` -o /usr/local/bin/docker-compose
    sudo chmod +x /usr/local/bin/docker-compose
    docker-compose --version


## Build Docker Image
Build Docker images for fuzzing & testing    
We are currently building images sequentially. So, this work takes a long time.    
In each work, build a bitcoin client in one build operation.     
In current setting makefile operation is currently configured to use 8 threads. (make -j 8)     
You can change setting at ./version/#/dockerfile

    # Build Docker image
    chmod +x ./dockerBuild.sh
    ./dockerBuild.sh

## (Optional) Reset Blockchain
This operation regenerate ./import/import.dat file.     
Transactions are required to create blocks through fuzzing     
This task set the environment for fuzzing by creating several blocks before start fuzzing.    

We are currently fuzzing with fixed import.dat.    
After this operation, all test cases became invalid because block consistency will be broken.    
So, you need to regenerate the seed (very first input) for fuzzing and need to re-generate fuzzing-corpus too.    
So we do not recommend you to do this task in general.

    # This script will build bitcoin at host machine 
    # for build regtest blockchain by generating blocks
    chmod +x ./genBlockchain.sh 
    ./genBlockchain.sh 5 [Your bitcoin root]

## Run fuzzing
    chmod +x startFuzzing.sh
    ## How to use
    ## sudo ./startFuzzing [start round#] [end round#]
    sudo ./startFuzzing.sh 0 99
