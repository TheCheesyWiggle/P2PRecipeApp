use clap::{Arg, ArgMatches, Command};

fn main() {
    // Define the CLI structure
    let matches = Command::new("recipe")
        .version("0.1.0")
        .author("The Cheesy Wiggle")
        .about("CLI for P2P Recipe sharing app")
        // Add a subcommand
        .subcommand(
            Command::new("show")
                .about("Shows recipe")
                .arg(
                    Arg::new("recipe")
                        .short('r')
                        .long("recipe")
                        .value_name("RECIPE")
                        .help("Recipe")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::new("all")
                        .short('a')
                        .long("all")
                        .help("shows all the recipes")
                        .takes_value(false),
                )
        )
        // Add a flag
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .takes_value(false),
        )
        .get_matches();

    // Handle global flag
    let verbose = matches.is_present("verbose");
    if verbose {
        println!("Verbose mode enabled");
    }

    // Handle subcommands
    match matches.subcommand() {
        Some(("show", sub_m)) => show(sub_m),
        _ => println!("No valid subcommand was used"),
    }
}

// Subcommand handler
fn show(matches: &ArgMatches) {
    if matches.get_flag("all") {
        println!("Showing all recipes...");
        // TODO: list all recipes
    } else if let Some(recipe) = matches.get_one::<String>("recipe") {
        println!("Showing recipe: {}. Sounds tasty!", recipe);
    } else {
        println!("Please specify a recipe name with -r or use -a for all recipes.");
    }
}