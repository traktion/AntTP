use std::io::{Read, Seek};
use log::debug;
use tar::Archive;

pub struct Tarchive;

impl Tarchive {
    pub fn index<R: Read + Seek>(mut reader: R) -> Result<String, std::io::Error> {
        let mut archive = Archive::new(&mut reader);
        let mut index = String::new();

        for entry in archive.entries()? {
            let entry = entry?;
            let header = entry.header();
            let path = entry.path()?.to_string_lossy().to_string();
            let offset = entry.raw_file_position();
            let size = header.size()?;

            if size > 0 {
                debug!("Tarchive::index - path: {}, offset: {}, size: {}", path, offset, size);
                index.push_str(&format!("{} {} {}\n", path, offset, size));
            }
        }

        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tar::Builder;

    #[test]
    fn test_index() {
        let mut builder = Builder::new(Vec::new());
        
        let data1 = b"hello";
        let mut header1 = tar::Header::new_gnu();
        header1.set_size(data1.len() as u64);
        header1.set_mode(0o644);
        builder.append_data(&mut header1, "file1.txt", &data1[..]).unwrap();

        let data2 = b"world!!";
        let mut header2 = tar::Header::new_gnu();
        header2.set_size(data2.len() as u64);
        header2.set_mode(0o644);
        builder.append_data(&mut header2, "dir/file2.txt", &data2[..]).unwrap();

        let tar_data = builder.into_inner().unwrap();
        let cursor = Cursor::new(tar_data);

        let index = Tarchive::index(cursor).unwrap();
        
        assert!(index.contains("file1.txt 512 5\n"));
        assert!(index.contains("dir/file2.txt 1536 7\n"));
    }
}
