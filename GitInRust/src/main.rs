use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
mod gitfns;
mod gitobj;
mod sha1;
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
        object_type: String,
        object: String,
    },

    CheckIgnore,
    Checkout {
        commit : String,    
        path : String,
    },
    Commit,
    HashObject {
        #[arg(
            short = 't',
            default_value = "blob",
            value_parser = ["blob","commit", "tag", "tree"]
        )]
        object_type: String,

        #[arg(short = 'w')]
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
    LsTree{
        #[arg(short = 'r')]
        recursive: bool,
        tree : String,
    },
    RevParse,   
    Rm,
    ShowRef,
    Status,
    Tag{
        #[arg(short = 'a', long = "annotate")]
        create_tag_object: bool,

        name: String,
        
        #[arg(default_value = "HEAD",)]
        object: String,

    },
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
            let repo = gitfns::repo_find(".",true).unwrap();
            gitobj::cat_file(&repo, &object, object_type);
        }

        Commands::CheckIgnore =>
            println!("git check-ignore"),

        Commands::Checkout {commit, path} => {
            let repo = gitfns::repo_find(".",true).unwrap();                     gitobj::git_checkout(&repo, &commit, &path);
            println!("git checkout")
        }
            
        Commands::Commit =>
            println!("git commit"),

        Commands::HashObject {object_type, write, path}=>{
            let repo = if write {
                Some(gitfns::repo_find(".",true).unwrap()) 
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
            println!("git hash-object");
        }
        Commands::Log {commit} =>{
            let repo = gitfns::repo_find(".",true).unwrap();
            let sha = gitobj::object_find(&repo, &commit, None, true);
            let mut seen = gitobj::HashSet::new();

            gitobj::log_graphviz(&repo, &sha, &mut seen).unwrap();

            println!("git log");
        }
        Commands::LsFiles =>
            println!("git ls-files"),

        Commands::LsTree {recursive, tree} => {
            let repo = gitfns::repo_find(".",true).unwrap();
            gitobj::ls_tree(&repo,&tree,recursive,String::new());
            println!("git ls-tree");

        } 
        Commands::RevParse =>
            println!("git rev-parse"),

        Commands::Rm =>
            println!("git rm"),

        Commands::ShowRef =>{
            let repo = gitfns::repo_find(".", true).unwrap();
            let refs = gitfns::ref_list(&repo, None).unwrap();
            gitfns::show_ref(&refs, true, "refs");
            println!("git show-ref");
        }
            

        Commands::Status =>
            println!("git status"),

        Commands::Tag {create_tag_object, name, object} => {
            let repo = gitfns::repo_find(".", true).unwrap();
            
            if !name.is_empty(){
                gitobj::tag_create(&repo, &name, &object, create_tag_object);
            }
            else{
                let refs = gitfns::ref_list(&repo, None).unwrap();
                gitfns::show_ref(&refs, true, "refs");
            }
            println!("git tag");
        }
    }

}
