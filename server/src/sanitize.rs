//! Strip server-side filesystem paths out of user-visible error messages.
//!
//! Compile and runtime errors from the Achronyme pipeline can embed
//! absolute paths that include the session workspace
//! (`/tmp/ach-sessions/<uuid>/src/main.ach`). Returning those to the
//! client leaks the server's layout and each session's identifier.
//! Swap the prefix for a stable placeholder (`<workspace>/...`) before
//! the message crosses the API boundary.

const SESSION_PREFIX: &str = "/tmp/ach-sessions/";

/// Replace the session workspace prefix with `<workspace>` in a message.
///
/// Looks for `/tmp/ach-sessions/<uuid>/` occurrences and rewrites them
/// to `<workspace>/`. The UUID segment is whatever lies between the
/// prefix and the next `/`, so the routine is robust to both v4 UUIDs
/// and any other non-slash identifier shape.
pub fn scrub_paths(msg: &str) -> String {
    if !msg.contains(SESSION_PREFIX) {
        return msg.to_string();
    }

    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;
    while let Some(idx) = rest.find(SESSION_PREFIX) {
        out.push_str(&rest[..idx]);
        out.push_str("<workspace>");
        let after = &rest[idx + SESSION_PREFIX.len()..];
        // Skip past the UUID / session id (up to the next '/').
        match after.find('/') {
            Some(slash) => rest = &after[slash..], // keep the leading slash
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Shorthand for scrubbing an `Option<String>` without unwrapping the
/// `Some` case manually at every call site.
pub fn scrub_option(msg: Option<String>) -> Option<String> {
    msg.map(|m| scrub_paths(&m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_plain_message_unchanged() {
        let m = "Runtime error: undefined variable `foo`";
        assert_eq!(scrub_paths(m), m);
    }

    #[test]
    fn scrubs_session_uuid_and_path_tail() {
        let m = "error at /tmp/ach-sessions/abc-123-def/src/main.ach:10:5";
        assert_eq!(scrub_paths(m), "error at <workspace>/src/main.ach:10:5");
    }

    #[test]
    fn scrubs_multiple_occurrences() {
        let m = "A=/tmp/ach-sessions/u1/a.ach B=/tmp/ach-sessions/u2/b.ach";
        assert_eq!(scrub_paths(m), "A=<workspace>/a.ach B=<workspace>/b.ach");
    }

    #[test]
    fn handles_prefix_with_no_trailing_slash() {
        // Pathological but shouldn't panic.
        let m = "orphan: /tmp/ach-sessions/xyz";
        assert_eq!(scrub_paths(m), "orphan: <workspace>");
    }

    #[test]
    fn does_not_clobber_unrelated_tmp_paths() {
        // Anything under /tmp that isn't /tmp/ach-sessions stays untouched —
        // the session-specific ID is the actual leak, not every absolute
        // path that happens to mention /tmp.
        let m = "could not open /tmp/other/file.txt";
        assert_eq!(scrub_paths(m), m);
    }

    #[test]
    fn scrub_option_passes_through_none() {
        assert_eq!(scrub_option(None), None);
    }
}
