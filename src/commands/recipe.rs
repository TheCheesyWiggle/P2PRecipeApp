use crate::cli::RecipeCommand;

pub fn handle(cmd: RecipeCommand) {
    match cmd {
        RecipeCommand::Add {
            title,
            servings,
            time,
            tags,
        } => {
            println!("Adding recipe: {title}");
            // store recipe
        }

        RecipeCommand::Show { recipe_id } => {
            println!("Showing recipe {recipe_id}");
        }

        RecipeCommand::List { mine, tag } => {
            println!("Listing recipes (mine={mine:?}, tag={tag:?})");
        }

        RecipeCommand::Delete { recipe_id } => {
            println!("Deleting recipe {recipe_id}");
        }
    }
}
