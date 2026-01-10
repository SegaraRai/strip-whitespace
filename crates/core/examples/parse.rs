use std::{fs, path::PathBuf};

use clap::Parser;
use tree_sitter::Parser as TsParser;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Language {
    Astro,
    Svelte,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "astro" => Some(Language::Astro),
            "svelte" => Some(Language::Svelte),
            _ => None,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "parse")]
#[command(about = "Parse a source file with tree-sitter and print the CST", long_about = None)]
struct Args {
    /// Path to the source file to parse
    input: PathBuf,

    /// Override language instead of inferring from file extension
    #[arg(long, short)]
    language: Option<Language>,

    /// Print the tree in S-expression format instead of the default dump format
    #[arg(long, short)]
    sexp: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let language = if let Some(lang) = &args.language {
        *lang
    } else if let Some(ext) = args.input.extension().and_then(|e| e.to_str()) {
        Language::from_extension(ext).ok_or_else(|| {
            std::io::Error::other(format!(
                "could not infer language from file extension: .{}",
                ext
            ))
        })?
    } else {
        return Err(std::io::Error::other(
            "could not infer language: no file extension and --language not provided",
        )
        .into());
    };

    let source = fs::read_to_string(&args.input)?;

    let mut parser = TsParser::new();
    let language_fn = match language {
        Language::Astro => tree_sitter_astro::LANGUAGE,
        Language::Svelte => tree_sitter_svelte_ng::LANGUAGE,
    };
    parser.set_language(&language_fn.into())?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| std::io::Error::other("tree-sitter failed to parse input"))?;

    let root = tree.root_node();

    if args.sexp {
        println!("{}", root.to_sexp());
    } else {
        dump_tree(&source, root, 0);
    }

    Ok(())
}

fn dump_tree(source: &str, node: tree_sitter::Node<'_>, depth: usize) {
    let indent = "  ".repeat(depth);

    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    let start = node.start_position();
    let end = node.end_position();

    let text_preview = node
        .utf8_text(source.as_bytes())
        .ok()
        .map(|t| t.replace('\n', "\\n"))
        .unwrap_or_else(|| "<non-utf8>".to_string());

    println!(
        "{indent}{kind} [{start_byte}..{end_byte}] ({sl}:{sc})..({el}:{ec}) \"{text}\"",
        kind = node.kind(),
        sl = start.row,
        sc = start.column,
        el = end.row,
        ec = end.column,
        text = truncate(&text_preview, 120),
    );

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        dump_tree(source, child, depth + 1);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }

    // Try to truncate on a UTF-8 boundary.
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}
