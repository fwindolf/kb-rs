pub const MARKER_START: &str = "<!-- kb:start -->";
pub const MARKER_END: &str = "<!-- kb:end -->";

/// Check whether content contains the kb marker section.
pub fn has_marker_section(content: &str) -> bool {
    content.contains(MARKER_START)
}

/// Replace the marker-bounded section with new content.
/// Returns None if no markers found.
pub fn replace_marker_section(content: &str, new_section: &str) -> Option<String> {
    let start_idx = content.find(MARKER_START)?;
    let end_idx = content.find(MARKER_END)?;

    let before = &content[..start_idx];
    let after = &content[end_idx + MARKER_END.len()..];

    Some(format!("{before}{new_section}{after}"))
}

/// Remove the marker-bounded section entirely. Cleans up extra newlines.
pub fn remove_marker_section(content: &str) -> String {
    let start_idx = match content.find(MARKER_START) {
        Some(i) => i,
        None => return content.to_string(),
    };
    let end_idx = match content.find(MARKER_END) {
        Some(i) => i,
        None => return content.to_string(),
    };

    let before = &content[..start_idx];
    let after = &content[end_idx + MARKER_END.len()..];
    let combined = format!("{before}{after}");

    // Clean up triple+ newlines
    let re = regex::Regex::new(r"\n{3,}").unwrap();
    let cleaned = re.replace_all(&combined, "\n\n");
    format!("{}\n", cleaned.trim())
}

/// Wrap a snippet in kb markers.
pub fn wrap_in_markers(snippet: &str) -> String {
    format!("{MARKER_START}\n{snippet}{MARKER_END}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_marker_section_works() {
        assert!(has_marker_section(
            "before\n<!-- kb:start -->\ncontent\n<!-- kb:end -->\nafter"
        ));
        assert!(!has_marker_section("no markers here"));
    }

    #[test]
    fn replace_marker_section_works() {
        let content = "before\n<!-- kb:start -->\nold\n<!-- kb:end -->\nafter";
        let result = replace_marker_section(content, "NEW").unwrap();
        assert_eq!(result, "before\nNEW\nafter");
    }

    #[test]
    fn replace_returns_none_without_markers() {
        assert!(replace_marker_section("no markers", "new").is_none());
    }

    #[test]
    fn remove_marker_section_works() {
        let content = "before\n<!-- kb:start -->\nstuff\n<!-- kb:end -->\nafter";
        let result = remove_marker_section(content);
        assert!(!result.contains("kb:start"));
        assert!(result.contains("before"));
        assert!(result.contains("after"));
    }

    #[test]
    fn wrap_in_markers_works() {
        let wrapped = wrap_in_markers("hello\n");
        assert!(wrapped.starts_with(MARKER_START));
        assert!(wrapped.ends_with(MARKER_END));
    }
}
