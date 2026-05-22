use std::fs;
use std::path::{Path, PathBuf};
use configparser::ini::Ini;
use std::io;
use dict::{Dict, DictIface};
pub use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};


pub struct GitRepo{
    worktree: PathBuf,
    gitdir: PathBuf,
    conf: Ini ,
}

fn repo_default_config() -> Ini{
        let mut conf = Ini::new();
        conf.set(
            "core",
            "repositoryformatversion",
            Some("0".to_string()),
        );
        
        conf.set(
            "core",
            "filemode",
            Some("false".to_string()),
        );

        conf.set(
            "core",
            "bare",
            Some("false".to_string()),
        );

        return conf;
}

pub fn repo_create(path : impl AsRef<Path>)->io::Result<GitRepo>
{
    let repo = GitRepo::new_with_force(path, true);
    if repo.worktree.exists(){
        if !repo.worktree.is_dir(){
                return Err(std::io::Error::new( std::io::ErrorKind::Other, "Path is not a directory",));
            }
        if repo.gitdir.exists() && fs::read_dir(&repo.gitdir)?.next().is_some(){
                return Err(std::io::Error::new( std::io::ErrorKind::Other, ".git is not empty",));
 
            }
        }
        else{
            fs::create_dir_all(&repo.worktree)?;
        }

        repo.repo_dir(["branches"],true)?;
        repo.repo_dir(["objects"],true)?;
        repo.repo_dir(["refs","tags"],true)?;
        repo.repo_dir(["refs","heads"],true)?;

        fs::write(
            repo.repo_file(["description"],false)?,
            "Unnamed repository; edit this file 'description' to name the repository.\n")?;

        fs::write(
            repo.repo_file(["HEAD"],false)?,
            "ref: refs/heads/master\n")?;

        let conf = repo_default_config();
        let config_path = repo.repo_file(["config"], false)?;

        conf.write(config_path.to_str().unwrap())?;
        return Ok(repo);


}

pub fn repo_find(path : impl AsRef<Path>, required : bool)->io::Result<GitRepo> {
    let path = fs::canonicalize(path)?;
    if path.join(".git").is_dir(){
        return Ok(GitRepo::new(path));
    }

    let parent = match path.parent(){
        Some(p) => p,
        None => {
            if required {
                return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "No git directory",
                ));
            } else {
                return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "No git repository found",
                ));
            }
        }
    };
    
    repo_find(parent, required)
}


impl GitRepo{

    pub fn new(path : impl AsRef<Path>) -> Self{
        Self::new_with_force(path, false) 
    }

    pub fn new_with_force(path: impl AsRef<Path>, force: bool) -> Self{
    
        let worktree = path.as_ref().to_path_buf();
        let gitdir = worktree.join(".git");
        
        let conf = Ini::new();


        if !(force || gitdir.is_dir()){
                println!("not a git rep, will do a panic! or exception here later");
        }

        Self{
            worktree,
            gitdir,
            conf,
            }
    }

    fn repo_path<I,P>(&self, paths: I) -> PathBuf
        where
            I: IntoIterator<Item = P>,
            P: AsRef<Path>,
        {
            let mut full = self.gitdir.clone();

            for p in paths{
                full.push(p);
            }

            return full;
    }

    fn repo_dir<I,P>(&self, paths: I, mkdir : bool) -> std::io::Result<PathBuf>
    where 
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let path = self.repo_path(paths);
        if path.exists(){
            if path.is_dir(){
                return Ok(path);
            }
            else{
             return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Path is not a directory",
    )); 
            }
        }

        if mkdir{
            fs::create_dir_all(&path)?;
            return Ok(path);
        }

        Err(std::io::Error::new( std::io::ErrorKind::Other, "Directory doesn't exist",)) 
           
    }

    pub fn repo_file<I, P>(&self, paths: I, mkdir: bool) -> io::Result<PathBuf>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {   
        let path = self.repo_path(paths);

        if mkdir {
            if let Some(parent) = path.parent(){
                self.repo_dir([parent],true)?;
            }
        }

        Ok(path)
    }

}

    pub fn ref_resolve(repo: &GitRepo, git_ref: &str) -> io::Result<Option<String>> {

        let path = repo.repo_file([git_ref], false)?;

        if !path.is_file(){
            return Ok(None);
        }

        let data = fs::read_to_string(path)?;

        let data = data.trim_end();
        
        if data.starts_with("ref: "){
            let next_ref = &data[5..];
            ref_resolve(repo, next_ref)
        } else{

            Ok(Some(data.to_string()))
        }
    }

pub enum GitRef {
    Value(Option<String>),
    Node(Dict<GitRef>),
}

pub fn ref_list(repo: &GitRepo, path: Option<PathBuf>) -> io::Result<Dict<GitRef>>{
    let path = match path {
        Some(p) => p,
        None => repo.repo_dir(["refs"], false)?,
    };

    let mut ret = Dict::<GitRef>::new();
    
    let mut entries: Vec<_> =
        fs::read_dir(&path)?
            .collect::<Result<Vec<_>, _>>()?;

    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let can = entry.path();
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        if can.is_dir() {
            let subtree =
                ref_list(repo, Some(can))?;

            ret.add(name,GitRef::Node(subtree),);

        } else {
            let relative = can.strip_prefix(&repo.gitdir).unwrap();

            let resolved = ref_resolve(repo,relative.to_str().unwrap(),)?;
            ret.add(name, GitRef::Value(resolved),);
        }
    }

    Ok(ret)
}       

pub fn show_ref(refs: &Dict<GitRef>, with_hash : bool, prefix : &str){
    
    let prefix = if prefix.is_empty(){
        String::new()
    } else {
        format!("{}/",prefix)
    };

    for entry in refs.iter(){
        let k = &entry.key;
        let v = &entry.val;
        match v {
            GitRef::Value(Some(hash)) => {
                if with_hash {
                    println!(
                        "{} {}{}",
                        hash,
                        prefix,
                        k
                    );
                } else {
                    println!(
                        "{}{}",
                        prefix,
                        k
                    );
                }
            }
            GitRef::Value(None) => {

            }

            GitRef::Node(subtree) => {
                let new_prefix = format!("{} {}", prefix, k);

                show_ref(&subtree, with_hash, &new_prefix);
            }

        }
    }
}
