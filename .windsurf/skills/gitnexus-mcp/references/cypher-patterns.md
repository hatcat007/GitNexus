# Cypher Patterns for GitNexus (KuzuDB)

GitNexus uses KuzuDB, not Neo4j. Key syntax differences are noted below.

## KuzuDB Gotchas

- Use `label(n)` not `labels(n)` — KuzuDB returns a single label string, not an array
- Array slicing: `list[0:10]` (Python-style), not `list[..10]`
- Variable-length paths: `(a)-[:CodeRelation*1..3]->(b)` — supported, but property filters on variable-length edges are NOT supported (e.g., `[:CodeRelation*1..3 {type: 'CALLS'}]` will error)
- No `labels()` function — use `label()` instead
- No `apoc` procedures — use native Cypher only
- `CALL` keyword is blocked by the sanitizer (security) — no stored procedures
- Always add `LIMIT` — KuzuDB can OOM on unconstrained traversals
- Multiple `OPTIONAL MATCH` in sequence creates cartesian products — split into separate queries and combine results

## Edge Table

All relationships use a single table: `CodeRelation`

```cypher
-- Edge type is a PROPERTY, not a label
[:CodeRelation {type: 'CALLS'}]

-- NOT this (wrong):
[:CALLS]
```

## Common Queries

### Find callers of a function
```cypher
MATCH (caller)-[:CodeRelation {type: 'CALLS'}]->(fn:Function {name: 'validateUser'})
RETURN caller.name, label(caller) AS type, caller.filePath
LIMIT 50
```

### Find all imports of a file
```cypher
MATCH (f:File)-[:CodeRelation {type: 'IMPORTS'}]->(target:File)
WHERE target.name = 'utils.ts'
RETURN f.name, f.filePath
LIMIT 50
```

### Class inheritance tree
```cypher
MATCH (child:Class)-[:CodeRelation {type: 'EXTENDS'}]->(parent:Class)
RETURN child.name AS child, parent.name AS parent
LIMIT 50
```

### Classes implementing an interface
```cypher
MATCH (c:Class)-[:CodeRelation {type: 'IMPLEMENTS'}]->(i:Interface)
RETURN c.name, i.name
LIMIT 50
```

### All members of a cluster (community)
```cypher
MATCH (m)-[:CodeRelation {type: 'MEMBER_OF'}]->(c:Community {label: 'Auth'})
RETURN m.name, label(m) AS type, m.filePath
LIMIT 50
```

### Steps in a process (execution flow)
```cypher
MATCH (s)-[r:CodeRelation {type: 'STEP_IN_PROCESS'}]->(p:Process)
WHERE p.label CONTAINS 'login'
RETURN p.label, s.name, r.step, label(s) AS type
ORDER BY r.step
LIMIT 50
```

### Count functions per file
```cypher
MATCH (file:File)-[:CodeRelation {type: 'DEFINES'}]->(fn:Function)
RETURN file.name, COUNT(fn) AS functionCount
ORDER BY functionCount DESC
LIMIT 20
```

### Find fuzzy/inferred edges (low confidence)
```cypher
MATCH (a)-[r:CodeRelation]->(b)
WHERE r.confidence IS NOT NULL AND r.confidence < 0.8
RETURN a.name, r.type, b.name, r.confidence, r.reason
ORDER BY r.confidence ASC
LIMIT 30
```

### Cross-cluster dependencies
```cypher
MATCH (a)-[:CodeRelation {type: 'MEMBER_OF'}]->(ca:Community)
MATCH (b)-[:CodeRelation {type: 'MEMBER_OF'}]->(cb:Community)
MATCH (a)-[:CodeRelation {type: 'CALLS'}]->(b)
WHERE ca.label <> cb.label
RETURN ca.label AS from_cluster, cb.label AS to_cluster, COUNT(*) AS calls
ORDER BY calls DESC
LIMIT 20
```

### Find all connections of a symbol (incoming + outgoing)
```cypher
MATCH (n:Function {name: 'handleRequest'})
OPTIONAL MATCH (n)-[r1:CodeRelation]->(dst)
WITH n, collect(DISTINCT {name: dst.name, type: r1.type}) AS outgoing
OPTIONAL MATCH (src)-[r2:CodeRelation]->(n)
RETURN outgoing, collect(DISTINCT {name: src.name, type: r2.type}) AS incoming
LIMIT 1
```

Note: Split the two OPTIONAL MATCHes with `WITH` to avoid cartesian product explosion.

### Multi-hop traversal (2 hops)
```cypher
MATCH (target:Function {name: 'handleRequest'})
MATCH (target)-[:CodeRelation {type: 'CALLS'}]->(a)
MATCH (a)-[:CodeRelation {type: 'CALLS'}]->(b)
WHERE b.id <> target.id
RETURN DISTINCT b.name, label(b) AS type, b.filePath
LIMIT 50
```

For deeper traversals (3+ hops), prefer the `impact` tool which handles deduplication and risk scoring automatically.

## Forbidden Keywords (Sanitizer)

The following are blocked: CREATE, MERGE, DELETE, DETACH, DROP, SET, REMOVE, CALL, LOAD, CSV, FOREACH, USING, INDEX, CONSTRAINT, DATABASE, USER, ROLE, GRANT, REVOKE, DENY, SHOW, START, STOP, ALTER, RENAME.

Only read-only clauses are allowed: MATCH, RETURN, WHERE, WITH, ORDER BY, LIMIT, SKIP, OPTIONAL, CASE/WHEN/THEN/ELSE/END, DISTINCT, UNWIND, and aggregate functions (COUNT, SUM, AVG, MIN, MAX, COLLECT).
