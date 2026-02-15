import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';

/**
 * Zod Schemas for GitNexus MCP Tools
 * 
 * All tool inputs are validated against these schemas before processing.
 * Uses zod-to-json-schema to convert for MCP protocol compatibility.
 */

// ============================================
// Individual Tool Schemas
// ============================================

/**
 * Context tool - returns codebase context
 * Supports an optional 'focus' parameter for adaptive context management.
 */
export const ContextSchema = z.object({
    focus: z.enum(['stats', 'hotspots', 'structure', 'tools', 'schema'], {
        errorMap: () => ({ message: 'Focus must be one of: stats, hotspots, structure, tools, schema' }),
    }).optional().describe('Optional section to focus on. Omit for full context.'),
});

/**
 * Search tool - hybrid search across codebase
 */
export const SearchSchema = z.object({
    query: z.string().min(1, 'Query cannot be empty'),
    limit: z.number().int().min(1).max(100).default(10),
    groupByProcess: z.boolean().default(true),
});

/**
 * Cypher tool - execute Cypher queries
 */
export const CypherSchema = z.object({
    query: z.string().min(1, 'Query cannot be empty'),
});

/**
 * Grep tool - regex search in file contents
 */
export const GrepSchema = z.object({
    pattern: z.string().min(1, 'Pattern cannot be empty'),
    caseSensitive: z.boolean().default(false),
    maxResults: z.number().int().min(1).max(500).default(50),
});

/**
 * Read tool - read file content
 */
export const ReadSchema = z.object({
    filePath: z.string().min(1, 'File path cannot be empty'),
    startLine: z.number().int().min(1).optional(),
    endLine: z.number().int().min(1).optional(),
}).refine(
    (data) => {
        if (data.startLine !== undefined && data.endLine !== undefined) {
            return data.endLine >= data.startLine;
        }
        return true;
    },
    { message: 'endLine must be greater than or equal to startLine' }
);

/**
 * Explore tool - deep dive on symbol/cluster/process
 */
export const ExploreSchema = z.object({
    name: z.string().min(1, 'Name cannot be empty'),
    type: z.enum(['symbol', 'cluster', 'process'], {
        errorMap: () => ({ message: 'Type must be one of: symbol, cluster, process' }),
    }),
});

/**
 * Overview tool - get codebase map
 */
export const OverviewSchema = z.object({
    showProcesses: z.boolean().default(true),
    showClusters: z.boolean().default(true),
    limit: z.number().int().min(1).max(100).default(20),
});

/**
 * Impact tool - analyze change impact
 */
export const ImpactSchema = z.object({
    target: z.string().min(1, 'Target cannot be empty'),
    direction: z.enum(['upstream', 'downstream'], {
        errorMap: () => ({ message: 'Direction must be upstream or downstream' }),
    }),
    maxDepth: z.number().int().min(1).max(10).default(3),
    relationTypes: z.array(z.string()).optional(),
    includeTests: z.boolean().default(false),
    minConfidence: z.number().min(0).max(1).default(0.7),
});

/**
 * Highlight tool - highlight nodes in graph visualization
 */
export const HighlightSchema = z.object({
    nodeIds: z.array(z.string().min(1)).min(1, 'At least one node ID is required'),
    color: z.string().optional(),
});

// ============================================
// New Tools: diff, composites, test_impact
// ============================================

/**
 * Diff tool - compare current index with previous
 */
export const DiffSchema = z.object({
    baseline: z.string().default('last_index'),
    includeContent: z.boolean().default(false),
    filter: z.enum(['all', 'added', 'modified', 'deleted']).default('all'),
});

/**
 * Deep Dive tool - complete analysis of a symbol in one call
 */
export const DeepDiveSchema = z.object({
    name: z.string().min(1, 'Symbol name cannot be empty'),
});

/**
 * Review File tool - full review context for a file
 */
export const ReviewFileSchema = z.object({
    filePath: z.string().min(1, 'File path cannot be empty'),
});

/**
 * Trace Flow tool - trace execution path between symbols
 */
export const TraceFlowSchema = z.object({
    from: z.string().min(1, 'Source symbol cannot be empty'),
    to: z.string().optional(),
    maxSteps: z.number().int().min(1).max(20).default(10),
});

/**
 * Find Similar tool - find structurally similar code
 */
export const FindSimilarSchema = z.object({
    name: z.string().min(1, 'Symbol name cannot be empty'),
    limit: z.number().int().min(1).max(20).default(5),
});

/**
 * Test Impact tool - risk assessment for file changes
 */
