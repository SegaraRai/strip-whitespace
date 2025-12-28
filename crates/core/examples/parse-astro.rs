use std::{fs, path::PathBuf};

use clap::Parser;
use tree_sitter::Parser as TsParser;

#[derive(Parser, Debug)]
#[command(name = "parse-astro")]
#[command(about = "Parse an .astro file with tree-sitter-astro and print the CST", long_about = None)]
struct Args {
    /// Path to the .astro file to parse
    input: PathBuf,

    /// Print the tree in S-expression format instead of the default dump format
    #[arg(long, short)]
    sexp: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let source = fs::read_to_string(&args.input)?;

    let mut parser = TsParser::new();
    let language = tree_sitter_astro::LANGUAGE;
    parser.set_language(&language.into())?;

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
