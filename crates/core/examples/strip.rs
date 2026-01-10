use std::{fs, path::PathBuf};

use clap::Parser;
use strip_whitespace::{
    Language as LanguageInner, StripConfig, strip_whitespace, strip_whitespace_no_sourcemap,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Language {
    Astro,
    Svelte,
}

impl From<Language> for LanguageInner {
    fn from(value: Language) -> Self {
        match value {
            Language::Astro => LanguageInner::Astro,
            Language::Svelte => LanguageInner::Svelte,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "strip")]
#[command(about = "Strip whitespace between markup nodes while preserving line/col as much as possible", long_about = None)]
struct Args {
    /// Path to the source file to transform
    input: PathBuf,

    /// Output path for transformed source (defaults to stdout)
    #[arg(long, short)]
    out: Option<PathBuf>,

    /// Output path for updated sourcemap (defaults to <out>.map if --out is provided)
    #[arg(long)]
    out_sourcemap: Option<PathBuf>,

    /// Override language instead of inferring from file extension
    #[arg(long, short)]
    language: Option<Language>,

    /// Preserve blank-line gaps ("\n\n" / "\r\n\r\n") between nodes
    #[arg(long)]
    preserve_blank_lines: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let source = fs::read_to_string(&args.input)?;

    let language = if let Some(lang) = &args.language {
        *lang
    } else if let Some(ext) = args.input.extension().and_then(|e| e.to_str()) {
        match ext {
            "astro" => Language::Astro,
            "svelte" => Language::Svelte,
            _ => {
                return Err(std::io::Error::other(format!(
                    "could not infer language from file extension: .{}",
                    ext
                ))
                .into());
            }
        }
    } else {
        return Err(std::io::Error::other(
            "could not infer language: no file extension and --language not provided",
        )
        .into());
    };
    let language: LanguageInner = language.into();

    let cfg = StripConfig {
        preserve_blank_lines: args.preserve_blank_lines,
    };

    let out_map_path = args.out_sourcemap.clone().or_else(|| {
        args.out
            .as_ref()
            .map(|out| PathBuf::from(format!("{}.map", out.display())))
    });

    let out_code = if let Some(out_map_path) = out_map_path {
        let res = strip_whitespace(
            &source,
            args.input
                .file_name()
                .and_then(|n| n.to_str())
                .expect("input file must have a valid filename"),
            language,
            &cfg,
        )
        .map_err(std::io::Error::other)?;
        fs::write(out_map_path, res.map)?;

        res.code
    } else {
        strip_whitespace_no_sourcemap(&source, language, &cfg).map_err(std::io::Error::other)?
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
