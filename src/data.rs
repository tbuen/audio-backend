use regex::Regex;

pub struct Data {
    current_dir: String,
    file_list: Vec<String>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct File {
    pub dir: bool,
    pub name: String,
}

impl Data {
    pub fn new() -> Self {
        Self {
            current_dir: String::from("/sdcard/"),
            file_list: Vec::new(),
        }
    }

    pub fn current_dir(&self) -> String {
        self.current_dir.clone()
    }

    pub fn file_list(&self) -> Vec<File> {
        let re_dir = Regex::new(&format!("^{}([a-zA-Z0-9]+)/.*$", &self.current_dir)).unwrap();
        let re_file = Regex::new(&format!(r"^{}([a-zA-Z0-9]+\.OGG)$", &self.current_dir)).unwrap();

        let mut list: Vec<File> = self
            .file_list
            .iter()
            .filter_map(|f| {
                if let Some(c) = re_dir.captures(f) {
                    Some(File {
                        dir: true,
                        name: String::from(c.get(1).unwrap().as_str()),
                    })
                } else if let Some(c) = re_file.captures(f) {
                    Some(File {
                        dir: false,
                        name: String::from(c.get(1).unwrap().as_str()),
                    })
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
