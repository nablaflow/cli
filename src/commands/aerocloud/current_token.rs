use crate::{
    aerocloud::{Client, fmt_progenitor_err, types::Token as TokenInfo},
    args::Args,
    utils::new_dynamic_table,
};
use chrono::Local;
use color_eyre::eyre;
use itertools::Itertools;

pub async fn run(args: &Args, client: &Client) -> eyre::Result<()> {
    let token_info: TokenInfo = client
        .tokens_caller()
        .await
        .map_err(fmt_progenitor_err)?
        .into_inner();

    if args.json {
        println!("{}", &serde_json::to_string(&token_info)?);
    } else {
        let mut table = new_dynamic_table();
        table.set_header(vec!["Name", "Created at", "Expires at", "Scopes"]);
        table.add_row(vec![
            token_info.name.clone(),
            format!("{}", token_info.created_at.with_timezone(&Local)),
            format!("{}", token_info.expires_at.with_timezone(&Local)),
            token_info.scopes.iter().map(ToString::to_string).join(", "),
        ]);

        println!("{table}");
    }

    Ok(())
}
