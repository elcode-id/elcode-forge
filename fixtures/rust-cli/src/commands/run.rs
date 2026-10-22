use crate::utils::format_output;

pub fn run_command(name: &str) {
    let formatted = format_output(name);
    println!("Executed: {formatted}");
}
