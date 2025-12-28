use std::{fs, path::PathBuf};

use astro_strip_whitespace::strip::{
    StripConfig, strip_astro_whitespace_no_sourcemap, strip_astro_whitespace_sourcemap_create,
    strip_astro_whitespace_sourcemap_rewrite,
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "strip-astro")]
#[command(about = "Strip whitespace between Astro markup nodes while preserving line/col as much as possible", long_about = None)]
struct Args {
    /// Path to the .astro file to transform
    input: PathBuf,

    /// Optional input sourcemap (.map) to update
    #[arg(long)]
    sourcemap: Option<PathBuf>,

    /// Output path for transformed Astro (defaults to stdout)
    #[arg(long, short)]
    out: Option<PathBuf>,

    /// Output path for updated sourcemap (defaults to <out>.map if --out is provided)
    #[arg(long)]
    out_sourcemap: Option<PathBuf>,

    /// Disable emitting a sourcemap
    #[arg(long)]
    no_sourcemap: bool,

    /// Preserve blank-line gaps ("\n\n" / "\r\n\r\n") between nodes
    #[arg(long)]
    preserve_blank_lines: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let source = fs::read_to_string(&args.input)?;
    let input_map = match &args.sourcemap {
        None => None,
        Some(p) => Some(fs::read_to_string(p)?),
    };

    let cfg = StripConfig {
        preserve_blank_lines: args.preserve_blank_lines,
    };

    let (code, sourcemap) = match (args.no_sourcemap, input_map) {
        (true, None) => {
            let res = strip_astro_whitespace_no_sourcemap(&source, &cfg)
                .map_err(std::io::Error::other)?;
            (res, None)
        }
        (false, None) => {
            let res = strip_astro_whitespace_sourcemap_create(
                &source,
                args.input
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("input.astro"),
                &cfg,
            )
            .map_err(std::io::Error::other)?;
            (res.code, Some(res.sourcemap))
        }
        (false, Some(input_map)) => {
            let res = strip_astro_whitespace_sourcemap_rewrite(&source, &input_map, &cfg)
                .map_err(std::io::Error::other)?;
            (res.code, Some(res.sourcemap))
        }
        (true, Some(_)) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot provide input sourcemap when --no-sourcemap is set",
            )
            .into());
        }
    };

    match &args.out {
        None => {
            print!("{code}");
        }
        Some(out) => {
            fs::write(out, code)?;
        }
    }

    if let Some(map) = sourcemap {
        let out_map_path = if let Some(p) = &args.out_sourcemap {
            p.clone()
        } else if let Some(out) = &args.out {
            PathBuf::from(format!("{}.map", out.display()))
        } else {
            // No sensible default without an output file.
            return Ok(());
        };

        fs::write(out_map_path, map)?;
    }

    Ok(())
}
