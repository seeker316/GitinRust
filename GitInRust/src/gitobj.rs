use std::fs;
use std::path::{Path, PathBuf};
use configparser::ini::Ini;
use std::io::{self, Write, Read};
use dict::{Dict, DictIface};
pub use std::collections::HashSet;
use crate::sha1;
use crate::gitfns::GitRepo;
use hex::decode;

pub use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};

trait GitObj {
    fn fmt(&self) -> &'static [u8];

    fn serialize(&self) -> Vec<u8>;
    
    fn deserialize(data: &[u8]) -> Self
        where
            Self: Sized;

}

struct GitBlob{
    blobdata: Vec<u8>
}

impl GitObj for GitBlob{
    fn serialize(&self) -> Vec<u8> {
        self.blobdata.clone()
    }

    fn deserialize(data: &[u8]) -> Self{
        GitBlob{
          blobdata: data.to_vec()
        }
    }

    fn fmt(&self) -> &'static [u8]{
        b"blob"
    }

}

type Kvlm = Dict<Vec<Vec<u8>>>;
struct GitCommit{
    kvlm: Kvlm, 
}

impl GitObj for GitCommit{

    fn serialize(&self) -> Vec<u8> {
        key_value_serialize(&self.kvlm)
    }

    fn deserialize(data: &[u8]) -> Self{
        GitCommit{
          kvlm: key_val_parse(data,0,None),
        }
    }

    fn fmt(&self) -> &'static [u8]{
        b"commit"
    }

}

#[derive(Clone)]

struct GitTreeLeaf{
    pub mode: Vec<u8>,
    pub path: String,
    pub sha_hash: [u8; 20],
}

struct GitTree{
    pub items: Vec<GitTreeLeaf>,
}

impl GitObj for GitTree{
    fn serialize(&self) -> Vec<u8> {
        let mut items = self.items.clone();

        items.sort_by_key(tree_leaf_sort);

        let mut ret: Vec<u8> = Vec::new();

        for item in &items{
            ret.extend_from_slice(&item.mode);

            ret.push(b' ');

            ret.extend_from_slice(item.path.as_bytes());

            ret.push(0);

            ret.extend_from_slice(&item.sha_hash);
        }
        ret
    }

    fn deserialize(data: &[u8]) -> Self{
        GitTree{
          items: tree_parse(data),
        }   
    }

    fn fmt(&self) -> &'static [u8]{
        b"tree"
    }

}

struct GitTag {
    kvlm: Kvlm,
}

impl GitObj for GitTag{
    fn serialize(&self) -> Vec<u8>{
        key_value_serialize(&self.kvlm)
    }

    fn deserialize(data : &[u8]) -> Self {
        GitTag{
            kvlm: key_val_parse(data, 0, None),
        }
    }

    fn fmt(&self) -> &'static [u8] {
        b"tag"
    }
}

pub enum GitObject{
    Blob(GitBlob),
    Commit(GitCommit),
    Tree(GitTree),
    Tag(GitTag)
}

impl GitObject {
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            GitObject::Blob(b) => b.serialize(),
            GitObject::Commit(c) => c.serialize(),
            GitObject::Tree(t) => t.serialize(),
            GitObject::Tag(t) => t.serialize(),
        }
    }

    pub fn fmt(&self) -> &'static [u8] {
        match self {
            GitObject::Blob(b) => b.fmt(),
            GitObject::Commit(c) => c.fmt(),
            GitObject::Tree(t) => t.fmt(),
            GitObject::Tag(t) => t.fmt(),
        }
    }
}

