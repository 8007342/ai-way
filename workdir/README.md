# workdir - Runtime Generated Files

**Purpose**: Temporary working directory for Yollayah runtime artifacts

---

## What's Here

This directory contains files generated during Yollayah's operation:

- **logs/** - Runtime logs (can be noisy, cleaned up periodically)
- **cache/** - Temporary cache files
- **state/** - Session state (if needed)

---

## Can I Delete This?

**Yes!** You can safely delete this entire directory.

Yollayah will recreate it on next run. Deleting it will:
- ✅ Free up disk space
- ✅ Clear old logs
- ⚠️  Lose cached data (will be regenerated, but slower on first access)

---

## For Average Joe (AJ)

Think of this like your computer's temp folder - Yollayah uses it while running, but you can clean it out anytime to save space. Next time you run Yollayah, it'll make a fresh one.

**Pro tip**: If Yollayah is acting weird, try deleting workdir/ and restarting. It's like turning it off and on again!

---

## For Privacy Joe (PJ)

This directory is on your local machine only. It's never uploaded anywhere. The logs here are for debugging and can contain:
- Model names used
- Conversation timestamps
- Performance metrics
- Error messages

If you're concerned about privacy, you can:
1. Delete workdir/ regularly
2. Set `YOLLAYAH_PERSIST_LOGS=0` to auto-clean on exit
3. Encrypt your home directory (Yollayah respects your encryption)

---

**Note**: This directory is .gitignore'd and won't be committed to version control.
