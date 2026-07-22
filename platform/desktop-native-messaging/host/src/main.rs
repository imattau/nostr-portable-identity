use std::env;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

fn get_socket_path() -> Option<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".nostr-portable-identity/ipc.sock"))
}

fn connect() -> io::Result<Box<dyn ReadWrite>> {
    #[cfg(unix)]
    {
        let path = get_socket_path().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "HOME not set")
        })?;
        let stream = UnixStream::connect(&path)?;
        Ok(Box::new(stream))
    }
    #[cfg(windows)]
    {
        use std::net::TcpStream;
        let stream = TcpStream::connect("127.0.0.1:48631")?;
        Ok(Box::new(stream))
    }
}

trait ReadWrite: Read + Write {}
#[cfg(unix)]
impl ReadWrite for UnixStream {}
#[cfg(windows)]
impl ReadWrite for std::net::TcpStream {}

fn read_message() -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    io::stdin().read_exact(&mut len_buf)?;
    let len = u32::from_ne_bytes(len_buf) as usize;
    if len > 1_000_000 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    let mut buf = vec![0u8; len];
    io::stdin().read_exact(&mut buf)?;
    Ok(buf)
}

fn write_message(data: &[u8]) -> io::Result<()> {
    let len = data.len() as u32;
    let len_buf = len.to_ne_bytes();
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(&len_buf)?;
    handle.write_all(data)?;
    handle.flush()?;
    Ok(())
}

fn main() {
    let request = match read_message() {
        Ok(data) => data,
        Err(e) => {
            let error = serde_json::json!({ "error": format!("read error: {}", e) });
            let _ = write_message(error.to_string().as_bytes());
            process::exit(1);
        }
    };

    let mut stream = match connect() {
        Ok(s) => s,
        Err(e) => {
            let error = serde_json::json!({ "error": format!("cannot reach signer: {}", e) });
            let _ = write_message(error.to_string().as_bytes());
            process::exit(1);
        }
    };

    if let Err(e) = stream.write_all(&request) {
        let error = serde_json::json!({ "error": format!("send error: {}", e) });
        let _ = write_message(error.to_string().as_bytes());
        process::exit(1);
    }

    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            Err(e) => {
                let error = serde_json::json!({ "error": format!("read error: {}", e) });
                let _ = write_message(error.to_string().as_bytes());
                process::exit(1);
            }
        }
    }

    if let Err(e) = write_message(&response) {
        eprintln!("Write error: {}", e);
        process::exit(1);
    }
}
