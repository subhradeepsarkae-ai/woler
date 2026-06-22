mod app;
mod cache;
mod db;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "woler",
    version = "0.1.0",
    about = "Browse, search, and remove installed Arch Linux packages"
)]
struct Cli {
    /// Show only GUI applications
    #[arg(long, conflicts_with = "cli", conflicts_with = "lib")]
    app: bool,

    /// Show only CLI tools
    #[arg(long, conflicts_with = "app", conflicts_with = "lib")]
    cli: bool,

    /// Show only libraries
    #[arg(long, conflicts_with = "app", conflicts_with = "cli")]
    lib: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List installed packages (prints to stdout)
    List {
        /// Show only GUI applications
        #[arg(long)]
        apps: bool,
        /// Show only CLI tools
        #[arg(long)]
        clis: bool,
        /// Show only libraries
        #[arg(long)]
        libs: bool,
        /// Filter by search term
        #[arg(short, long)]
        search: Option<String>,
    },
    /// Remove a package (uses sudo pacman -Rns)
    Remove {
        /// Package name to remove
        package: String,
        /// Also remove orphaned dependencies
        #[arg(short, long)]
        orphans: bool,
    },
    /// Force refresh the package cache
    Refresh,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let has_filter = cli.app || cli.cli || cli.lib;

    match cli.command {
        None if has_filter => {
            let tab = if cli.app {
                Some(app::Tab::Apps)
            } else if cli.cli {
                Some(app::Tab::Clis)
            } else {
                Some(app::Tab::Libraries)
            };
            app::run_tui_with_tab(tab)?;
        }
        None => app::run_tui()?,
        Some(Commands::List {
            apps,
            clis,
            libs,
            search,
        }) => cmd_list(apps, clis, libs, search)?,
        Some(Commands::Remove { package, orphans }) => cmd_remove(&package, orphans)?,
        Some(Commands::Refresh) => cmd_refresh()?,
    }

    Ok(())
}

fn cmd_list(apps: bool, clis: bool, libs: bool, search: Option<String>) -> Result<()> {
    let packages = load_packages()?;

    let filtered: Vec<_> = packages
        .iter()
        .filter(|p| {
            if apps && !p.has_desktop {
                return false;
            }
            if clis && p.bins.is_empty() {
                return false;
            }
            if libs && (p.has_desktop || !p.bins.is_empty()) {
                return false;
            }
            if let Some(ref q) = search {
                let q = q.to_lowercase();
                if !p.name.to_lowercase().contains(&q)
                    && !p.description.to_lowercase().contains(&q)
                {
                    return false;
                }
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        println!("No packages found.");
        return Ok(());
    }

    let cat_label = if apps {
        "GUI apps"
    } else if clis {
        "CLI tools"
    } else if libs {
        "Libraries"
    } else {
        "All packages"
    };

    println!(
        " {:<24} {:<14} {:>8}  {:>8}  {}",
        "Name", "Version", "Size", "Type", "Description"
    );
    println!(" {}", "-".repeat(80));

    for pkg in &filtered {
        let type_str = match pkg.category() {
            db::Category::App => "APP",
            db::Category::Cli => "CLI",
            db::Category::Library => "LIB",
        };
        let desc = if pkg.description.len() > 40 {
            format!("{}...", &pkg.description[..37])
        } else {
            pkg.description.clone()
        };
        println!(
            " {:<24} {:<14} {:>8}  {:>8}  {}",
            pkg.name,
            pkg.version,
            pkg.size_human(),
            type_str,
            desc
        );
    }

    println!();
    println!(
        " {} {} listed (out of {} total)",
        filtered.len(),
        cat_label,
        packages.len()
    );

    Ok(())
}

fn cmd_remove(package: &str, orphans: bool) -> Result<()> {
    println!("Remove {}?", package);
    print!("Confirm [y/N]: ");
    use std::io::Write;
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input != "y" && input != "yes" {
        println!("Cancelled.");
        return Ok(());
    }

    let status = if orphans {
        std::process::Command::new("sudo")
            .args([
                "pacman",
                "-Rns",
                package,
                "$(pacman -Qdtq)",
            ])
            .status()
    } else {
        std::process::Command::new("sudo")
            .args(["pacman", "-Rns", package])
            .status()
    };

    match status {
        Ok(s) if s.success() => {
            cache::clear()?;
            println!("Removed {}", package);
        }
        Ok(s) => {
            eprintln!("Failed with exit code: {:?}", s.code());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}

fn cmd_refresh() -> Result<()> {
    println!("Scanning packages...");
    let packages = db::scan()?;
    cache::save(&packages)?;
    let (apps, clis, libs) = db::packages_by_category(&packages);
    println!(
        "Done: {} packages ({} apps, {} clis, {} libs)",
        packages.len(),
        apps,
        clis,
        libs
    );
    Ok(())
}

fn load_packages() -> Result<Vec<db::Package>> {
    if let Some(cached) = cache::load()? {
        return Ok(cached);
    }
    let packages = db::scan()?;
    cache::save(&packages)?;
    Ok(packages)
}
