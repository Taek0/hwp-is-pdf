use std::path::PathBuf;
fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let contours = args.iter().any(|a| a == "--contour-wrap");
    args.retain(|a| a != "--contour-wrap");
    let force = args
        .iter()
        .any(|a| matches!(a.as_str(), "--reflow" | "--reflow-text"));
    args.retain(|a| !matches!(a.as_str(), "--reflow" | "--reflow-text"));
    if args.len() < 2 {
        eprintln!(
            "Usage: cargo run -- INPUT.hwp|INPUT.hwpx OUTPUT.pdf [FONT_DIRECTORY] [--reflow] [--contour-wrap]"
        );
        eprintln!("본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.");
        std::process::exit(2)
    }
    let fonts = args.get(2).map(PathBuf::from).unwrap_or_else(|| {
        if cfg!(windows) {
            PathBuf::from("C:/Windows/Fonts")
        } else {
            PathBuf::from("/mnt/c/Windows/Fonts")
        }
    });
    let r = hwp_engine::convert_with_options(
        &PathBuf::from(&args[0]),
        &PathBuf::from(&args[1]),
        &fonts,
        force,
        contours,
    )?;
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
