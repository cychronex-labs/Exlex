# Exlex 
> **STATUS: AT BETA STAGE** > *Readme is still a work in progress.*

Exlex is a config parser I built on 3 core rules:
1. **No structs inside an array**
2. **No Vectors inside Vectors**
3. **Minimize string copying to the absolute minimum** (I believe I am not copying strings at all inside the core of Exlex).

**Why?** Because it makes Exlex incredibly cache-friendly. When the total number of properties in a section isn't massive, a flat linear search heavily outperforms a HashMap or BTree due to memory contiguity.

## Syntax
```exl
# Comments were originally created so I can do some debugging
# All literals must be quoted!
"name": "Exlex"
"version": "1.0.0"

# A section can carry multiple properties and also supports nesting 
sect "Server" {
    "host": "127.0.0.1"
    "port": "8080"
}

sect "Database" {
    "driver": "postgres"
    "pool": "32"

    sect "ClientDB" {
        "host": "0.0.1"
        "port": "0980"
    }
    sect "LoremIpsum" {
        "user": "user1"
        "auth": "userauth"
    }
    sect "Credentials" {
        "user": "sys_admin"
        "auth": "ed25519"
    }
}

sect "Client" {
    "host": "127.0.0.1"
    "port": "8080"
}
```

## Pros and Cons

### Pros

  - **Data-Oriented Design & Cache friendliness** (TLB Miss was 0.07% and IPC was 1.7)
  - Support for `no_std`
  - Extremely low memory usage
  - SIMD acceleration on specific portions (Not the entire parser like `sonic-rs` or `simdjson` because that is too complex for me right now)
  - Minimalistic syntax (modified specifically for maximum parsing performance)
  - Lock-free data mutation via a separate arena
  - High-performance Parsing and Writing

### Cons

  - **No inbuilt Datatype support:** I am trying to keep it minimalistic. I feel it's not worth adding that overhead directly into the parser, but the interface does provide ways to handle it.
  - **No direct array support:** There is a workaround currently, but I am planning to implement native arrays later.

## Rules

To retain high performance for config files, the following rules are imposed by the parser:

  - Quotes are enforced on all literals.
  - All properties must be defined *before* defining a nested section in a scope.

-----

## Benchmarks

