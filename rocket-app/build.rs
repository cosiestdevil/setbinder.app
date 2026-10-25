use lightningcss::{printer::PrinterOptions, stylesheet::ParserOptions};
use path_slash::PathBufExt;
use std::{env, fs, path::PathBuf};
mod build_assets;
const CSS: &str = include_str!("./static/style.css");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=static/style.css");
    println!("cargo:rerun-if-changed=static/app.js");
    println!("cargo:rerun-if-changed=static/browse.mjs");
    println!("cargo:rerun-if-changed=build_assets.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let styles = lightningcss::stylesheet::StyleSheet::parse(CSS, ParserOptions::default())?;
    let css = styles.to_css(PrinterOptions {
        minify: true,
        ..PrinterOptions::default()
    })?;
    let assets = build_assets::bundle(
        &css.code,
        include_str!("static/app.js"),
        include_str!("static/browse.mjs"),
    );
    fs::write(out_dir.join("styles.min.css"), css.code)?;
    fs::write(
        out_dir.join("assets.rs"),
        format!(
            "pub const CSS_URL: &str = {:?};\npub const APP_URL: &str = {:?};\npub const BROWSE_URL: &str = {:?};\npub const APP_JS: &str = {:?};\n",
            assets.css_url, assets.app_url, assets.browse_url, assets.app_js
        ),
    )?;

    let templates = fs::read_dir("templates")?;
    let mut rust_code = "pub const TEMPLATES:&[(&'static str,&'static str)] = &[".to_string();
    let base_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    for file in templates {
        if let Ok(file) = file {
            let path = file.path();
            let name = PathBuf::from(&path.file_stem().unwrap());
            let name = &name.file_stem().unwrap();
            //let name = file.file_name();
            let name = name.to_str().unwrap();
            rust_code = format!(
                "{rust_code}\n\t(\"{name}\",include_str!(\"{}\")),",
                base_path.join(&path).to_slash().unwrap()
            );
            println!("cargo:rerun-if-changed={}", &path.to_slash().unwrap());
        }
    }
    rust_code = format!("{rust_code}\n];");
    fs::write(out_dir.join("templates.rs"), rust_code)?;
    Ok(())
}
