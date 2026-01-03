# TODO: Animation Cache System - Initial Implementation

**Created**: 2026-01-03
**Priority**: P3 - Performance Optimization
**Status**: 🔵 PROPOSED - Awaiting Planning

---

## Overview

Design and implement an aggressive multi-layer caching system for avatar animations. The goal is to enable "lots AND LOTS AND LOTS of pretty graphics" without impacting performance, allowing the Conductor to request animations freely without concern for disk I/O or parsing overhead.

---

## Problem Statement

### Current State (After TODO-sprites-init)
- Sprite animations loaded from disk on demand
- Parsing overhead for each animation load
- Disk I/O for every frame read
- No reuse of loaded animations

### Desired State
- **Instant animation playback** - No loading delays
- **Aggressive caching** - Keep everything in memory that's been used
- **Layered caching** - Memory → Disk → Generation
- **Conductor doesn't care** - Request any animation anytime, cache handles it

---

## Goals

1. **In-memory cache** - Keep recently used animations in RAM
2. **Disk cache** - Pre-processed animations ready to load
3. **Cache invalidation** - Smart eviction when memory pressure
4. **Cache statistics** - Monitor hit rates and memory usage
5. **Preloading** - Predictive loading of likely-next animations

---

## Caching Layers

### Layer 1: Hot Memory Cache (Highest Priority)
**Storage**: In-memory HashMap/LRU cache
**Contents**: Currently displayed + recently displayed animations
**Size**: Configurable (default: 50MB or last 20 animations)
**Eviction**: LRU (Least Recently Used)

**Why**: Instant access, zero latency

### Layer 2: Warm Disk Cache (Processed Sprites)
**Storage**: ~/.cache/yollayah/animations/ or XDG_CACHE_HOME
**Contents**: Pre-processed, ready-to-render frames
**Format**: Fast-loading binary format (not original PNGs)
**Size**: Unlimited (user can clear if needed)
**Eviction**: Manual or based on disk space thresholds

**Why**: Faster than parsing original sprites, persistent across runs

### Layer 3: Cold Storage (Original Sprites)
**Storage**: Application assets directory
**Contents**: Original sprite files (PNG, sprite sheets, etc.)
**Format**: Source format from TODO-sprites-init
**Size**: Small (only source assets)
**Eviction**: Never (part of application)

**Why**: Source of truth, versioned with application

### Layer 4: Generation (Future)
**Storage**: N/A (computed on demand)
**Contents**: Procedurally generated animations
**Why**: Infinite variety without disk space

**Note**: Layer 4 is out of scope for init, plan for future

---

## Cache Architecture

```rust
struct AnimationCache {
    // Layer 1: Hot cache
    hot: LruCache<AnimationId, LoadedAnimation>,

    // Layer 2: Disk cache manager
    disk: DiskCache,

    // Statistics
    stats: CacheStats,

    // Configuration
    config: CacheConfig,
}

struct LoadedAnimation {
    id: AnimationId,
    frames: Vec<Frame>,
    metadata: AnimationMetadata,
    loaded_at: Instant,
    last_used: Instant,
    use_count: u32,
}

struct CacheStats {
    hot_hits: u64,
    disk_hits: u64,
    cold_misses: u64,
    evictions: u64,
    memory_used: usize,
}
```

---

## Cache Operations

### Get Animation (Fast Path)
```
1. Check Layer 1 (hot cache)
   └─> Hit? Return immediately ✅
   └─> Miss? Continue to Layer 2

2. Check Layer 2 (disk cache)
   └─> Hit? Load, insert into Layer 1, return
   └─> Miss? Continue to Layer 3

3. Load from Layer 3 (cold storage)
   └─> Parse sprite files
   └─> Process into fast format
   └─> Write to Layer 2 (disk cache)
   └─> Insert into Layer 1 (hot cache)
   └─> Return
```

### Preload Animation (Background)
```
1. Check if already in Layer 1 → Skip
2. Check Layer 2 → Load into Layer 1
3. Otherwise, queue for background load from Layer 3
```

### Eviction Policy
```
When memory limit reached:
1. Identify LRU animations in hot cache
2. Keep at least N most recent (configurable, default: 5)
3. Evict until memory usage < threshold
4. Layer 2 remains untouched (persistent)
```

---

## Configuration

### Default Settings
```rust
struct CacheConfig {
    hot_cache_max_memory: usize, // Default: 50MB
    hot_cache_min_entries: usize, // Default: 5 (always keep)
    disk_cache_enabled: bool,      // Default: true
    disk_cache_path: PathBuf,      // Default: XDG_CACHE_HOME/yollayah
    preload_enabled: bool,         // Default: true
    preload_on_mood_change: bool,  // Default: true
}
```

### User Overrides (Environment Variables)
- `YOLLAYAH_CACHE_MEMORY=100M` - Set hot cache size
- `YOLLAYAH_CACHE_DISABLE=1` - Disable all caching (debug mode)
- `YOLLAYAH_CACHE_DIR=/path/to/cache` - Override cache location

