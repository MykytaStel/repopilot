package com.example;

public final class App {
    public void run(boolean ready) {
        if (!ready) {
            throw new RuntimeException("not ready");
        }
    }

    public void later() {
        throw new NotImplementedError("wire implementation");
    }
}
