use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env,
    fs,
    io::{self, Write},
    path::Path,
};

#[derive(Debug, Deserialize)]
struct SyntaxRule {
    pattern: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct MultiLineRule {
    start: String,
    end: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct LanguageRules {
    name: String,
    extensions: Vec<String>,
    single_line: Vec<SyntaxRule>,
    multi_line: Vec<MultiLineRule>,
}

#[derive(Debug, Deserialize)]
struct SyntaxRules {
    #[serde(flatten)]
    languages: HashMap<String, LanguageRules>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Remove comments from a source file
    Remove {
        /// Path to the source file
        file: String,
        
        /// Automatic mode (remove all comments without asking)
        #[arg(short, long)]
        auto: bool,
        
        /// Force mode (overwrite without backup)
        #[arg(short, long)]
        force: bool,

        /// Verbose mode (show detailed information)
        #[arg(short, long)]
        verbose: bool,
    },
    /// Display detailed information about the tool
    Info,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("Failed to load syntax rules: {0}")]
    SyntaxRulesError(String),
}

fn load_syntax_rules() -> Result<SyntaxRules> {
    // Get the directory where the executable is located
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent()
        .ok_or_else(|| Error::SyntaxRulesError("Could not get executable directory".to_string()))?;
    
    // Try to find syntax_rules.json in the executable directory
    let rules_path = exe_dir.join("syntax_rules.json");
    
    if !rules_path.exists() {
        // If not found in executable directory, try the current directory
        let current_dir = env::current_dir()?;
        let current_rules_path = current_dir.join("syntax_rules.json");
        
        if !current_rules_path.exists() {
            return Err(Error::SyntaxRulesError(
                format!("Could not find syntax_rules.json in {} or {}", 
                    rules_path.display(), 
                    current_rules_path.display())
            ).into());
        }
        
        let rules_content = fs::read_to_string(&current_rules_path)
            .with_context(|| format!("Failed to read syntax rules from current directory"))?;
        
        return serde_json::from_str(&rules_content)
            .map_err(|e| Error::SyntaxRulesError(e.to_string()).into());
    }
    
    let rules_content = fs::read_to_string(&rules_path)
        .with_context(|| format!("Failed to read syntax rules from {}", rules_path.display()))?;
    
    serde_json::from_str(&rules_content)
        .map_err(|e| Error::SyntaxRulesError(e.to_string()).into())
}

fn detect_file_type<'a>(file_path: &str, rules: &'a SyntaxRules) -> Result<&'a LanguageRules> {
    let extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| Error::UnsupportedFileType("No file extension found".to_string()))?;

    for (_, lang_rules) in &rules.languages {
        if lang_rules.extensions.iter().any(|ext| ext == extension) {
            return Ok(lang_rules);
        }
    }

    Err(Error::UnsupportedFileType(extension.to_string()).into())
}

fn get_comment_patterns(language: &LanguageRules, verbose: bool) -> Vec<Regex> {
    if verbose {
        println!("Detecting patterns for language: {}", language.name);
    }
    let mut patterns = Vec::new();

    // Add single-line comment patterns
    for rule in &language.single_line {
        let pattern = format!(r"(?m)^\s*{}\s*.*$", regex::escape(&rule.pattern));
        patterns.push(Regex::new(&pattern).unwrap());
        if verbose {
            println!("Added pattern for {}: {}", rule.description, pattern);
        }
    }

    // Add multi-line comment patterns
    for rule in &language.multi_line {
        let pattern = format!(
            r"{}\s*[\s\S]*?\s*{}",
            regex::escape(&rule.start),
            regex::escape(&rule.end)
        );
        patterns.push(Regex::new(&pattern).unwrap());
        if verbose {
            println!("Added pattern for {}: {}", rule.description, pattern);
        }
    }

    patterns
}

