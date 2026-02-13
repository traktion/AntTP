use std::io::{Read, Seek};
use tar::Archive;

pub struct Tarchive;

impl Tarchive {
    pub fn sanitise_path(path: &str) -> String {
        path.trim_start_matches('/').trim_end_matches('/').to_string()
    }

    /// Generates a tar index string for the given tar file.
    /// The index format follows: "filename offset size"
    pub fn index<R: Read + Seek>(reader: &mut R) -> Result<String, std::io::Error> {
        let mut archive = Archive::new(reader);
        let mut index = String::new();

        let entries = archive.entries()?;
        for entry_result in entries {
            let entry = entry_result?;
            let header = entry.header();
            
            // We only index files
            if header.entry_type().is_file() {
                let path = entry.path()?;
                let path_str = path.to_str().unwrap_or("");
                let offset = entry.raw_file_position();
                let size = entry.header().size()?;
                
                index.push_str(&format!("{} {} {}\n", path_str, offset, size));
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
    fn test_tarchive_index() {
        let mut buf = Vec::new();
        {
            let mut builder = Builder::new(&mut buf);
            
            let data1 = b"hello world";
            let mut header1 = tar::Header::new_gnu();
            header1.set_size(data1.len() as u64);
            header1.set_path("file1.txt").unwrap();
            header1.set_cksum();
            builder.append(&header1, &data1[..]).unwrap();

            let data2 = b"anttp tarchive support";
            let mut header2 = tar::Header::new_gnu();
            header2.set_size(data2.len() as u64);
            header2.set_path("dir/file2.txt").unwrap();
            header2.set_cksum();
            builder.append(&header2, &data2[..]).unwrap();
            
            builder.finish().unwrap();
        }

        let mut cursor = Cursor::new(buf);
        let index = Tarchive::index(&mut cursor).unwrap();
        
        let lines: Vec<&str> = index.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("file1.txt "));
        assert!(lines[1].starts_with("dir/file2.txt "));
        
        // Verify offset and size
        let parts1: Vec<&str> = lines[0].split_whitespace().collect();
        assert_eq!(parts1[2], "11");
        
        let parts2: Vec<&str> = lines[1].split_whitespace().collect();
        assert_eq!(parts2[2], "22");
    }
}
