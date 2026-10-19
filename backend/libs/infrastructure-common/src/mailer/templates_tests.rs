use std::path::PathBuf;

use super::*;

const APP: Option<&str> = Some("https://rift.example.com/");
const LINK: &str = "https://rift.example.com/reset-password?token=0123456789abcdef0123456789abcdef.aa";

fn check_snapshot(name: &str, email: &RenderedEmail) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/mailer/snapshots");
    for (ext, body) in [("html", &email.html), ("txt", &format!("Subject: {}\n\n{}", email.subject, email.text))] {
        let path = dir.join(format!("{name}.{ext}"));
        if std::env::var_os("UPDATE_EMAIL_SNAPSHOTS").is_some() {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, body).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing snapshot {}; run with UPDATE_EMAIL_SNAPSHOTS=1", path.display()));
        assert_eq!(&expected, body, "snapshot {} changed; review and rerun with UPDATE_EMAIL_SNAPSHOTS=1", path.display());
    }
}

#[test]
fn every_email_matches_its_snapshot() {
    for (locale, tag) in [(Locale::Ru, "ru"), (Locale::En, "en")] {
        check_snapshot(&format!("registration_code.{tag}"), &verification_code("482913", CodePurpose::Registration, locale, APP));
        check_snapshot(&format!("email_change_code.{tag}"), &verification_code("482913", CodePurpose::EmailChange, locale, APP));
        check_snapshot(&format!("password_reset.{tag}"), &password_reset(LINK, locale, APP));
    }
}

#[test]
fn every_email_has_the_layout_and_a_text_version() {
    let emails = [
        verification_code("482913", CodePurpose::Registration, Locale::Ru, APP),
        verification_code("482913", CodePurpose::EmailChange, Locale::En, APP),
        password_reset(LINK, Locale::Ru, APP),
    ];
    for email in &emails {
        assert!(email.html.starts_with("<!DOCTYPE html>"));
        assert!(email.html.contains("max-width:560px"));
        assert!(email.html.contains("src=\"cid:rift-logo\""));
        assert!(!email.html.contains("/icons/"));
        assert!(!email.html.contains("<style"));
        assert!(!email.html.contains("<script"));
        assert!(!email.text.contains('<'));
        assert!(email.text.contains("https://rift.example.com"));
    }
    assert!(emails[0].text.contains("482913"));
    assert!(emails[2].text.contains(LINK));
}

#[test]
fn values_are_escaped() {
    let email = verification_code("<b>&\"'</b>", CodePurpose::Registration, Locale::En, Some("https://x.test/\"><script>"));
    assert!(email.html.contains("&lt;b&gt;&amp;&quot;&#39;&lt;/b&gt;"));
    assert!(!email.html.contains("<b>&"));
    assert!(!email.html.contains("\"><script>"));
    assert!(email.html.contains("https://x.test/&quot;&gt;&lt;script&gt;"));

    let reset = password_reset("https://x.test/r?token=a\"onmouseover=\"x", Locale::Ru, None);
    assert!(reset.html.contains("token=a&quot;onmouseover=&quot;x"));
    assert!(!reset.html.contains("a\"onmouseover"));
}

#[test]
fn unsafe_links_are_dropped() {
    let reset = password_reset("javascript:alert(1)", Locale::En, Some("javascript:alert(2)"));
    assert!(!reset.html.contains("javascript:"));
    assert!(!reset.text.contains("javascript:"));
    assert!(reset.html.contains("src=\"cid:rift-logo\""));
}
