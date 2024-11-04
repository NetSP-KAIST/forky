#!/bin/bash

while true
do
    # Start the command in the background
    go test -v -fuzz=FuzzForky -run=^$ -test.fuzzcachedir ./corpus -parallel 1 &

    # Save its PID
    PID=$!
    echo "$PID"

    # Wait
    sleep 60

    # Kill the process
    pkill -9 -f FuzzForky

    # Wait for the process to terminate
    wait $PID
done