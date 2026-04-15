# Security Policy

## Supported Versions

The following versions of Auto-AI are currently supported with security updates:

| Version | Supported          | End of Support |
| ------- | ------------------ | -------------- |
| 1.2.x   | :white_check_mark: | Current        |
| 1.1.x   | :white_check_mark: | 2026-06-30     |
| 1.0.x   | :x:                | 2026-03-31     |
| < 1.0   | :x:                | Ended          |

**Recommendation**: Always use the latest stable version for security updates.

---

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report via:

1. **GitHub Security Advisories** (Preferred)
   - Go to the [Security tab](https://github.com/kardelitaitu/auto-ai/security/advisories)
   - Click "Report a vulnerability"
   - Provide details privately

2. **Email** (Alternative)
   - Send details to: security@auto-ai.local (configure in production)
   - Include "Security Vulnerability" in the subject

### What to Include

Please provide as much information as possible:

- **Type of issue**: e.g., buffer overflow, SQL injection, XSS, CSRF, etc.
- **Full paths** of source file(s) related to the issue
- **Location** of affected source code (tag/branch/commit or direct URL)
- **Step-by-step** instructions to reproduce the issue
- **Proof-of-concept** or exploit code (if possible)
- **Impact** of the issue, including how an attacker might exploit it

### Response Timeline

- **Acknowledgment**: Within 48 hours of report
- **Initial Assessment**: Within 5 business days
- **Fix Timeline**: Depends on severity (see below)

### Severity Levels

| Severity | Response Time | Fix Timeline |
|----------|---------------|--------------|
| Critical | 24 hours      | 7 days       |
| High     | 48 hours      | 14 days      |
| Medium   | 5 days        | 30 days      |
| Low      | 10 days       | 60 days      |

### What to Expect

1. **Confirmation**: We'll confirm receipt of your report
2. **Assessment**: We'll evaluate the vulnerability and determine impact
3. **Fix Development**: We'll work on a fix and test it thoroughly
4. **Disclosure**: We'll coordinate disclosure with you

### Disclosure Policy

We follow a **coordinated disclosure** approach:

1. Reporter submits vulnerability privately
2. We develop and test a fix
3. Fix is released in a security update
4. Advisory is published after users have time to update (typically 2-4 weeks)
5. Reporter is credited (unless they wish to remain anonymous)

---

## Security Best Practices

### For Users

When using Auto-AI, follow these security practices:

#### API Keys & Secrets

```env
# NEVER commit .env files to version control
# Add .env to .gitignore
.env

# Use environment variables in production
OPENROUTER_API_KEY=${OPENROUTER_API_KEY}

# Rotate keys regularly
# Use separate keys for development and production
```

#### Browser Security

- Use dedicated browser profiles for automation
- Keep browsers updated to latest versions
- Enable remote debugging only when needed
- Use firewall rules to restrict CDP access

#### Network Security

```bash
# Restrict CDP ports to localhost
# Use firewall rules like:
# Windows: netsh advfirewall firewall add rule ...
# Linux: ufw deny from any to any port 9222
```

#### Dependency Security

```bash
# Regularly update dependencies
pnpm update

# Audit for vulnerabilities
pnpm audit

# Use lock files
pnpm install --frozen-lockfile
```

### For Contributors

When contributing code:

1. **No secrets in code**: Never commit API keys, passwords, or tokens
2. **Input validation**: Validate all user inputs
3. **Error handling**: Don't expose sensitive info in errors
4. **Dependencies**: Keep dependencies updated
5. **Code review**: All changes require review

---

## Known Vulnerabilities

This section lists publicly known vulnerabilities that have been fixed.

| CVE ID | Severity | Version Fixed | Description |
|--------|----------|---------------|-------------|
| None reported yet | - | - | - |

---

## Security Updates

Security updates are released as part of regular releases. To stay secure:

1. **Watch the repository** for release notifications
2. **Update regularly**: `pnpm update`
3. **Check advisories**: https://github.com/kardelitaitu/auto-ai/security/advisories

---

## Contact

For security-related questions:

- **GitHub Security Advisories**: https://github.com/kardelitaitu/auto-ai/security/advisories
- **Email**: security@auto-ai.local (configure for production)

---

## Acknowledgments

We thank the following security researchers for their responsible disclosures:

- (To be populated as reports are received)

---

*Last updated: 2026-03-31*
