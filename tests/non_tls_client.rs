use std::io::{Read, Write};
use std::net::TcpStream;

fn main() {
    println!("Starting non-TLS client test...");
    
    // Connect to the proxy server
    match TcpStream::connect("127.0.0.1:8443") {
        Ok(mut stream) => {
            println!("Connected to proxy server");
            
            // Send a non-TLS message
            let message = "Hello, this is a non-TLS message";
            match stream.write(message.as_bytes()) {
                Ok(_) => println!("Sent message: {}", message),
                Err(e) => println!("Failed to send message: {}", e),
            }
            
            // Try to read a response (will likely fail due to RST)
            let mut buffer = [0; 1024];
            match stream.read(&mut buffer) {
                Ok(n) => {
                    if n > 0 {
                        println!("Received response: {}", String::from_utf8_lossy(&buffer[..n]));
                    } else {
                        println!("Connection closed by server (as expected)");
                    }
                },
                Err(e) => println!("Failed to receive response: {}", e),
            }
        },
        Err(e) => println!("Failed to connect: {}", e),
    }
    
    println!("Non-TLS client test completed");
}
