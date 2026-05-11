use std::fs;
use std::path::{Path, PathBuf};
use configparser::ini::Ini;
use std::io;

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


