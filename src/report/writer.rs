use std::fs;
use std::io;
use std::path::Path;

pub fn write_report(content: &str, output_path: Option<&Path>) -> io::Result<()> {
    match output_path {
        Some(path) => write_to_file(content, path),
        None => write_to_stdout(content),
    }
}

fn write_to_file(content: &str, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)
}

fn write_to_stdout(content: &str) -> io::Result<()> {
    println!("{content}");
    Ok(())
}
