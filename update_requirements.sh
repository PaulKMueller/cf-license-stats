#!/bin/bash

# Exit if the file doesn't exist
if [ ! -f "pixi.toml" ]; then
    echo "pixi.toml not found!"
    exit 1
fi

# Flag to track when we're inside the [dependencies] section
inside_deps=false

# Truncate or create requirements.txt
> requirements.txt

while IFS= read -r line; do
    # Trim leading/trailing whitespace
    trimmed=$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

    # Skip empty lines or comments
    [[ -z "$trimmed" || "$trimmed" =~ ^# ]] && continue

    # Detect section headers
    if [[ "$trimmed" =~ ^\[[^]]+\]$ ]]; then
        if [[ "$trimmed" == "[dependencies]" ]]; then
            inside_deps=true
        else
            inside_deps=false
        fi
        continue
    fi

    # If we're inside [dependencies], process lines with =
    if $inside_deps && [[ "$trimmed" == *=* ]]; then
        key=$(echo "$trimmed" | cut -d'=' -f1 | xargs) # xargs trims whitespace
        echo "$key" >> requirements.txt
    fi

done < pixi.toml

echo "requirements.txt created with dependencies!"