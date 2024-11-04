#!/bin/bash

# Directory containing the test cases
testcase_dir="./testcases"

# Directory where the test cases will be split
split_dir="./testcases_splitted"

# Number of groups to split the test cases into
N_GROUPS=8

# Remove existing one and Create the target directory if it doesn't exist
rm -rf "$split_dir"
mkdir -p "$split_dir"

# Counter for the subdirectories
count=0

# Loop through the directories and split them into groups
for directory in "$testcase_dir"/*; do
    # Calculate the group number
    group_number=$((count % N_GROUPS))

    # Create the target subdirectory if it doesn't exist
    target_subdirectory="$split_dir/group_$group_number"
    mkdir -p "$target_subdirectory"

    # Copy the directory to the target subdirectory
    cp -r "$directory" "$target_subdirectory/"

    # Output progress
    echo "Copied: $directory to $target_subdirectory"

    # Increment the counter
    ((count++))
done

echo "Done!"
