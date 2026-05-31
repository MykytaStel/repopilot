// False positive guard: small, single-purpose production functions that stay
// well under the threshold must not be flagged, even though the file defines
// several of them.
pub fn add(a: i64, b: i64) -> i64 {
    a + b
}

pub fn sub(a: i64, b: i64) -> i64 {
    a - b
}

pub fn mul(a: i64, b: i64) -> i64 {
    a * b
}

pub fn negate(value: i64) -> i64 {
    -value
}
