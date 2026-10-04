//! 将 tracing-flame 的脱敏 folded 栈转换为标准 Inferno SVG。
use std::{
    error::Error,
    fs::File,
    io::{BufReader, Write},
    path::Path,
};

fn generate(input: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    let reader = BufReader::new(File::open(input)?);
    let mut svg = tempfile::NamedTempFile::new_in(
        output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    let mut options = inferno::flamegraph::Options::default();
    options.title = "Socks Proxy runtime spans".into();
    options.count_name = "nanoseconds".into();
    inferno::flamegraph::from_reader(&mut options, reader, &mut svg)?;
    svg.flush()?;
    svg.persist(output)?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = args
        .next()
        .ok_or("Usage: runtime-flamegraph <profile.folded> <flamegraph.svg>")?;
    let output = args.next().ok_or("Missing output SVG path")?;
    if args.next().is_some() {
        return Err("Unexpected argument".into());
    }
    generate(Path::new(&input), Path::new(&output))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folded_stacks_generate_svg_and_bad_input_preserves_existing_output(
    ) -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let input = directory.path().join("profile.folded");
        let output = directory.path().join("profile.svg");
        std::fs::write(
            &input,
            "root;configuration_apply;runtime_start 1000\nroot;configuration_load 500\n",
        )?;
        generate(&input, &output)?;
        let svg = std::fs::read_to_string(&output)?;
        assert!(svg.contains("<svg"));
        assert!(svg.contains("configuration_apply"));
        assert!(svg.contains("nanoseconds"));
        assert!(generate(&directory.path().join("missing"), &output).is_err());
        assert_eq!(std::fs::read_to_string(&output)?, svg);
        Ok(())
    }
}
