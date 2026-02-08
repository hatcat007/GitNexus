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

ALWAYS call this first to understand the codebase before searching or querying.`,
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
    name: 'highlight',
    description: `Highlight nodes in the GitNexus graph visualization.
Use after search/analysis to show the user what you found.

The user will see the nodes glow in the graph view.
Great for visual confirmation of your findings.`,
    inputSchema: toolSchemas.gitnexus_highlight,
  },
];
