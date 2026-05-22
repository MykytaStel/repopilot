package com.example;

public final class App {
    public void run(boolean ready) throws IllegalArgumentException {
        if (!ready) {
            throw new IllegalArgumentException("not ready");
        }
    }
}