---

## Predictive Preloading

### Mood-Based Preloading
When mood changes to "Thinking":
- Preload "Thinking" animation (if not already loaded)
- Preload likely next moods: "Happy", "Patient", "Waiting"
- Background load, don't block

### Conversation-Based Preloading
- At startup: Preload "Idle" and "Thinking" (most common)
- After user message: Preload "Happy" (likely completion state)
- After long wait: Preload "Bored" (staleness transition)

**Goal**: Next animation is already in Layer 1 when mood changes

---

## Implementation Phases

### Phase 1: Basic Hot Cache (This TODO)
1. Implement in-memory LRU cache
2. Load animations on demand
3. Basic eviction policy
4. Cache statistics tracking

### Phase 2: Disk Cache
1. Design binary cache format
2. Implement disk cache layer
3. Automatic cache population
4. Cache versioning (invalidate on app upgrade)

### Phase 3: Preloading
1. Implement background loading
2. Mood-based preloading
3. Predictive loading based on conversation state

### Phase 4: Advanced Features
1. Cache warming at startup
2. Intelligent eviction (mood-aware)
3. Cache compression
4. Telemetry and tuning

---

## Performance Targets

| Operation | Target Latency | Notes |
|-----------|----------------|-------|
| Get from hot cache | < 1μs | Pointer lookup |
| Get from disk cache | < 5ms | Disk read + deserialize |
| Get from cold storage | < 50ms | Parse + process + cache |
| Preload (background) | N/A | Async, doesn't block |
| Cache eviction | < 1ms | Fast LRU operation |

---

## Cache Statistics & Monitoring

### Metrics to Track
- **Hit Rate**: (hot_hits + disk_hits) / total_requests
- **Hot Hit Rate**: hot_hits / total_requests
- **Disk Hit Rate**: disk_hits / (total_requests - hot_hits)
- **Memory Usage**: Total bytes in hot cache
- **Eviction Count**: How often we run out of space
- **Preload Success Rate**: Preloaded animations that were used

### Debug Output
```
Animation Cache Stats:
  Hot cache: 15 animations, 32.5 MB
  Disk cache: 142 animations, 489 MB
  Hit rate: 94.3% (hot: 78.2%, disk: 16.1%)
  Evictions: 3
  Preload success: 89.7%
```

---

## Cache Invalidation

### When to Invalidate
1. **Application upgrade** - Version mismatch, invalidate disk cache
2. **Sprite changes** - Source file modified, invalidate cached version
3. **Manual clear** - User requests cache clear
4. **Corruption detected** - Cache file fails validation

### Invalidation Strategy
- Hot cache: Clear immediately, reload from disk/cold
- Disk cache: Delete affected files, regenerate on next access
- Never invalidate cold storage (source of truth)

---

## Disk Cache Format

### Directory Structure
```
~/.cache/yollayah/animations/
├── index.json           # Cache metadata
├── v1/                  # Cache version 1
│   ├── idle-001.anim    # Binary animation file
│   ├── thinking-001.anim
│   └── ...
└── stats.json           # Cache statistics
```

### Binary Format (`.anim` file)
```
Header (32 bytes):
  - Magic: "YOLA" (4 bytes)
  - Version: u32 (4 bytes)
  - Frame count: u32 (4 bytes)
  - Metadata offset: u64 (8 bytes)
  - Reserved: (12 bytes)

Frames:
  - Frame 1 data
  - Frame 2 data
  - ...

Metadata (JSON):
  - Animation ID
  - Mood
  - Duration
  - Staleness threshold
  - Creation timestamp
  - Source file hash
```

---

## Dependencies

- **Blocks**: None
- **Blocked by**: TODO-sprites-init.md (need animations to cache!)
- **Enables**: High-frequency animation changes, rich avatar personality

---

## Acceptance Criteria

- ✅ In-memory LRU cache implemented
- ✅ Cache hit rate > 80% in typical usage
- ✅ Animation retrieval < 1ms for hot cache hits
- ✅ Disk cache implemented with binary format
- ✅ Cache statistics tracking and logging
- ✅ Environment variable configuration working
- ✅ Cache invalidation on app upgrade
- ✅ Documentation for cache management

---

## Related Documents

- **Sprites**: `progress/active/TODO-sprites-init.md` - Sprite system foundation
- **Mood**: `progress/active/TODO-coherent-evolving-mood-system-init.md` - Mood system
- **Performance**: `progress/audits/PERFORMANCE-AUDIT-*.md` - Performance considerations

---

## Notes

- **Aggressive caching is the goal** - Don't be conservative, cache everything
- **Memory is cheap** - 50MB is nothing on modern systems, can go higher
- **Disk cache is persistent** - Survives restarts, makes cold starts fast
- **AJ shouldn't notice** - Cache should be invisible, just fast
- **Measure everything** - Cache stats guide tuning decisions

---

**Next Steps**: Review design, implement Phase 1 when TODO-sprites-init is complete, iterate based on performance measurements.
