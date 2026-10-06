// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

//#![allow(dead_code)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use clap::{Args, Parser, Subcommand, ValueHint};
use unicode_normalization::UnicodeNormalization;

#[allow(unused_imports)]
use dotkei::logging::{LevelFilter, debug, error, info, trace, warn};

#[allow(dead_code)]
const BIN_NAME: &str = env!("CARGO_BIN_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

#[derive(Parser)]
#[command(
    version = PKG_VERSION,
    about = PKG_DESCRIPTION,
    long_about = None,
    propagate_version = true
)]
struct Cli {
    #[arg(long, value_parser = parse_valid_log_level, default_value_t = LevelFilter::Info)]
    /// Set output logging level
    log_level: LevelFilter,

    #[arg(long)]
    /// Silence output messages except for critical errors
    quiet: bool,

    #[arg(long = "no-color", action = clap::ArgAction::SetTrue)]
    /// Do not output any color codes
    nocolor: bool,

    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Interactive terminal session
    Repl {
        #[arg(short = 'D')]
        /// Print debugging output (compiler internal operations)
        debug_print: bool,

        #[arg(long)]
        /// Code as parameter for non-interactive mode
        code: Option<OsString>,
    },
    /// Compile and check source code; output a binary
    Build {
        #[command(flatten)]
        build_args: BuildArgs,
    },
    /// Build, but then run the corresponding binary
    Run {
        #[command(flatten)]
        build_args: BuildArgs,

        #[arg(last = true, allow_hyphen_values = true)]
        /// Any arguments to pass to the child program
        pass_args: Vec<String>,
    },
    //Format,
    //Clean,
}

#[derive(Args, Clone)]
struct BuildArgs {
    #[arg(value_hint = ValueHint::FilePath, value_parser = parse_valid_file_path)]
    /// Path to source file
    file: PathBuf,

    #[arg(short = 'D')]
    /// Print debugging output (compiler internal operations)
    debug_print: bool,

    #[arg(long)]
    /// Build in release mode with optimizations and stripped debuginfo
    release: bool,
}

/// parse_valid_log_level
///
/// Takes a string and attempts to match it against log::LevelFilter
/// "off" -> LevelFilter:Off
/// "error" -> LevelFilter::Error
/// "warn" -> LevelFilter::Warn
/// "info" -> LevelFilter::Info
/// "debug" -> LevelFilter::Debug
/// "trace" -> LevelFilter::Trace
fn parse_valid_log_level(level_name: &str) -> Result<LevelFilter, String> {
    use std::str::FromStr;

    if let Ok(level) = LevelFilter::from_str(&level_name.to_lowercase()) {
        Ok(level)
    } else {
        Err(format!(
            "Available levels are: {}",
            LevelFilter::iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        ))
    }
}

/// parse_valid_file_path
///
/// this function does one thing: take a string representation of a path
/// returning a PathBuf result if and only if:
/// 1. The path is confirmed to exist
/// 2. The path refers to a file, not a directory
pub fn parse_valid_file_path(path_str: &str) -> Result<PathBuf, String> {
    let path = Path::new(path_str);

    match path.try_exists() {
        Ok(true) => {
            if path.is_file() {
                path.canonicalize().map_err(|e| e.to_string())
            } else {
                Err(format!(
                    "Expected file, found directory: '{}'",
                    path.display()
                ))
            }
        }

        Ok(false) => Err(format!("File '{}' does not exist", path.display())),

        Err(e) => Err(e.to_string()),
    }
}

enum CommandResult {
    Success,
    Failure,
}

#[allow(unused_variables)]
fn repl_command(debug_print: bool, code: Option<OsString>) -> CommandResult {
    // error!("error in repl\nA\nB\nC\nD");
    // warn!("warn in repl");
    // info!("info in repl");
    // debug!(a = 1, b = 2, c = 3, vec:? = vec!['a', 'b', 'c', 'd', 'e']; "debug in repl");
    // trace!("trace in repl");

    todo!();
}

pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => {
            let nfc_normed: String = s.nfc().collect();

            let mut index = 0;

            // Unicode BOM stripping
            if nfc_normed[0..3] == *"\u{ffef}" {
                index = 3;
            }

            // Shebang stripping
            while nfc_normed[index..index + 2] == *"#!" {
                if let Some(nl) = s.find('\n') {
                    index = nl;
                } else {
                    break;
                }
            }

            // CRLF normalization
            Ok(nfc_normed[index..].replace("\r\n", "\n"))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[allow(unused_variables)]
fn build_command(path: PathBuf, debug_print: bool, release_mode: bool) -> CommandResult {
    if let Ok(s) = read_from_path(&path) {
        debug!(target:"print_source", "\n{}", s);
        let mut cursor = 0;

        for token in dotkei::lexer::basic::tokenize(&s) {
            info!(target:"token", kind:? = token.kind; "{}", &s[cursor..cursor+token.len as usize]);

            cursor += token.len as usize;
        }

        return CommandResult::Success;
    }
    // if let Ok(t) = frontend::source::read_from_path(&path) {
    //     info!("{}:\n{}", path.display(), t);
    // }

    // match frontend::lex_from_path(path) {
    //     Ok(ts) => {
    //         // for (token, src, triv) in ts {
    //         //     if token.kind == TokenKind::Newline {
    //         //         continue;
    //         //     } else {
    //         //         info!(target:"build", kind:? = token.kind, tok_len = token.span.end-token.span.start, trivia:? = triv; "{}", src);
    //         //     }
    //         // }
    //         CommandResult::Success
    //     }
    //     Err(e) => {
    //         error!(target:"build", "Build error: {}", e);
    //         CommandResult::Failure
    //     }
    // }

    CommandResult::Failure
}

#[allow(unused_variables)]
fn run_command(
    path: PathBuf,
    debug_print: bool,
    release_mode: bool,
    pass_args: Vec<String>,
) -> CommandResult {
    todo!()
}

fn main() {
    let args = Cli::parse();

    if let Err(e) = dotkei::logging::init(args.log_level, args.nocolor) {
        eprintln!("Error configuring logging: {}", e)
    }

    let command_result = match args.command {
        CliCommand::Repl { debug_print, code } => repl_command(debug_print, code),
        CliCommand::Build { build_args } => {
            build_command(build_args.file, build_args.debug_print, build_args.release)
        }
        CliCommand::Run {
            build_args,
            pass_args,
        } => run_command(
            build_args.file,
            build_args.debug_print,
            build_args.release,
            pass_args,
        ),
    };

    match command_result {
        CommandResult::Success => info!("Success"),
        CommandResult::Failure => error!("Command failed"),
    };

    // println!(
    //     "size_of::<Token> => {} bytes",
    //     size_of::<frontend::token::Token>()
    // );
}
