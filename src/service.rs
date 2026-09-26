//! Service records: validation and duplicate rules, free of I/O.
use rmcp::schemars;
use serde::{Deserialize, Deserializer, Serialize};

pub const MAX_NAME: usize = 200;
pub const MAX_URL: usize = 2048;
pub const MAX_SOURCE: usize = 1000;
pub const MAX_NOTE: usize = 10_000;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AddressStatus {
    /// 旧住所のまま
    Old,
    /// 変更済み
    Changed,
    /// 不明
    #[default]
    Unknown,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ChangeMethod {
    /// 一括手段 (郵便局の転居届・引越し手続きサービスなど)
    Bulk,
    /// 会員ページ
    MemberPage,
    /// 電話・書面
    PhoneOrPaper,
    /// 未確認
    #[default]
    Unknown,
}

impl AddressStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Old => "old",
            Self::Changed => "changed",
            Self::Unknown => "unknown",
        }
    }
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        [Self::Old, Self::Changed, Self::Unknown]
            .into_iter()
            .find(|v| v.as_str() == value)
    }
}

impl ChangeMethod {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bulk => "bulk",
            Self::MemberPage => "member_page",
            Self::PhoneOrPaper => "phone_or_paper",
            Self::Unknown => "unknown",
        }
    }
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::Bulk,
            Self::MemberPage,
            Self::PhoneOrPaper,
            Self::Unknown,
        ]
        .into_iter()
        .find(|v| v.as_str() == value)
    }
}

/// Editable fields of one service after validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fields {
    pub name: String,
    pub website_url: Option<String>,
    pub address_change_url: Option<String>,
    pub change_method: ChangeMethod,
    pub address_status: AddressStatus,
    pub source: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Service {
    pub id: i64,
    #[serde(flatten)]
    pub fields: Fields,
    pub created_at: String,
    pub updated_at: String,
}

/// A new registration from HTTP or MCP. Unknown keys (e.g. password) are rejected.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewService {
    /// Service name as the user knows it.
    pub name: String,
    /// Absolute http(s) URL of the service's website.
    #[serde(default)]
    pub website_url: Option<String>,
    /// Absolute http(s) URL of the page where the registered address is changed.
    #[serde(default)]
    pub address_change_url: Option<String>,
    /// `bulk` / `member_page` / `phone_or_paper` / `unknown` (default `unknown`).
    #[serde(default)]
    pub change_method: Option<ChangeMethod>,
    /// `old` / `changed` / `unknown` (default `unknown`).
    #[serde(default)]
    pub address_status: Option<AddressStatus>,
    /// Where the service was found (e.g. "Gmail receipt 2026-04", "Bitwarden").
    pub source: String,
    /// Optional free-form memo.
    #[serde(default)]
    pub note: Option<String>,
}

/// A partial update. Omitted keys keep their value; `null` clears an optional URL.
#[allow(clippy::option_option)] // omitted / null / value are three different requests
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub website_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    pub address_change_url: Option<Option<String>>,
    #[serde(default)]
    pub change_method: Option<ChangeMethod>,
    #[serde(default)]
    pub address_status: Option<AddressStatus>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[allow(clippy::option_option)]
fn present<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

impl NewService {
    pub fn validate(self) -> Result<Fields, String> {
        Fields {
            name: self.name,
            website_url: self.website_url,
            address_change_url: self.address_change_url,
            change_method: self.change_method.unwrap_or_default(),
            address_status: self.address_status.unwrap_or_default(),
            source: self.source,
            note: self.note.unwrap_or_default(),
        }
        .validate()
    }
}

impl Patch {
    pub fn apply(self, current: &Fields) -> Result<Fields, String> {
        let mut next = current.clone();
        if let Some(v) = self.name {
            next.name = v;
        }
        if let Some(v) = self.website_url {
            next.website_url = v;
        }
        if let Some(v) = self.address_change_url {
            next.address_change_url = v;
        }
        if let Some(v) = self.change_method {
            next.change_method = v;
        }
        if let Some(v) = self.address_status {
            next.address_status = v;
        }
        if let Some(v) = self.source {
            next.source = v;
        }
        if let Some(v) = self.note {
            next.note = v;
        }
        next.validate()
    }
}

