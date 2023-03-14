use regex::Regex;

pub struct Data {
    current_dir: String,
    file_list: Vec<String>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DirEntry {
    Dir(String),
    File(String),
}

impl Data {
    pub fn new() -> Self {
        Self {
            current_dir: String::from("/"),
            file_list: Vec::new(),
        }
    }

    pub fn current_dir(&self) -> String {
        let re = Regex::new(r"/([^/]+)/$").unwrap();
        if let Some(c) = re.captures(&self.current_dir) {
            String::from(c.get(1).unwrap().as_str())
        } else {
            String::new()
        }
    }

    pub fn dir_up(&mut self) {
        if self.current_dir != "/" {
            let re = Regex::new(r"/[^/]+/$").unwrap();
            self.current_dir = re.replace(&self.current_dir, "/").into_owned();
        }
    }

    pub fn dir_enter(&mut self, dir: &str) {
        self.current_dir.push_str(dir);
        self.current_dir.push('/');
    }

    pub fn dir_content(&self) -> Vec<DirEntry> {
        let re_dir = Regex::new(&format!("^{}([^/]+)/.*$", &self.current_dir)).unwrap();
        let re_file = Regex::new(&format!(r"^{}([^/]+\.ogg)$", &self.current_dir)).unwrap();

        let mut list: Vec<DirEntry> = self
            .file_list
            .iter()
            .filter_map(|f| {
                if let Some(c) = re_dir.captures(f) {
                    Some(DirEntry::Dir(String::from(c.get(1).unwrap().as_str())))
                } else if let Some(c) = re_file.captures(f) {
                    Some(DirEntry::File(String::from(c.get(1).unwrap().as_str())))
                } else {
                    None
                }
            })
            .collect();

        list.sort();
        list.dedup();

        list
    }

    pub fn set_file_list(&mut self, lst: Vec<String>) {
        self.file_list = lst;
        println!("Now stored in data: {:?}", self.file_list);
    }
}
