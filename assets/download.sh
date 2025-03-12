#!/bin/bash

DIR='./assets/downloads'
FILES=(
    "$DIR/earth.jpg" 'https://i.imgur.com/2kbKhHA.jpg'
)
for i in $(seq 0 2 $((${#FILES[@]} - 1))); do
    FILE="${FILES[$i]}"
    LINK="${FILES[$(($i + 1))]}"
    if [[ ! -e "$FILE" ]]; then
        echo "downloading $LINK to $FILE"
        curl "$LINK" -o "$FILE"
    fi
done
