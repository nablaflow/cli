use crate::{args::Args, config::Config, utils::new_dynamic_table};
use color_eyre::eyre;

pub fn run(args: &Args, config: &Config) -> eyre::Result<()> {
    if args.json {
        print_json(config)?;
    } else {
        print_human(config);
    }

    Ok(())
}

fn print_json(config: &Config) -> eyre::Result<()> {
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "hostname": config.hostname().to_string(),
        }))?
    );

    Ok(())
}

fn print_human(config: &Config) {
    let mut table = new_dynamic_table();
    table
        .set_header(vec!["Key", "Value"])
        .add_row(vec!["Hostname", config.hostname().as_ref()]);

    println!("{table}");
}
