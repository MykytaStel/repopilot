//! Shared recognition for explicit nested-shell command vectors.

pub(super) fn is_shell_executable(raw: &str) -> bool {
    let executable = raw
        .trim_matches(['\'', '"', '`'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw)
        .to_ascii_lowercase();
    let executable = executable.strip_suffix(".exe").unwrap_or(&executable);
    ["sh", "bash", "dash", "zsh", "cmd", "powershell", "pwsh"]
        .iter()
        .any(|shell| executable.eq_ignore_ascii_case(shell))
}

pub(super) fn is_shell_command_flag(raw: &str) -> bool {
    let flag = raw.trim_matches(['\'', '"', '`']);
    if [
        "/c",
        "/k",
        "-command",
        "-encodedcommand",
        "-e",
        "-ec",
        "-enc",
    ]
    .iter()
    .any(|known| flag.eq_ignore_ascii_case(known))
    {
        return true;
    }
    let Some(short_options) = flag.strip_prefix('-') else {
        return false;
    };
    !short_options.starts_with('-')
        && short_options.contains('c')
        && short_options
            .chars()
            .all(|option| "bcefhiklmnpsuvxBCEH".contains(option))
}

pub(super) fn has_shell_command_flag<'a>(args: impl IntoIterator<Item = &'a str>) -> bool {
    args.into_iter().any(is_shell_command_flag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_shell_paths_case_insensitively() {
        assert!(is_shell_executable("`C:\\tools\\BASH.EXE`"));
        assert!(is_shell_executable("'/usr/bin/pwsh'"));
        assert!(!is_shell_executable("git"));
    }

    #[test]
    fn recognizes_shell_command_options_without_matching_long_options() {
        for flag in ["-c", "-lc", "-xc", "-ec", "/C", "-Command"] {
            assert!(is_shell_command_flag(flag), "{flag}");
        }
        for flag in ["-EncodedCommand", "-eNc", "/k"] {
            assert!(is_shell_command_flag(flag), "{flag}");
        }
        for flag in ["--color", "-n", "/q", "-execute", "-encodedarg"] {
            assert!(!is_shell_command_flag(flag), "{flag}");
        }
    }

    #[test]
    fn scans_all_shell_arguments_for_command_modes() {
        assert!(has_shell_command_flag(["-l", "-c", "echo hi"]));
        assert!(has_shell_command_flag([
            "-NoProfile",
            "-EncodedCommand",
            "abc"
        ]));
        assert!(!has_shell_command_flag(["--color", "-NoProfile"]));
    }
}
