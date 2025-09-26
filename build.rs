use std::{env, fs, path::PathBuf};
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::ParserOptions,
};
const CSS: &str = include_str!("./static/style.css");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=static/style.css");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let styles = lightningcss::stylesheet::StyleSheet::parse(CSS, ParserOptions::default())?;
    let css = styles.to_css(PrinterOptions{minify:true,..PrinterOptions::default()})?;
    fs::write(out_dir.join("styles.min.css"), css.code)?;
    Ok(())
}
