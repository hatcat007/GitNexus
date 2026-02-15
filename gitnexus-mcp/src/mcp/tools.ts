/**
 * MCP Tool Definitions
 * 
 * Defines the tools that GitNexus exposes to external AI agents.
 * Each tool has a rich description with examples to help agents use them correctly.
 */

import { toolSchemas } from './schemas.js';

export interface ToolDefinition {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

export const GITNEXUS_TOOLS: ToolDefinition[] = [
  {
    name: 'context',
    description: `Get GitNexus codebase context. CALL THIS FIRST before using other tools.

Returns:
- Project name and stats (files, functions, classes)
- Hotspots (most connected/important nodes)
- Directory structure (TOON format for token efficiency)
- Tool usage guidance

OPTIONAL 'focus' parameter — request a specific section to reduce context window usage:
- 'stats': Only project statistics
- 'hotspots': Only hotspot nodes
- 'structure': Only folder tree
- 'tools': Only available tools list
- 'schema': Only graph schema

Omit 'focus' for full context. ALWAYS call this first to understand the codebase.`,
    inputSchema: toolSchemas.gitnexus_context,
  },
  {
    name: 'search',
    description: `Hybrid search (keyword + semantic) across the codebase.
Returns code nodes with their graph connections, grouped by process.

WHEN TO USE:
- Finding implementations ("where is auth handled?")
- Understanding code flow ("what calls UserService?")
- Locating patterns ("find all API endpoints")

RETURNS: Array of {name, type, filePath, code, connections[], cluster, processes[]}`,
    inputSchema: toolSchemas.gitnexus_search,
  },
  {
    name: 'cypher',
    description: `Execute Cypher query against the code knowledge graph.

SCHEMA:
- Nodes: File, Folder, Function, Class, Interface, Method, Community, Process
- Edges via CodeRelation.type: CALLS, IMPORTS, EXTENDS, IMPLEMENTS, CONTAINS, DEFINES, MEMBER_OF, STEP_IN_PROCESS

EXAMPLES:
• Find callers of a function:
  MATCH (a)-[:CodeRelation {type: 'CALLS'}]->(b:Function {name: "validateUser"}) RETURN a.name, a.filePath

• Find all functions in a community:
  MATCH (f:Function)-[:CodeRelation {type: 'MEMBER_OF'}]->(c:Community {label: "Auth"}) RETURN f.name

• Find steps in a process:
  MATCH (s)-[r:CodeRelation {type: 'STEP_IN_PROCESS'}]->(p:Process {label: "UserLogin"}) RETURN s.name, r.step ORDER BY r.step

TIPS:
- All relationships use CodeRelation table with 'type' property
- Community = functional cluster detected by Leiden algorithm
- Process = execution flow trace from entry point to terminal`,
    inputSchema: toolSchemas.gitnexus_cypher,
  },
  {
    name: 'grep',
    description: `Regex search for exact patterns in file contents.

WHEN TO USE:
- Finding exact strings: error codes, TODOs, specific API keys
- Pattern matching: all console.log, all fetch calls
- Finding imports of specific modules

BETTER THAN search for: exact matches, regex patterns, case-sensitive

RETURNS: Array of {filePath, line, lineNumber, match}`,
    inputSchema: toolSchemas.gitnexus_grep,
  },
  {
    name: 'read',
    description: `Read file content from the codebase.

WHEN TO USE:
- After search/grep to see full context
- To understand implementation details
- Before making changes

ALWAYS read before concluding - don't guess from names alone.

RETURNS: {filePath, content, language, lines}`,
    inputSchema: toolSchemas.gitnexus_read,
  },
  {
    name: 'explore',
    description: `Deep dive on a symbol, cluster, or process.

TYPE: symbol | cluster | process

For SYMBOL: Shows cluster membership, process participation, callers/callees
For CLUSTER: Shows members, cohesion score, processes touching it
For PROCESS: Shows step-by-step trace, clusters traversed, entry/terminal points

Use after search to understand context of a specific node.`,
    inputSchema: toolSchemas.gitnexus_explore,
  },
  {
    name: 'overview',
    description: `Get codebase map showing all clusters and processes.

Returns:
- All communities (clusters) with member counts and cohesion scores
- All processes with step counts and types (intra/cross-community)
- High-level architectural view

Use to understand overall codebase structure before diving deep.`,
    inputSchema: toolSchemas.gitnexus_overview,
  },
  {
    name: 'impact',
    description: `Analyze the impact of changing a code element.
Returns all nodes affected by modifying the target, with distance, edge type, and confidence.

USE BEFORE making changes to understand ripple effects.

Output includes:
- Affected processes (with step positions)
- Affected clusters (direct/indirect)
- Risk assessment (critical/high/medium/low)
- Callers/dependents grouped by depth

EdgeType: CALLS, IMPORTS, EXTENDS, IMPLEMENTS
Confidence: 100% = certain, <80% = fuzzy match

Depth groups:
- d=1: WILL BREAK (direct callers/importers)
- d=2: LIKELY AFFECTED (indirect)
- d=3: MAY NEED TESTING (transitive)`,
    inputSchema: toolSchemas.gitnexus_impact,
  },
  {
    name: 'graph_action',
    description: `Drive the GitNexus graph visualization. Replaces the old 'highlight' tool.

ACTIONS:
- 'highlight': Make nodes glow in the graph view (visual confirmation of findings).
- 'focus': Pan and zoom the graph to center on specific nodes.
- 'annotate': Attach a text label to nodes in the graph (e.g., "entry point", "bug here").
- 'reset': Clear all highlights, focus, and annotations.

'nodeIds' is required for highlight, focus, and annotate.
'color' is optional (hex or named color) for highlight.
'label' is optional text for annotate.

Great for visual storytelling during code exploration.`,
    inputSchema: toolSchemas.gitnexus_graph_action,
  },

  // ── New Tools ──────────────────────────────────────────────

  {
    name: 'diff',
    description: `Compare current codebase index with previous version.
Shows added/modified/deleted files since the last reindex.

WHEN TO USE:
- After a reindex to understand what changed
- To review recent code changes
- Before deep analysis to scope investigation

RETURNS: { added[], modified[], deleted[], unchanged: number, summary }
Use 'filter' to narrow: 'added', 'modified', or 'deleted'.
Set 'includeContent: true' for line-level diffs of modified files (slower).`,
    inputSchema: toolSchemas.gitnexus_diff,
  },
  {
    name: 'deep_dive',
    description: `Complete analysis of a code symbol in ONE call.
Combines explore + impact + read into a single response.

Internally runs:
1. Explore symbol → cluster membership, process participation, connections
2. Impact analysis → downstream affected nodes, risk level
3. Read source → actual code

MUCH faster than calling explore, impact, read separately.

RETURNS: { symbol, source, impact, connections }`,
    inputSchema: toolSchemas.gitnexus_deep_dive,
  },
  {
    name: 'review_file',
    description: `Full review context for a file in ONE call.
Everything you need to understand a file: content, symbols, dependencies, cluster, processes.

Internally runs:
1. Read file content
2. Query all symbols defined in the file
3. Find cluster membership and processes
4. Find imports and reverse-imports

RETURNS: { file, content, symbols[], cluster, processes[], dependencies, complexity }`,
    inputSchema: toolSchemas.gitnexus_review_file,
  },
  {
    name: 'trace_flow',
    description: `Trace execution path between two symbols, or from an entry point.

If 'to' is provided: finds shortest path from → to in the call graph.
If only 'from': finds all processes containing that symbol, returns traces.

Each step includes the code snippet for context.

RETURNS: { paths: [{ steps: [{ name, type, filePath, code }] }] }`,
    inputSchema: toolSchemas.gitnexus_trace_flow,
  },
  {
    name: 'find_similar',
    description: `Find structurally similar code to a given symbol.
Uses cluster membership and connection patterns to find related symbols.

WHEN TO USE:
- Finding duplicate/redundant implementations
- Understanding patterns in the codebase
- Finding candidates for refactoring/extraction

RETURNS: { target, similar: [{ name, type, filePath, sharedCluster, sharedConnections, similarity }] }`,
    inputSchema: toolSchemas.gitnexus_find_similar,
  },
  {
    name: 'test_impact',
    description: `Risk assessment for a set of file changes (like a PR).
Traces graph dependencies to find ALL affected code and compute a risk score.

WHEN TO USE:
- Before merging a PR
- Planning refactoring scope
- Understanding blast radius of changes

RETURNS:
- riskScore: 0-100
- riskLevel: critical | high | medium | low
- affectedProcesses: processes that touch changed files
- affectedClusters: communities impacted
- impactedFiles: files depending on changed files (by depth)
- suggestedTests: test files related to impacted code
- summary: human-readable risk assessment

Also highlights all affected nodes in the graph visualization.`,
    inputSchema: toolSchemas.gitnexus_test_impact,
  },
];
