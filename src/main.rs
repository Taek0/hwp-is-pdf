use std::path::PathBuf;
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -- INPUT.hwp|INPUT.hwpx OUTPUT.pdf [FONT_DIRECTORY]");
        std::process::exit(2)
    }
    let fonts = args.get(2).map(PathBuf::from).unwrap_or_else(|| {
        if cfg!(windows) {
            PathBuf::from("C:/Windows/Fonts")
        } else {
            PathBuf::from("/mnt/c/Windows/Fonts")
        }
    });
    let r = hwp_engine::convert(&PathBuf::from(&args[0]), &PathBuf::from(&args[1]), &fonts)?;
    println!(
        "{} pages; {} equations; {} pictures; {} table draws; {} warnings",
        r.pages,
        r.equations,
        r.pictures,
        r.tables,
        r.warnings.len()
    );
    Ok(())
}
