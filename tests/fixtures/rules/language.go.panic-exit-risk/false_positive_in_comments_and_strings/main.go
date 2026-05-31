package main

import "fmt"

// AST precision guard: panic("boom"), log.Fatal("x"), and os.Exit(1) below
// appear only inside a comment and string literals, so AST detection must NOT
// flag them.
func describe() string {
	note := "call panic(\"x\") only at the boundary; avoid log.Fatal and os.Exit"
	return fmt.Sprintf("%s", note)
}
