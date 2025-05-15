use crate::{args::Args, config::Config};
use color_eyre::eyre;

pub fn run(
    args: &Args,
    config: &Config,
    include_secrets: bool,
) -> eyre::Result<()> {
    if args.json {
        print_json(config)?;
    } else {
        print_human(config, include_secrets);
    }

    Ok(())
}

fn print_json(config: &Config) -> eyre::Result<()> {
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "token": config.token,
            "hostname": config.hostname().to_string(),
        }))?
    );

    Ok(())
}

fn print_human(config: &Config, include_secrets: bool) {
    let token_to_show = if let Some(ref token) = config.token {
        if include_secrets { token } else { "<SECRET>" }
    } else {
        "<UNSET>"
    };

    let mut table = comfy_table::Table::new();
    table
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec!["Key", "Value"])
        .add_row(vec!["Token", token_to_show])
        .add_row(vec!["Hostname", config.hostname().as_ref()]);

    println!("{table}");
}
