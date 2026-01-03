# TODO: Move Bash Fallback Components to Module

**Created**: 2026-01-03
**Priority**: P2 - Code Organization
**Status**: 🔵 PROPOSED - Awaiting Planning

---

## Overview

Extract bash-based prompt and interaction components from the main `yollayah.sh` script into a dedicated `lib/bash-fallback/` module. This improves code organization, testability, and maintainability while preparing for the bash minimal fallback interface.

---

## Problem Statement

### Current State
- `yollayah.sh` contains inline bash prompt handling
- UX components (colors, prompts, formatting) mixed with bootstrap logic
- Difficult to reuse for fallback mode
- Hard to test in isolation

### Desired State
- Clean separation: `yollayah.sh` is pure bootstrap/orchestration
- `lib/bash-fallback/` contains all prompt/UX logic
- Reusable components for both testing mode and fallback mode
- Easy to extend with new features (colors, animations)

---

## Goals

1. **Extract UX components** - Colors, prompts, formatting → module
2. **Extract prompt logic** - User input handling → module
3. **Keep bootstrap clean** - yollayah.sh only orchestrates
4. **Enable reuse** - Fallback mode and test mode share code
5. **Maintain functionality** - No behavior changes, pure refactor

---

## Scope

### In Scope
- ✅ Extract color/formatting functions from yollayah.sh
- ✅ Extract user input/prompt handling
- ✅ Create `lib/bash-fallback/` module structure
- ✅ Update yollayah.sh to source module
- ✅ Test that existing functionality works

### Out of Scope
- ❌ Adding new features (save for TODO-bash-minimal-fallback.md)
- ❌ Changing behavior of existing code
- ❌ Full fallback mode implementation

---

## Components to Extract

### Current yollayah.sh Analysis
Let me identify what needs to be extracted:

1. **Terminal output functions**
   - Color codes (if any)
   - Status messages
   - Error formatting

2. **User input handling**
   - Prompt display
   - Input reading
   - Input validation

3. **UX polish**
   - Activity indicators (if any)
   - Progress messages
   - Formatting utilities

---

## Target Module Structure

```
lib/bash-fallback/
├── init.sh              # Module initialization
├── ui.sh                # Colors, formatting, box drawing
├── input.sh             # User input handling
├── output.sh            # Message output, rendering
├── activity.sh          # Spinners, progress indicators
└── README.md            # Module documentation
```

### File Contents

**`lib/bash-fallback/init.sh`**
```bash
#!/bin/bash
# Initialize bash-fallback module
# Sources all submodules

BASH_FALLBACK_DIR="$(dirname "${BASH_SOURCE[0]}")"

source "${BASH_FALLBACK_DIR}/ui.sh"
source "${BASH_FALLBACK_DIR}/input.sh"
source "${BASH_FALLBACK_DIR}/output.sh"
source "${BASH_FALLBACK_DIR}/activity.sh"
```

**`lib/bash-fallback/ui.sh`**
```bash
#!/bin/bash
# Terminal UI utilities: colors, formatting, box drawing

# Color definitions
declare -r COLOR_RESET='\033[0m'
declare -r COLOR_RED='\033[0;31m'
declare -r COLOR_GREEN='\033[0;32m'
declare -r COLOR_YELLOW='\033[0;33m'
declare -r COLOR_CYAN='\033[0;36m'

# Functions: print_color, format_bold, draw_box, etc.
```

**`lib/bash-fallback/input.sh`**
```bash
#!/bin/bash
# User input handling

# Read user input with prompt
read_user_input() {
    local prompt="$1"
    local input
    read -r -p "$prompt" input
    echo "$input"
}

# Validate input
validate_input() {
    # ...
}
```

**`lib/bash-fallback/output.sh`**
```bash
#!/bin/bash
# Message output and rendering

# Print user message
print_user_message() {
    local message="$1"
    echo -e "${COLOR_CYAN}You:${COLOR_RESET} $message"
}

# Print assistant message
print_assistant_message() {
    local message="$1"
    echo -e "${COLOR_GREEN}🦎 Yollayah:${COLOR_RESET} $message"
}

# Print system message
print_system_message() {
    local message="$1"
    echo -e "${COLOR_YELLOW}$message${COLOR_RESET}"
}
```

**`lib/bash-fallback/activity.sh`**
```bash
#!/bin/bash
# Activity indicators: spinners, progress

# Spinner frames (Braille patterns)
declare -ra SPINNER_FRAMES=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧')

# Show spinner during long operation
show_spinner() {
    local pid=$1
    local i=0
    while kill -0 $pid 2>/dev/null; do
        printf "\r🦎 Thinking... %s" "${SPINNER_FRAMES[$i]}"
        i=$(( (i + 1) % ${#SPINNER_FRAMES[@]} ))
        sleep 0.1
    done
    printf "\r"
}
```

