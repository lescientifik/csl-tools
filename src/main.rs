//! CLI for csl-tools - Format citations and bibliographies in Markdown documents.

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};

use csl_tools::{
    builtin_style, extract_citation_clusters, extract_citations, format_bibliography,
    format_citations_clusters, generate_output, load_refs, load_style,
    processor::ProcessorError, replace_citations, style::builtin_style_names,
};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Format citations and bibliographies in Markdown documents
#[derive(Parser)]
#[command(name = "csl-tools")]
#[command(version)]
#[command(after_help = "\
Examples:
  csl-tools process article.md --bib refs.json --csl style.csl
  csl-tools process article.md --bib refs.json --csl minimal -o output.html
  echo '[@key]' | csl-tools process - --bib refs.json --csl minimal
  csl-tools styles")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a Markdown file with citations
    #[command(after_help = "\
Examples:
  csl-tools process paper.md --bib refs.json --csl minimal
  csl-tools process paper.md -b refs.json -c ieee.csl -o paper.html
  csl-tools process paper.md -b refs.json -c minimal --no-bib

Citation syntax: [@key], [@key](url), [@key, p. 42], [@a; @b; @c]")]
    Process {
        /// Input Markdown file (use '-' for stdin)
        input: PathBuf,

        /// Bibliography file (CSL-JSON array or JSONL)
        #[arg(short, long)]
        bib: PathBuf,

        /// CSL style: path to a .csl file, or builtin name (see 'styles' command)
        #[arg(short, long)]
        csl: String,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Don't include bibliography
        #[arg(long)]
        no_bib: bool,

        /// Custom bibliography header
        #[arg(long, default_value = "## References")]
        bib_header: String,
    },

    /// List available builtin CSL styles
    Styles,
}

// ---------------------------------------------------------------------------
// AppError — semantic exit codes
// ---------------------------------------------------------------------------

enum AppError {
    /// Exit 10 — input file not found / unreadable
    InputFile(String),
    /// Exit 11 — bibliography file not found / invalid
    BibFile(String),
    /// Exit 12 — CSL style not found / invalid
    Style(String),
    /// Exit 13 — citation key not found in bibliography
    ReferenceNotFound(String),
    /// Exit 14 — CSL processing engine error
    CslProcessing(String),
    /// Exit 15 — cannot write output file
    OutputFile(String),
}

impl AppError {
    fn exit_code(&self) -> i32 {
        match self {
            AppError::InputFile(_) => 10,
            AppError::BibFile(_) => 11,
            AppError::Style(_) => 12,
            AppError::ReferenceNotFound(_) => 13,
            AppError::CslProcessing(_) => 14,
            AppError::OutputFile(_) => 15,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InputFile(msg) => {
                write!(f, "{}\n  hint: verify the file path is correct", msg)
            }
            AppError::BibFile(msg) => {
                write!(
                    f,
                    "{}\n  hint: the file must be a JSON array of CSL-JSON objects, or JSONL (one object per line)",
                    msg
                )
            }
            AppError::Style(msg) => {
                let names = builtin_style_names().join(", ");
                write!(
                    f,
                    "{}\n  available builtin styles: {}\n  hint: provide a path to a .csl file, or use a builtin style name",
                    msg, names
                )
            }
            AppError::ReferenceNotFound(msg) => {
                write!(
                    f,
                    "{}\n  hint: check that this citation key exists in your bibliography file",
                    msg
                )
            }
            AppError::CslProcessing(msg) => {
                write!(f, "{}", msg)
            }
            AppError::OutputFile(msg) => {
                write!(
                    f,
                    "{}\n  hint: check that the output directory exists and is writable",
                    msg
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(e.exit_code());
    }
}

fn run() -> Result<(), AppError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Process {
            input,
            bib,
            csl,
            output,
            no_bib,
            bib_header,
        } => {
            process_command(&input, &bib, &csl, output.as_deref(), no_bib, &bib_header)?;
        }
        Commands::Styles => {
            styles_command();
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Process a Markdown file with citations.
fn process_command(
    input: &Path,
    bib: &Path,
    csl: &str,
    output: Option<&Path>,
    no_bib: bool,
    bib_header: &str,
) -> Result<(), AppError> {
    // 1. Read the Markdown file (support '-' for stdin)
    let markdown = if input == Path::new("-") {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| AppError::InputFile(format!("failed to read from stdin: {}", e)))?;
        buf
    } else {
        fs::read_to_string(input).map_err(|e| {
            AppError::InputFile(format!("'{}': {}", input.display(), e))
        })?
    };

    // 2. Load references
    let refs_json = load_refs(bib)
        .map_err(|e| AppError::BibFile(format!("'{}': {}", bib.display(), e)))?;

    // 3. Load style (builtin or file)
    let style_csl = if let Some(builtin) = builtin_style(csl) {
        builtin.to_string()
    } else {
        let style_path = PathBuf::from(csl);
        load_style(&style_path).map_err(|e| {
            if style_path.exists() {
                AppError::Style(format!("invalid CSL style '{}': {}", csl, e))
            } else {
                AppError::Style(format!(
                    "'{}' is not a builtin style name and no file with this path exists",
                    csl
                ))
            }
        })?
    };

    // 4. Extract citation clusters (adjacent citations grouped)
    let clusters = extract_citation_clusters(&markdown);

    // 5. Format citation clusters via csl_proc
    let processed =
        format_citations_clusters(&clusters, &refs_json, &style_csl).map_err(map_processor_error)?;

    // 6. Replace citations in text
    let content = replace_citations(&markdown, &processed);

    // 7. Format bibliography
    let citations = extract_citations(&markdown);
    let bibliography = if no_bib {
        None
    } else {
        let bib_html =
            format_bibliography(&citations, &refs_json, &style_csl).map_err(map_processor_error)?;
        if bib_html.is_empty() {
            None
        } else {
            Some(bib_html)
        }
    };

    // 8. Generate output
    let result = generate_output(&content, bibliography.as_deref(), bib_header);

    // 9. Write to file or stdout
    if let Some(output_path) = output {
        fs::write(output_path, &result).map_err(|e| {
            AppError::OutputFile(format!("'{}': {}", output_path.display(), e))
        })?;
        eprintln!(
            "processed {} citation(s), wrote {}",
            processed.len(),
            output_path.display()
        );
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        write!(handle, "{}", result).map_err(|e| {
            AppError::OutputFile(format!("stdout: {}", e))
        })?;
    }

    Ok(())
}

/// Maps a ProcessorError to an AppError using type-safe matching.
fn map_processor_error(e: ProcessorError) -> AppError {
    match e {
        ProcessorError::ReferenceNotFound(_) => AppError::ReferenceNotFound(e.to_string()),
        _ => AppError::CslProcessing(e.to_string()),
    }
}

/// List available builtin CSL styles.
fn styles_command() {
    for name in builtin_style_names() {
        println!("{}", name);
    }
}
