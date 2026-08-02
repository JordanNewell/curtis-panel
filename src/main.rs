mod cli;
mod config;

fn main() -> anyhow::Result<()> {
    cli::run()
}

pub fn run_app() -> anyhow::Result<()> {
    println!("openpanel GUI not yet implemented — use `openpanel check` to validate config");
    Ok(())
}