---

## Migration Steps

### Phase 1: Create Module Structure
1. Create `lib/bash-fallback/` directory
2. Create stub files (init.sh, ui.sh, input.sh, output.sh, activity.sh)
3. Add basic functions (even if empty)

### Phase 2: Extract from yollayah.sh
1. Identify functions/code to extract
2. Move to appropriate module file
3. Replace in yollayah.sh with module function calls

### Phase 3: Update Bootstrap
1. Source `lib/bash-fallback/init.sh` in yollayah.sh
2. Update all references to use module functions
3. Remove duplicated code from yollayah.sh

### Phase 4: Test & Validate
1. Run `./yollayah.sh` - verify existing behavior works
2. Run `./yollayah.sh --test` - verify test mode works
3. Check for regressions in output/formatting

---

## yollayah.sh Before & After

### Before (Hypothetical)
```bash
#!/bin/bash
# yollayah.sh

# Inline color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
RESET='\033[0m'

# Inline functions
print_status() {
    echo -e "${GREEN}$1${RESET}"
}

# ... rest of bootstrap logic ...
print_status "Starting Yollayah..."
```

### After
```bash
#!/bin/bash
# yollayah.sh

# Source bash-fallback module
source lib/bash-fallback/init.sh

# Use module functions
print_system_message "Starting Yollayah..."

# ... rest of bootstrap logic (unchanged) ...
```

---

## Testing Strategy

### Manual Tests
1. **Existing functionality**
   - `./yollayah.sh` - Full startup works
   - `./yollayah.sh --test` - Test mode works
   - Colors, prompts, messages display correctly

2. **Module isolation**
   - Source `lib/bash-fallback/init.sh` in test script
   - Call module functions directly
   - Verify functions work standalone

3. **Regression tests**
   - Compare output before/after migration
   - Verify no behavior changes

### Automated Tests (Future)
```bash
# tests/bash-fallback/test_ui.sh
source lib/bash-fallback/ui.sh

test_color_output() {
    local output=$(print_color "Test" "$COLOR_GREEN")
    # Assert contains green escape code
}
```

---

## Benefits

### For Developers
- **Cleaner code** - Separation of concerns
- **Reusability** - Module functions usable anywhere
- **Testability** - Functions testable in isolation
- **Maintainability** - Changes localized to module

### For Fallback Mode
- **Foundation ready** - Module prepared for fallback implementation
- **Shared code** - No duplication between test mode and fallback mode
- **Consistency** - Same UX across modes

### For Future Features
- **Easy to extend** - Add new functions to module
- **Composable** - Combine functions for complex UX
- **Documented** - Module serves as reference

---

## Dependencies

- **Blocks**: None
- **Blocked by**: None (can start immediately)
- **Enables**: TODO-bash-minimal-fallback.md (uses this module)
- **Related**: Code organization, maintainability

---

## Acceptance Criteria

- ✅ `lib/bash-fallback/` module created with structure
- ✅ Color/formatting functions extracted to `ui.sh`
- ✅ Input handling extracted to `input.sh`
- ✅ Output rendering extracted to `output.sh`
- ✅ Activity indicators extracted to `activity.sh`
- ✅ `yollayah.sh` sources module via `init.sh`
- ✅ All existing functionality works (no regressions)
- ✅ Module documented in `lib/bash-fallback/README.md`

---

## Related Documents

- **Fallback**: `progress/active/TODO-bash-minimal-fallback.md` - Uses this module
- **Architecture**: `knowledge/requirements/REQUIRED-separation.md` - Separation of concerns
- **Code Quality**: Maintainability, testability, reusability

---

## Notes

- **Pure refactor** - No new features, just reorganization
- **Preserve behavior** - Existing code should work identically
- **Git history** - Use `git mv` if renaming files, preserve history
- **Comments** - Add comments explaining each module's purpose
- **Examples** - Include usage examples in module README

---

## Open Questions

1. **Should we extract logging functions too?**
   - Currently in `lib/logging/`
   - Should bash-fallback use those or have its own?
   - **Proposed**: Reuse `lib/logging/`, don't duplicate

2. **How to handle configuration?**
   - Colors, spinner frames, etc. - hardcoded or configurable?
   - **Proposed**: Hardcode for init, make configurable later if needed

3. **Should module functions be namespaced?**
   - Example: `bf_print_message` vs `print_message`
   - **Proposed**: Use descriptive names, no prefix (avoid pollution)

---

**Next Steps**: Review existing yollayah.sh code, identify extractable components, create module structure, perform migration, test thoroughly.
