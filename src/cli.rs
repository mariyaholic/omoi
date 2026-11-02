use crate::{
    config::Config,
    paths::{PathEnvironment, Paths},
    Error, Result,
};
use clap::{ArgGroup, Parser, Subcommand};
use std::{io::Write, path::PathBuf};
#[derive(Parser)]
#[command(name = "omoi", version, about = "Local video history catalogue")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Collect video visits from browser-history snapshots into the local catalogue.
    Collect {
        #[arg(long,value_parser=["chrome","edge","brave","firefox","all"])]
        browser: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    #[command(group(ArgGroup::new("format").required(true).args(["csv","json"])))]
    /// Export deduplicated views to CSV or JSON; never overwrite an existing file.
    Export {
        #[arg(long)]
        csv: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Create/migrate the catalogue or inspect its schema and counts.
    Db {
        #[command(subcommand)]
        command: Database,
    },
    /// Show catalogue and browser locations without creating files.
    Paths,
}
#[derive(Subcommand)]
enum Database {
    Migrate,
    Info,
}
pub fn run() -> u8 {
    let cli = Cli::parse();
    match execute(cli) {
        Ok(code) => code,
        Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::BrokenPipe => 0,
        Err(e) => {
            eprintln!("{e}");
            e.exit_code()
        }
    }
}
fn output(value: &impl serde::Serialize) -> Result<()> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer(&mut lock, value).map_err(|e| Error::Export(e.to_string()))?;
    lock.write_all(b"\n").map_err(|source| Error::Io {
        context: "stdout",
        source,
    })?;
    Ok(())
}
fn execute(cli: Cli) -> Result<u8> {
    let env = PathEnvironment::capture();
    let paths = Paths::resolve(&env)?;
    let mut config = Config::load(&paths.config)?;
    match cli.command {
        Commands::Paths => {
            output(
                &serde_json::json!({"home":paths.home,"config":paths.config,"db":paths.db,"logs":paths.logs,"tmp":paths.tmp,"thumbs":paths.thumbs,"browser_roots":crate::readers::discover::roots(&env)?}),
            )?;
        }
        Commands::Collect { browser, dry_run } => {
            if browser.iter().any(|b| b == "all") && browser.len() != 1 {
                return Err(Error::Config(
                    "all cannot be combined with another browser".into(),
                ));
            }
            if !browser.is_empty() {
                config.browsers = if browser == ["all"] {
                    Config::default().browsers
                } else {
                    browser
                        .iter()
                        .map(|s| {
                            serde_json::from_value(serde_json::Value::String(s.clone()))
                                .expect("clap validated browser")
                        })
                        .collect()
                };
            }
            let report = crate::collect::collect(&config, &paths, &env, dry_run)?;
            let code = if report.profiles_failed > 0 { 3 } else { 0 };
            output(&report)?;
            return Ok(code);
        }
        Commands::Db {
            command: Database::Migrate,
        } => {
            std::fs::create_dir_all(&paths.home).map_err(|source| Error::Io {
                context: "data directory",
                source,
            })?;
            let mut c = crate::db::open(&paths.db, &config, true)?;
            crate::db::migrate(&mut c)?;
            output(&crate::db::info(&c)?)?;
        }
        Commands::Db {
            command: Database::Info,
        } => {
            let c = crate::db::open(&paths.db, &config, false)?;
            output(&crate::db::info(&c)?)?;
        }
        Commands::Export { json, out, .. } => {
            let c = crate::db::open(&paths.db, &config, false)?;
            let version = crate::db::info(&c)?.user_version;
            if version != crate::db::schema::VERSION {
                return Err(Error::Schema("unsupported export schema".into()));
            }
            if let Some(path) = out {
                let absolute = if path.is_absolute() {
                    path
                } else {
                    std::env::current_dir()
                        .map_err(|source| Error::Io {
                            context: "export directory",
                            source,
                        })?
                        .join(path)
                };
                let parent = absolute
                    .parent()
                    .ok_or_else(|| Error::Config("invalid export path".into()))?
                    .canonicalize()
                    .map_err(|source| Error::Io {
                        context: "export directory",
                        source,
                    })?;
                if parent.starts_with(paths.home.canonicalize().map_err(|source| Error::Io {
                    context: "data directory",
                    source,
                })?) {
                    return Err(Error::Config(
                        "export outside the Omoi data directory".into(),
                    ));
                }
                let temporary = crate::readers::snapshot::TempDir::create(&parent)?;
                let staging = temporary.0.join("export");
                let mut f = std::fs::File::create(&staging).map_err(|source| Error::Io {
                    context: "export create",
                    source,
                })?;
                crate::export::export(&c, json, &mut f)?;
                f.flush()
                    .and_then(|_| f.sync_all())
                    .map_err(|source| Error::Io {
                        context: "export flush",
                        source,
                    })?;
                drop(f);
                std::fs::hard_link(&staging, &absolute).map_err(|source| Error::Io {
                    context: "export publish (destination must not exist)",
                    source,
                })?;
            } else {
                crate::export::export(&c, json, &mut std::io::stdout().lock())?;
            }
        }
    }
    Ok(0)
}
