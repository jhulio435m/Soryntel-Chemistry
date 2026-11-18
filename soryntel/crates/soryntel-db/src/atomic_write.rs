use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(crate) fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let directory = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let (temp, mut file) = loop {
        let seq = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temp = directory.join(format!(".soryntel-{}-{seq}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => break (temp, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = (|| {
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        #[cfg(unix)]
        fs::File::open(directory)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
