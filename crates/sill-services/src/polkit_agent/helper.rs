//! polkit's helper: where it is and its line protocol (F545), pure.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::auth::UserName;

use super::types::Cookie;

/// polkit 124+'s socket-activated helper.
pub const SOCKET_PATH: &str = "/run/polkit/agent-helper.socket";

/// Where distributions put the setuid helper, first found wins: Fedora and Arch, Debian and
/// Ubuntu, older Debian.
pub const HELPER_PATHS: [&str; 3] = [
    "/usr/lib/polkit-1/polkit-agent-helper-1",
    "/usr/libexec/polkit-agent-helper-1",
    "/usr/lib/policykit-1/polkit-agent-helper-1",
];

/// How to reach the helper.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HelperRoute {
    /// Connect to this socket.
    Socket(PathBuf),
    /// Run this program.
    Setuid(PathBuf),
    /// Neither is installed: every request fails.
    Missing,
}

/// The socket when it exists (`exists`), else the first of [`HELPER_PATHS`] that exists.
pub fn helper_route(exists: impl Fn(&Path) -> bool) -> HelperRoute {
    if exists(Path::new(SOCKET_PATH)) {
        return HelperRoute::Socket(PathBuf::from(SOCKET_PATH));
    }
    HELPER_PATHS
        .iter()
        .map(Path::new)
        .find(|path| exists(path))
        .map_or(HelperRoute::Missing, |path| {
            HelperRoute::Setuid(path.to_path_buf())
        })
}

/// How one helper run starts: the program's arguments (the socket takes none) and the lines
/// written first.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HelperOpening {
    pub args: Vec<String>,
    pub lines: Vec<String>,
}

/// The opening for `user` and `cookie` on `route`: the socket gets the user and the cookie as
/// two lines; the setuid helper gets the user as its argument and the cookie as one line.
/// `None` for `Missing`.
pub fn opening(route: &HelperRoute, user: &UserName, cookie: &Cookie) -> Option<HelperOpening> {
    match route {
        HelperRoute::Socket(_) => Some(HelperOpening {
            args: Vec::new(),
            lines: vec![user.0.clone(), cookie.0.clone()],
        }),
        HelperRoute::Setuid(_) => Some(HelperOpening {
            args: vec![user.0.clone()],
            lines: vec![cookie.0.clone()],
        }),
        HelperRoute::Missing => None,
    }
}

/// One line from the helper.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HelperLine {
    /// `PAM_PROMPT_ECHO_OFF <prompt>`.
    PromptHidden(String),
    /// `PAM_PROMPT_ECHO_ON <prompt>`.
    PromptShown(String),
    /// `PAM_ERROR_MSG <text>`.
    Error(String),
    /// `PAM_TEXT_INFO <text>`.
    Info(String),
    Success,
    Failure,
}

/// A line that is not the protocol.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("not a polkit helper line: {0:?}")]
pub struct UnknownLine(pub String);

/// `line` (its newline stripped; the text after the word kept as sent, trailing space and all).
pub fn parse_helper_line(line: &str) -> Result<HelperLine, UnknownLine> {
    type Make = fn(String) -> HelperLine;
    let worded: [(&str, Make); 4] = [
        ("PAM_PROMPT_ECHO_OFF ", HelperLine::PromptHidden),
        ("PAM_PROMPT_ECHO_ON ", HelperLine::PromptShown),
        ("PAM_ERROR_MSG ", HelperLine::Error),
        ("PAM_TEXT_INFO ", HelperLine::Info),
    ];
    let found = worded
        .iter()
        .find_map(|(word, make)| line.strip_prefix(word).map(|rest| make(rest.to_owned())));
    match (found, line) {
        (Some(parsed), _) => Ok(parsed),
        (None, "SUCCESS") => Ok(HelperLine::Success),
        (None, "FAILURE") => Ok(HelperLine::Failure),
        (None, _) => Err(UnknownLine(line.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        HelperLine, HelperOpening, HelperRoute, UnknownLine, helper_route, opening,
        parse_helper_line,
    };
    use crate::auth::UserName;
    use crate::polkit_agent::types::Cookie;

    #[test]
    fn the_socket_first_then_the_setuid_helper() {
        let fedora = "/usr/lib/polkit-1/polkit-agent-helper-1";
        let debian = "/usr/libexec/polkit-agent-helper-1";
        let socket = "/run/polkit/agent-helper.socket";
        let cases: [(&[&str], HelperRoute); 4] = [
            (
                &[socket, fedora],
                HelperRoute::Socket(PathBuf::from(socket)),
            ),
            (&[fedora], HelperRoute::Setuid(PathBuf::from(fedora))),
            (&[debian], HelperRoute::Setuid(PathBuf::from(debian))),
            (&[], HelperRoute::Missing),
        ];
        for (present, want) in cases {
            let got = helper_route(|path: &Path| present.iter().any(|p| Path::new(p) == path));
            assert_eq!(got, want, "{present:?}");
        }
    }

    #[test]
    fn openings() {
        let user = UserName("ada".to_owned());
        let cookie = Cookie("3-abc-1".to_owned());
        let lines = |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let cases = [
            (
                HelperRoute::Socket(PathBuf::from("/run/polkit/agent-helper.socket")),
                Some(HelperOpening {
                    args: Vec::new(),
                    lines: lines(&["ada", "3-abc-1"]),
                }),
            ),
            (
                HelperRoute::Setuid(PathBuf::from("/usr/lib/polkit-1/polkit-agent-helper-1")),
                Some(HelperOpening {
                    args: lines(&["ada"]),
                    lines: lines(&["3-abc-1"]),
                }),
            ),
            (HelperRoute::Missing, None),
        ];
        for (route, want) in cases {
            assert_eq!(opening(&route, &user, &cookie), want, "{route:?}");
        }
    }

    #[test]
    fn helper_lines() {
        let cases = [
            (
                "PAM_PROMPT_ECHO_OFF Password: ",
                Ok(HelperLine::PromptHidden("Password: ".to_owned())),
            ),
            (
                "PAM_PROMPT_ECHO_ON Login: ",
                Ok(HelperLine::PromptShown("Login: ".to_owned())),
            ),
            (
                "PAM_ERROR_MSG Authentication failure",
                Ok(HelperLine::Error("Authentication failure".to_owned())),
            ),
            (
                "PAM_TEXT_INFO Swipe your finger",
                Ok(HelperLine::Info("Swipe your finger".to_owned())),
            ),
            ("SUCCESS", Ok(HelperLine::Success)),
            ("FAILURE", Ok(HelperLine::Failure)),
            ("MAYBE", Err(UnknownLine("MAYBE".to_owned()))),
        ];
        for (line, want) in cases {
            assert_eq!(parse_helper_line(line), want, "{line:?}");
        }
    }
}
