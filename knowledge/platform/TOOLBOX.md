# Toolbox Mode

ai-way uses **toolbox containers** on Fedora Silverblue for clean dependency isolation. This guide explains what toolbox is, why ai-way uses it, and how to manage it.

---

## What is Toolbox?

Toolbox is a containerization system built into Fedora Silverblue. It creates lightweight containers that:

- Share your home directory with the host
- Share GPU devices automatically
- Run as your user (no root/sudo needed)
- Feel like a normal Linux environment

Think of it as a safe sandbox for installing software without affecting your main system.

---

## Why ai-way Uses Toolbox

On Fedora Silverblue (an immutable operating system), installing system packages requires special steps. Toolbox solves this by providing a container where:

1. **Dependencies stay isolated**: ollama and AI libraries install in the container, not on your host system
2. **GPU access just works**: toolbox automatically mounts GPU devices (`/dev/nvidia*`, `/dev/dri/*`)
3. **Clean uninstall**: Delete the container to remove everything
4. **No permission hassles**: Install packages without sudo or layering

---

## Automatic vs Manual Usage

### Automatic (Recommended)

Just run `./yollayah.sh` - it automatically:

1. Detects you're on Silverblue
2. Creates the `ai-way` toolbox if needed (one-time setup, ~30 seconds)
3. Enters the toolbox
4. Continues normal startup

You don't need to do anything special. The toolbox is invisible to you.

**Example**:
```bash
cd ~/src/ai-way
./yollayah.sh
# First run: Creates toolbox, installs ollama, launches TUI (~2-3 minutes)
# Later runs: Enters toolbox, launches TUI immediately (~5 seconds)
```

### Manual Usage

If you want to manually enter the toolbox:

```bash
# Enter the ai-way toolbox
toolbox enter ai-way

# Now you're inside the container
# Run yollayah.sh or any other commands
./yollayah.sh

# Exit the container
exit
```

---

## Toolbox Management

### Create Toolbox

Normally happens automatically, but you can create it manually:

```bash
toolbox create ai-way
```

### List Toolboxes

See all your toolbox containers:

```bash
toolbox list
```

### Remove Toolbox (Clean Uninstall)

To completely remove ai-way and all dependencies:

```bash
# Stop ai-way first (if running)
# Press Ctrl+C in the TUI, or:
pkill -f yollayah

# Remove the toolbox container
toolbox rm ai-way -f
```

This deletes:
- The toolbox container
- ollama installation inside container
- All downloaded AI models inside container
- All container filesystem data

**Important**: Your code in `~/src/ai-way` stays untouched (it lives on the host, not in the container).

**What gets removed**:
- Ollama binary (~1GB)
- All AI models (qwen2:0.5b ~352MB, llama3.2:1b ~1.3GB, etc.)
- Container OS packages
- Container filesystem (~2-5GB total depending on models)

**What stays**:
- ai-way source code (`~/src/ai-way/`)
- Host system completely unchanged
- Other toolbox containers (if any)

**To reinstall**: Just run `./yollayah.sh` again - it will create a fresh toolbox automatically.

### Recreate Toolbox

If something breaks, you can start fresh:

```bash
# Remove old toolbox
toolbox rm ai-way

# Next run will create a fresh one
./yollayah.sh
```

---

## Data Persistence

Understanding where data lives in toolbox mode:

### What's Inside the Container

Data stored in the container filesystem (destroyed when you run `toolbox rm ai-way`):

- **Ollama binary**: `/usr/local/bin/ollama`
- **AI models**: `~/.ollama/models/` (inside container)
- **Ollama service data**: `/tmp/ollama-*`
- **Installed packages**: Container's package manager database

**Total size**: ~2-5GB depending on how many models you've downloaded

### What's Shared with Host

Data that persists because it's on the host filesystem:

- **ai-way source code**: `~/src/ai-way/` (bind-mounted from host)
- **Your home directory**: `~/` (shared with container)
- **Any files you create in ~**: Saved to host, persist after toolbox removal

### Session Data (Ephemeral)

Yollayah uses ephemeral logging - logs are deleted on clean shutdown:

- **Logs**: `.logs/` directory (deleted on exit by default)
- **To persist logs**: Set `YOLLAYAH_PERSIST_LOGS=1`
- **Location**: `~/src/ai-way/.logs/` (on host, even in toolbox mode)

### Backing Up Before Uninstall

If you want to save your downloaded models before removing the toolbox:

```bash
# Enter the toolbox
toolbox enter ai-way

# List your models
ollama list

# Export models (if needed for another system)
# Models are ~352MB-8GB each depending on size
# Example: save qwen2:0.5b
ollama save qwen2:0.5b > ~/qwen2-0.5b.tar
# This saves to your host home directory

# Exit toolbox
exit

# Now safe to remove
toolbox rm ai-way -f
```

