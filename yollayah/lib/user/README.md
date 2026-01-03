# User Customizations Module

## Privacy Manifesto

This module handles AJ's personal customizations and data. It is designed with
**privacy as the non-negotiable foundation**.

### Constitution References

This module implements the Four Protections from our [Constitution](../../agents/CONSTITUTION.md):

1. **Protect AJ from ai-way** - We don't collect data we don't need
2. **Protect AJ from AJ** - We warn before risky actions, make deletion easy
3. **Protect AJ from third parties** - Nothing leaves this machine
4. **Protect the mission from corruption** - Privacy principles are immutable

And the relevant Laws of Evolution:

- **Law of Care**: "First, do no harm" - We don't leak data
- **Law of Truth**: "Be honest, always" - We tell AJ exactly what's stored
- **Law of Service**: "Serve genuine interests" - Convenience never trumps privacy

### Core Principles

#### 1. Nothing in /home

All user data stays in the ai-way directory (`$SCRIPT_DIR/.user/`).
No dotfiles in home directory. No config in ~/.config/.
When AJ deletes the ai-way folder, everything goes with it.

**Why**: Standard locations are the first place attackers look.
If AJ's machine is compromised, we don't want breadcrumbs.

#### 2. Minimal by Default

We store nothing unless AJ explicitly opts in.
Each category of data requires separate consent.

**Why**: The safest data is data that doesn't exist.

#### 3. Local Only

Nothing ever leaves this machine. No sync, no cloud, no analytics.
Not even anonymized. Not even "to improve the service."

**Why**: Once data leaves, you lose control. Forever.

#### 4. Plaintext Where Possible

Encrypted data creates key management problems.
If AJ forgets the password, they lose access.
For most data, obscurity (local, non-standard location) is enough.

**Why**: Security should not create new failure modes.

#### 5. Easy Deletion

One command clears everything. No confirmation loops.
`rm -rf .user/` works perfectly.

**Why**: The right to be forgotten is fundamental.

---

## What This Module Handles

### Implemented Now

#### Preferences (`preferences.sh`)
- Yollayah personality tweaks (sass level, language)
- UI preferences (colors, verbosity)
- Model preferences (override auto-selection)

**Privacy Note**: Low-risk but could fingerprint AJ if combined.
Preferences don't leak via model behavior.

#### Conversation History (`history.sh`)
- Opt-in conversation logging
- Session persistence across restarts

**Privacy Note**: HIGH RISK module. Conversations may contain sensitive
data. Currently opt-in with clear warnings.

#### Storage Abstraction (`storage.sh`)
- Handles data persistence with privacy-first design
- All data stays in `$SCRIPT_DIR/.user/`
- Supports secure deletion where available

### Planned (Not Yet Created)

#### Context Memory (`memory.sh`)
- Long-term facts AJ teaches Yollayah
- Project context that persists

**Privacy Review Needed**: VERY HIGH RISK. This is essentially a
dossier on AJ. Could be weaponized if exfiltrated.

#### Usage Analytics (`analytics.sh`) [Not Yet Created]
- Local-only usage patterns
- No transmission ever

**Privacy Review Needed**: Even local analytics could be used for
fingerprinting if the machine is compromised. Is this worth it?

---

## Development Guidelines

### Before Adding Any Feature

1. Ask: "Does AJ actually need this stored?"
2. Ask: "What's the worst case if this leaks?"
3. Ask: "Can we provide the same value without storing?"
4. Ask: "Is this opt-in with informed consent?"

### Code Review Checklist

- [ ] Data stays in SCRIPT_DIR/.user/
- [ ] No network calls
- [ ] No writes to standard locations (/home, ~/.config, etc.)
- [ ] Explicit opt-in required
- [ ] Easy to delete
- [ ] Privacy implications documented

### Testing Privacy

```bash
# Before running Yollayah
find ~ -newer /tmp/marker -type f 2>/dev/null > /tmp/before

# Run Yollayah, use features, exit

# After running Yollayah
find ~ -newer /tmp/marker -type f 2>/dev/null > /tmp/after

# Diff should be empty (no files created in home)
diff /tmp/before /tmp/after
```

---

## Module Structure

```
lib/user/
├── README.md        # This file - privacy manifesto
├── init.sh          # Module initialization (consent check)
├── storage.sh       # Storage abstraction (where/how)
├── preferences.sh   # User preferences
└── history.sh       # Conversation history (opt-in)
```

Future submodules (not yet created):
- `memory.sh` - Long-term context memory
- `analytics.sh` - Local-only usage patterns
- `export.sh` - Data export for portability
- `delete.sh` - Secure deletion utilities

---

## The Hard Truth

We cannot make AJ completely invisible. Local storage can be found.
Conversations can be reconstructed from model state.
Timing patterns can be analyzed.

What we CAN do:
- Make attacks expensive
- Leave no obvious breadcrumbs
- Give AJ control over their data
- Be honest about limitations

Read `agents/dangers/` for the full threat analysis.
