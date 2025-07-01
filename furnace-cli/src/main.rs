mod cli;
mod config;
mod platform;
// mod services;
// mod recipe;
// mod php;
mod nginx_util;
use clap::Parser;
use tracing_subscriber;
// Use core business logic
use furnace_core::{services, recipe, php};

fn main() {
    tracing_subscriber::fmt::init();

    let cli = cli::Cli::parse();

    match &cli.command {
        cli::Commands::Serve => services::serve(),
        cli::Commands::Stop => services::stop(),
        cli::Commands::Restart => services::restart(),
        cli::Commands::Dispose { name } => recipe::dispose_recipe_cli(name.clone()),
        cli::Commands::Install => services::install(),
        cli::Commands::Status => services::status(),
        cli::Commands::Cook { command } => match command {
            cli::CookCommand::Here { name } => recipe::cook_here(name.clone()),
            cli::CookCommand::Dispose { name } => recipe::dispose_recipe_cli(name.clone()),
        },
        cli::Commands::Recipe { command } => match command {
            cli::RecipeCommand::List => recipe::list_recipes(),
        },
        cli::Commands::Php { command } => match command {
            cli::PhpCommand::Install { version } => {
                if let Err(e) = php::php_install(version) {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
            cli::PhpCommand::List => php::php_list(),
            cli::PhpCommand::Use { version } => {
                if let Err(e) = php::php_use(version) {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        },
    }
}