pub fn object_read(repo: &GitRepo, sha_hash: [u8;20]) -> io::Result<Option<GitObject>>{
    let sha =  sha_hash.iter().map(|b| format!("{:02x}",b)).collect::<String>();

    let path = repo.repo_file([
        "objects",
        &sha[0..2],
        &sha[2..]],
        false,
        )?;

    if !path.is_file(){
        return Ok(None);
    }

    let compressed = fs::read(path)?;

    let mut decoder = ZlibDecoder::new(&compressed[..]);

    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw)?;

    let x = raw.iter()
        .position(|&b| b == b' ')
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing space in git object",
            )
        })?;

    let fmt = &raw[0..x];

    let y = raw.iter()
        .position(|&b| b == 0)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing Null Byte in git object",
            ) 
        })?;

    let size_str = std::str::from_utf8(&raw[x+1..y])
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid size encoding",
            )
        })?;

    let size: usize = size_str.parse()
        .map_err(|_| {
            io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid size number",
            )
        })?;

    if size != raw.len() - y - 1{
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Malformed object: bad length",
            ));
    }

    let data =  raw[y + 1..].to_vec();
    
    let obj =  match fmt{
        b"commit" => GitObject::Commit(GitCommit::deserialize(&data)),
        b"tree" => GitObject::Tree(GitTree::deserialize(&data)),
        //b"tag" => Box::new(GitTag::deserialize(&data)),
        b"tag" => unimplemented!(),
        b"blob" => GitObject::Blob(GitBlob::deserialize(&data)),
        _ => {
            return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unknown git object type",
            ))
        }
    };
    return Ok(Some(obj));
}

 pub fn object_write(obj : &dyn GitObj, repo: Option<&GitRepo>) -> std::io::Result<String> {
    let data = obj.serialize();
    
    let header = format!(
        "{} {}\0",
        std::str::from_utf8(obj.fmt()).unwrap(),
        data.len()
    );

    let mut result = Vec::new();
    
    result.extend_from_slice(header.as_bytes());
    result.extend_from_slice(&data);

    let mut sha_bytes = [0u8; 20];

    sha1::sha1(&mut sha_bytes, &result);

    let sha = sha_bytes.iter()
                .map(|b| format!("{:02x}",b))
                .collect::<String>();
    
    if let Some(repo) = repo{
        
        let path = repo.repo_file(
            ["objects", &sha[0..2], &sha[2..]],true)?;

        if !path.exists(){
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&result)?;

            let compressed = encoder.finish()?;
            
            fs::write(path, compressed)?;
        }
    }

    return Ok(sha);    

 }

pub fn object_find(repo: &GitRepo, name: &str, fmt: Option<&str>, follow: bool)-> [u8; 20] {
    println!("{}", name);
    let bytes = hex::decode(name).expect("SHA"); 
    bytes.try_into().expect("SHA MUST BE EXACTLY 20 BYTES")
    
}

pub fn cat_file(repo: &GitRepo, object: &str, fmt: String) -> io::Result<()>{
    let sha = object_find(repo, &object, Some(&fmt), true);

    let obj = object_read(&repo, sha)?
    .ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Git object not found",
        )
    })?;

    io::stdout().write_all(&obj.serialize())?;

    return Ok(());
}

pub fn object_hash(path : impl AsRef<Path>, fmt: &str ,repo : Option<&GitRepo>) -> io::Result<String>{
    let data = fs::read(path)?;

   let obj: Box<dyn GitObj> = match fmt{
        "commit" => Box::new(GitCommit::deserialize(&data)),
        "tree" => Box::new(GitTree::deserialize(&data)),
        //"tag" => Box::new(GitTag::deserialize(&data)),
        "tag" => unimplemented!(),
        "blob" => Box::new(GitBlob::deserialize(&data)),

        _ => {
            return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Unkown type!",
            ))
        }
    };

    object_write(obj.as_ref(), repo) 
}


pub fn key_val_parse(raw: &[u8] ,start :usize, dct: Option<Kvlm>) -> Kvlm{
    
    let mut dct = match dct{
        Some(d) => d,
        None => Kvlm::new(),
    };

    let spc = raw[start..].iter().position(|&b| b == b' ').map(|p| p+ start);

    let nl = raw[start..].iter().position(|&b| b == b'\n').map(|p| p + start).unwrap();
    
    if spc.is_none() || nl < spc.unwrap(){
           assert_eq!(nl, start);
            
           dct.add("__message__".to_string(), vec![raw[start + 1..].to_vec()]);
           return dct;
    }
    let spc = spc.unwrap();

    let key = String::from_utf8((&raw[start..spc]).to_vec()).unwrap().to_string();

    let mut end = start;
    
    loop {
        end = raw[end + 1..].iter().position(|&b| b == b'\n').map(|p| p + end + 1).unwrap();
        if end + 1 >= raw.len() || raw[end + 1] != b' ' {
            break;
        }
    }

    let mut value = raw[spc + 1..end].to_vec();

    let mut i = 0;
    
    while i + 1 < value.len(){
        if value[i] == b'\n' && value[i + 1] == b' '{
            value.remove(i + 1);
        }

        i += 1;
    }

    // if dct.contains_key(&key){
    //     dct.get_mut(&key).unwrap().push(value);
    // } 
    // else{
    //     dct.add(key.clone(),vec![value]);
    // }
        
    if dct.contains_key(&key) {
        for entry in dct.iter_mut() {
            if entry.key == key {
                entry.val.push(value);
                break;
            }
        }
    } else {
        dct.add(key.clone(), vec![value]);
    }
        return key_val_parse(raw,end+1,Some(dct));      
}