**Benchmark Repo:** [Exlex-Benchmark](https://github.com/cychronex-labs/Exlex-Benchmark)

*(Note: I wrote the core parser myself, but heavily utilized AI to help design and write this Benchmark Methodology).*

### 📊 Benchmark Methodology

To ensure `exlex` performs consistently across all use cases, the benchmark suite tests both **Operations** (what we do to the data) and **Topologies** (the physical shape of the data).

#### 1\. The Operations

  * **Parse Init:** Raw ingestion throughput. Measures how fast the engine converts a string into the flat-array DOD structure.
  * **Cold Start:** Measures parse + section traverse + first key read as one atomic operation. Represents every application's startup sequence.
  * **Lookup Flat:** Key retrieval speed within a single section, testing both worst-case linear scans and hash-collision resolution.
  * **Lookup Nested:** Path traversal speed. Tests the cost of navigating down deeply nested section hierarchies.
  * **Iteration Drain:** Sequential reading speed. Proves the cache-locality advantage of flat arrays by iterating over every property in a section.
  * **Mutation Arena:** The speed of editing memory (Add, Update, Delete). Tests the performance of the lock-free, append-only arena.
  * **Mixed Workload:** Simulates real-world server config hot-reloading (70% reads / 30% writes interleaved). Tests the engine's ability to mutate and query simultaneously without allocator thrashing.
  * **Roundtrip Pipeline:** A real-world lifecycle test. Times the full process of parsing a file, executing mass updates, and saving it back to a string.
  * **Allocation Audit (DHAT):** Hooks into the global memory allocator to measure the exact byte footprint and `malloc` call count, mathematically proving the zero-copy claims.
  * **Asymptotic Crossover:** The algorithmic threshold test. Determines the exact property count where Exlex's $O(N)$ linear scan loses its L1 cache advantage and is overtaken by a traditional $O(1)$ HashMap. (Around 65-70 on Intel Core i3 6006U).

#### 2\. The Data Topologies

Data shape heavily impacts CPU caching. We test against several mathematically generated shapes:

  * **Bushy Normal:** A balanced mix of depth and breadth (e.g., 3 levels deep, 5 branches per level, 10 properties each). This mimics a standard, real-world server or game config.
  * **Flat Dense:** Shallow sections, but packed with hundreds of properties. This explicitly stress-tests linear array scanning and cache prefetching.
  * **Flat Sparse:** Many shallow sections with only 1 or 2 properties each.
  * **Deep Sparse:** Extremely nested hierarchies (e.g., a section inside a section inside a section, 8 levels deep). This stresses pointer/path resolution.
  * **Wide Sparse:** Hundreds of parallel sections at the root level, simulating a massive directory-style config.
  * **Lopsided Dense:** One massive, heavy section sitting next to dozens of completely empty sections. This tests memory allocation and edge-case scaling.

### Results

![Exlex Benchmark: ParseInit](assets/ParseInit.png)

![Exlex Benchmark: ColdStart](assets/ColdStart.png)
> **Cold Start:** Measures parse + section traverse + first key read as one atomic operation. Represents every application's startup sequence. All parsers build a queryable structure from scratch per iteration.

![Exlex Benchmark: LookupFlat](assets/LookupFlat.png)

![Exlex Benchmark: LookupNested](assets/LookupNested.png)

![Exlex Benchmark: MutationArena](assets/MutationArena.png)

![Exlex Benchmark: MixedWorkload](assets/MixedWorkload.png)
> **Mixed Workload:** 70% reads / 30% writes interleaved on the same section. *Note: Serde JSON column shows read-only cost (it has no mutation API). TOML Edit clones the document per iteration (its architecture cost). Exlex reuses the immutable parsed core and resets only the lightweight mutator overlay.*

![Exlex Benchmark: IterationDrain](assets/IterationDrain.png)

![Exlex Benchmark: RoundTrip](assets/RoundTrip.png)

![Exlex Benchmark: AsymptoticCrossover](assets/AsymptoticCrossover.png)

![Exlex Benchmark: HeapFootprint](assets/HeapFootprint.png)

![Exlex Benchmark: HeapAllocations](assets/HeapAllocations.png)

### 🚀 Performance & Memory Summary

| Benchmark Category | Scenario | Exlex (DOD) | Industry Standard | Speedup / Diff |
| :--- | :--- | :--- | :--- | :--- |
| **Ingestion (Parse)** | Lopsided Dense | **29,579 ns** | 163,179 ns (Serde JSON) | **~5.5x Faster** |
| **Cold Start** | Parse+Traverse+Read (Bushy) | **221,089 ns** | 854,230 ns (Serde JSON) | **~3.8x Faster** |
| **Mutation (Delete)** | Section Removal | **496 ns** | 619,245 ns (TOML Edit) | **~1,247x Faster** |
| **Mixed Workload** | 70% Read / 30% Write | **1,194 ns** | 646,444 ns (TOML Edit) | **~541x Faster** |
| Lookup (Single) |	Dense Last (100 items) | 211 ns |	173 ns (Serde JSON) |	~1.2x Slower |
| Lookup (Nested)	| Path Depth 6 | 249 ns |	160 ns (Serde JSON) |	~1.5x Slower |
| **Iteration** | Section Drain | **46 ns** | 96 ns (Serde JSON) | **~2.0x Faster** |
| **Roundtrip** | Parse+Mutate+Save | **391,534 ns** | 4,594,995 ns (TOML Edit) | **~11.7x Faster** |
| **Memory Footprint** | Parse Init (DHAT) | **84 KB** | 2.7 MB (TOML Edit) | **~32x Smaller** |
| **Heap Allocations** | Parse Init (DHAT) | **16 calls** | 13,445 calls (TOML Edit) | **~840x Fewer** |
| **Heap Allocations** | Mutation (DHAT) | **13 calls** | 15,000 calls (TOML Edit) | **~1,153x Fewer** |

### Perf_stat output 
I measured the hardware execution on an **Intel i3-6006U (Skylake, 2C/4T, 2.0GHz)**:
* **Instructions Per Cycle (IPC):** **1.7**. This confirms the CPU's pipeline is nearly always fed and rarely waiting for memory stalls.
* **L1 Cache Locality:** By using flat parallel vectors instead of a standard node tree, I achieved a very high cache hit rate, mathematically evidenced by the **0.07% TLB miss rate**.

"These metrics were captured using `perf stat` on Linux. I'm focusing on Data-Oriented Design to maximize hardware efficiency, and these numbers represent the initial results of that approach."

### The Naming
**Exlex** - Comes from latin for **"Lawless"**. 
**Reasons**: 
- DOD-based parsers usually don't offer data mutation; I do, and it is exceptionally fast.
- Standard parsers default to HashMaps, but I use Linear Search. In the domain of configuration parsing, sacrificing baseline performance just to support massive property counts in a single section is not ideal. For contiguous arrays of numbers, a linear search on a modern processor is incredibly fast.
