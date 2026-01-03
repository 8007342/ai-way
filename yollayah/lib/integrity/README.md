# Integrity Module

Script integrity verification for Yollayah.

## Quick Reference

```bash
# Normal operation (Level 1 - checksums)
./yollayah.sh

# Paranoid mode (Level 0 - git pull every time)
YOLLAYAH_INTEGRITY_LEVEL=0 ./yollayah.sh

# Skip verification (PJ mode - not recommended)
YOLLAYAH_SKIP_INTEGRITY=1 ./yollayah.sh

# Show integrity status
YOLLAYAH_INTEGRITY_STATUS=1 ./yollayah.sh

# Generate checksums (developers only)
YOLLAYAH_INTEGRITY_GENERATE=1 ./yollayah.sh
```

## Integrity Levels

### Level 0: Git Pull (Paranoid Mode)

**What it does**: Fetches from GitHub on every startup, compares local to remote.

**Trade-offs**:
- Slow (network required)
- Can't run offline
- Destructive (may overwrite local changes)
- Most thorough protection

**When to use**:
- You don't trust your local filesystem
- You want guaranteed fresh code
- You have reliable internet

**Set with**: `YOLLAYAH_INTEGRITY_LEVEL=0`

### Level 1: Checksums (Default)

**What it does**: Verifies SHA-256 hashes of all script files against stored manifest.

**Trade-offs**:
- Fast (local only)
- Works offline
- Catches corruption and simple tampering
- Can't prove authorship

**When to use**:
- Normal operation
- Offline environments
- Good balance of security and convenience

**Set with**: `YOLLAYAH_INTEGRITY_LEVEL=1` (or don't set anything)

### Level 2: Signatures (Future)

**What it will do**: Verify GPG or Sigstore cryptographic signatures.

**Status**: NOT YET IMPLEMENTED

Falls back to Level 1 if selected.

### Disabled: Skip All Checks

**What it does**: Runs scripts without any verification.

**Trade-offs**:
- Fastest (no verification)
- No protection against tampering
- Useful for development

**When to use**:
- You're modifying the scripts
- You're debugging an integrity issue
- You understand and accept the risks

**Set with**: `YOLLAYAH_SKIP_INTEGRITY=1`

## Always-On Protection

Regardless of level, these protections **cannot be disabled**:

1. **PATH sanitization**: Resets PATH to safe directories
2. **LD_PRELOAD protection**: Clears dangerous linker variables
3. **Location validation**: Refuses to run from /tmp, /dev/shm
4. **Permission checks**: Warns on world-writable scripts

These are in `environment.sh` and run before any other code.

## Threat Model

### What This Protects Against

| Threat | Level 0 | Level 1 | Disabled |
|--------|---------|---------|----------|
| Disk corruption | ✓ | ✓ | ✗ |
| Incomplete download | ✓ | ✓ | ✗ |
| PATH injection | ✓ | ✓ | ✓* |
| LD_PRELOAD injection | ✓ | ✓ | ✓* |
| Local tampering (unsophisticated) | ✓ | ✓ | ✗ |
| Running from /tmp | ✓ | ✓ | ✓* |

*Environment sanitization always runs

### What This Does NOT Protect Against

| Threat | Why Not |
|--------|---------|
| Repository compromise | Attacker modifies source AND checksums |
| Supply chain attacks | Compromised Ollama, Python, etc. |
| Physical access | Root always wins |
| Nation-state actors | If they're targeting you, this won't help |

### The Honest Truth

> Most bash script integrity verification is security theater against sophisticated attackers.

The real value is:
1. **Detecting accidents** (disk errors, bad copies)
2. **Catching script kiddies** (unsophisticated tampering)
3. **Building confidence** (signals we care about security)
4. **Audit trail** (knowing what code ran)

If a sophisticated attacker has access to your machine or our repository, integrity checks won't save you. But they catch 90% of real-world issues with minimal overhead.

## The Chicken-and-Egg Problem

Level 1 (checksums) has a fundamental limitation:

> If an attacker can modify the script, they can modify the checksums too.

The checksums are stored in `.integrity/checksums.sha256` in the same repository. An attacker with write access can modify both.

**Solutions** (not implemented yet):
1. Level 0: Always pull from remote (trusts GitHub's TLS)
2. Level 2: Cryptographic signatures (trusts our signing key)
3. Out-of-band: Verify checksums from separate source

For now, we accept this limitation and document it honestly.

## For Developers

### Updating Checksums

When you modify any script files:

```bash
# Regenerate the manifest
YOLLAYAH_INTEGRITY_GENERATE=1 ./yollayah.sh

# Commit the updated checksums
git add .integrity/
git commit -m "Update integrity checksums"
```

### Adding New Files

New files are automatically included when you regenerate the manifest.

### Pre-commit Hook (Recommended)

Install the git hooks to auto-update checksums on commit:

```bash
./scripts/install-hooks.sh
```

This installs a pre-commit hook that:
1. Detects when any `.sh` file is staged
2. Regenerates `.integrity/checksums.sha256`
3. Stages the updated checksums

Checksums stay in sync automatically - no manual updates needed.

## Module Structure

```
lib/integrity/
├── README.md           # This file
├── init.sh             # Entry point, level selection
├── environment.sh      # Always-on sanitization
├── level-0-git.sh      # Git pull verification
├── level-1-checksums.sh # Checksum verification
└── level-2-signatures.sh # Signatures (placeholder)
```

## Constitution Reference

This module implements several constitutional principles:

- **Law of Care**: Default to safety (Level 1)
- **Law of Truth**: Honest about limitations (this README)
- **Four Protections**: "Protect AJ from ai-way" - we verify our own code

The PJ escape hatch (`YOLLAYAH_SKIP_INTEGRITY=1`) respects user autonomy while defaulting to protection.

## See Also

- [Script Integrity ADR](../../agents/ai-way-docs/script-integrity-adr.md) - Full architecture decision record
- [CONSTITUTION.md](../../agents/CONSTITUTION.md) - Ethical principles
- [dangers/](../../agents/dangers/) - Threat research
