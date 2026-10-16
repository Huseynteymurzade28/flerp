//! Handing the loaded file to the user's own editor.
//!
//! flerp finds things; it does not change them. When a search turns up the line
//! that needs fixing, `e` opens that line in whatever editor the user already
//! lives in, and the file is reloaded when they come back.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Used when neither `$VISUAL` nor `$EDITOR` is set, as git and crontab do.
const FALLBACK_EDITOR: &str = "vi";

/// A file and the 1-based line it should open on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditRequest {
    pub path: PathBuf,
    pub line: usize,
}

/// The editor command, by the convention every Unix tool follows: `$VISUAL`
/// first, then `$EDITOR`, then `vi`. An empty variable counts as unset.
pub fn resolve_editor(visual: Option<String>, editor: Option<String>) -> String {
    [visual, editor]
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| FALLBACK_EDITOR.to_string())
}

/// The program and arguments that open `path` on `line` with `editor`.
///
/// The editor string is split on whitespace so `emacs -nw` and `vim -u NONE`
/// work, without going through a shell that Windows does not have. Quoted
/// arguments are not supported; a path with spaces belongs in a wrapper script.
///
/// `+N` is the line flag vi, vim, neovim, nano, emacs, micro and kakoune all
/// understand. It goes before the path because several of them only read
/// it there.
pub fn editor_argv(editor: &str, path: &Path, line: usize) -> Option<Vec<OsString>> {
    let mut argv: Vec<OsString> = editor.split_whitespace().map(OsString::from).collect();
    if argv.is_empty() {
        return None;
    }
    argv.push(format!("+{}", line.max(1)).into());
    argv.push(path.as_os_str().to_owned());
    Some(argv)
}

/// The editor this process would launch, for messages.
pub fn current_editor() -> String {
    resolve_editor(std::env::var("VISUAL").ok(), std::env::var("EDITOR").ok())
}

/// Run the editor on `request` and wait for it to exit.
///
/// The caller owns the terminal and has to give it back to a normal screen
/// first; this only spawns and waits.
pub fn open(request: &EditRequest) -> io::Result<ExitStatus> {
    let editor = current_editor();
    let argv = editor_argv(&editor, &request.path, request.line)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the editor is empty"))?;

    Command::new(&argv[0]).args(&argv[1..]).status()
}
