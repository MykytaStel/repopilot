package main

import "fmt"

func validate(value string) error {
    if value == "" {
        return fmt.Errorf("value is required")
    }
    return nil
}
