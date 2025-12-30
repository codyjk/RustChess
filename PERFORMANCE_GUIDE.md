# Performance Profiling and Optimization Guide

This guide provides a systematic approach to profiling and optimizing codepaths in the chess engine. Use this as a template when optimizing any component.

## Profiling Tools

### CPU Profiling (Text-Based)

**macOS:**
```bash
chess count-positions --depth 6 2>&1 &
CHESS_PID=$!
sleep 1
sample $CHESS_PID 30 -file /tmp/profile.txt
wait $CHESS_PID
tail -n 100 /tmp/profile.txt  # See "Sort by top of stack" summary
```

**Linux:**
```bash
chess count-positions --depth 6 2>&1 &
CHESS_PID=$!
sleep 1
perf record -p $CHESS_PID sleep 30
perf report
```

Analyze the "Sort by top of stack" section to identify hot functions.

### Memory Profiler (Built-In)

The engine includes instrumentation for tracking allocations:
- Automatically tracks board clones and MoveGenerator allocations
- Output shown after `count-positions` command
- Add custom counters using `MemoryProfiler::record_*()` methods in `src/diagnostics/memory_profiler.rs`

### Visual Profiling

**Flamegraph:**
```bash
sudo cargo flamegraph --bench pvp_benchmark
```

## Optimization Workflow

### 1. Establish Baseline

Measure performance before making changes:
```bash
chess count-positions --depth 5 | tail -n 1
```

Record key metrics: positions/second, duration, memory profiler stats.

**Quick feedback loops are essential**: Use shallow depths (depth 4-5) during development for fast iteration. Deep profiling (depth 6+) should be reserved for final validation, as it takes significantly longer. The `count-positions` command provides immediate feedback on performance changes, allowing you to iterate quickly and validate optimizations before investing time in comprehensive profiling.

### 2. Profile to Identify Bottlenecks

Use CPU profiling to find:
- Thread synchronization overhead (e.g., `__psynch_cvwait` on macOS)
- Excessive allocations (board clones, object creations)
- Algorithmic bottlenecks (not just hot functions)

### 3. Focus on Algorithmic Improvements

Prioritize optimizations that:
- **Reduce operation counts**: Eliminate redundant allocations, avoid unnecessary cloning
- **Improve parallelization strategy**: Multi-depth parallelization, conditional cloning
- **Share resources**: Pass references instead of creating new instances

### 4. Measure After Each Change

**Critical:** Always verify correctness before measuring performance:
```bash
cargo test  # Must pass before measuring
chess count-positions --depth 5 | tail -n 1
```

Compare results to baseline and verify position counts match exactly.

**Maintain quick feedback cycles**: After each optimization, run tests and a quick performance check. This allows you to:
- Catch regressions immediately
- Validate that optimizations actually improve performance
- Iterate rapidly without waiting for long-running benchmarks
- Build confidence before investing in deeper profiling

Only run comprehensive profiling (depth 6+, full CPU profiling) after you've validated improvements with quick checks.

## Key Lessons

- **High sample counts ≠ slow functions**: Functions called frequently may show high counts but be fast
- **Compiler optimizations**: Modern compilers with LTO already optimize many micro-operations
- **Real bottlenecks**: Often in resource allocation patterns, parallelization strategy, or algorithmic complexity
- **Instrumentation is essential**: Add counters to track allocations, clones, and operation counts
- **System variability**: ±10% variation is normal; measure multiple times for small improvements

