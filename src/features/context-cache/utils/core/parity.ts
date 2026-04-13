export type CommandParityReport = {
  matched: Array<{ cliCommand: string; mcpTool: string }>;
  cliOnly: string[];
  mcpOnly: string[];
};

function tokenizeCliCommand(name: string): Set<string> {
  return new Set(name.split('-').filter(Boolean));
}

function tokenizeMcpTool(name: string): Set<string> {
  const STOP_WORDS = new Set(['or', 'the', 'and', 'to', 'of', 'a', 'an']);
  return new Set(name.split('_').filter((t) => !STOP_WORDS.has(t)));
}

function computeOverlap(a: Set<string>, b: Set<string>): number {
  let count = 0;
  for (const token of a) {
    if (b.has(token)) count += 1;
  }
  return count;
}

function findBestMatch(
  cliCommand: string,
  mcpToolNames: ReadonlyArray<string>,
): string | undefined {
  const cliTokens = tokenizeCliCommand(cliCommand);
  let bestMatch: string | undefined;
  let bestScore = 0;

  for (const tool of mcpToolNames) {
    const toolTokens = tokenizeMcpTool(tool);
    const overlap = computeOverlap(cliTokens, toolTokens);
    const minSize = Math.min(cliTokens.size, toolTokens.size);
    // Require at least 50% token overlap to count as a match
    if (overlap > 0 && overlap >= minSize * 0.5 && overlap > bestScore) {
      bestScore = overlap;
      bestMatch = tool;
    }
  }

  return bestMatch;
}

export function evaluateParity(
  cliCommands: ReadonlyArray<string>,
  mcpToolNames: ReadonlyArray<string>,
): CommandParityReport {
  const matched: Array<{ cliCommand: string; mcpTool: string }> = [];
  const cliOnly: string[] = [];
  const matchedMcpTools = new Set<string>();

  for (const cmd of cliCommands) {
    const match = findBestMatch(cmd, mcpToolNames);
    if (match) {
      matched.push({ cliCommand: cmd, mcpTool: match });
      matchedMcpTools.add(match);
    } else {
      cliOnly.push(cmd);
    }
  }

  const mcpOnly = mcpToolNames.filter((t) => !matchedMcpTools.has(t));

  return { matched, cliOnly, mcpOnly };
}
