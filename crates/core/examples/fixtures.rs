use std::{fs, path::PathBuf};

use clap::Parser;
use strip_whitespace::{Language, StripConfig, strip_whitespace};

#[derive(Parser, Debug)]
#[command(name = "fixtures")]
#[command(about = "Create or validate fixture files", long_about = None)]
struct Args {
    /// Write fixtures instead of validating them
    #[arg(long, short)]
    write: bool,

    /// Path to the fixtures directory (defaults to "./fixtures")
    #[arg(long, default_value = "fixtures")]
    dir: PathBuf,

    /// Preserve blank-line gaps ("\n\n" / "\r\n\r\n") between nodes
    #[arg(long)]
    preserve_blank_lines: bool,
}

fn detect_language(filename: &str) -> Option<Language> {
    if filename.ends_with(".astro") {
        Some(Language::Astro)
    } else if filename.ends_with(".svelte") {
        Some(Language::Svelte)
    } else {
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let cfg = StripConfig {
        preserve_blank_lines: args.preserve_blank_lines,
    };

    // Read all files in the fixtures directory
    let entries = fs::read_dir(&args.dir)?;

    let mut input_files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid filename")?;

        // Skip .out. files and files without supported extensions
        if filename.contains(".out.") {
            continue;
        }

        if detect_language(filename).is_some() {
            input_files.push(path);
        }
    }

    input_files.sort();

    if args.write {
        println!("Creating fixtures...");
        for input_path in &input_files {
            let filename = input_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("invalid filename")?;

            let extension = filename
                .rsplit_once('.')
                .ok_or("filename missing extension")?
                .1;
            let basename = filename
                .rsplit_once('.')
                .ok_or("filename missing extension")?
                .0;

            let language = detect_language(filename).ok_or("unsupported language")?;

            let source = fs::read_to_string(input_path)?;
            let res = strip_whitespace(&source, filename, language, &cfg)?;

            // Determine output paths
            let out_path = args.dir.join(format!("{basename}.out.{extension}"));
            let out_map_path = args.dir.join(format!("{basename}.out.{extension}.map"));

            fs::write(&out_path, &res.code)?;
            fs::write(&out_map_path, &res.map)?;

            println!(
                "  Created {} and {}",
                out_path.display(),
                out_map_path.display()
            );
        }
        println!("Done creating {} fixtures.", input_files.len());
    } else {
        println!("Validating fixtures...");
        let mut mismatches = Vec::new();

        for input_path in &input_files {
            let filename = input_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("invalid filename")?;

            let extension = filename
                .rsplit_once('.')
                .ok_or("filename missing extension")?
                .1;
            let basename = filename
                .rsplit_once('.')
                .ok_or("filename missing extension")?
                .0;

            let language = detect_language(filename).ok_or("unsupported language")?;

            let source = fs::read_to_string(input_path)?;
            let res = strip_whitespace(&source, filename, language, &cfg)?;

            // Determine expected output paths
            let out_path = args.dir.join(format!("{basename}.out.{extension}"));
            let out_map_path = args.dir.join(format!("{basename}.out.{extension}.map"));

            // Check code output
            if out_path.exists() {
                let expected_code = fs::read(out_path.clone())?;
                if res.code.as_bytes() != expected_code {
                    mismatches.push(format!("{filename}: code mismatch"));
                }
            } else {
                mismatches.push(format!(
                    "{filename}: missing output file {}",
                    out_path.display()
                ));
            }

            // Check sourcemap output
            if out_map_path.exists() {
                let expected_map = fs::read(out_map_path.clone())?;
                if res.map.as_bytes() != expected_map {
                    mismatches.push(format!("{filename}: sourcemap mismatch"));
                }
            } else {
                mismatches.push(format!(
                    "{filename}: missing sourcemap file {}",
                    out_map_path.display()
                ));
            }

            if !mismatches.iter().any(|m| m.starts_with(filename)) {
                println!("  ✓ {filename}");
            }
        }

        if !mismatches.is_empty() {
            eprintln!("\nValidation failed:");
            for mismatch in &mismatches {
                eprintln!("  ✗ {mismatch}");
            }
            return Err(format!("{} validation error(s)", mismatches.len()).into());
        }

        println!(
            "\nAll {} fixtures validated successfully!",
            input_files.len()
        );
    }

    Ok(())
}
