/// Iterator over all files in a given folder. Returns paths relative to that given folder.
/// Directories are _not_ returned, only the files they contain.
pub struct RelativeFiles {
    root: std::path::PathBuf,
    worklist: Vec<std::fs::DirEntry>,
}

impl RelativeFiles {
    pub fn open<P>(folder: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        let folder = folder.as_ref();
        let worklist = if let Ok(read_dir) = folder.read_dir() {
            // flatten gets rid of Err(e) entries
            read_dir.flatten().collect()
        } else {
            vec![]
        };
        Self {
            root: folder.to_path_buf(),
            worklist,
        }
    }
}

impl Iterator for RelativeFiles {
    type Item = std::path::PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        let last_element = self.worklist.pop()?.path();
        if last_element.is_file() {
            match last_element.strip_prefix(&self.root) {
                Ok(last_element) => return Some(last_element.to_path_buf()),
                Err(e) => eprintln!("Boo boo in RelativeFiles {}", e),
            }
        }
        if last_element.is_dir() {
            if let Ok(read_dir) = last_element.read_dir() {
                let mut contents: Vec<_> = read_dir.flatten().collect();
                self.worklist.append(&mut contents);
                return self.next();
            }
        }
        None
    }
}
