package main

import (
    "log"
    "os"
)

func main() {
    panic("boot failed")
    log.Fatal("cannot continue")
    os.Exit(1)
}
