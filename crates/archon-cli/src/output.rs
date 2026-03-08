use colored::Colorize;

pub fn print_header(title: &str) {
    println!("\n{}", title.bold());
}

pub fn print_created(path: &str) {
    println!("  {} {}", "created".green(), path);
}

pub fn print_skipped(path: &str, reason: &str) {
    println!("  {} {} ({})", "skipped".yellow(), path, reason);
}

pub fn print_dry_run(path: &str) {
    println!("  {} {}", "would create".cyan(), path);
}
