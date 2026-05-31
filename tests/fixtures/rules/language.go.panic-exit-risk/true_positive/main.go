package main

import (
    "log"
    "os"
)

func main() {
    panic("boot failed")
    log.Fatal("cannot continue")
    log.Fatalf("exit code %d", 1)
    os.Exit(1)
}
