# Exlex 
> **STATUS: AT ALPHA STAGE**

Exlex is a config parser I built on just 2 rules:
1. **No structs inside an array**
2. **NO Vectors inside vectors**
3. **Minimize copying of strings** to absolute minimum [I believe I am not copying strings at all inside Exlex]
**Why?** Because it makes veloxia more cache friendly, On my processor (Intel core i3 6006U), I was able put around 40 properties in a single section and linear search is faster than Hashmap and Btree. But if you want more properties you can always make more sections.

## Syntax
```vml
# Comments originally created so I can kind of simulate the state of vector in vml (Velox Markup Language) file

# Property keys are strictly unquoted and can contain only a-z, A-Z, 1-0 and underscores
# and only quoted literals are allowed to have any other symbols
name: "Exlex"
version: "1.0.0"

# Section, A section can carry multiple properties and it also supports nesting 
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
### RULES
To retain high performance for config files the following rules are imposed:
- Quotes are enforced on all literals
- Unicodes are not supported (Can lead to Data corruption in current state)
- All properties must be defined before defining a section in a scope
