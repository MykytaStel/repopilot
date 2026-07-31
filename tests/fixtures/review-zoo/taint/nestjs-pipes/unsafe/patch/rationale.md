The controller uses a custom pipe whose semantics are unknown and replaces a parameterized query with SQL string concatenation, so request taint must still reach the raw SQL sink.
