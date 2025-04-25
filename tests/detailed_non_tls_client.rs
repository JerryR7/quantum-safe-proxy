use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Starting detailed non-TLS client test...");
    
    // Connect to the proxy server
    match TcpStream::connect("127.0.0.1:8443").await {
        Ok(mut stream) => {
            println!("Connected to proxy server");
            
            // Send a non-TLS message
            let message = "Hello, this is a detailed non-TLS message";
            match stream.write_all(message.as_bytes()).await {
                Ok(_) => println!("Sent message: {}", message),
                Err(e) => println!("Failed to send message: {}", e),
            }
            
            // Wait a moment
            sleep(Duration::from_millis(100)).await;
            
            // Try to read a response (will likely fail due to RST)
            let mut buffer = [0; 1024];
            match stream.read(&mut buffer).await {
                Ok(n) => {
                    if n > 0 {
                        println!("Received response: {}", String::from_utf8_lossy(&buffer[..n]));
                    } else {
                        println!("Connection closed by server (as expected)");
                    }
                },
                Err(e) => {
                    println!("Failed to receive response: {}", e);
                    println!("This is expected behavior - the server should send a TCP RST");
                }
            }
        },
        Err(e) => println!("Failed to connect: {}", e),
    }
    
    println!("Detailed non-TLS client test completed");
}
