#[path = "../build_assets.rs"]
mod build_assets;

#[test]
fn hashes_follow_content_and_imported_dependencies() {
    let first = build_assets::bundle("css", "import '/browse.mjs';", "browse");
    let same = build_assets::bundle("css", "import '/browse.mjs';", "browse");
    assert_eq!(first.css_url, same.css_url);
    assert_eq!(first.app_url, same.app_url);
    assert!(first.app_js.contains(&first.browse_url));
    assert!(!first.app_js.contains("'/browse.mjs'"));
    let dependency = build_assets::bundle("css", "import '/browse.mjs';", "new browse");
    assert_ne!(first.browse_url, dependency.browse_url);
    assert_ne!(first.app_url, dependency.app_url);
    assert_eq!(first.css_url, dependency.css_url);
    let style = build_assets::bundle("new css", "import '/browse.mjs';", "browse");
    assert_ne!(first.css_url, style.css_url);
    assert_eq!(first.app_url, style.app_url);
    let entry = build_assets::bundle("css", "import '/browse.mjs'; changed();", "browse");
    assert_ne!(first.app_url, entry.app_url);
    assert_eq!(first.browse_url, entry.browse_url);
}
