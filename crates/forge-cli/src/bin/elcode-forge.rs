//! `elcode-forge` — Secondary CLI binary alias for ELCODE Forge.

use std::process;

fn main() {
    let builder = std::thread::Builder::new().stack_size(8 * 1024 * 1024);
    let handler = builder.spawn(forge_cli::run_cli_from_env);
    let exit_code = match handler {
        Ok(join_handle) => match join_handle.join() {
            Ok(code) => code,
            Err(_) => forge_cli::ExitCode::ExecutionError,
        },
        Err(_) => forge_cli::run_cli_from_env(),
    };
    if exit_code.code() != 0 {
        process::exit(exit_code.code());
    }
}
