// Fitzroy Woodruff
// Rustic - A simple static site generator in Rust
// December 2025

use anyhow::{Context, Result};
use clap::Parser;
use fs_extra::dir::{copy, CopyOptions};
use gray_matter::{engine::YAML, Matter};
use pulldown_cmark::{html, Parser as MarkdownParser};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};
use walkdir::WalkDir;

/// A simple static site generator that builds HTML from Markdown files.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The path to the directory containing Markdown files
    #[arg(short, long, default_value = "content")]
    input_dir: PathBuf,

    /// The output directory where HTML and CSS files will be generated
    #[arg(short, long, default_value = "public")]
    out_dir: PathBuf,
}

/// Represents the front matter of a markdown file.
#[derive(Debug, Deserialize)]
struct FrontMatter {
    title: String,
    stinger: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let input_dir = &cli.input_dir;
    let out_dir = &cli.out_dir;

    // Clean and recreate the output directory
    if out_dir.exists() {
        fs::remove_dir_all(out_dir)
            .with_context(|| format!("Failed to remove existing output directory: {:?}", out_dir))?;
    }
    fs::create_dir_all(out_dir)
        .with_context(|| format!("Failed to create output directory: {:?}", out_dir))?;

    // Copy static assets
    let static_dir = PathBuf::from("static");
    if static_dir.exists() {
        let mut options = CopyOptions::new();
        options.overwrite = true;
        copy(&static_dir, out_dir, &options)
            .with_context(|| format!("Failed to copy static assets from {:?}", static_dir))?;
    }

    // Initialize Tera templating engine
    let tera = Tera::new("templates/**/*.html")
        .with_context(|| "Failed to initialize Tera templating engine")?;

    // Process all markdown files
    for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            process_markdown_file(path, input_dir, out_dir, &tera)?;
        }
    }

    println!("✅ Site generated successfully!");
    Ok(())
}

/// Processes a single markdown file: parses, converts to HTML, and renders in a template.
fn process_markdown_file(path: &Path, input_dir: &Path, out_dir: &Path, tera: &Tera) -> Result<()> {
    println!("Processing: {:?}", path);

    // Read file and parse front matter
    let file_content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read markdown file: {:?}", path))?;
    
    let matter = Matter::<YAML>::new();
    let parsed_entity = matter.parse(&file_content);
    
    let front_matter: FrontMatter = parsed_entity.data
        .ok_or_else(|| anyhow::anyhow!("Missing front matter in {:?}", path))?
        .deserialize()
        .context("Failed to deserialize front matter")?;
    
    let markdown_content = parsed_entity.content;

    // Convert markdown body to an HTML string
    let parser = MarkdownParser::new(&markdown_content);
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

    // Calculate the relative path prefix for assets
    let relative_path = path.strip_prefix(input_dir)?;
    let depth = relative_path.ancestors().count() - 2; // -1 for self, -1 for root component
    let path_prefix = "..".repeat(depth);
    
    // Render the full HTML page using the template
    let mut context = TeraContext::new();
    context.insert("title", &front_matter.title);
    context.insert("stinger", &front_matter.stinger);
    context.insert("content", &html_body);
    context.insert("path_prefix", &path_prefix); // Pass the new prefix to Tera

    let full_html = tera.render("template.html", &context)
        .with_context(|| "Failed to render template")?;

    // Determine the output path, preserving directory structure
    let mut output_path = out_dir.join(relative_path);
    output_path.set_extension("html");

    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for {:?}", output_path))?;
    }

    // Write the final HTML to the output file
    fs::write(&output_path, full_html)
        .with_context(|| format!("Failed to write HTML file: {:?}", output_path))?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*; // Import things from parent

    #[test]
    fn test_markdown_to_html() {
        // Arrange
        let markdown_input = "## Hello";
        let expected_html = "<h2>Hello</h2>\n".to_string();

        // Act
        let parser = MarkdownParser::new(markdown_input);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        // Assert
        assert_eq!(html_output, expected_html);
    }
}