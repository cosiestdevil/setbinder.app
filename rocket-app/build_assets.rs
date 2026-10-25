use sha2::{Digest, Sha256};

pub struct Assets {
    pub css_url: String,
    pub app_url: String,
    pub browse_url: String,
    pub app_js: String,
}

fn url(name: &str, extension: &str, content: &str) -> String {
    let hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    format!("/assets/{name}.{}.{extension}", &hash[..32])
}

pub fn bundle(css: &str, app: &str, browse: &str) -> Assets {
    let browse_url = url("browse", "mjs", browse);
    // Hash the final entry module so a dependency change also changes its URL.
    assert!(
        app.contains("'/browse.mjs'"),
        "browse module import must be rewritten"
    );
    let app_js = app.replace("'/browse.mjs'", &format!("'{browse_url}'"));
    Assets {
        css_url: url("style", "css", css),
        app_url: url("app", "js", &app_js),
        browse_url,
        app_js,
    }
}
