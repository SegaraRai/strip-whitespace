use std::{fs, path::PathBuf};

use clap::Parser;
use strip_whitespace::strip::{
    StripConfig, strip_astro_whitespace, strip_astro_whitespace_no_sourcemap,
};

#[derive(Parser, Debug)]
#[command(name = "strip-astro")]
#[command(about = "Strip whitespace between Astro markup nodes while preserving line/col as much as possible", long_about = None)]
struct Args {
    /// Path to the .astro file to transform
    input: PathBuf,

    /// Output path for transformed Astro (defaults to stdout)
    #[arg(long, short)]
    out: Option<PathBuf>,

    /// Output path for updated sourcemap (defaults to <out>.map if --out is provided)
    #[arg(long)]
    out_sourcemap: Option<PathBuf>,

    /// Preserve blank-line gaps ("\n\n" / "\r\n\r\n") between nodes
    #[arg(long)]
    preserve_blank_lines: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let source = fs::read_to_string(&args.input)?;

    let cfg = StripConfig {
        preserve_blank_lines: args.preserve_blank_lines,
    };

    let out_map_path = args.out_sourcemap.clone().or_else(|| {
        args.out
            .as_ref()
            .map(|out| PathBuf::from(format!("{}.map", out.display())))
    });

    let out_code = if let Some(out_map_path) = out_map_path {
        let res = strip_astro_whitespace(
            &source,
            args.input
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("input.astro"),
            &cfg,
        )
        .map_err(std::io::Error::other)?;

        fs::write(out_map_path, res.sourcemap)?;

        res.code
    } else {
        strip_astro_whitespace_no_sourcemap(&source, &cfg).map_err(std::io::Error::other)?
    };

    match &args.out {
        None => {
            print!("{out_code}");
        }
        Some(out) => {
            fs::write(out, out_code)?;
        }
    }

    Ok(())
}
