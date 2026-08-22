pub fn mention_names(content: &str) -> Vec<String> {
    let bytes = content.as_bytes();
    let mut names = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'@' {
            i += 1;
            continue;
        }
        if i > 0 {
            let prev = bytes[i - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'@' {
                i += 1;
                continue;
            }
        }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
        {
            end += 1;
        }
        if end > start {
            let name = content[start..end].to_ascii_lowercase();
            if !names.contains(&name) {
                names.push(name);
            }
        }
        i = end.max(i + 1);
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_unique_lowercased_names() {
        assert_eq!(
            mention_names("hi @Alice and @bob, @alice again"),
            vec!["alice".to_string(), "bob".to_string()],
        );
    }

    #[test]
    fn requires_token_boundary_before_at() {
        assert!(mention_names("mail@example.com").is_empty());
        assert!(mention_names("@@alice").is_empty());
        assert_eq!(mention_names("(@alice)"), vec!["alice".to_string()]);
        assert_eq!(mention_names("@alice"), vec!["alice".to_string()]);
    }

    #[test]
    fn ignores_bare_at() {
        assert!(mention_names("just @ nothing").is_empty());
        assert!(mention_names("trailing @").is_empty());
    }
}
