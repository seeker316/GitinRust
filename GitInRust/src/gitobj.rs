use std::fs;
use std::path::{Path, PathBuf};
use configparser::ini::Ini;
use std::io;
use dict::{Dict, DictIface};
use std::colletions::HashSet;
mod sha1;


use flate2::{reader::ZlibDecoder, write::ZlibEncoder, Compression};

trait GitObj {
    fn fmt(&self) -> &'static [u8];

    fn serialize(&self) -> Vec<u8>;
    
    fn deserialize(data: &[u8]) -> Self
        where
            Self: Sized;
}

struct GitBlob{
    blobdata: Vec<u8>,
}

impl GitObj for GitBlob{
    fn serialize(&self) -> Vec<u8> {
        self.blobdata.clone()
    }

    fn deserialize(data: &[u8]) -> Self{
        GitBlob{
          blobdata: data.to_vec()},
        }
    }

    fn fmt(&self) -> &'static [u8]{
        b"blob"
    }
}

pub fn object_read(repo: GitRepo, sha_hash: [0u8,20]){
    let sha =  sha_hash.iter().map(|b| format!("{:02x}",b)).collect::<String>();

    let path = repo.repo_file([
        "objects",
        &sha_hash[0..2],
        &sha_hash[2..]],
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
    
    let obj: Box<dyn GitObj> =  match fmt{
        b"commit" => Box::new(GitCommit::deserialize(&data)),
        b"tree" => Box::new(GitTree::deserialize(&data)),
        b"tag" => Box::new(GitTag::deserialize(&data)),
        b"blob" => Box::new(GitBlob::deserialize(&data)),
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

    sha1(&mut sha_bytes, &result);

    let sha = sha_bytes.iter()
                .map(|b| format!("{:02x}",b)
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

pub fn object_find(repo: &GitRepo, name: &str, fmt: Option<&str>, follow: bool)-> String {
    name.to_string();
}

pub fn cat_file(repo: &GitRepo, object: &str, fmt: ObjectType) -> io::Result<()>{
    let sha = object_find(repo, &object, fmt, true);

    let obj = object_read(repo, &sha)?;

    io::stdout().write_all(&obj.serialize())?;

    return Ok();
}

pub fn object_hash(path : impl AsRef<Path>, fmt: &str ,repo : Option<&GitRepo>) -> io::Result<String>{
    let data = fs::read(path)?;

    let obj = match fmt{
        "commit" => GitCommit::deserialize(data),
        "tree" => let obj = GitTree::deserialize(data),
        "tag" => let obj = GitTag::deserialize(data),
        "blob" => let obj = GitBlob::deserialize(data),

        _ => {
            return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Unkown type!",
            ));
        }
    };

    object_write(obj.as_ref(), repo); 
}

type Kvlm = Dict<Vec<Vec<u8>>>;

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

    let key = String::from_utf8(&raw[start..spc]).to_string();

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

    if dct.contains_key(&key){
        dct.get_mut(&key).unwrap().push(value);
    } 
    else{
        dct.add(key.clone(),vec![value]);
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
            ret.exted_from_slice(
                item.key.as_bytes()
            );

            ret.push(b' ');

            let mut v = Vec::<u8>::new();

            for &b in value{
                v.push(b);
                
                if b == b'\n' {
                    b.push(b' ');
                }
            }

            ret.extend_from_slice(&b);
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

struct GitCommit{
    kvlm : Kvlm
}

impl GitObj for GitCommit{
    fn serialize(&self) -> Vec<u8> {
        key_value_serialize(&self.kvlm);
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

pub fn log_graphviz(repo: &GitRepo, sha_hash: &[u8,20], seen: &mut HashSet<Vec<u8>> ){

    if seen.contains(sha_hash.as_slice()){
        return
    }
    
    seen.insert(sha_hash.to_vec());

    let commit = object_read(repo,sha_hash);

    let raw_message = &commit.kvlm["__message__"][0];
    
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

    assert_eq!(commit.fmt(),b"commit");

    if !commit.kvlm.contains_key(b"parent"){
        return;
    }

    let parents = commit.kvlm[b"parent"];
    

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

        log_graphviz(repo, &p_array, seen);
    }

}
