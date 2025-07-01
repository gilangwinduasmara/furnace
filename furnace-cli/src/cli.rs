#[derive(Subcommand)]
pub enum PhpCommand {
    /// Install a PHP version
    Install { version: String },
    /// List installed PHP versions
    List,
    /// Use a PHP version for this project
    Use { version: String },
}
use clap::{Parser, Subcommand};


#[derive(Parser)]
#[command(name = "furnace", version, about = r#"
   
  _____                                
 |  ___|   _ _ __ _ __   __ _  ___ ___ 
 | |_ | | | | '__| '_ \ / _` |/ __/ _ \
 |  _|| |_| | |  | | | | (_| | (_|  __/
 |_|   \__,_|_|  |_| |_|\__,_|\___\___|
Powerful, hot, ready to cook your code.
        "#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}


#[derive(Subcommand)]
pub enum Commands {
    /// Start services
    Serve,
    /// Stop all Furnace services
    Stop,
    /// Restart Furnace services
    Restart,
    /// Remove current project from Furnace management
    Dispose {
        /// Optionally set the project name to dispose
        #[arg(long)]
        name: Option<String>,
    },
    /// Install dependencies/services
    Install,
    /// Show status of services
    Status,
    /// Recipe management
    Recipe {
        #[command(subcommand)]
        command: RecipeCommand,
    },
    /// Cook management
    Cook {
        #[command(subcommand)]
        command: CookCommand,
    },
    /// PHP version management
    Php {
        #[command(subcommand)]
        command: PhpCommand,
    },
}

#[derive(Subcommand)]
pub enum RecipeCommand {
    /// List all registered site configurations
    List,
}

#[derive(Subcommand)]
pub enum CookCommand {
    /// Cook a recipe from the current directory (Laravel)
    Here {
        /// Optionally set the project name
        #[arg(long)]
        name: Option<String>,
    },
    /// Dispose a recipe (optionally by name)
    Dispose {
        /// Optionally set the project name to dispose
        #[arg(long)]
        name: Option<String>,
    },
}