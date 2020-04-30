#!/bin/sh

LIB_PATH="../../target/release"
gcc -L $LIB_PATH -lreowolf_rs -Wl,-R$LIB_PATH amy.c -o amy