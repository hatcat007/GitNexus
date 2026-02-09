/**
 * Tool Quality Test
 * 
 * Validates tool definitions against 2025/2026 best practices from:
 * - OpenAI o3/o4-mini Function Calling Guide
 * - Anthropic Advanced Tool Use
 * - PromptingGuide.ai Agent Function Calling
 * 
 * Metrics scored:
 * 1. Description prescriptiveness (key rules up front)
 * 2. Usage criteria (when to use / when NOT to use)
 * 3. Return format documentation
 * 4. Parameter description quality
 * 5. Error recovery guidance in descriptions
 * 6. Cross-tool disambiguation
 * 7. Few-shot examples for complex params
 */

import { describe, test, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

// Read the raw file to parse tool definitions
const toolsFile = fs.readFileSync(
  path.join(__dirname, '..', 'tools.ts'),
  'utf-8'
);
const agentFile = fs.readFileSync(
  path.join(__dirname, '..', 'agent.ts'),
  'utf-8'
);

// Extract tool definition blocks
interface ToolDef {
  name: string;
  description: string;
  paramDescriptions: Map<string, string>;
}

function extractToolDefs(source: string): ToolDef[] {
  const tools: ToolDef[] = [];
  // Match tool name + description blocks
  const toolRegex = /name:\s*'(\w+)',\s*\n\s*description:\s*[`'"]([^`]*?)[`'"]\s*,?\s*\n\s*schema/gs;
  let match;
  while ((match = toolRegex.exec(source)) !== null) {
    const name = match[1];
    const description = match[2];
    
    // Find param descriptions in the schema block after this match
    const afterMatch = source.slice(match.index);
    const schemaBlock = afterMatch.match(/schema:\s*z\.object\(\{([\s\S]*?)\}\)/);
    const paramDescriptions = new Map<string, string>();
    
    if (schemaBlock) {
      const paramRegex = /(\w+):\s*z\.\w+\([^)]*\)(?:\.\w+\([^)]*\))*\.describe\(['"`]([^'"`]+)['"`]\)/g;
      let paramMatch;
      while ((paramMatch = paramRegex.exec(schemaBlock[1])) !== null) {
        paramDescriptions.set(paramMatch[1], paramMatch[2]);
      }
    }
    
    tools.push({ name, description, paramDescriptions });
  }
  return tools;
}

const tools = extractToolDefs(toolsFile);

