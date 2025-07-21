# WAF *(to be named)*

A **high-performance, standalone Web Application Firewall (WAF) proxy** that operates seamlessly between the internet and your backend (PHP, Node.js, Rust, WordPress, etc.).

Designed for stealth, it leaves minimal footprint by leveraging polymorphic injection filtering, generic blocking rules, and subtle CAPTCHAs that blend into normal traffic—neutralizing threats without revealing defensive measures. Invisible to attackers, uncompromising in protection. WAF build on top of [wi1dcard's fingerprint reverse proxy](https://github.com/wi1dcard/fingerproxy)

## Planned Features

- [ ] **AI Blocking** - Detect and block AI agents/scrapers using behavioral analysis and fingerprinting techniques
- [ ] **Scraper Blocking** - Prevent automated scraping tools through advanced request pattern detection
- [ ] **Realtime Alerts** - Immediate notifications for active threats with severity classification
- [ ] **Reporting** - Comprehensive security logs with export capabilities for analysis
- [ ] **Challenge Captchas** - Deploy interactive challenges for suspicious traffic verification
- [ ] **Privacy Options** - Anonymize user data while maintaining security protections
- [x] **TLS Fingerprinting** - Fingerprint clients server side with [JA3](https://engineering.salesforce.com/tls-fingerprinting-with-ja3-and-ja3s-247362855967/), thanks to [fingerproxy](https://github.com/wi1dcard/fingerproxy)

## HTTPS Setup  

### Option 1: Self-Signed Certificate (Testing Only)

For basic testing, generate a self-signed TLS certificate:  

```bash
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:secp384r1 -days 3650 \
  -nodes -keyout tls.key -out tls.crt -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,DNS:*.localhost,IP:127.0.0.1"
```  

⚠️ **Note:** Self-signed certificates may cause browser warnings and are not suitable for production.  

### Option 2: Let’s Encrypt (Recommended)  

For a trusted certificate, use [Certbot](https://certbot.eff.org):  

1. Obtain certificates:  

   ```bash
   sudo certbot certonly --standalone -d <YOUR_DOMAIN>
   ```  

2. Copy the certificates to your WAF directory:  

   ```bash
   sudo cp /etc/letsencrypt/live/<YOUR_DOMAIN>/fullchain.pem ./tls.crt  
   sudo cp /etc/letsencrypt/live/<YOUR_DOMAIN>/privkey.pem ./tls.key  
   ```  

🔹 **Tip:** Automate renewal with `certbot renew` if using Let’s Encrypt in production.  
