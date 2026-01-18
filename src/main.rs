//! CLI for csl-tools - Format citations and bibliographies in Markdown documents.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};

use csl_tools::{
    builtin_style, extract_citation_clusters, extract_citations, format_bibliography,
    format_citations_clusters, generate_output, load_refs, load_style, replace_citations,
};

/// CLI for formatting citations and bibliographies in Markdown documents
#[derive(Parser)]
#[command(name = "csl-tools")]
#[command(about = "Format citations and bibliographies in Markdown documents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a Markdown file with citations
    Process {
        /// Input Markdown file
        input: PathBuf,

        /// Bibliography file (CSL-JSON or JSONL)
        #[arg(short, long)]
        bib: PathBuf,

        /// CSL style file or builtin style name
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

    /// Install the Claude Code skill for citation formatting
    SkillInstall {
        /// Installation directory (default: .claude/skills in current directory)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
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
            process_command(
                &input,
                &bib,
                &csl,
                output.as_deref(),
                no_bib,
                &bib_header,
            )?;
        }
        Commands::SkillInstall { dir } => {
            skill_install_command(dir.as_deref())?;
        }
    }

    Ok(())
}

/// Process a Markdown file with citations.
fn process_command(
    input: &Path,
    bib: &Path,
    csl: &str,
    output: Option<&Path>,
    no_bib: bool,
    bib_header: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Read the Markdown file
    let markdown = fs::read_to_string(input)
        .map_err(|e| format!("Failed to read input file '{}': {}", input.display(), e))?;

    // 2. Load references with load_refs()
    let refs_json = load_refs(bib).map_err(|e| {
        format!(
            "Failed to load bibliography file '{}': {}",
            bib.display(),
            e
        )
    })?;

    // 3. Load style with load_style() or builtin_style()
    let style_csl = if let Some(builtin) = builtin_style(csl) {
        builtin.to_string()
    } else {
        // Try to load as a file path
        let style_path = PathBuf::from(csl);
        load_style(&style_path).map_err(|e| format!("Failed to load CSL style '{}': {}", csl, e))?
    };

    // 4. Extract citation clusters with extract_citation_clusters()
    // This detects adjacent citations and groups them together
    let clusters = extract_citation_clusters(&markdown);

    // 5. Format citation clusters with format_citations_clusters()
    // This sends grouped citations to csl_proc for proper formatting (e.g., "(1-3)" instead of "(1) (2) (3)")
    let processed = format_citations_clusters(&clusters, &refs_json, &style_csl)
        .map_err(|e| format!("Failed to format citations: {}", e))?;

    // 6. Replace citations with replace_citations()
    // Each cluster is replaced as a single unit
    let content = replace_citations(&markdown, &processed);

    // 7. If --no-bib is not present, format the bibliography with format_bibliography()
    // Note: format_bibliography still uses individual citations for reference collection
    let citations = extract_citations(&markdown);
    let bibliography = if no_bib {
        None
    } else {
        let bib_html = format_bibliography(&citations, &refs_json, &style_csl)
            .map_err(|e| format!("Failed to format bibliography: {}", e))?;
        if bib_html.is_empty() {
            None
        } else {
            Some(bib_html)
        }
    };

    // 8. Generate the output with generate_output()
    let result = generate_output(&content, bibliography.as_deref(), bib_header);

    // 9. Write to output file or stdout
    if let Some(output_path) = output {
        fs::write(output_path, &result).map_err(|e| {
            format!(
                "Failed to write output file '{}': {}",
                output_path.display(),
                e
            )
        })?;
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{}", result)?;
    }

    Ok(())
}

/// The embedded Claude Code skill content
const SKILL_CONTENT: &str = include_str!("skill.md");

/// Install the Claude Code skill for citation formatting.
fn skill_install_command(dir: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    // Determine installation directory
    let base_dir = if let Some(d) = dir {
        d.to_path_buf()
    } else {
        PathBuf::from(".claude/skills")
    };

    let skill_dir = base_dir.join("csl-format");

    // Create directory if it doesn't exist
    fs::create_dir_all(&skill_dir).map_err(|e| {
        format!(
            "Failed to create skill directory '{}': {}",
            skill_dir.display(),
            e
        )
    })?;

    // Write the skill file
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, SKILL_CONTENT).map_err(|e| {
        format!(
            "Failed to write skill file '{}': {}",
            skill_path.display(),
            e
        )
    })?;

    println!("Claude Code skill installed successfully!");
    println!("  Location: {}", skill_path.display());
    println!();
    println!("The skill is now available in Claude Code when working in this directory.");
    println!("Use it by asking Claude to format your citations, or invoke it with:");
    println!("  /csl-format");

    Ok(())
}
