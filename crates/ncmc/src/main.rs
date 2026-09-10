mod discovery;
mod process;

use bpaf::Bpaf;
use discovery::Discovery;
use process::{auto_one, dump_one, ExistingPolicy, Outcome};
use std::{path::PathBuf, process as std_process};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
struct Opts {
    #[bpaf(external, fallback(Mode::Auto))]
    mode: Mode,

    /// Recursively scan directory inputs.
    #[bpaf(short, long)]
    recursive: bool,

    /// Skip a conversion when its audio output already exists.
    #[bpaf(long)]
    skip_existing: bool,

    #[bpaf(positional("INPUT"))]
    input: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, Bpaf, Default)]
enum Mode {
    /// Convert files with metadata. This is the default mode.
    #[default]
    Auto,
    /// Dump decrypted key, metadata, cover, and audio data.
    Dump,
}

#[derive(Default)]
struct Summary {
    total: usize,
    succeeded: usize,
    skipped: usize,
    failed: usize,
}

fn main() {
    let opts = opts().run();
    let summary = run(opts);

    eprintln!(
        "Summary: total={} succeeded={} skipped={} failed={}",
        summary.total, summary.succeeded, summary.skipped, summary.failed
    );

    if summary.failed > 0 {
        std_process::exit(1);
    }
}

fn run(opts: Opts) -> Summary {
    let discovery = Discovery::discover(&opts.input, opts.recursive);
    let existing_policy =
        if opts.skip_existing { ExistingPolicy::Skip } else { ExistingPolicy::Overwrite };
    let mut summary = Summary::default();

    for error in discovery.errors {
        summary.total += 1;
        summary.failed += 1;
        eprintln!("[error] {}: {:#}", error.path.display(), error.error);
    }

    for path in discovery.files {
        summary.total += 1;
        let outcome = match opts.mode {
            Mode::Auto => auto_one(&path, existing_policy),
            Mode::Dump => dump_one(&path, existing_policy),
        };

        match outcome {
            Ok(Outcome::Written { output, meta }) => {
                summary.succeeded += 1;
                eprintln!("{meta}");
                println!("{}", output.display());
            }
            Ok(Outcome::Skipped { output }) => {
                summary.skipped += 1;
                eprintln!("[skipped] {}", output.display());
            }
            Err(error) => {
                summary.failed += 1;
                eprintln!("[error] {}: {error:#}", path.display());
            }
        }
    }

    summary
}
