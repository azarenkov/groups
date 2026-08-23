pub fn merge_client_mentions(
    mentions: &mut Vec<uuid::Uuid>,
    client_ids: &[uuid::Uuid],
    members: &[domain::user::id::UserId],
    mut unresolved_tokens: usize,
) {
    let member_set: std::collections::HashSet<uuid::Uuid> =
        members.iter().map(|id| id.uuid()).collect();
    for id in client_ids {
        if unresolved_tokens == 0 {
            break;
        }
        if member_set.contains(id) && !mentions.contains(id) {
            mentions.push(*id);
            unresolved_tokens -= 1;
        }
    }
    mentions.truncate(50);
}

fn url_ranges(content: &str) -> Vec<(usize, usize)> {
    let lower = content.to_ascii_lowercase();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for pat in ["http://", "https://"] {
        let mut from = 0;
        while let Some(pos) = lower[from..].find(pat) {
            let start = from + pos;
            let end = content[start..]
                .find(char::is_whitespace)
                .map(|o| start + o)
                .unwrap_or(content.len());
            ranges.push((start, end));
            from = end.max(start + 1);
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
    fn client_ids_bounded_by_unresolved_tokens() {
        let member_a = domain::user::id::UserId::new(uuid::Uuid::from_u128(1));
        let member_b = domain::user::id::UserId::new(uuid::Uuid::from_u128(2));
        let members = vec![member_a, member_b];
        let mut mentions = Vec::new();
        merge_client_mentions(
            &mut mentions,
            &[uuid::Uuid::from_u128(1), uuid::Uuid::from_u128(2)],
            &members,
            1,
        );
        assert_eq!(mentions, vec![uuid::Uuid::from_u128(1)]);

        let mut none_allowed = Vec::new();
        merge_client_mentions(
            &mut none_allowed,
            &[uuid::Uuid::from_u128(1)],
            &members,
            0,
        );
        assert!(none_allowed.is_empty());
    }
}
