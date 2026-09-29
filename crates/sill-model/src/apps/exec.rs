//! Turning an entry's `Exec` into an argument vector (Desktop Entry Specification, "The Exec
//! key": quoting, then field codes `%f %F %u %U %i %c %k %%`; the deprecated `%d %D %n %N %v
//! %m` expand to nothing).
//!
//! The input is the value after the desktop-file level escapes (`\s`, `\n`, `\\`, ...) have been
//! applied by the entry parser; this function applies the second, Exec-level quoting.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// What an `Exec` line is expanded with.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExecArgs {
    /// Local files (`%f` takes the first, `%F` all).
    pub files: Vec<PathBuf>,
    /// URLs (`%u` takes the first, `%U` all). When empty, `%u`/`%U` fall back to `files`.
    pub uris: Vec<String>,
    /// `Icon=`, for `%i` (`--icon <icon>`); empty for none.
    pub icon: String,
    /// The localised `Name=`, for `%c`.
    pub name: String,
    /// The `.desktop` file, for `%k`; `None` expands to nothing.
    pub desktop_file: Option<PathBuf>,
}

/// Why an `Exec` line could not be expanded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExecError {
    #[error("Exec is empty")]
    Empty,
    #[error("Exec has an unterminated double quote")]
    UnterminatedQuote,
    #[error("Exec uses an unknown field code %{0}")]
    UnknownFieldCode(char),
    #[error("Exec uses %{0} inside a larger argument; it must stand alone")]
    FieldCodeNotAlone(char),
}

/// One piece of an argument: text from inside double quotes is literal; bare text has its
/// field codes expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Quoted(String),
    Bare(String),
}

/// `exec` as `argv`, with `args` substituted.
pub fn expand_exec(exec: &str, args: &ExecArgs) -> Result<Vec<String>, ExecError> {
    let mut argv = Vec::new();
    for token in tokenize(exec)? {
        argv.extend(expand_token(&token, args)?);
    }
    if argv.is_empty() {
        return Err(ExecError::Empty);
    }
    Ok(argv)
}

/// Split on unquoted spaces and tabs; inside double quotes, `\` escapes the next character.
fn tokenize(exec: &str) -> Result<Vec<Vec<Piece>>, ExecError> {
    let mut tokens = Vec::new();
    let mut token: Vec<Piece> = Vec::new();
    let mut bare = String::new();
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => {
                if !bare.is_empty() {
                    token.push(Piece::Bare(std::mem::take(&mut bare)));
                }
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            '"' => {
                if !bare.is_empty() {
                    token.push(Piece::Bare(std::mem::take(&mut bare)));
                }
                token.push(Piece::Quoted(quoted(&mut chars)?));
            }
            other => bare.push(other),
        }
    }
    if !bare.is_empty() {
        token.push(Piece::Bare(bare));
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    Ok(tokens)
}

/// The rest of a double-quoted string, the closing quote consumed.
fn quoted(chars: &mut std::str::Chars<'_>) -> Result<String, ExecError> {
    let mut text = String::new();
    loop {
        match chars.next() {
            None => return Err(ExecError::UnterminatedQuote),
            Some('"') => return Ok(text),
            Some('\\') => text.push(chars.next().ok_or(ExecError::UnterminatedQuote)?),
            Some(other) => text.push(other),
        }
    }
}

/// One token as zero or more arguments.
fn expand_token(token: &[Piece], args: &ExecArgs) -> Result<Vec<String>, ExecError> {
    if let [Piece::Bare(only)] = token {
        match only.as_str() {
            "%F" => return Ok(files(args)),
            "%U" => return Ok(uris(args)),
            "%i" if args.icon.is_empty() => return Ok(Vec::new()),
            "%i" => return Ok(vec!["--icon".to_owned(), args.icon.clone()]),
            _ => {}
        }
    }
    let mut out = String::new();
    let mut literal = false;
    for piece in token {
        match piece {
            Piece::Quoted(text) => {
                out.push_str(text);
                literal = true;
            }
            Piece::Bare(text) => literal |= expand_bare(text, args, &mut out)?,
        }
    }
    Ok(if out.is_empty() && !literal {
        Vec::new()
    } else {
        vec![out]
    })
}

/// Append `text` with its field codes expanded to `out`; whether any literal character was
/// written (an argument that was only field codes expanding to nothing is dropped).
fn expand_bare(text: &str, args: &ExecArgs, out: &mut String) -> Result<bool, ExecError> {
    let mut literal = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            literal = true;
            continue;
        }
        match chars.next() {
            Some('%') => {
                out.push('%');
                literal = true;
            }
            Some('f') => out.push_str(&files(args).into_iter().next().unwrap_or_default()),
            Some('u') => out.push_str(&uris(args).into_iter().next().unwrap_or_default()),
            Some('c') => out.push_str(&args.name),
            Some('k') => {
                if let Some(path) = &args.desktop_file {
                    out.push_str(&path.to_string_lossy());
                }
            }
            Some('d' | 'D' | 'n' | 'N' | 'v' | 'm') => {}
            Some(code @ ('F' | 'U' | 'i')) => return Err(ExecError::FieldCodeNotAlone(code)),
            Some(code) => return Err(ExecError::UnknownFieldCode(code)),
            None => return Err(ExecError::UnknownFieldCode('%')),
        }
    }
    Ok(literal)
}

fn files(args: &ExecArgs) -> Vec<String> {
    args.files
        .iter()
        .map(|f| f.to_string_lossy().into_owned())
        .collect()
}