pub fn key_value_serialize(dct : &Kvlm) -> Vec<u8>{
    let mut ret = Vec::<u8>::new();
    
    for item in dct{
        if item.key == "__message__" {
            continue;
        }

        for val in &item.val{
            ret.extend_from_slice(item.key.as_bytes());

            ret.push(b' ');

            let mut v = Vec::<u8>::new();

            for &b in val{
                v.push(b);
                
                if b == b'\n' {
                    v.push(b' ');
                }
            }

            ret.extend_from_slice(&v);
            ret.push(b'\n');
        }
    }

    ret.push(b'\n');

    if let Some(msg) = dct.get("__message__"){
        if !msg.is_empty(){
            ret.extend_from_slice(&msg[0]);
        }
    }

    return ret;
}


pub fn log_graphviz(repo: &GitRepo, sha_hash: &[u8;20], seen: &mut HashSet<Vec<u8>> ) -> io::Result<()>{

    if seen.contains(sha_hash.as_slice()){
        return Ok(());
    }
    
    seen.insert(sha_hash.to_vec());

    let obj = object_read(repo, *sha_hash)?
    .ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Commit object not found",
        )
    })?;

    let commit = match obj {
        GitObject::Commit(c) => c,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Object is not a commit",
            ))
        }
    };
    
    assert_eq!(commit.fmt(),b"commit");

    let raw_message = &commit.kvlm.get("__message__").unwrap()[0];  

    let mut message = String::from_utf8_lossy(raw_message).trim().to_string();

    message = message.replace("\\", "\\\\"); //replace \ with \\
    message =  message.replace("\"","\\\""); //replace " with \"
    
    let message = message.lines().next().unwrap();
    let sha = hex::encode(sha_hash);

    println!(
        " c_{} [label=\"{}: {}\"]",
        sha,
        &sha[..7],
        message);

   
    if !commit.kvlm.contains_key("parent"){
        return Ok(());
    }

    let parents = commit.kvlm.get("parent").unwrap();   

    for p in parents {

        // ASCII hex SHA from commit object
        let p_str = String::from_utf8_lossy(p);

        let sha_str = hex::encode(sha_hash);

        println!(
            " c_{} -> c_{};",
            sha_str,
            p_str
        );

        // Convert hex -> raw bytes
        let p_bytes = hex::decode(p_str.as_ref()).unwrap();

        let p_array: [u8;20] =
            p_bytes.try_into().unwrap();

        log_graphviz(repo, &p_array, seen)?;
    }

    Ok(())

}


pub fn tree_parse_one(raw: &[u8],start: usize) -> io::Result<(usize, GitTreeLeaf)>{
    let x = raw[start..]
        .iter()
        .position(|&b| b == b' ')
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing space in tree object",
            )
        })?
        + start;
           
    assert!(x - start == 5 || x - start == 6);

    let mut mode = raw[start..x].to_vec();

    if mode.len() == 5{
        mode.insert(0,b'0');
    }
    
    let y = raw[x+1..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| p + x + 1)
            .unwrap();

    let path = String::from_utf8(raw[x+1..y].to_vec()).unwrap();

    let sha_slice = &raw[y+1..y+21];

    let sha_hash: [u8; 20] = sha_slice.try_into().unwrap();

    return Ok ((y + 21, 
        GitTreeLeaf{mode, path, sha_hash}
        ));
}

pub fn tree_parse(raw: &[u8]) -> Vec<GitTreeLeaf> {
    let mut pos : usize = 0;
    let max = raw.len();

    let mut ret = Vec::<GitTreeLeaf>::new();

    while pos < max{
        let (new_pos,data) = tree_parse_one(raw,pos).unwrap();
        pos = new_pos;
        ret.push(data)
    }

    return ret;

}

pub fn tree_leaf_sort( leaf: &GitTreeLeaf) -> String{

    if leaf.mode.starts_with(b"4"){
        return format!("{}/",leaf.path);
    }
    else{
        return leaf.path.clone();
    }
}

