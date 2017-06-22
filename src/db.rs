
use serde::{Serialize, Deserialize};
use serde_json;
use serde_json::Value;
use serde_json::value::Map;
use serde_json::ser::{Serializer, PrettyFormatter};

use std::io::prelude::*;
use std::io::{Error, ErrorKind, Result};
use uuid::Uuid;


use std::path::{Path, PathBuf};
use std::fs::{read_dir, rename, create_dir_all, remove_file, metadata, OpenOptions};
use std::collections::BTreeMap;
use fs2::FileExt;

type Object = Map<String, Value>;
use config::Config;


#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
    cfg: Config,
}

impl Database {
    fn id_to_path(&self, id: &str) -> PathBuf {
        if self.cfg.single {
            self.path.clone()
        } else {
            self.path.join(id).with_extension("json")
        }
    }

    fn path_buf_to_id(&self, p: PathBuf) -> Result<String> {
        p.file_stem()
            .and_then(|n| n.to_os_string().into_string().ok())
            .ok_or_else(|| Error::new(ErrorKind::Other, "invalid id"))
    }

    pub fn set_path(&mut self, p: PathBuf) {
        self.path = p;
    }

    fn to_writer_pretty<W: Write, T: Serialize>(&self, writer: &mut W, value: &T) -> Result<()> {
        let mut indent: Vec<char> = vec![];
        for _ in 0..self.cfg.indent {
            indent.push(' ');
        }
        let b = indent.into_iter().collect::<String>().into_bytes();
        let mut s = Serializer::with_formatter(writer, PrettyFormatter::with_indent(&b));
        value.serialize(&mut s).map_err(|err| {
            Error::new(ErrorKind::InvalidData, err)
        })?;
        Ok(())
    }