// Quality scoring functions
function scoreDescription(tool: ToolDef): { score: number; issues: string[] } {
  const issues: string[] = [];
  let score = 0;
  const desc = tool.description;
  
  // 1. Length: should be substantial (>100 chars for complex tools)
  if (desc.length > 200) score += 10;
  else if (desc.length > 100) score += 5;
  else issues.push(`Description too short (${desc.length} chars) — should explain WHEN and HOW`);
  
  // 2. Prescriptive: starts with action verb or clear purpose
  if (/^(Search|Execute|Read|Analyze|Find|Show|Get|Deep|Impact|Codebase)/i.test(desc)) score += 5;
  else issues.push('Description should start with prescriptive action verb');
  
  // 3. Usage criteria: "Use when" / "Use this"
  if (/use (when|this|for|to)/i.test(desc)) score += 10;
  else issues.push('Missing usage criteria (WHEN to use this tool)');
  
  // 4. Anti-criteria: "Do not use" / "Prefer X instead"
  if (/do not|don't|prefer|instead|not for/i.test(desc)) score += 10;
  else issues.push('Missing anti-criteria (when NOT to use / prefer alternatives)');
  
  // 5. Return format documentation
  if (/return|output|result|format/i.test(desc)) score += 10;
  else issues.push('Missing return format documentation');
  
  // 6. Examples (especially for complex params like cypher)
  if (/example|e\.g\.|such as|like:/i.test(desc)) score += 10;
  else if (tool.name === 'cypher' || tool.name === 'grep') issues.push('Missing examples for complex params');
  
  // 7. Cross-tool references (disambiguation)
  const otherTools = ['search', 'cypher', 'grep', 'read', 'overview', 'explore', 'impact'];
  const referencesOther = otherTools.filter(t => t !== tool.name && desc.includes(t));
  if (referencesOther.length > 0) score += 10;
  else issues.push('No cross-tool disambiguation (should reference when to prefer other tools)');
  
  // 8. Key rules front-loaded (first 100 chars should contain the most important info)
  const first100 = desc.slice(0, 100);
  if (/must|always|never|important|critical/i.test(first100)) score += 5;
  
  return { score, issues };
}

function scoreParams(tool: ToolDef): { score: number; issues: string[] } {
  const issues: string[] = [];
  let score = 0;
  
  for (const [param, desc] of tool.paramDescriptions) {
    // Each param should have >20 char description
    if (desc.length >= 20) score += 3;
    else issues.push(`Param '${param}' description too short: "${desc}"`);
    
    // Should include examples or constraints
    if (/e\.g\.|example|like|default|must|format/i.test(desc)) score += 2;
    else issues.push(`Param '${param}' missing examples or constraints`);
  }
  
  return { score, issues };
}

function scoreSystemPrompt(prompt: string): { score: number; issues: string[] } {
  const issues: string[] = [];
  let score = 0;
  
  // 1. Tool ordering guidance
  if (/order|first|then|before|after|workflow|step/i.test(prompt)) score += 10;
  else issues.push('Missing explicit tool ordering/workflow guidance');
  
  // 2. Decision tree for tool selection
  if (/(when|if).*use.*tool|prefer.*over|instead of/i.test(prompt)) score += 10;
  else issues.push('Missing decision tree for tool selection');
  
  // 3. Anti-hallucination directives
  if (/do not promise|emit it now|ground|cite|retract/i.test(prompt)) score += 10;
  else issues.push('Missing anti-hallucination directives');
  
  // 4. Tool-specific behavioral rules
  if (/impact.*trusted|read.*before|grep.*exact/i.test(prompt)) score += 10;
  else issues.push('Missing tool-specific behavioral rules');
  
  // 5. Output format specification
  if (/table|mermaid|diagram|format/i.test(prompt)) score += 10;
  else issues.push('Missing output format specification');
  
  return { score, issues };
}

// ====== TESTS ======

describe('Tool Quality Audit', () => {
  test('should find all 7 tools', () => {
    const names = tools.map(t => t.name);
    expect(names).toContain('search');
    expect(names).toContain('cypher');
    expect(names).toContain('grep');
    expect(names).toContain('read');
    expect(names).toContain('overview');
    expect(names).toContain('explore');
    expect(names).toContain('impact');
  });
  
  test('tool description quality scores', () => {
    const results: Array<{ name: string; descScore: number; paramScore: number; total: number; issues: string[] }> = [];
    
    for (const tool of tools) {
      const descResult = scoreDescription(tool);
      const paramResult = scoreParams(tool);
      const total = descResult.score + paramResult.score;
      results.push({
        name: tool.name,
        descScore: descResult.score,
        paramScore: paramResult.score,
        total,
        issues: [...descResult.issues, ...paramResult.issues],
      });
    }
    
    // Print scorecard
    console.log('\n=== TOOL QUALITY SCORECARD ===\n');
    console.log('| Tool | Desc | Params | Total | Grade |');
    console.log('|------|------|--------|-------|-------|');
    
    let grandTotal = 0;
    for (const r of results) {
      const maxDesc = 60;
      const maxParam = 5 * r.issues.length + r.paramScore; // rough max
      const grade = r.total >= 50 ? 'A' : r.total >= 35 ? 'B' : r.total >= 20 ? 'C' : 'D';
      grandTotal += r.total;
      console.log(`| ${r.name.padEnd(8)} | ${String(r.descScore).padStart(4)} | ${String(r.paramScore).padStart(6)} | ${String(r.total).padStart(5)} | ${grade}     |`);
    }
    
    console.log(`\nGrand Total: ${grandTotal} / ${tools.length * 70} (${Math.round(grandTotal / (tools.length * 70) * 100)}%)`);
    
    console.log('\n=== ISSUES ===\n');
    for (const r of results) {
      if (r.issues.length > 0) {
        console.log(`[${r.name}]`);
        r.issues.forEach(i => console.log(`  ❌ ${i}`));
      }
    }
    
    // The test always passes — it's a diagnostic
    expect(results.length).toBe(7);
  });
  
  test('system prompt quality score', () => {
    // Extract BASE_SYSTEM_PROMPT
    const promptMatch = agentFile.match(/export const BASE_SYSTEM_PROMPT = `([\s\S]*?)`;/);
    expect(promptMatch).toBeTruthy();
    
    const prompt = promptMatch![1];
    const result = scoreSystemPrompt(prompt);
    
    console.log('\n=== SYSTEM PROMPT SCORECARD ===\n');
    console.log(`Score: ${result.score} / 50`);
    if (result.issues.length > 0) {
      console.log('Issues:');
      result.issues.forEach(i => console.log(`  ❌ ${i}`));
    }
  });
});
