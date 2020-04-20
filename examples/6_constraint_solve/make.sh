#!/bin/sh

LIB_PATH="../"
gcc -L $LIB_PATH -lreowolf_rs -Wl,-R$LIB_PATH main.c -o main
