//! The page of the real binary in a real browser, for what only a browser's engine shows:
//! where things lie and how big they are, the colours the theme resolves to, scrolling, and
//! the focus the keyboard moves with its ring. The rest of the page is tested in jsdom
//! (client/src/**/*.test.ts).
//!
//! One browser and one ledger: the server starts on a free port with a throwaway database of
//! fictional services, and the tests run one after another, in the order they are written.
//! `cargo test --test page -- <words>` runs the tests whose titles hold any of the words;
//! `--skip <words>` leaves those out.
#![allow(clippy::unwrap_used)] // helpers outside #[test] fns
use serde_json::{Value, json};
use std::{
    future::Future,
    pin::Pin,
    process::{Child, Command, ExitCode, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

use browser::{Page, TRIES, free_port};

#[path = "support/browser.rs"]
mod browser;

type Run = Pin<Box<dyn Future<Output = ()> + Send>>;
/// A test: its title, and what it does on the page and the ledger.
type Test = (&'static str, fn(Arc<Page>, Arc<Ledger>) -> Run);

macro_rules! test {
    ($title:literal, $test:ident) => {
        ($title, |page, ledger| Box::pin($test(page, ledger)))
    };
}

/// The tests, in the order they run.
const TESTS: &[Test] = &[
    test!("the list flows with the page", list_flows),
    test!(
        "375px: the address link is a 44px target",
        address_link_target
    ),
    test!("320px: the edit page does not spill sideways", edit_fits),
    test!(
        "keyboard reaches search, filters, cards and the address link with a visible ring",
        keyboard
    ),
    test!("theme choice overrides the OS and persists", theme),
];

/// The page's background in each theme.
const DARK: &str = "rgb(25, 25, 25)";
const LIGHT: &str = "rgb(250, 246, 239)";
const STATUSES: [&str; 3] = ["old", "unknown", "changed"];

/// The server binary on a free port, with a throwaway database of fictional services.
struct Ledger {
    server: Child,
    base: String,
    /// The id of the first service, `Example Service 001`.
    first: i64,
    _data: tempfile::TempDir,
}

impl Ledger {
    async fn start() -> Self {
        let data = tempfile::tempdir().unwrap();
        let http = reqwest::Client::new();
        let mut tries = 0..TRIES;
        // a port that was free may be taken by the time the server binds it
        let (server, base) = loop {
            assert!(tries.next().is_some(), "the server did not start");
            let port = free_port();
            let mut server = Command::new(env!("CARGO_BIN_EXE_service-ledger"))
                .env("PORT", port.to_string())
                .env("DB_PATH", data.path().join("ledger.db"))
                .env("LOG_LEVEL", "off")
                .stdout(Stdio::null())
                .spawn()
                .unwrap();
            let base = format!("http://127.0.0.1:{port}");
            if answers(&http, &base, &mut server).await {
                break (server, base);
            }
            let _ = server.wait();
        };
        let mut ledger = Self {
            server,
            base,
            first: 0,
            _data: data,
        };
        for i in 1..=100 {
            let n = format!("{i:03}");
            let id = ledger
                .create(
                    &http,
                    json!({
                        "name": format!("Example Service {n}"),
                        "website_url": format!("https://service{n}.example.com/"),
                        "address_change_url": (i % 2 == 1)
                            .then(|| format!("https://service{n}.example.com/address")),
                        "address_status": STATUSES[i % 3],
                        "change_method": if i % 4 == 0 { "phone_or_paper" } else { "member_page" },
                    }),
                )
                .await;
            if i == 1 {
                ledger.first = id;
            }
        }
        ledger
            .create(
                &http,
                json!({
                    "name": "Example 長い名前のサービス".repeat(8),
                    "address_status": "old",
                    "address_change_url": "https://long.example.net/address",
                }),
            )
            .await;
        ledger
    }

    /// Registers a service through the API and answers with its id.
    async fn create(&self, http: &reqwest::Client, fields: Value) -> i64 {
        let mut body = json!({"source": "page test fixture"});
        body.as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        let answer = http
            .post(self.url("/api/services"))
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = answer.status();
        let said: Value = answer.json().await.unwrap();
        assert_eq!(status.as_u16(), 201, "{body} was not registered: {said}");
        said["id"].as_i64().unwrap()
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}

impl Drop for Ledger {
    fn drop(&mut self) {
        let _ = self.server.kill();
        let _ = self.server.wait();
    }
}

/// Waits until the server answers its health check. `false` when it ended instead.
async fn answers(http: &reqwest::Client, base: &str, server: &mut Child) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(answer) = http.get(format!("{base}/healthz")).send().await {
            return answer.status().is_success();
        }
        if server.try_wait().unwrap().is_some() {
            return false;
        }
        assert!(Instant::now() < deadline, "the server did not answer");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// ---- on the page ----

/// Clicks the element `element` (a JavaScript expression, given `args` as `arguments`) with
/// the mouse at its middle, once it is in sight and nothing else lies over that point.
async fn click(page: &Page, what: &str, element: &str, args: Value) {
    let script = format!(
        "const element = {element}; \
         if (!element) return null; \
         element.scrollIntoView({{block: 'nearest'}}); \
         const box = element.getBoundingClientRect(); \
         const x = box.left + box.width / 2, y = box.top + box.height / 2; \
         const hit = document.elementFromPoint(x, y); \
         return hit && element.contains(hit) ? [x, y] : null;"
    );
    let at = page
        .until(&format!("{what} can be clicked"), &script, args)
        .await;
    let (x, y) = (at[0].as_f64().unwrap(), at[1].as_f64().unwrap());
    page.mouse("moved", x, y).await;
    page.mouse("pressed", x, y).await;
    page.mouse("released", x, y).await;
}

/// Chooses `choice` in the theme window, opened from the menu.
async fn choose_theme(page: &Page, choice: &str) {
    click(
        page,
        "the menu",
        "document.querySelector('button[aria-label=\"メニュー\"]')",
        Value::Null,
    )
    .await;
    click(
        page,
        "テーマ設定",
        "[...document.querySelectorAll('nav button')].find((b) => b.textContent.trim() === 'テーマ設定')",
        Value::Null,
    )
    .await;
    click(
        page,
        choice,
        "[...document.querySelectorAll('[role=radio]')].find((r) => r.textContent.trim() === arguments[0])",
        json!(choice),
    )
    .await;
}

/// Waits until the page's background is `colour`.
async fn background(page: &Page, what: &str, colour: &str) {
    page.until(
        what,
        "return getComputedStyle(document.body).backgroundColor === arguments[0];",
        json!(colour),
    )
    .await;
}

// ---- the tests ----

/// Where the cards lie in the window, measured after they have all come.
const GEOMETRY: &str = "const cards = [...document.querySelectorAll('.card')] \
       .map((card) => card.getBoundingClientRect()); \
     const list = document.querySelector('.cards'); \
     return { \
       firstTop: cards[0].top, \
       visible: cards.filter((c) => c.top >= 48 && c.bottom <= innerHeight).length, \
       innerScroll: list.scrollHeight - list.clientHeight, \
       documentWidth: document.documentElement.scrollWidth, \
       background: getComputedStyle(document.body).backgroundColor, \
     };";

/// At desktop and phone widths in both themes, the 101 cards lie in the page, which scrolls
/// as a whole: nothing spills sideways, the list does not scroll inside, the first cards
/// come within the budgets of DESIGN.md, and the background is the theme's.
async fn list_flows(page: Arc<Page>, ledger: Arc<Ledger>) {
    for (width, height) in [(1440, 900), (375, 812), (320, 640)] {
        for theme in ["dark", "light"] {
            let look = format!("{width} {theme}");
            page.look(theme, width, height).await;
            page.open(&ledger.url("/")).await;
            page.until(
                &format!("{look}: 101 cards"),
                "return document.querySelectorAll('.card').length === 101;",
                Value::Null,
            )
            .await;
            let geometry = page.js(GEOMETRY, Value::Null).await;
            assert_eq!(
                geometry["documentWidth"],
                json!(width),
                "{look}: the page is wider than the window ({geometry})"
            );
            assert!(
                geometry["innerScroll"].as_f64().unwrap() <= 1.0,
                "{look}: the list scrolls inside ({geometry})"
            );
            // budgets measured with the header, search field and filters (DESIGN.md)
            let narrow = width == 320;
            assert!(
                geometry["firstTop"].as_f64().unwrap() <= if narrow { 260.0 } else { 210.0 },
                "{look}: the first card starts too low ({geometry})"
            );
            assert!(
                geometry["visible"].as_u64().unwrap() >= if narrow { 4 } else { 8 },
                "{look}: too few cards are wholly in sight ({geometry})"
            );
            assert_eq!(
                geometry["background"],
                json!(if theme == "dark" { DARK } else { LIGHT }),
                "{look}: the background is not the theme's"
            );
            page.wheel(f64::from(width) / 2.0, f64::from(height) / 2.0, 600.0)
                .await;
            page.until(
                &format!("{look}: the page scrolls with the wheel"),
                "return scrollY > 500;",
                Value::Null,
            )
            .await;
        }
    }
}

/// At 375px the link to a service's address page is a target 44px tall.
async fn address_link_target(page: Arc<Page>, ledger: Arc<Ledger>) {
    page.look("light", 375, 812).await;
    page.open(&ledger.url("/?q=service001")).await;
    let height = page
        .until(
            "the address link of Example Service 001",
            "const link = document.querySelector('.change-link'); \
             return link && link.getBoundingClientRect().height;",
            Value::Null,
        )
        .await;
    assert!(
        height.as_f64().unwrap() >= 44.0,
        "the address link is {height}px tall"
    );
}

/// At 320px the edit page of a service is no wider than the window.
async fn edit_fits(page: Arc<Page>, ledger: Arc<Ledger>) {
    page.look("light", 320, 640).await;
    page.open(&ledger.url(&format!("/services/{}", ledger.first)))
        .await;
    page.until(
        "the edit page of Example Service 001",
        "return document.querySelector('h1.sub-title')?.textContent.trim() === 'Example Service 001';",
        Value::Null,
    )
    .await;
    let width = page
        .js("return document.documentElement.scrollWidth;", Value::Null)
        .await;
    assert_eq!(
        width,
        json!(320),
        "the edit page is {width}px wide at 320px"
    );
}

/// Tab goes from the search field through the filters to the card and its address link, the
/// card showing a solid ring, and Enter on the card opens it.
async fn keyboard(page: Arc<Page>, ledger: Arc<Ledger>) {
    page.open(&ledger.url("/?q=service001")).await;
    page.until(
        "one card",
        "return document.querySelectorAll('.card').length === 1;",
        Value::Null,
    )
    .await;
    page.js(
        "document.querySelector('input[type=search]').focus();",
        Value::Null,
    )
    .await;
    for expected in ["すべて", "旧住所", "不明", "変更済み"] {
        page.key("Tab").await;
        page.until(
            &format!("the focus is on {expected}"),
            "return document.activeElement?.textContent.includes(arguments[0]);",
            json!(expected),
        )
        .await;
    }
    page.key("Tab").await;
    page.until(
        "the focus is on the card, with a solid ring",
        "const at = document.activeElement; \
         return at?.matches('.card-main') && getComputedStyle(at).outlineStyle === 'solid';",
        Value::Null,
    )
    .await;
    page.key("Tab").await;
    page.until(
        "the focus is on the address link",
        "return document.activeElement?.matches('.change-link');",
        Value::Null,
    )
    .await;
    page.key("Shift+Tab").await;
    page.until(
        "the focus is back on the card",
        "return document.activeElement?.matches('.card-main');",
        Value::Null,
    )
    .await;
    page.key("Enter").await;
    page.until(
        "the card's page is open",
        "return /^\\/services\\/\\d+$/.test(location.pathname);",
        Value::Null,
    )
    .await;
}

/// Under a dark system, ライト makes the page light and stays after a reload, and 自動 follows
/// the system again.
async fn theme(page: Arc<Page>, ledger: Arc<Ledger>) {
    page.look("dark", 1280, 720).await;
    page.open(&ledger.url("/")).await;
    choose_theme(&page, "ライト").await;
    background(&page, "the page is light", LIGHT).await;
    page.key("Escape").await;
    page.open(&ledger.url("/")).await;
    page.until(
        "the light theme is kept after a reload",
        "return document.documentElement.dataset.theme === 'light';",
        Value::Null,
    )
    .await;
    choose_theme(&page, "自動").await;
    background(&page, "the page follows the dark system", DARK).await;
}

// ---- running them ----

/// Which tests a run takes, as `cargo test` passes its words on: titles that hold any of the
/// words (all, when there are none), and none that hold a word after `--skip`; with
/// `--exact`, a word is a whole title. Other options are left alone.
fn chosen(args: &[String]) -> impl Fn(&str) -> bool {
    let mut words = Vec::new();
    let mut skipped = Vec::new();
    let mut exact = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--skip" => skipped.extend(args.next().cloned()),
            "--exact" => exact = true,
            option if option.starts_with('-') => {}
            word => words.push(word.to_owned()),
        }
    }
    move |title: &str| {
        let holds = |word: &String| {
            if exact {
                title == word
            } else {
                title.contains(word.as_str())
            }
        };
        (words.is_empty() || words.iter().any(holds)) && !skipped.iter().any(holds)
    }
}

/// What a panic said.
fn said(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|&said| said.to_owned()))
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let chosen = chosen(&args);
    let tests: Vec<&Test> = TESTS.iter().filter(|test| chosen(test.0)).collect();
    if args.iter().any(|arg| arg == "--list") {
        for (title, _) in &tests {
            println!("{title}: test");
        }
        return ExitCode::SUCCESS;
    }
    println!("\nrunning {} tests", tests.len());
    if tests.is_empty() {
        return ExitCode::SUCCESS;
    }
    let began = Instant::now();
    let ledger = Arc::new(Ledger::start().await);
    let page = Arc::new(Page::start().await);
    let mut failed = Vec::new();
    for (title, test) in &tests {
        // each test begins on a page of no state, in a window of 1280 by 720 in the light
        page.open(&ledger.url("/healthz")).await;
        page.js("localStorage.clear(); sessionStorage.clear();", Value::Null)
            .await;
        page.look("light", 1280, 720).await;
        let began = Instant::now();
        let ran = tokio::spawn(test(page.clone(), ledger.clone())).await;
        let took = began.elapsed().as_secs_f64();
        match ran {
            Ok(()) => println!("test {title} ... ok ({took:.2}s)"),
            Err(error) => {
                println!("test {title} ... FAILED: {}", said(&*error.into_panic()));
                failed.push(*title);
            }
        }
    }
    let page = Arc::try_unwrap(page).ok().unwrap();
    page.close().await;
    drop(ledger);
    let seconds = began.elapsed().as_secs_f64();
    let count = tests.len();
    if failed.is_empty() {
        println!("\ntest result: ok. {count} passed; 0 failed; finished in {seconds:.2}s\n");
        ExitCode::SUCCESS
    } else {
        println!("\nfailures:");
        for title in &failed {
            println!("    {title}");
        }
        println!(
            "\ntest result: FAILED. {} passed; {} failed; finished in {seconds:.2}s\n",
            count - failed.len(),
            failed.len()
        );
        ExitCode::from(101)
    }
}
