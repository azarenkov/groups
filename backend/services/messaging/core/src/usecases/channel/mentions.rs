pub async fn resolve_mentions<C, R>(
    content: &str,
    channel_id: &domain::channel::id::ChannelId,
    members: &[domain::user::id::UserId],
    channel_repo: &C,
    resolver: &R,
) -> Result<Vec<uuid::Uuid>, domain::shared::repository::error::RepositoryError>
where
    C: domain::channel::repository::ChannelRepository + ?Sized,
    R: crate::ports::mention_resolver::MentionResolver + ?Sized,
{
    let names = mention_names(content);
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let mut mentions: Vec<uuid::Uuid> = Vec::new();
    let mut resolved_names = std::collections::HashSet::new();
    for (id, name) in channel_repo
        .text_members_by_names(channel_id, &names)
        .await?
    {
        let uid = id.uuid();
        if !mentions.contains(&uid) {
            mentions.push(uid);
        }
        resolved_names.insert(name);
    }
    let unresolved: Vec<String> = names
        .into_iter()
        .filter(|n| !resolved_names.contains(n))
        .collect();
    if !unresolved.is_empty() {
        let member_set: std::collections::HashSet<uuid::Uuid> =
            members.iter().map(|id| id.uuid()).collect();
        match resolver.resolve_names(&unresolved).await {
            Ok(resolved) => {
                for (id, _) in resolved {
                    if member_set.contains(&id) && !mentions.contains(&id) {
                        mentions.push(id);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Authoritative mention resolution failed; using projection only");
            }
        }
    }
    Ok(mentions)
}

fn is_url_terminator(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(byte, b'<' | b'>' | b'"' | b'\'' | b')' | b']' | b'}')
}

fn url_ranges(content: &str) -> Vec<(usize, usize)> {
    let bytes = content.as_bytes();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for pat in ["http://", "https://"] {
        let mut from = 0;
        while let Some(pos) = content[from..].find(pat) {
            let start = from + pos;
            let mut end = start;
            while end < bytes.len() && !is_url_terminator(bytes[end]) {
                end += 1;
            }
            while end > start
                && matches!(bytes[end - 1], b'.' | b',' | b';' | b':' | b'!' | b'?')
            {
                end -= 1;
            }
            ranges.push((start, end));
            from = end.max(start + pat.len());
        }
    }
    ranges
}

pub fn mention_names(content: &str) -> Vec<String> {
    let bytes = content.as_bytes();
    let urls = url_ranges(content);
    let in_url = |pos: usize| urls.iter().any(|&(s, e)| pos >= s && pos < e);
    let mut names = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'@' || in_url(i) {
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

    #[test]
    fn ignores_mentions_inside_urls() {
        assert!(mention_names("see https://mastodon.social/@alice").is_empty());
        assert!(mention_names("http://x.com/@bob?tab=1").is_empty());
        assert_eq!(
            mention_names("https://mastodon.social/@alice cc @bob"),
            vec!["bob".to_string()],
        );
    }

    #[test]
    fn uppercase_scheme_is_not_a_url() {
        assert_eq!(
            mention_names("HTTPS://mastodon.social/@alice"),
            vec!["alice".to_string()],
        );
    }

    #[test]
    fn url_ends_at_frontend_link_boundary() {
        assert_eq!(
            mention_names("https://example.com)@alice"),
            vec!["alice".to_string()],
        );
        assert_eq!(
            mention_names("(https://example.com/path) @bob"),
            vec!["bob".to_string()],
        );
        assert!(mention_names("https://example.com/@carol.").is_empty());
    }

}
