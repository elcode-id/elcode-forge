//! `eforge` — Primary CLI binary entrypoint for ELCODE Forge.

use std::process;

fn main() {
    let exit_code = forge_cli::run_cli_from_env();
    if exit_code.code() != 0 {
        process::exit(exit_code.code());
    }
}