fn uris(args: &ExecArgs) -> Vec<String> {
    if args.uris.is_empty() {
        files(args)
    } else {
        args.uris.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecArgs, ExecError, expand_exec};
    use std::path::PathBuf;

    fn args() -> ExecArgs {
        ExecArgs {
            files: vec![PathBuf::from("/tmp/a b.txt"), PathBuf::from("/tmp/c.txt")],
            uris: vec![
                "https://example.org/".to_owned(),
                "file:///tmp/c.txt".to_owned(),
            ],
            icon: "firefox".to_owned(),
            name: "Firefox".to_owned(),
            desktop_file: Some(PathBuf::from("/usr/share/applications/firefox.desktop")),
        }
    }

    fn none() -> ExecArgs {
        ExecArgs {
            name: "Foot".to_owned(),
            ..ExecArgs::default()
        }
    }

    #[test]
    fn field_codes_and_quoting_expand_per_the_spec() {
        let cases: Vec<(&str, ExecArgs, Vec<&str>)> = vec![
            ("plain", none(), vec!["foot"]),
            (
                "args split on spaces and tabs",
                none(),
                vec!["foot", "-e", "htop"],
            ),
            (
                "%f takes the first file",
                args(),
                vec!["gedit", "/tmp/a b.txt"],
            ),
            (
                "%F takes every file",
                args(),
                vec!["gedit", "/tmp/a b.txt", "/tmp/c.txt"],
            ),
            (
                "%u takes the first url",
                args(),
                vec!["firefox", "https://example.org/"],
            ),
            (
                "%U takes every url",
                args(),
                vec!["firefox", "https://example.org/", "file:///tmp/c.txt"],
            ),
            (
                "%U falls back to files",
                ExecArgs {
                    uris: Vec::new(),
                    ..args()
                },
                vec!["vlc", "/tmp/a b.txt", "/tmp/c.txt"],
            ),
            ("%i with an icon", args(), vec!["app", "--icon", "firefox"]),
            ("%i without an icon", none(), vec!["app"]),
            ("%c is the name", args(), vec!["app", "--title=Firefox"]),
            (
                "%k is the desktop file",
                args(),
                vec!["app", "/usr/share/applications/firefox.desktop"],
            ),
            ("%k without a file is dropped", none(), vec!["app"]),
            ("%f without files is dropped", none(), vec!["gedit"]),
            ("%F without files is nothing", none(), vec!["gedit"]),
            (
                "a field code inside an argument",
                args(),
                vec!["app", "--file=/tmp/a b.txt"],
            ),
            (
                "an emptied argument with text stays",
                none(),
                vec!["app", "--file="],
            ),
            ("%% is a percent sign", none(), vec!["printf", "100%"]),
            ("deprecated codes vanish", none(), vec!["app"]),
            (
                "double quotes keep spaces",
                none(),
                vec!["sh", "-c", "echo hi there"],
            ),
            (
                "escapes inside quotes",
                none(),
                vec!["sh", "-c", "echo \"$HOME\" \\"],
            ),
            (
                "field codes inside quotes are literal",
                args(),
                vec!["sh", "-c", "open %f"],
            ),
            ("quotes join bare text", none(), vec!["--name=a b"]),
            ("an empty quoted argument survives", none(), vec!["app", ""]),
            ("repeated spaces", none(), vec!["a", "b"]),
        ];
        let lines = [
            "foot",
            "foot -e\thtop",
            "gedit %f",
            "gedit %F",
            "firefox %u",
            "firefox %U",
            "vlc %U",
            "app %i",
            "app %i",
            "app --title=%c",
            "app %k",
            "app %k",
            "gedit %f",
            "gedit %F",
            "app --file=%f",
            "app --file=%f",
            "printf 100%%",
            "app %d %D %n %N %v %m",
            "sh -c \"echo hi there\"",
            r#"sh -c "echo \"\$HOME\" \\""#,
            r#"sh -c "open %f""#,
            r#"--name="a b""#,
            r#"app """#,
            "  a   b  ",
        ];
        assert_eq!(lines.len(), cases.len());
        for ((name, given, want), exec) in cases.into_iter().zip(lines) {
            assert_eq!(
                expand_exec(exec, &given),
                Ok(want.into_iter().map(str::to_owned).collect()),
                "{name}: {exec}"
            );
        }
    }

    #[test]
    fn a_malformed_exec_is_refused() {
        let cases: &[(&str, &str, ExecError)] = &[
            ("empty", "", ExecError::Empty),
            ("blank", "   ", ExecError::Empty),
            ("only an emptied code", "%f", ExecError::Empty),
            (
                "unterminated quote",
                "sh -c \"echo",
                ExecError::UnterminatedQuote,
            ),
            (
                "escape at the end",
                "sh -c \"echo\\",
                ExecError::UnterminatedQuote,
            ),
            ("unknown code", "app %z", ExecError::UnknownFieldCode('z')),
            (
                "a lone percent",
                "app 100%",
                ExecError::UnknownFieldCode('%'),
            ),
            (
                "%F inside an argument",
                "app --files=%F",
                ExecError::FieldCodeNotAlone('F'),
            ),
            (
                "%U inside an argument",
                "app x%U",
                ExecError::FieldCodeNotAlone('U'),
            ),
            (
                "%i inside an argument",
                "app -%i",
                ExecError::FieldCodeNotAlone('i'),
            ),
        ];
        for (name, exec, want) in cases {
            assert_eq!(
                expand_exec(exec, &none()),
                Err(want.clone()),
                "{name}: {exec}"
            );
        }
    }
}
