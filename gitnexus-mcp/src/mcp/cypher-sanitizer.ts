/**
 * Cypher Query Sanitizer
 * 
 * Validates Cypher queries to ensure only read-only operations are executed.
 * Blocks destructive keywords that could modify the graph database.
 */

/**
 * Allowed Cypher clauses for read-only queries
 */
const ALLOWED_CLAUSES = [
    'MATCH', 'RETURN', 'WHERE', 'WITH', 'ORDER', 'BY', 'LIMIT', 'SKIP',
    'AS', 'OPTIONAL', 'CASE', 'WHEN', 'THEN', 'ELSE', 'END', 'DISTINCT',
    'COUNT', 'SUM', 'AVG', 'MIN', 'MAX', 'COLLECT', 'SIZE', 'HEAD', 'LAST',
    'TYPE', 'LABELS', 'ID', 'COALESCE', 'NULL', 'AND', 'OR', 'NOT', 'IN',
    'EXISTS', 'WITHIN', 'STARTS', 'ENDS', 'CONTAINS', 'TRUE', 'FALSE',
    'UNWIND', // Read-only list expansion
];

/** Maximum allowed query length in characters */
const MAX_QUERY_LENGTH = 10_000;

/**
 * Forbidden keywords that could modify the graph database
 */
const FORBIDDEN_KEYWORDS = [
    'CREATE', 'MERGE', 'DELETE', 'DETACH', 'DROP', 'SET', 'REMOVE',
    'CALL', 'LOAD', 'CSV', 'FOREACH', 'USING', 'INDEX',
    'CONSTRAINT', 'DATABASE', 'USER', 'ROLE', 'GRANT', 'REVOKE',
    'DENY', 'SHOW', 'START', 'STOP', 'ALTER', 'RENAME',
];

/**
 * Result of Cypher query sanitization
 */
export interface SanitizationResult {
    /** Whether the query is valid and safe */
    valid: boolean;
    /** Error message if validation failed */
    error?: string;
    /** Sanitized query (trimmed) if valid */
    query?: string;
    /** Forbidden keyword detected (if any) */
    forbiddenKeyword?: string;
}

/**
 * Sanitizes and validates a Cypher query for read-only execution
 * 
 * @param query - The Cypher query to validate
 * @returns SanitizationResult with validity status and error details
 * 
 * @example
 * const result = sanitizeCypher("MATCH (n:User) RETURN n.name");
 * if (result.valid) {
 *   // Safe to execute result.query
 * } else {
 *   // Handle result.error
 * }
 */
export function sanitizeCypher(query: string): SanitizationResult {
    // Check for empty or whitespace-only query
    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
        return {
            valid: false,
            error: 'Query cannot be empty',
        };
    }

    // Check for minimum length
    if (trimmedQuery.length < 6) {
        return {
            valid: false,
            error: 'Query is too short to be valid Cypher',
        };
    }

    // Check for maximum length
    if (trimmedQuery.length > MAX_QUERY_LENGTH) {
        return {
            valid: false,
            error: `Query exceeds maximum length of ${MAX_QUERY_LENGTH} characters`,
        };
    }

    const upperQuery = trimmedQuery.toUpperCase();

    // Check for forbidden keywords using word boundary matching
    for (const keyword of FORBIDDEN_KEYWORDS) {
        // Use word boundary regex to avoid false positives
        // e.g., "CREATE" should match but "CREATED" in a string shouldn't
        const regex = new RegExp(`\\b${keyword}\\b`, 'g');
        if (regex.test(upperQuery)) {
            return {
                valid: false,
                error: `Forbidden keyword detected: ${keyword}. Only read-only Cypher operations are allowed.`,
                forbiddenKeyword: keyword,
            };
        }
    }

    // Check that query starts with an allowed clause
    const firstWord = upperQuery.split(/\s+/)[0];
    if (!ALLOWED_CLAUSES.includes(firstWord)) {
        return {
            valid: false,
            error: `Query must start with an allowed clause (MATCH, RETURN, WITH, etc.). Got: ${firstWord}`,
        };
    }

    // Additional safety: check for suspicious patterns
    // Prevent semicolon-separated multiple statements
    if (trimmedQuery.includes(';')) {
        // Allow semicolons only within string literals (basic check)
        const outsideStrings = trimmedQuery.replace(/'[^']*'/g, '').replace(/"[^"]*"/g, '');
        if (outsideStrings.includes(';')) {
            return {
                valid: false,
                error: 'Multiple statements (semicolon separated) are not allowed',
            };
        }
    }

    // All checks passed
    return {
        valid: true,
        query: trimmedQuery,
    };
}

/**
 * List of all forbidden keywords for documentation purposes
 */
export const getForbiddenKeywords = (): readonly string[] => [...FORBIDDEN_KEYWORDS];

/**
 * List of all allowed clauses for documentation purposes
 */
export const getAllowedClauses = (): readonly string[] => [...ALLOWED_CLAUSES];
