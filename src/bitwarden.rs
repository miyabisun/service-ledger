//! Bitwarden vault listings: keep only each entry's name and website origin.
//!
//! Accepted shapes: `rbw list --raw` (an array of `{name, uris: [..], type, ..}`) and the
//! official `bw export --format json` (`{"items": [{name, login: {uris: [{uri}]}, ..}]}`).
//! Every other key (user, password, notes, fields, totp, folder …) is never read.
use serde_json::Value;

use crate::service::{origin_of, same_service};

#[derive(Debug, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub website_url: Option<String>,
}

const NOT_A_LISTING: &str =
    "Bitwarden の一覧 JSON (rbw list --raw の配列、または bw export の items) を渡してください";

pub fn parse(text: &str) -> Result<Vec<Candidate>, String> {
    let value: Value = serde_json::from_str(text).map_err(|_| NOT_A_LISTING.to_owned())?;
    let items = match &value {
        Value::Array(items) => items,
        Value::Object(map) => map
            .get("items")
            .and_then(Value::as_array)
            .ok_or(NOT_A_LISTING)?,
        _ => return Err(NOT_A_LISTING.into()),
    };
    let mut candidates = Vec::new();
    for item in items {
        let item = item.as_object().ok_or(NOT_A_LISTING)?;
        if !is_service(item.get("type")) {
            continue;
        }
        let Some(name) = item.get("name").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let uris = item
            .get("uris")
            .or_else(|| item.get("login").and_then(|login| login.get("uris")));
        let website_url = uris
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|uri| uri.as_str().or_else(|| uri.get("uri")?.as_str()))
            .find_map(|uri| origin_of(uri.trim()));
        candidates.push(Candidate {
            name: name.to_owned(),
            website_url,
        });
    }
    Ok(candidates)
}

/// Logins and cards are services; notes, identities and keys are not.
/// rbw names the type (`Login`), the official export numbers it (1 login, 3 card).
fn is_service(kind: Option<&Value>) -> bool {
    match kind {
        None | Some(Value::Null) => true,
        Some(Value::String(kind)) => kind == "Login" || kind == "Card",
        Some(Value::Number(kind)) => matches!(kind.as_u64(), Some(1 | 3)),
        Some(_) => false,
    }
}

/// Split candidates into those to create and a duplicate count, against existing
/// `(name, website_url)` pairs and earlier candidates in the same file.
#[must_use]
pub fn plan<'a>(
    existing: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    candidates: Vec<Candidate>,
) -> (Vec<Candidate>, usize) {
    let existing: Vec<_> = existing.into_iter().collect();
    let mut create: Vec<Candidate> = Vec::new();
    let mut duplicates = 0;
    for candidate in candidates {
        let url = candidate.website_url.as_deref();
        let seen = existing
            .iter()
            .copied()
            .chain(
                create
                    .iter()
                    .map(|c| (c.name.as_str(), c.website_url.as_deref())),
            )
            .any(|(name, other)| same_service(&candidate.name, url, name, other));
        if seen {
            duplicates += 1;
        } else {
            create.push(candidate);
        }
    }
    (create, duplicates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rbw_list_raw_keeps_name_and_origin_only() {
        let text = r#"[
          {"id":"1","name":"Example Mobile","user":"someone@example.com","folder":null,
           "uris":["androidapp://com.example.mobile","https://www.example.com/login?session=abc"],"type":"Login"},
          {"id":"2","name":"  Example Power  ","user":null,"folder":"Home","uris":[],"type":"Login"},
          {"id":"3","name":"Recovery codes","uris":[],"type":"Note"},
          {"id":"4","name":"Example Card","uris":["https://card.example.net/"],"type":"Card"},
          {"id":"5","name":"","uris":["https://blank.example.org"],"type":"Login"},
          {"id":"6","name":"Me","type":"Identity"}
        ]"#;
        assert_eq!(
            parse(text).unwrap(),
            vec![
                Candidate {
                    name: "Example Mobile".into(),
                    website_url: Some("https://www.example.com".into())
                },
                Candidate {
                    name: "Example Power".into(),
                    website_url: None
                },
                Candidate {
                    name: "Example Card".into(),
                    website_url: Some("https://card.example.net".into())
                },
            ]
        );
    }

    #[test]
    fn official_export_ignores_secret_values() {
        let text = r#"{"encrypted":false,"items":[
          {"type":1,"name":"Example Books","notes":"secret note",
           "login":{"username":"u","password":"hunter2","totp":"otpauth://x",
                    "uris":[{"match":null,"uri":"https://user:pw@books.example.com/a"},
                            {"match":null,"uri":"https://books.example.com/member"}]},
           "fields":[{"name":"pin","value":"1234"}]},
          {"type":2,"name":"A secure note","notes":"x"}
        ]}"#;
        let parsed = parse(text).unwrap();
        assert_eq!(
            parsed,
            vec![Candidate {
                name: "Example Books".into(),
                website_url: Some("https://books.example.com".into())
            }]
        );
        let debug = format!("{parsed:?}");
        for secret in [
            "hunter2",
            "1234",
            "otpauth",
            "secret note",
            "pw@",
            "/member",
        ] {
            assert!(!debug.contains(secret), "{secret}");
        }
    }

    #[test]
    fn rejects_non_listing_input() {
        for text in ["", "not json", "{}", "42", r#"{"items":3}"#, "[1]"] {
            assert!(parse(text).is_err(), "{text:?}");
        }
        assert_eq!(parse("[]").unwrap(), vec![]);
    }

    #[test]
    fn plan_skips_existing_and_in_file_duplicates() {
        let candidates = vec![
            Candidate {
                name: "Example Mobile".into(),
                website_url: Some("https://example.com".into()),
            },
            Candidate {
                name: "example mobile".into(),
                website_url: None,
            },
            Candidate {
                name: "Example Shop".into(),
                website_url: Some("https://www.shop.example.org".into()),
            },
            Candidate {
                name: "Example Shop (second account)".into(),
                website_url: Some("https://shop.example.org".into()),
            },
            Candidate {
                name: "Example Water".into(),
                website_url: None,
            },
        ];
        let existing = [
            ("EXAMPLE WATER", None),
            ("Other", Some("https://other.example")),
        ];
        let (create, duplicates) = plan(existing, candidates);
        assert_eq!(
            create.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["Example Mobile", "Example Shop"]
        );
        assert_eq!(duplicates, 3);
    }
}
