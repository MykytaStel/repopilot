package com.example;

public final class App {
    // AST precision guard: throw new RuntimeException("x"), throw new
    // NotImplementedError("y"), and TODO() below appear only inside a comment
    // and string literals, so AST detection must NOT flag them.
    public String describe() {
        String note = "call throw new RuntimeException(...) at the boundary; avoid TODO()";
        return note;
    }
}