export const TestImpactSchema = z.object({
    changedFiles: z.array(z.string().min(1)).min(1, 'At least one changed file is required'),
    maxDepth: z.number().int().min(1).max(5).default(2),
    suggestTests: z.boolean().default(true),
});

/**
 * Graph Action tool - drive the GitNexus visualization
 */
export const GraphActionSchema = z.object({
    action: z.enum(['highlight', 'focus', 'annotate', 'reset'], {
        errorMap: () => ({ message: 'Action must be one of: highlight, focus, annotate, reset' }),
    }),
    nodeIds: z.array(z.string().min(1)).optional().describe('Node IDs to target (required for highlight/focus/annotate)'),
    color: z.string().optional().describe('Highlight color (hex or named)'),
    label: z.string().optional().describe('Annotation label text for annotate action'),
}).refine(
    (data) => {
        if (['highlight', 'focus', 'annotate'].includes(data.action) && (!data.nodeIds || data.nodeIds.length === 0)) {
            return false;
        }
        return true;
    },
    { message: 'nodeIds is required for highlight, focus, and annotate actions' }
);

// ============================================
// Schema Registry and Validation
// ============================================

/**
 * Map of tool names to their Zod schemas
 */
export const toolSchemaMap = {
    gitnexus_context: ContextSchema,
    gitnexus_search: SearchSchema,
    gitnexus_cypher: CypherSchema,
    gitnexus_grep: GrepSchema,
    gitnexus_read: ReadSchema,
    gitnexus_explore: ExploreSchema,
    gitnexus_overview: OverviewSchema,
    gitnexus_impact: ImpactSchema,
    gitnexus_highlight: HighlightSchema,
    gitnexus_diff: DiffSchema,
    gitnexus_deep_dive: DeepDiveSchema,
    gitnexus_review_file: ReviewFileSchema,
    gitnexus_trace_flow: TraceFlowSchema,
    gitnexus_find_similar: FindSimilarSchema,
    gitnexus_test_impact: TestImpactSchema,
    gitnexus_graph_action: GraphActionSchema,
} as const;

export type ToolName = keyof typeof toolSchemaMap;

/**
 * Convert Zod schemas to JSON Schema format for MCP protocol
 */
export const toolSchemas = Object.fromEntries(
    Object.entries(toolSchemaMap).map(([name, schema]) => {
        // Omit name param — passing { name } wraps output in $ref + definitions
        // which MCP clients/routers can't parse. Flat JSON Schema is required.
        const jsonSchema = zodToJsonSchema(schema);
        // Remove $schema key — MCP protocol doesn't need it
        const { $schema, ...rest } = jsonSchema as Record<string, unknown>;
        return [name, rest];
    })
) as Record<string, Record<string, unknown>>;

/**
 * Validates tool input against its schema
 * 
 * @param toolName - Name of the tool (e.g., 'gitnexus_search')
 * @param input - Input to validate
 * @returns SafeParseResult with success/failure and data/errors
 */
export function validateToolInput<T extends ToolName>(
    toolName: T,
    input: unknown
): z.SafeParseReturnType<unknown, z.infer<typeof toolSchemaMap[T]>> {
    const schema = toolSchemaMap[toolName];

    if (!schema) {
        // Return a failed parse result for unknown tool
        return {
            success: false,
            error: new z.ZodError([
                {
                    code: 'custom',
                    message: `Unknown tool: ${toolName}`,
                    path: [],
                },
            ]),
        } as z.SafeParseReturnType<unknown, z.infer<typeof toolSchemaMap[T]>>;
    }

    return schema.safeParse(input);
}

// Type exports for inferred schema types
export type ContextInput = z.infer<typeof ContextSchema>;
export type SearchInput = z.infer<typeof SearchSchema>;
export type CypherInput = z.infer<typeof CypherSchema>;
export type GrepInput = z.infer<typeof GrepSchema>;
export type ReadInput = z.infer<typeof ReadSchema>;
export type ExploreInput = z.infer<typeof ExploreSchema>;
export type OverviewInput = z.infer<typeof OverviewSchema>;
export type ImpactInput = z.infer<typeof ImpactSchema>;
export type HighlightInput = z.infer<typeof HighlightSchema>;
export type DiffInput = z.infer<typeof DiffSchema>;
export type DeepDiveInput = z.infer<typeof DeepDiveSchema>;
export type ReviewFileInput = z.infer<typeof ReviewFileSchema>;
export type TraceFlowInput = z.infer<typeof TraceFlowSchema>;
export type FindSimilarInput = z.infer<typeof FindSimilarSchema>;
export type TestImpactInput = z.infer<typeof TestImpactSchema>;
export type GraphActionInput = z.infer<typeof GraphActionSchema>;