fn should_remove_comment(comment: &str, auto: bool) -> bool {
    if auto {
        return true;
    }

    println!("\nFound comment:");
    println!("{}", comment.yellow());
    print!("Remove this comment? (y/n): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_lowercase() == "y"
}

fn remove_comments(content: &str, patterns: &[Regex], auto: bool, verbose: bool) -> (String, usize, usize) {
    let mut result = content.to_string();
    let mut comments_found = 0;
    let mut comments_removed = 0;
    
    if verbose {
        println!("Original content preview:\n{}", content.lines().take(5).collect::<Vec<_>>().join("\n"));
    }
    
    for pattern in patterns {
        let mut offset = 0;
        while let Some(mat) = pattern.find_at(&result.clone(), offset) {
            let comment = mat.as_str();
            comments_found += 1;
            if verbose {
                println!("Found comment at position {}: {}", mat.start(), comment);
            }
            
            if should_remove_comment(comment, auto) {
                result.replace_range(mat.start()..mat.end(), "");
                offset = mat.start();
                comments_removed += 1;
            } else {
                offset = mat.end();
            }
        }
    }
    
    if verbose {
        if comments_found == 0 {
            println!("No comments were found in the file");
            println!("Content preview after processing:\n{}", result.lines().take(5).collect::<Vec<_>>().join("\n"));
        } else {
            println!("Found {} comments, removed {} comments", comments_found, comments_removed);
        }
    }
    
    (result, comments_found, comments_removed)
}

fn print_info() {
    println!("\n{}", "Comment Removal CLI".bold().green());
    println!("A tool to remove comments from source code files");
    println!("Will only remove non-inline comments\n");
    
    println!("{}", "USAGE:".bold());
    println!("  comment_remover [COMMAND] [OPTIONS]\n");
    
    println!("{}", "COMMANDS:".bold());
    println!("  remove <file>    Remove comments from a source file");
    println!("  info            Display detailed information about the tool\n");
    
    println!("{}", "OPTIONS:".bold());
    println!("  -a, --auto      Remove all comments without asking for confirmation");
    println!("  -f, --force     Skip creating backup file before modifications");
    println!("  -v, --verbose   Give detailed information while exicuting\n");
    
    println!("{}", "EXAMPLES:".bold());
    println!("  comment_remover remove main.rs");
    println!("  comment_remover remove --auto main.rs");
    println!("  comment_remover remove --force main.rs");
    println!("  comment_remover remove --auto --force main.rs\n");
    
    println!("{}", "SUPPORTED LANGUAGES:".bold());
    println!("  • Rust (.rs)");
    println!("  • Python (.py)");
    println!("  • JavaScript (.js, .jsx)");
    println!("  • TypeScript (.ts, .tsx)");
    println!("  • Java (.java)");
    println!("  • C (.c, .h)");
    println!("  • C++ (.cpp, .hpp)");
    println!("  • Go (.go)\n");
    
    println!("{}", "NOTES:".bold());
    println!("  • By default, the tool runs in interactive mode");
    println!("  • A backup file (.bak) is created unless --force is used");
    println!("  • Comments are detected based on language-specific syntax");
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let syntax_rules = load_syntax_rules()?;

    match cli.command {
        Commands::Remove { file, auto, force, verbose } => {
            let file_path = &file;
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path))?;

            if verbose {
                println!("File content length: {} bytes", content.len());
            }
            
            let language = detect_file_type(file_path, &syntax_rules)?;
            println!("Detected language: {}", language.name.green());

            let patterns = get_comment_patterns(language, verbose);
            let (new_content, comments_found, comments_removed) = remove_comments(&content, &patterns, auto, verbose);

            if new_content != content {
                if !force {
                    let backup_path = format!("{}.bak", file_path);
                    fs::write(&backup_path, content)
                        .with_context(|| format!("Failed to create backup file: {}", backup_path))?;
                    println!("Created backup file: {}", backup_path.blue());
                }

                fs::write(file_path, new_content)
                    .with_context(|| format!("Failed to write modified file: {}", file_path))?;
                println!("Successfully removed comments from: {}", file_path.green());
                if verbose {
                    println!("Statistics:");
                    println!("  - Total comments found: {}", comments_found);
                    println!("  - Comments removed: {}", comments_removed);
                    println!("  - Comments preserved: {}", comments_found - comments_removed);
                }
            } else {
                println!("No comments were removed from: {}", file_path.yellow());
                if verbose {
                    println!("  - No comments were found in the file");
                }
            }
        }
        Commands::Info => {
            print_info();
        }
    }

    Ok(())
}