**Note**: Usually you don't need to back up models - they'll just re-download when you recreate the toolbox (takes 1-2 minutes depending on model size and network speed).

### Disk Space Considerations

Toolbox containers use your home directory for storage. Check disk space:

```bash
# Check container size
podman system df

# Check specific container
podman ps -a --size

# Clean up old/unused containers
podman container prune
```

If you're low on disk space, remove unused models:

```bash
toolbox enter ai-way
ollama list              # See what you have
ollama rm <model-name>   # Remove specific model
exit
```

---

## Troubleshooting

### Problem: "toolbox command not found"

**Solution**: You're probably not on Fedora Silverblue. Toolbox is pre-installed on Silverblue but may not exist on other distros.

On other distros, ai-way runs directly on the host (no toolbox needed).

### Problem: GPU not detected inside toolbox

**Symptoms**:
- Models run slowly
- `nvidia-smi` doesn't work inside toolbox
- ollama shows CPU-only inference

**Debugging**:

1. Check GPU works on host:
   ```bash
   nvidia-smi
   # Should show your GPU
   ```

2. Enter toolbox and check:
   ```bash
   toolbox enter ai-way
   nvidia-smi
   # Should also show your GPU
   ls /dev/nvidia*
   # Should list device files
   ```

3. If GPU missing in toolbox:
   - Update toolbox: `sudo rpm-ostree install toolbox` (then reboot)
   - Recreate container: `toolbox rm ai-way && toolbox create ai-way`
   - Check NVIDIA drivers on host: `nvidia-smi` should work

### Problem: "Container already exists" error

**Solution**: Either enter the existing container or remove it first:

```bash
# Option A: Enter existing container
toolbox enter ai-way

# Option B: Remove and recreate
toolbox rm ai-way
toolbox create ai-way
```

### Problem: Want to use host ollama instead of toolbox

**Solution**: ai-way is designed to use ollama inside the toolbox for isolation. Using host ollama is not currently supported on Silverblue.

If you're on a different distro (like Fedora Workstation), ai-way will automatically detect and use host ollama instead of toolbox.

### Problem: Slow first-time startup

**This is normal!** First run inside a fresh toolbox:

1. Creates toolbox (~30 seconds)
2. Installs ollama (~1 minute)
3. Downloads AI model (~1-2 minutes for small models)

**Total time**: 2-3 minutes

Later runs are fast (~5 seconds) because everything is already installed.

---

## GPU Passthrough Verification

To verify GPU passthrough is working:

1. Enter toolbox:
   ```bash
   toolbox enter ai-way
   ```

2. Check GPU visible:
   ```bash
   nvidia-smi
   # Should show your GPU model, memory, driver version
   ```

3. Test ollama GPU usage:
   ```bash
   # Inside toolbox
   ollama run qwen2:0.5b "Hello"
   # Watch for GPU activity in nvidia-smi output
   ```

If GPU works in toolbox, ai-way will automatically use it for fast inference.

---

## Technical Details

### What Gets Mounted

Toolbox automatically mounts:

- **Home directory**: `~/` (read/write)
- **GPU devices**: `/dev/nvidia*`, `/dev/dri/*` (for NVIDIA/AMD)
- **System directories**: `/usr`, `/etc`, `/var` (read-only from host)

### Container Lifecycle

- **Created**: First time you run `./yollayah.sh` on Silverblue
- **Persistent**: Container stays even after exiting
- **Shared**: Multiple terminal sessions can enter the same container
- **Removed**: Only when you run `toolbox rm ai-way`

### Performance

Toolbox has near-zero overhead:

- GPU performance: Identical to host (direct device access)
- Disk I/O: Native (home directory is bind-mounted, not copied)
- Network: Native (shares host network stack)
- Startup: ~0.1 seconds to enter container

The only overhead is initial creation (~30 seconds, one-time).

---

## Other Distros (Non-Silverblue)

If you're on Fedora Workstation, Ubuntu, or other traditional distros:

- ai-way detects toolbox is not available
- Runs directly on host (no container)
- All features work the same
- No toolbox commands needed

Toolbox mode is specifically for Silverblue users who need dependency isolation.

---

## See Also

- **Build Commands**: See `CLAUDE.md` for all build and test commands
- **Architecture**: See `TODO-epic-2026Q1-toolbox.md` for technical design
- **Troubleshooting**: See `yollayah.sh --help` for usage examples

---

**Last Updated**: 2026-01-02
**Epic**: TODO-epic-2026Q1-toolbox.md
**Sprint**: TODO-sprint-toolbox-1.md (Phase 1.4)
