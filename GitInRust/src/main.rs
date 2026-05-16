use clap::{Parser, Subcommand};
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
    CatFile {
        #[arg(value_parser = ["blob", "commit", "tag", "tree"])]
        object_type: String;
        object: String;
    },

    CheckIgnore,
    Checkout,
    Commit,
    HashObject {
        #[arg(
            short = 't',
            default_value = "blob",
            value_parser = ["blob","commit", "tag", "tree"]
        )]
        object_type: String,

        #[arg(short = "w")]
        write: bool,
        path: String,
    },

Init {
        #[arg(default_value = ".")]
        path: String,
    },      

    Log {
        #[arg( 
            default_value = "HEAD",
            )]
        commit: String
    },
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

        Commands::CatFile {object_type, object} => {
            println!("git cat-file");
            repo = gitfns::repo_find(["."],true);
            gitobj::cat_file(&repo, &object, &object_type);
        }

        Commands::CheckIgnore =>
            println!("git check-ignore"),

        Commands::Checkout =>
            println!("git checkout"),

        Commands::Commit =>
            println!("git commit"),

        Commands::HashObject {object_type, write, path}=>{
            let repo = if write {
                Some(gitfns::repo_find(["."],true).unwrap()); 
            }
            else{
                None
            };

            let sha = gitobj::object_hash(
                &path,
                &object_type,
                repo.as_ref(),
            ).unwrap();
            println!("{}", sha);
            println!("git hash-object"),
        }
        Commands::Log =>
            repo = gitfns::repo_find(["."],true);
            
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