fn single_line(label: &str, value: &str, max: usize, required: bool) -> Result<String, String> {
    let value = value.trim();
    if required && value.is_empty() {
        return Err(format!("{label}は必須です"));
    }
    if value.chars().count() > max {
        return Err(format!("{label}は{max}文字以内にしてください"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label}に改行や制御文字は使えません"));
    }
    Ok(value.to_owned())
}

impl Fields {
    /// Trim and check every field; empty URLs become `None`.
    pub fn validate(self) -> Result<Self, String> {
        let url = |label: &str, value: Option<String>| -> Result<Option<String>, String> {
            match value.as_deref().map(str::trim) {
                None | Some("") => Ok(None),
                Some(v) => parse_web_url(v).map(Some).ok_or_else(|| {
                    format!("{label}は http:// か https:// で始まる URL にしてください")
                }),
            }
        };
        let note = self.note.trim().to_owned();
        if note.chars().count() > MAX_NOTE {
            return Err(format!("メモは{MAX_NOTE}文字以内にしてください"));
        }
        if note
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
        {
            return Err("メモに制御文字は使えません".into());
        }
        Ok(Self {
            name: single_line("名前", &self.name, MAX_NAME, true)?,
            website_url: url("Web サイト URL", self.website_url)?,
            address_change_url: url("住所変更ページ URL", self.address_change_url)?,
            change_method: self.change_method,
            address_status: self.address_status,
            source: single_line("根拠", &self.source, MAX_SOURCE, true)?,
            note,
        })
    }
}

/// Split a web URL into its lowercased scheme and authority.
fn scheme_authority(value: &str) -> Option<(&'static str, &str)> {
    if value.len() > MAX_URL || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let (scheme, rest) = value.split_once("://")?;
    let scheme = ["http", "https"]
        .into_iter()
        .find(|s| s.eq_ignore_ascii_case(scheme))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = hostname(authority)?;
    let port = &authority[host.len()..];
    let port_ok = port.is_empty()
        || port
            .strip_prefix(':')
            .is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    (!authority.contains('@') && !host.is_empty() && port_ok).then_some((scheme, authority))
}

/// The host part of an authority, keeping IPv6 brackets.
fn hostname(authority: &str) -> Option<&str> {
    if authority.starts_with('[') {
        return authority.find(']').map(|end| &authority[..=end]);
    }
    Some(authority.split(':').next().unwrap_or_default())
}

/// Accept an absolute http(s) URL with a host and no credentials or whitespace.
#[must_use]
pub fn parse_web_url(value: &str) -> Option<String> {
    scheme_authority(value).map(|_| value.to_owned())
}

/// Lowercased host without `www.`, port or credentials.
#[must_use]
pub fn host_of(url: &str) -> Option<String> {
    let host = hostname(scheme_authority(url)?.1)?.to_lowercase();
    Some(host.strip_prefix("www.").map(str::to_owned).unwrap_or(host))
}

/// `scheme://host[:port]` of a valid web URL: what an import keeps from a vault URI.
#[must_use]
pub fn origin_of(url: &str) -> Option<String> {
    let (scheme, authority) = scheme_authority(url)?;
    Some(format!("{scheme}://{}", authority.to_lowercase()))
}

/// Two records name the same service when their names match (trimmed, case-insensitive)
/// or both have websites on the same host.
#[must_use]
pub fn same_service(a_name: &str, a_url: Option<&str>, b_name: &str, b_url: Option<&str>) -> bool {
    if a_name.trim().to_lowercase() == b_name.trim().to_lowercase() {
        return true;
    }
    match (a_url.and_then(host_of), b_url.and_then(host_of)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// SQL LIKE pattern for a case-insensitive substring search, escaped with `\`.
#[must_use]
pub fn like_pattern(query: &str) -> String {
    let mut pattern = String::from("%");
    for c in query.trim().chars() {
        if matches!(c, '\\' | '%' | '_') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> Fields {
        Fields {
            name: "Example Mobile".into(),
            website_url: Some("https://www.example.com/".into()),
            address_change_url: None,
            change_method: ChangeMethod::Unknown,
            address_status: AddressStatus::Unknown,
            source: "fixture".into(),
            note: String::new(),
        }
    }

    #[test]
    fn web_urls_must_be_absolute_http_without_credentials() {
        for ok in [
            "https://example.com",
            "http://example.com/path?q=1#f",
            "HTTPS://Example.COM:8443/a",
            "https://例え.jp/住所",
        ] {
            assert_eq!(parse_web_url(ok).as_deref(), Some(ok), "{ok}");
        }
        for bad in [
            "",
            "example.com",
            "javascript:alert(1)",
            "data:text/html,<script>",
            "ftp://example.com",
            "https://",
            "https:///path",
            "https://user:pass@example.com/",
            "https://exa mple.com",
            "https://example.com/\n",
            "//example.com",
        ] {
            assert_eq!(parse_web_url(bad), None, "{bad:?}");
        }
        let long = format!("https://example.com/{}", "a".repeat(MAX_URL));
        assert_eq!(parse_web_url(&long), None);
    }

    #[test]
    fn host_and_origin_ignore_www_port_and_path() {
        assert_eq!(
            host_of("https://WWW.Example.com:443/login?x=1").as_deref(),
            Some("example.com")
        );
        assert_eq!(
            host_of("http://shop.example.com").as_deref(),
            Some("shop.example.com")
        );
        assert_eq!(host_of("not a url"), None);
        assert_eq!(
            origin_of("https://www.example.com/account/login?token=secret#x").as_deref(),
            Some("https://www.example.com")
        );
        assert_eq!(
            origin_of("http://example.com:8080/a").as_deref(),
            Some("http://example.com:8080")
        );
        assert_eq!(origin_of("androidapp://com.example"), None);
    }

    #[test]
    fn same_service_by_name_or_host() {
        assert!(same_service(
            " Example Mobile ",
            None,
            "example mobile",
            None
        ));
        assert!(same_service(
            "Example",
            Some("https://www.example.com/a"),
            "Example Store",
            Some("https://example.com")
        ));
        assert!(!same_service(
            "Example",
            Some("https://example.com"),
            "Other",
            Some("https://other.example.com")
        ));
        assert!(!same_service(
            "Example",
            None,
            "Other",
            Some("https://example.com")
        ));
    }

    #[test]
    fn like_pattern_escapes_wildcards() {
        assert_eq!(like_pattern("abc"), "%abc%");
        assert_eq!(like_pattern(" 100%_off\\ "), "%100\\%\\_off\\\\%");
    }

    #[test]
    fn validation_trims_and_clears_empty_urls() {
        let mut input = fields();
        input.name = "  Example Mobile  ".into();
        input.website_url = Some("  ".into());
        input.note = "  line1\nline2  ".into();
        let valid = input.validate().unwrap();
        assert_eq!(valid.name, "Example Mobile");
        assert_eq!(valid.website_url, None);
        assert_eq!(valid.note, "line1\nline2");
    }

    #[test]
    fn validation_rejects_bad_values() {
        let cases: [fn(&mut Fields); 7] = [
            |f| f.name = "   ".into(),
            |f| f.name = "a".repeat(MAX_NAME + 1),
            |f| f.name = "two\nlines".into(),
            |f| f.website_url = Some("javascript:alert(1)".into()),
            |f| f.address_change_url = Some("example.com/address".into()),
            |f| f.source = String::new(),
            |f| f.note = "\u{7}".into(),
        ];
        for (index, change) in cases.iter().enumerate() {
            let mut input = fields();
            change(&mut input);
            assert!(input.validate().is_err(), "case {index}");
        }
    }

    #[test]
    fn new_service_defaults_and_rejects_unknown_keys() {
        let new: NewService =
            serde_json::from_value(serde_json::json!({"name":"Example","source":"test"})).unwrap();
        let valid = new.validate().unwrap();
        assert_eq!(valid.address_status, AddressStatus::Unknown);
        assert_eq!(valid.change_method, ChangeMethod::Unknown);
        assert!(
            serde_json::from_value::<NewService>(
                serde_json::json!({"name":"Example","source":"test","password":"x"})
            )
            .is_err()
        );
    }

    #[test]
    fn patch_keeps_omitted_clears_null_and_revalidates() {
        let current = fields().validate().unwrap();
        let patch: Patch = serde_json::from_value(serde_json::json!({
            "website_url": null,
            "address_status": "changed"
        }))
        .unwrap();
        let next = patch.apply(&current).unwrap();
        assert_eq!(next.website_url, None);
        assert_eq!(next.address_status, AddressStatus::Changed);
        assert_eq!(next.name, current.name);
        let bad: Patch = serde_json::from_value(serde_json::json!({"name": ""})).unwrap();
        assert!(bad.apply(&current).is_err());
        assert!(serde_json::from_value::<Patch>(serde_json::json!({"status":"old"})).is_err());
    }

    #[test]
    fn enums_round_trip_their_wire_names() {
        for s in ["old", "changed", "unknown"] {
            assert_eq!(AddressStatus::parse(s).unwrap().as_str(), s);
        }
        for s in ["bulk", "member_page", "phone_or_paper", "unknown"] {
            assert_eq!(ChangeMethod::parse(s).unwrap().as_str(), s);
        }
        assert_eq!(AddressStatus::parse("Old"), None);
    }
}