pub fn ls_tree(repo : &GitRepo, tree_ref: &str, recursive : bool, prefix : String) -> io::Result<()>{

    let sha = object_find(&repo, tree_ref, Some("tree"), true);
    
    let obj = object_read(repo, sha)?
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Tree object not found",
            )
    })?;

    let tree = match obj {
        GitObject::Tree(t) => t,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Object is not a tree",
            ))
        }
    };
    
    for item in tree.items{
       let mode_type = if item.mode.len() == 5{
        &item.mode[0..1]
       } 
       else{
           &item.mode[0..2]
       };

       let obj_type = match mode_type {
           b"04" => "tree",
           b"10" => "blob",
           b"12" => "blob",
           b"16" => "commit",
           _ => {
               return Err(io::Error::new(
                       io::ErrorKind::InvalidData,
                       format!("Weird tree leaf mode {:?}", item.mode),
               ))
           } 
       };
        
       let full_path = if prefix.is_empty(){
           item.path.clone()
       } else{
           format!("{}/{}",prefix,item.path)
       };
       
       if !(recursive && obj_type=="tree"){
           let mode_str = String::from_utf8_lossy(&item.mode);

           println!(
               "{:0>6} {} {}\t{}",
               mode_str,
               obj_type,
               hex::encode(item.sha_hash),
               full_path
           );
       
       }else{
           let sha = hex::encode(item.sha_hash);
           ls_tree(repo,&sha,recursive,full_path)?;
       }
       
    } 
    Ok(())
}

pub fn git_checkout(repo: &GitRepo, commit: &String, path: &String){

   let obj_sha = object_find(repo, commit, None, true); 
   let mut obj = object_read(repo, obj_sha).unwrap().expect("object not found");
    
   if let GitObject::Commit(commit_obj) = &obj {
        let tree_sha_vec = commit_obj.kvlm.get("tree").unwrap();
        let tree_sha: [u8; 20] = tree_sha_vec[0]
            .clone()
            .try_into()
            .expect("invalid sha");
        // let tree_sha = std::str::from_utf8(tree_sha).unwrap();
        //let tree_sha: [u8;20] = *commit_obj.kvlm.get(b"tree").unwrap();
        obj = object_read(&repo , tree_sha).unwrap().expect("tree object not found");
    }
    
    let path = Path::new(&path);
    if path.exists(){
        if !path.is_dir(){
            panic!("Not a git directory {}",path.display());
        }
        
        if std::fs::read_dir(path).unwrap().next().is_some(){
            panic!("Not empty {}",path.display());
        }
    } 
    else{
        std::fs::create_dir_all(path).unwrap();
    }

    let real_path = std::fs::canonicalize(path).unwrap();
                    
    tree_checkout(repo, &obj, &real_path).unwrap();
   
}

pub fn tree_checkout(repo: &GitRepo, tree: &GitObject, path : impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>>{
    let path = path.as_ref();
    
    if let GitObject::Tree(tree_obj) = tree{
        for item in &tree_obj.items{
            let obj = object_read(&repo, item.sha_hash)?.ok_or("object not found")?;
           
            let dest = path.join(&item.path);

            if obj.fmt() == b"tree"{
                fs::create_dir_all(&dest)?;
                tree_checkout(&repo, &obj, &dest);
            
            } else if obj.fmt() == b"blob" {
                let mut file = fs::File::create(&dest)?;

                if let GitObject::Blob(blob_obj) = &obj {
                    file.write_all(&blob_obj.blobdata)?;
                }
            }

        }

    }    
        Ok(())    
}

pub fn ref_create(repo: &GitRepo, ref_name: &str, sha: &str) -> io::Result<()>{
    let path = repo.repo_file([format!("refs/{}", ref_name)], true)?;
    
    let mut file = fs::File::create(path)?;
    
    writeln!(file, "{}", sha)?;

    Ok(())

}

pub fn tag_create(repo: &GitRepo, name : &str, git_ref: &str, create_tag_object : bool)-> io::Result<()>{

    let sha = object_find(repo, git_ref, None, true);
    
    if create_tag_object{
        let mut kvlm = Kvlm::new();

        kvlm.add(
            "object".to_string(),
            vec![
                hex::encode(sha).into_bytes()
            ],
        );

        kvlm.add(
            "type".to_string(),
            vec![b"commit".to_vec()],
        );

        kvlm.add(
            "tag".to_string(),
            vec![name.as_bytes().to_vec()],
        );

        kvlm.add(
            "tagger".to_string(),
            vec![
                b"GIR <robusthuss.com>"
                    .to_vec()
            ],
        );

        kvlm.add(
            "__message__".to_string(),
            vec![
                b"A tag generated by GIR!\n"
                    .to_vec()
            ],
        );

        let tag = GitTag{
            kvlm,
        };

        let tag_sha = object_write(&tag, Some(repo))?;
        
        ref_create(&repo, &format!("tags/{}",name),&tag_sha)?;
    }
    else {
        ref_create(
            &repo,
            &format!("tags/{}", name),
            &hex::encode(sha)
        )?;
        }

    Ok(())
}

