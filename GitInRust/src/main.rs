use clap::{Parser, Subcommand};
mod sha1;
mod gitfns;
//derive is used to invoke the macros
#[derive(Parser)] //tells clap, to genrate a cli for this struct,
#[command(name = "gir")]
#[command(version = "1.0")]
#[command(about = "an implementation of git in rust")]

struct Cli{
    #[command(subcommand)] //tells clap, this field contains nested commands
    command: Commands,
}

#[derive(Subcommand)] //tells clap each enum is a subcommand variant.
enum Commands {
    Add,
    CatFile,
    CheckIgnore,
    Checkout,
    Commit,
    HashObject,

    Init {
        #[arg(default_value = ".")]
        path: String,
    },

    Log,
    LsFiles,
    LsTree,
    RevParse,
    Rm,
    ShowRef,
    Status,
    Tag,
}

fn main(){
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { path } => {
            println!("git init");
            gitfns::repo_create(&path);
        }

        Commands::Add =>
            println!("git add"),

        Commands::CatFile =>
            println!("git cat-file"),

        Commands::CheckIgnore =>
            println!("git check-ignore"),

        Commands::Checkout =>
            println!("git checkout"),

        Commands::Commit =>
            println!("git commit"),

        Commands::HashObject =>
            println!("git hash-object"),

        Commands::Log =>
            println!("git log"),

        Commands::LsFiles =>
            println!("git ls-files"),

        Commands::LsTree =>
            println!("git ls-tree"),

        Commands::RevParse =>
            println!("git rev-parse"),

        Commands::Rm =>
            println!("git rm"),

        Commands::ShowRef =>
            println!("git show-ref"),

        Commands::Status =>
            println!("git status"),

        Commands::Tag =>
            println!("git tag"),
    }

}
