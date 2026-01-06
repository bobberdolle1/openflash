# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **DO NOT** open a public issue
2. Email security concerns to: [security@openflash.dev] (or create a private security advisory on GitHub)
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 1 week
- **Resolution Timeline**: Depends on severity
  - Critical: 24-72 hours
  - High: 1-2 weeks
  - Medium: 2-4 weeks
  - Low: Next release

### Scope

Security issues we're interested in:

- **Desktop App**: Code execution, privilege escalation, data leaks
- **USB Protocol**: Buffer overflows, injection attacks
- **Firmware**: Memory corruption, unauthorized access
- **Dependencies**: Vulnerable third-party libraries

### Out of Scope

- Physical attacks requiring hardware access
- Social engineering
- Denial of service (unless severe)
- Issues in unsupported versions

### Recognition

Security researchers who responsibly disclose vulnerabilities will be:
- Credited in release notes (unless anonymity requested)
- Added to our Security Hall of Fame
- Eligible for swag (stickers, t-shirts)

## Security Best Practices for Users

1. **Download from official sources** only (GitHub Releases)
2. **Verify checksums** of downloaded binaries
3. **Keep software updated** to latest version
4. **Use trusted USB devices** - malicious firmware could compromise your system
5. **Backup important data** before flash operations

## Firmware Security

The firmware runs on microcontrollers with direct hardware access. While we implement safety checks:

- Always verify chip detection before operations
- Use write-protect features when available
- Never flash unknown firmware to production devices

Thank you for helping keep OpenFlash secure! 🔒
