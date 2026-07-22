use std::env;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process;

fn get_socket_path() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("HOME must be set");
    PathBuf::from(home).join(".nostr-portable-identity/ipc.sock")
}

fn read_message() -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    io::stdin().read_exact(&mut len_buf)?;
    let len = u32::from_ne_bytes(len_buf) as usize;
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
    let socket_path = get_socket_path();

    let request = match read_message() {
        Ok(data) => data,
        Err(e) => {
            let error = serde_json::json!({
                "error": format!("failed to read request: {}", e)
            });
            let _ = write_message(error.to_string().as_bytes());
            process::exit(1);
        }
    };

    let mut stream = match UnixStream::connect(&socket_path) {
        Ok(s) => s,
        Err(e) => {
            let error = serde_json::json!({
                "error": format!("cannot connect to signer agent: {}", e)
            });
            let _ = write_message(error.to_string().as_bytes());
            process::exit(1);
        }
    };

    // Set timeout for the connection
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));

    // Forward the request to the signer agent
    if let Err(e) = stream.write_all(&request) {
        let error = serde_json::json!({
            "error": format!("failed to send request to signer: {}", e)
        });
        let _ = write_message(error.to_string().as_bytes());
        process::exit(1);
    }

    // Read the response from the signer agent
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Try again with a small delay
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            Err(e) => {
                let error = serde_json::json!({
                    "error": format!("failed to read response from signer: {}", e)
                });
                let _ = write_message(error.to_string().as_bytes());
                process::exit(1);
            }
        }
    }

    // Forward the response back to the browser
    if let Err(e) = write_message(&response) {
        eprintln!("Failed to write response to stdout: {}", e);
        process::exit(1);
    }
}
