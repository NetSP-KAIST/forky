# forky-eth1

# How to Generate Testcases with Fuzzer
```
# Install Go
sudo apt install golang
# set your PATH to include go
go version
# We've tested with go version go1.21.5 linux/amd64
```

```
cd ./go-ethereum/core
chmod +x timeout.sh
mkdir -p corpus/FuzzForky
./timeout.sh
```
