use rustls::server::ClientHello;

pub struct TlsInspector;

impl TlsInspector {
    pub fn fingerprint_ja3(&self, client_hello: &ClientHello) -> Option<String> {
        let mut ja3 = String::new();

        // 1. TLS Version (available through the connection, not ClientHello)
        // We'll get this later during the handshake
        
        // 2. Cipher Suites
        let ciphers: Vec<String> = client_hello.cipher_suites()
            .iter()
            .map(|cs| cs.get_u16().to_string())
            .collect();
        ja3.push_str(&ciphers.join("-"));
        ja3.push(',');

        // 3. Extensions - only those explicitly exposed by rustls
        let mut extensions = Vec::new();
        
        if client_hello.server_name().is_some() {
            extensions.push("0".to_string());  // SNI
        }
        if client_hello.alpn().is_some() {
            extensions.push("16".to_string());  // ALPN
        }
        // Add other detectable extensions here
        
        ja3.push_str(&extensions.join("-"));
        ja3.push(',');

        // 4. Elliptic Curves - not directly available
        ja3.push_str("");  // Leave empty

        Some(ja3)
    }
}