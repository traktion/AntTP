# Archives & Tarchives

Archives are the primary way to organize and serve multiple files on the Autonomi Network. AntTP supports both standard public archives and the more efficient Tarchive format.

## Public Archives

A public archive is a collection of files uploaded to the network that can be browsed by their original filenames.

### Uploading an Archive
To upload a directory as an archive using the `ant` CLI:
```bash
ant file upload -p /path/to/your/directory
```
The CLI will return an "At address" which is the XOR address of your archive.

### Accessing an Archive
Once uploaded, you can access the files via AntTP:
- **Direct Link:** `http://localhost:18888/[ARCHIVE_ADDRESS]/[FILENAME]`
- **Via Proxy:** `http://[ARCHIVE_ADDRESS]/[FILENAME]`

If you access the archive address directly (with a trailing slash), AntTP will generate a file listing.

---

## Tarchives

The Tarchive format is optimized for handling many small files (less than 4 MB). It combines multiple files into a single `.tar` file with an index appended at the end, allowing for faster sequential access and chronological ordering.

### Creating a Tarchive
You can use [Tarindexer](https://github.com/devsnd/tarindexer) to generate the required index.

1.  **Create the tar:**
    ```bash
    tar -cf archive.tar file1 file2 file3
    ```
2.  **Generate the index:**
    ```bash
    tarindexer.py -i archive.tar archive.tar.idx
    ```
3.  **Append the index to the tar:**
    ```bash
    tar -rf archive.tar archive.tar.idx
    ```
4.  **Upload the tarchive:**
    ```bash
    ant file upload -p archive.tar
    ```

### Appending to a Tarchive
Tarchives support appending new files while maintaining their chronological sequence.

1.  **Append new files:**
    ```bash
    tar -rf archive.tar new_file.txt
    ```
2.  **Regenerate and append the index:**
    ```bash
    tarindexer.py -i archive.tar archive.tar.idx
    tar -rf archive.tar archive.tar.idx
    ```
3.  **Upload the updated tarchive:**
    ```bash
    ant file upload -p archive.tar
    ```

AntTP transparently handles the retrieval of individual files from within a Tarchive.

---
[<< Previous](configuration.md) | [Up](../README.md) | [Next >>](web_app.md)
