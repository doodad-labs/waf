> [!IMPORTANT]
> Development Notice: This project is pre-release software. Use development builds at your own risk.

# waf *(to be named)*

A high-performance, standalone **Web Application Firewall (WAF) proxy** that sits between the internet and any backend server (PHP, Node.js, Rust, Wordpress, Etc.).

## Planned Features

- [ ] **AI Blocking** - Detect and block AI agents/scrapers using behavioral analysis and fingerprinting techniques
- [ ] **Scraper Blocking** - Prevent automated scraping tools through advanced request pattern detection
- [ ] **Realtime Alerts** - Immediate notifications for active threats with severity classification
- [ ] **Reporting** - Comprehensive security logs with export capabilities for analysis
- [ ] **Challenge Captchas** - Deploy interactive challenges for suspicious traffic verification
- [ ] **Privacy Options** - Anonymize user data while maintaining security protections

## Issues

- **Websockets** - Websockets don't tunnel correctly resulting in issues establishing socket connections.

## Testing

Branch | Test Result | Coverage | Code Quality
-------|-------------|----------|--------------
[`main`](https://github.com/doodad-labs/waf/tree/main) | ![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/doodad-labs/waf/rust.yml?style=for-the-badge&label=%20) | ![Codacy coverage](https://img.shields.io/codacy/coverage/85f7bc2e552544508b0c5a10a05cd5a3?style=for-the-badge&label=%20) | ![Codacy grade](https://img.shields.io/codacy/grade/c510c56cd1ba4c9f83db5a0e253572ac?style=for-the-badge&label=%20)
