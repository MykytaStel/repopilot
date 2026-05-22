pub fn build_regex() {
    let _pattern = Regex::new("^[a-z]+$").expect("valid regex");
}

pub fn comment_only() {
    // panic!("this is documentation inside a comment");
}