    fn to_vec_pretty<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        let mut writer: Vec<u8> = vec![];
        self.to_writer_pretty(&mut writer, value)?;
        Ok(writer)
    }

    fn object_to_string<T: Serialize>(&self, obj: &T) -> Result<String> {
        if self.cfg.pretty {
            let vec = self.to_vec_pretty(obj)?;
            String::from_utf8(vec).map_err(|err| Error::new(ErrorKind::Other, err))
        } else {
            serde_json::to_string(obj).map_err(|err| Error::new(ErrorKind::Other, err))
        }
    }

    fn save_object_to_file<T: Serialize>(&self, obj: &T, file_name: &PathBuf) -> Result<()> {
        let json_string = self.object_to_string(obj)?;
        let tmp_filename = Path::new(&Uuid::new_v4().to_string()).with_extension("tmp");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_name)?;
        let mut tmp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_filename)?;
        file.lock_exclusive()?;
        tmp_file.lock_exclusive()?;

        match Write::write_all(&mut tmp_file, json_string.as_bytes()) {
            Err(err) => return Err(err),
            Ok(_) => {
                let fp = file_name.to_str().unwrap().to_string();

                rename(tmp_filename, file_name)?;
                tmp_file.unlock()?;
                file.unlock()
            }
        }
    }

    fn get_string_from_file(file_name: &PathBuf) -> Result<String> {
        let file_str = file_name.to_str().unwrap().to_string();

        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file_name)
            .unwrap();

        // let mut f = match(oFile) {
        //     Ok(file) => {
        //         return Ok(file);
        //     },
        //     Err(err) => {
        //         println!("error!: {}", err.to_string());
        //         return E[<8;29;31m>]rr(err);
        //     }
        // };

        let mut buffer = String::new();

        f.lock_shared()?;

        f.read_to_string(&mut buffer)?;

        f.unlock()?;
        Ok(buffer)
    }

    pub fn get_db_path(&self) -> &PathBuf {
        return &self.path;
    }

    fn get_json_from_file(&self, file_name: &PathBuf) -> Result<Value> {
        let path = file_name.to_str().unwrap().to_string();

        let mut s: String = Database::get_string_from_file(file_name)?;
        let err_handler = |err| { return Error::new(ErrorKind::Other, err); };

        if (s == "") {
            s = "{}".to_string();
        }

        return serde_json::from_str(&s).map_err(err_handler);
    }

    fn get_object_from_json(json: &Value) -> Result<&Object> {
        json.as_object().ok_or_else(|| {
            Error::new(ErrorKind::InvalidData, "invalid file content")
        })
    }

    pub fn new(name: &str) -> Result<Database> {
        Database::new_with_cfg(name, Config::default())
    }

    pub fn new_with_cfg(name: &str, cfg: Config) -> Result<Database> {
        let mut s = Database {
            path: name.into(),
            cfg: cfg,
        };

        if cfg.single {
            s.path = s.path.with_extension("json");
            if !s.path.exists() {
                let o = Object::new();
                s.save_object_to_file(&o, &s.path)?;
            }
        } else if let Err(err) = create_dir_all(&s.path) {
            if err.kind() != ErrorKind::AlreadyExists {
                return Err(err);
            }
        }

        Ok(s)
    }

    pub fn save<T: Serialize + Deserialize>(&self, obj: &T) -> Result<String> {
        self.save_with_id(obj, &Uuid::new_v4().to_string())
    }

    pub fn save_with_id<T: Serialize + Deserialize>(&self, obj: &T, id: &str) -> Result<String> {
        if self.cfg.single {
            let json = self.get_json_from_file(&self.path)?;
            let o = Database::get_object_from_json(&json)?;
            let mut x = o.clone();
            let j = serde_json::to_value(&obj).map_err(|err| {
                Error::new(ErrorKind::Other, err)
            })?;
            x.insert(id.to_string(), j);
            self.save_object_to_file(&x, &self.path)?;

        } else {
            self.save_object_to_file(obj, &self.id_to_path(id))?;
        }
        Ok(id.to_owned())
    }

    fn decode<T: Deserialize>(o: Value) -> Result<T> {
        serde_json::from_value(o).map_err(|err| Error::new(ErrorKind::Other, err))
    }

    pub fn get<T: Deserialize>(&self, id: &str) -> Result<T> {
        let json = self.get_json_from_file(&self.id_to_path(id))?;
        let o = if self.cfg.single {
            let x = json.get(id).ok_or(Error::new(
                ErrorKind::NotFound,
                "no such object",
            ))?;
            x.clone()
        } else {
            json
        };
        Self::decode(o)
    }

    pub fn all<T: Deserialize>(&self) -> Result<BTreeMap<String, T>> {
        if self.cfg.single {
            let json = self.get_json_from_file(&self.id_to_path(""))?;
            let o = Database::get_object_from_json(&json)?;
            let mut result = BTreeMap::new();
            for x in o.iter() {
                let (k, v) = x;
                if let Ok(r) = Self::decode(v.clone()) {
                    result.insert(k.clone(), r);
                }
            }
            Ok(result)
        } else {
            let meta = metadata(&self.path)?;
            if !meta.is_dir() {
                Err(Error::new(ErrorKind::NotFound, "invalid path"))
            } else {
                let entries = read_dir(&self.path)?;
                Ok(
                    entries
                        .filter_map(|e| {
                            e.and_then(|x| {
                                x.metadata().and_then(|m| if m.is_file() {
                                    self.path_buf_to_id(x.path())
                                } else {
                                    Err(Error::new(ErrorKind::Other, "not a file"))
                                })
                            }).ok()
                        })
                        .filter_map(|id| match self.get(&id) {
                            Ok(x) => Some((id.clone(), x)),
                            _ => None,
                        })
                        .collect::<BTreeMap<String, T>>(),
                )
            }
        }
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        if self.cfg.single {
            let json = self.get_json_from_file(&self.path)?;
            let o = Database::get_object_from_json(&json)?;
            let mut x = o.clone();
            if x.contains_key(id) {
                x.remove(id);
            } else {
                return Err(Error::new(ErrorKind::NotFound, "no such object"));
            }
            self.save_object_to_file(&x, &self.path)
        } else {
            remove_file(self.id_to_path(id))
        }
    }
}
