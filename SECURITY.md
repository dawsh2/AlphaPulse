# Security Checklist for AlphaPulse

## ⚠️ CRITICAL - Never Commit These Files

### Private Keys & Secrets
- [ ] **NEVER** commit files containing private keys (*.key, *.pem)
- [ ] **NEVER** commit .env files with real credentials
- [ ] **NEVER** commit wallet files or mnemonics
- [ ] **NEVER** commit API keys or tokens

### Before Every Commit
```bash
# Check for sensitive files
git status
git diff --cached --name-only | grep -E '\.(env|key|pem|p12|pfx)$'

# Check file contents for keys
git diff --cached | grep -iE '(private_key|api_key|secret|password|mnemonic|seed_phrase)'
```

## 🔐 Environment Variables

### Development Setup
1. Copy example files:
```bash
cp .env.example .env
cp backend/contracts/.env.example backend/contracts/.env
cp backend/services/capital_arb_bot/.env.example backend/services/capital_arb_bot/.env
```

2. **NEVER** put real keys in example files
3. Use dummy values in .env.example files

### Production Deployment
- Use environment variables from CI/CD system
- Use secret management services (AWS Secrets Manager, HashiCorp Vault)
- Rotate keys regularly

## 🚨 Blockchain & DeFi Specific

### Smart Contract Security
- [ ] Never commit deployed contract addresses with private keys
- [ ] Keep deployment artifacts in .gitignore
- [ ] Use separate wallets for testing and production
- [ ] Never expose owner/deployer private keys

### Arbitrage Bot Security
- [ ] Use dedicated trading wallets
- [ ] Limit wallet balances to acceptable loss amounts
- [ ] Keep private keys in environment variables only
- [ ] Use hardware wallets for large amounts

### RPC Endpoints
- [ ] Don't commit RPC URLs with API keys embedded
- [ ] Use public endpoints for examples
- [ ] Keep premium RPC endpoints in env vars

## 📋 Quick Audit Commands

### Check for exposed secrets
```bash
# Search for potential secrets in staged files
git diff --cached | grep -iE '(0x[a-fA-F0-9]{64}|sk_live_|pk_live_|api_key|private_key)'

# Check if .gitignore is working
git ls-files | grep -E '\.(env|key|pem)$'

# Find untracked sensitive files
find . -type f \( -name "*.key" -o -name "*.env" -o -name "*.pem" \) -not -path "./node_modules/*" -not -path "./venv/*"
```

### Verify .gitignore
```bash
# Test if a file would be ignored
git check-ignore .env
git check-ignore private_key.pem

# See why a file is/isn't ignored
git check-ignore -v .env
```

## 🛡️ Best Practices

### API Keys
1. **Development**: Use test/sandbox keys
2. **CI/CD**: Use encrypted secrets
3. **Production**: Use secret management service
4. **Rotation**: Rotate keys quarterly

### Git Hygiene
1. Review every file before committing
2. Use `git add -p` for selective staging
3. Never use `git add .` without reviewing
4. Set up pre-commit hooks

### Pre-commit Hook Setup
Create `.git/hooks/pre-commit`:
```bash
#!/bin/bash
# Prevent committing files with sensitive extensions
FORBIDDEN_PATTERNS="\.env$|\.key$|\.pem$|private_key|secret_key|api_key|mnemonic"

if git diff --cached --name-only | grep -qE "$FORBIDDEN_PATTERNS"; then
    echo "❌ ERROR: Attempting to commit sensitive files!"
    echo "Files matching sensitive patterns:"
    git diff --cached --name-only | grep -E "$FORBIDDEN_PATTERNS"
    exit 1
fi

# Check for sensitive strings
if git diff --cached | grep -qE "(PRIVATE_KEY|API_KEY|SECRET|PASSWORD|MNEMONIC|0x[a-fA-F0-9]{64})"; then
    echo "⚠️  WARNING: Possible sensitive data in commit"
    echo "Please review your changes carefully"
    read -p "Continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi
```

### Emergency Response

If you accidentally commit sensitive data:

1. **Immediately** rotate the exposed credentials
2. Remove from history:
```bash
# Remove file from all commits
git filter-branch --force --index-filter \
  "git rm --cached --ignore-unmatch PATH_TO_FILE" \
  --prune-empty --tag-name-filter cat -- --all

# Force push (coordinate with team)
git push origin --force --all
git push origin --force --tags
```

3. Consider the credentials permanently compromised
4. Audit systems for unauthorized access
5. Use BFG Repo-Cleaner for complex cases

## 📝 Security Checklist for PRs

- [ ] No .env files included
- [ ] No private keys or mnemonics
- [ ] No hardcoded API keys
- [ ] No production endpoints with credentials
- [ ] No wallet files or keystores
- [ ] All sensitive config uses env vars
- [ ] Example files contain only dummy data
- [ ] No sensitive data in logs
- [ ] No blockchain private keys exposed

## 🔍 Monitoring

### GitHub Secret Scanning
- Enable secret scanning in repository settings
- Review and resolve alerts immediately
- Add custom patterns for project-specific secrets

### Local Scanning Tools
```bash
# Install and run gitleaks
brew install gitleaks
gitleaks detect --source . -v

# Install and run truffleHog
pip install truffleHog
trufflehog --regex --entropy=False .
```

## 📚 Additional Resources

- [GitHub Secret Scanning](https://docs.github.com/en/code-security/secret-scanning)
- [Git Security Best Practices](https://git-scm.com/book/en/v2/Git-Tools-Credential-Storage)
- [OWASP Cryptographic Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)
- [Ethereum Security Best Practices](https://consensys.github.io/smart-contract-best-practices/)

---

**Remember**: It's better to be overly cautious with secrets. When in doubt, don't commit!