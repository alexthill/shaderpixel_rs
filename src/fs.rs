use std::io::{self, Cursor};
use std::path::Path;

pub fn load<P: AsRef<Path>>(path: P) -> Result<Cursor<Vec<u8>>, io::Error> {
    use std::fs::File;
    use std::io::Read;

    let mut buf = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(Cursor::new(buf))
}
