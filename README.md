Exlex is built as educational project to test how much a DOD-based human readable configuration can achieve. its current features are:
- Zero copy immutable parser
- Native no_std support
- SIMD byte search via memchr on specific functions
- Supports modifying data and dumping it back into string (Arena mutator)
- Human readable format
- Low memory usage even on mutations

### Performance benchmarking
Some extra benchmark data used for testing purposes is available in Benchmarks.html file
Due to lack of 1:1 direct competitor, I was forced to use toml_edit and others for baseline.

Note on Mutation Overhead: In the mutation graphs, Exlex takes a slight initial hit due to the preallocation of the String arena. However, on heavy or continuous mutation workloads.But scales significantly better and keeps allocations low.

![HeapFootprint.png](assets/HeapFootprint.png)
Exlex
![MixedWorkload.png](assets/MixedWorkload.png)

_AI was strictly restricted to generating the benchmarking and testing suites. This allowed me to focus on hand-coding the actual parser and memory layout without spending days writing fuzzing boilerplate_

### Tradeoffs
- Currently the tradeoffs are slower retrieval compared to hashmap, when a section have more than 65 sections and 65 properties (Having lot of Properties won't affect speed of section retreival!)
- A need to follow rules of syntax (Performance above convenience)
- No inbuilt datatypes but arrays with string literal elements maybe achievable without sacrificing performance. 

## What it is and What it does not aim to be:
- Built for hardware constraint environment.
- Built to be Cache-friendly and Memory friendly as much as it can.
- Built for overall speed in lifecycle of a program (Parse -> Read -> Mutate -> Save).
- Syntax specifically designed to make parser fast while maintaining human readability
- It is NOT a feature rich or highly flexible syntax (Use json or toml if you need dynamic typing or complex data structures).

## Issues
- Still in work in progress! (Expect bugs)
- Interface has a lot of work to do
- Docs are incomplete!
- Due to zero-copy architecture escape character has to be implemented

## Parser and Mutator Stability
- Proptested 
- Test file available at Exlex-Benchmark repo
```bash
~/Projects/exlex_bench main*
❯ PROPTEST_CASES=10000 cargo test proptest_mutator_engine --release

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/proptest_fuzz.rs (target/release/deps/proptest_fuzz-6b0455884b22cdc2)

running 1 test
test proptest_mutator_engine ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 5.87s
```

## Quick Start (Rust API)
Exlex splits its lifecycle into two different parts: an immutable, zero-copy Reader and an allocation efficient Mutator.

```rust
use exlex::{Exlex, ExlexArena};

fn main() {
    let config_data = r#"
    "name": "Exlex"
    "version": "1.0.0"

    sect "Server" {
        "host": "127.0.0.1"
        "port": "8080"
    }
    "#;

    // 1. Initialize the Zero-Copy Reader
    let exlex = Exlex::init_reader(config_data, None, None, None, None).unwrap();
    let root = exlex.get_root();

    // 2. Read Properties
    let name = exlex.get_property("name", root).unwrap();
    println!("Name: {}", name);

    // Retrieve typed properties from a nested section
    let server_sect = exlex.get_child("Server", root).unwrap();
    
    let port: u16 = exlex.get_property_as("port", server_sect).unwrap();
    
    // OR 
    // As Server is first defined section (0 is reserved as ROOT)
    // let server_sect = ExlexSection(1);
    // Benefit of this method, Completely overrides Linear Search of Section getting O(1) retrieval for sections
    // let port: u16 = exlex.get_property_as("port", server_sect).unwrap();


    println!("Port: {}", port);

    // 3. Mutate Data (Arena-based)
    let mut arena = ExlexArena(String::new());
    let mut write_buffer = String::new();
    
    let mut mutator = exlex.init_mutator(&mut arena, &mut write_buffer).unwrap();
    
    // Update existing property
    mutator.update_prop("port", "9000", server_sect);
    
    // Add a new section and property
    mutator.new_section("Database", root).unwrap();
    let db_sect = exlex.get_child("Database", root).unwrap(); 
    mutator.update_prop("driver", "postgres", db_sect);

    // 4. Save to String
    mutator.save();
    println!("Updated Config:\n{}", write_buffer);
}
```

## Syntax
```exl
# Comments were originally created so I can do some debugging
# All literals must be quoted!
"name": "Exlex"
"version": "1.0.0"

# A section can carry multiple properties and also supports nesting 
# Each user defined structure in top to down order gets +1 section id that it can be used directly via ExlexSection(<id>)

# Section id: 1
sect "Server" {
    "host": "127.0.0.1"
    "port": "8080"
}

# Section id: 2 
sect "Database" {
    "driver": "postgres"
    "pool": "32"

    # Section id: 3
    sect "ClientDB" {
        "host": "0.0.1"
        "port": "0980"
    }
    
    # Section id: 4
    sect "LoremIpsum" {
        "user": "user1"
        "auth": "userauth"
    }
    
    # Section id: 5
    sect "Credentials" {
        "user": "sys_admin"
        "auth": "ed25519"
    }
}

# Section id: 6
sect "Client" {
    "host": "127.0.0.1"
    "port": "8080"
}
```

## Rules

To retain high performance for config files, the following rules are imposed by the parser:
  - Quotes are enforced on all literals.
  - All properties must be defined *before* defining a nested section in a scope.

## Benchmark repo
There is a seperate repository that contains the boilerplate of criterion and proptest code if you want to check you can visit: https://github.com/cychronex-labs/Exlex-Benchmark
