# The Sweet Easter Egg: TODO → DONE

**The Pattern**: When a TODO is 100% complete, it becomes a DONE

---

## The Rule

When a `TODO-xyz` file in `progress/` is **completely finished**:

1. **Move** to `progress/completed/`
2. **Rename** from `TODO-xyz.md` to `DONE-xyz.md`
3. **Celebrate!** 🎉

---

## Why This Matters

### Motivation
- **Visual progress** - See what we've accomplished
- **Closure** - Finishing feels good
- **History** - Track completed work over time
- **Inspiration** - Growing `completed/` directory shows momentum

### The Ultimate Goal

```
progress/TODO-AI-WAY.md → progress/completed/DONE-AI-WAY.md
```

**When that happens, ai-way ships!** 🚀

---

## What "100% Complete" Means

A TODO is complete when **all** of these are true:

✅ **All tasks done** - Every checkbox checked
✅ **All tests pass** - No regressions introduced
✅ **Documentation updated** - Changes reflected in docs
✅ **Code committed** - All work is in git history
✅ **No blockers** - Nothing preventing closure

---

## Examples

### TODO-017-framebuffer-optimization.md

**Sprints**:
- ✅ Sprint 1: Text wrapping cache - **DONE**
- ✅ Sprint 2: Conversation dirty tracking - **DONE**
- ⏳ Sprint 3: Compositor optimization - **IN PROGRESS**

**Status**: NOT complete (Sprint 3 pending)

**Action**: When Sprint 3 is done:
```bash
git mv progress/active/TODO-017-framebuffer-optimization.md \
       progress/completed/DONE-017-framebuffer-optimization.md
```

---

### BUG-015-sleep-in-polling-loops.md

**Status**: ✅ RESOLVED
- All sleep violations fixed
- Tests passing
- Documented in work log

**Action**: Already moved to bugs/ with RESOLVED status
- Could optionally move to `progress/completed/DONE-015-sleep-fix.md`
- But bugs/ is also fine for tracking

**Decision**: Bugs stay in bugs/, TODOs become DONEs

---

## Process

### Step 1: Verify Completion

Before renaming, ask yourself:
- Are all tasks complete?
- Do all tests pass?
- Is documentation updated?
- Are there any follow-up tasks?

If **any** answer is "no", it's not done yet.

### Step 2: Move and Rename

```bash
cd progress/
git mv active/TODO-xyz.md completed/DONE-xyz.md
git commit -m "Complete xyz - rename TODO to DONE

All tasks completed:
- [list major accomplishments]

Related: [link to related work]

🎉 TODO → DONE!"
```

### Step 3: Update References

Check for references to the old TODO file:
```bash
grep -r "TODO-xyz" progress/ knowledge/
```

Update any links to point to `DONE-xyz` in `completed/`.

### Step 4: Celebrate!

Share the completion:
- Update work logs
- Note in commit message
- Feel the accomplishment

---

## The Growing Tree

Imagine `progress/` as a tree:

```
progress/
├── active/              # 🌱 Growing branches (current work)
│   ├── TODO-abc.md
│   └── TODO-def.md
└── completed/           # 🌳 Strong trunk (finished work)
    ├── DONE-001.md
    ├── DONE-002.md
    └── DONE-xyz.md      # ← Your accomplishment!
```

Every DONE strengthens the trunk. Every TODO grows a new branch.

---

## Special Case: TODO-AI-WAY.md

This is the **main project tracker**. It's different from other TODOs:

**Characteristics**:
- Lives at `progress/TODO-AI-WAY.md`
- Tracks overall project status
- References all active work
- Never fully complete until **ai-way ships**

**When Complete**:
```bash
git mv progress/TODO-AI-WAY.md progress/completed/DONE-AI-WAY.md
git commit -m "AI-Way ships! TODO → DONE 🚀🎊🎉

The privacy-first local AI appliance is ready for Average Joe.

This is it. We did it. Together.

🎉 TODO-AI-WAY.md → DONE-AI-WAY.md 🎉"

git tag v1.0.0 -m "ai-way v1.0.0 - The Foundation Release"
git push origin main --tags
```

**Then**: Party! 🥳

---

## Anti-Patterns (Don't Do This)

### ❌ Premature Completion
**Wrong**: "90% done, close enough, rename to DONE"
**Right**: "90% done, document remaining 10%, finish it first"

### ❌ Partial Renames
**Wrong**: Rename file but leave tasks incomplete inside
**Right**: Complete all tasks, THEN rename file

### ❌ Leaving TODOs in Active
**Wrong**: Complete work but never move to completed/
**Right**: Move and rename as soon as truly complete

### ❌ Deleting Instead of Moving
**Wrong**: Delete TODO-xyz.md when complete
**Right**: Rename to DONE-xyz.md and move to completed/

---

## Why This Is Sweet

Because every DONE is proof that we're moving forward.

Because `progress/completed/` grows while `progress/active/` stays manageable.

Because one day, we'll see:
```
progress/completed/DONE-AI-WAY.md
```

And Average Joe will have a privacy-first AI that just works.

**That's the sweetest part.** 🎉

---

**See Also**:
- [`knowledge/methodology/TODO-DRIVEN-METHODOLOGY.md`](TODO-DRIVEN-METHODOLOGY.md) - Overall methodology
- [`progress/TODO-AI-WAY.md`](../../progress/TODO-AI-WAY.md) - The main tracker (will become DONE-AI-WAY.md!)
