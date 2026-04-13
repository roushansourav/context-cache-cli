# context-cache-cli

TypeScript CLI + Rust core for fast context caching and graph-aware code analysis.

## Top 10 Daily Commands

```bash
# 1) Initialize once per repo
context-cache init

# 2) Refresh cache
context-cache refresh

# 3) Build/update graph
context-cache graph-build --refresh

# 4) Check graph health
context-cache graph-status

# 5) Quick risk summary for recent changes
context-cache minimal-context --base HEAD~1

# 6) Detailed risk rows
context-cache detect-changes --base HEAD~1

# 7) Find impacted files
context-cache impact-radius --base HEAD~1 --depth 3

# 8) Focused review bundle
context-cache review-context --base HEAD~1 --depth 2 --max-lines 120

# 9) Semantic lookup
context-cache semantic-search "cache refresh" --limit 15

# 10) Ready-to-paste prompt for Copilot Chat
context-cache ready --max-chars 64000
```

## Quick Start

1. Build native + TypeScript layers:

```bash
npm run build
```

2. Initialize config (first run in a repo):

```bash
context-cache init
```

3. Generate cache and graph:

```bash
context-cache refresh
context-cache graph-build --refresh
```

## Parser Coverage

Current AST-first support:

- JavaScript
- TypeScript
- TSX
- Python
- Rust
- Go
- Java
- C
- C++
- C#
- Ruby

Fallback parser remains active for unsupported grammars (for example PHP/Lua today), so indexing can still proceed.

## Usage

```bash
context-cache <command> [options]
```

## NPM Publishing (No Rust Required For Users)

This project is configured to publish prebuilt native binaries so end users can run:

```bash
npm install -g context-cache
```

without installing Cargo/Rust locally.

How it works:

- Root package: `context-cache`
- Platform packages (optional dependencies):
	- `context-cache-darwin-arm64`
	- `context-cache-darwin-x64`
	- `context-cache-linux-arm64-gnu`
	- `context-cache-linux-x64-gnu`
	- `context-cache-win32-arm64-msvc`
	- `context-cache-win32-x64-msvc`

The runtime loader first attempts the matching prebuilt package for the current OS/arch, then falls back to local `binding.node` for developer builds.

### Release Flow

1. Tag a release (for example `v0.1.1`).
2. GitHub Actions workflow builds native artifacts for all configured targets.
3. Workflow publishes packages to npm using `NPM_TOKEN`.

Workflow file: `.github/workflows/release.yml`

Required GitHub secret:

- `NPM_TOKEN` (npm automation token with publish rights)

### Dry-Run Release Checklist

Before creating a release tag, run this checklist locally:

```bash
# 1) Clean install
npm ci

# 2) Ensure TypeScript and native build are healthy
npm run build

# 3) Ensure tests pass
npm test

# 4) Verify CLI starts
node dist/bin/cli.js --help

# 5) Generate npm prebuild package metadata
npm run prepublish:napi
```

Then validate release assets:

1. Confirm expected platform packages are generated under `npm/`.
2. Confirm `package.json` version is correct.
3. Confirm `NPM_TOKEN` is present in repository secrets.
4. Create tag and push (for example `git tag v0.1.1 && git push origin v0.1.1`).

Local commands used by the release pipeline:

```bash
# local native build for your machine
npm run build:native

# generate npm prebuild package metadata/artifacts
npm run prepublish:napi
```

## Full Command Reference (With Examples)

### Project Setup And Cache

`init` - Create global config for current repository.

```bash
context-cache init
context-cache init --preset nextjs --force
```

`detect` - Detect framework preset.

```bash
context-cache detect
```

`refresh` - Refresh cache for current repository.

```bash
context-cache refresh
```

`status` - Show cache status.

```bash
context-cache status
```

`cache-path` - Print the cache JSON path for this repository.

```bash
context-cache cache-path
```

### Prompt And AI Context Helpers

`prompt` - Print cached context to stdout.

```bash
context-cache prompt
context-cache prompt --max-chars 48000
```

`prompt-ready` - Refresh and write prompt file.

```bash
context-cache prompt-ready
context-cache prompt-ready --out .context/prompt.txt --max-chars 64000
```

`ready` - One-step refresh + prompt write + next-step hint.

```bash
context-cache ready
context-cache ready --out .context/prompt.txt
```

`prompt-copy` - Copy prompt output to clipboard.

```bash
context-cache prompt-copy
context-cache prompt-copy --no-refresh --max-chars 32000
```

`mcp-prompts` - List built-in MCP prompt templates, or show one template.

```bash
context-cache mcp-prompts
context-cache mcp-prompts --show review_changes
```

### Graph Build And Inspection

`graph-build` - Build/update graph from cache.

```bash
context-cache graph-build
context-cache graph-build --refresh
context-cache graph-build --full
```

`graph-status` - Show graph DB stats.

```bash
context-cache graph-status
```

`query-graph` - Query structural relationships.

```bash
context-cache query-graph callers_of src/bin/cli.ts::graph-build
context-cache query-graph imports_of file::src/index.ts --limit 50
```

`impact-radius` - Estimate blast radius from changed files.

```bash
context-cache impact-radius --base HEAD~1 --depth 3
context-cache impact-radius --changed src/index.ts,src/bin/cli.ts --depth 2
```

`detect-changes` - Risk-scored change analysis.

```bash
context-cache detect-changes
context-cache detect-changes --base origin/main
```

`minimal-context` - Compact risk + impact summary.

```bash
context-cache minimal-context
context-cache minimal-context --base origin/main
```

`graph-postprocess` - Recompute flow/community artifacts.

```bash
context-cache graph-postprocess
```

### Flows, Communities, And Architecture

`list-flows` - List execution flows.

```bash
context-cache list-flows
context-cache list-flows --limit 60
```

`get-flow` - Get one flow by id or name.

```bash
context-cache get-flow --id 1
context-cache get-flow --name auth
```

`affected-flows` - List flows touched by changed files.

```bash
context-cache affected-flows --base HEAD~1
context-cache affected-flows --changed src/index.ts,src/bin/cli.ts --limit 20
```

`list-communities` - List detected code communities.

```bash
context-cache list-communities
context-cache list-communities --limit 50
```

`get-community` - Get one community by id/name (optionally with members).

```bash
context-cache get-community --id 1
context-cache get-community --name src --members
```

`architecture-overview` - Print architecture overview and warnings.

```bash
context-cache architecture-overview
```

`review-context` - Build focused review context from changed files and impact radius.

```bash
context-cache review-context --base HEAD~1
context-cache review-context --changed src/index.ts --depth 2 --max-lines 120 --no-source
```

`find-large-functions` - Find oversized symbols.

```bash
context-cache find-large-functions
context-cache find-large-functions --min-lines 80 --kind function --file src/lib/rust --limit 25
```

`docs-section` - Print a local README section by heading name.

```bash
context-cache docs-section "Parser Coverage"
context-cache docs-section "Usage"
```

### Semantic Search And Embeddings

`embed-graph` - Compute embeddings for semantic search.

```bash
context-cache embed-graph
context-cache embed-graph --model hash-v1
context-cache embed-graph --model ollama:nomic-embed-text
```

`semantic-search` - Semantic search over graph nodes.

```bash
context-cache semantic-search "cache refresh path"
context-cache semantic-search "mcp server" --kind function --limit 15 --model hash-v1
```

### Refactor Tools

`refactor-preview` - Preview rename impact.

```bash
context-cache refactor-preview getRepoRoot resolveRepoRoot
context-cache refactor-preview refresh refreshCache --limit 200
```

`refactor-apply` - Apply rename changes.

```bash
context-cache refactor-apply getRepoRoot resolveRepoRoot --yes
context-cache refactor-apply oldName newName --max-files 300 --yes
```

### Wiki Generation

`generate-wiki` - Generate wiki pages from communities.

```bash
context-cache generate-wiki
context-cache generate-wiki --force
```

`get-wiki-page` - Print one generated wiki page.

```bash
context-cache get-wiki-page src
```

### Symbol And Runtime Utilities

`query-symbol` - Find definitions, references, probable tests.

```bash
context-cache query-symbol refresh
context-cache query-symbol buildGraph --limit 200 --refresh
```

`watch` - Watch filesystem and auto-refresh cache.

```bash
context-cache watch
```

`doctor` - Run environment checks.

```bash
context-cache doctor
```

`vscode-setup` - Add user-level VS Code tasks.

```bash
context-cache vscode-setup
```

### MCP Installation And Serving

`install` - Install MCP configuration for supported tools.

```bash
context-cache install
context-cache install --platform claude
context-cache install --platform all --dry-run
```

`mcp-serve` - Run MCP stdio server.

```bash
context-cache mcp-serve
```

### Multi-Repo Registry

`registry-list` - List registered repositories.

```bash
context-cache registry-list
```

`registry-add` - Register a repository.

```bash
context-cache registry-add ../other-repo
context-cache registry-add ../service-a --alias service-a
```

`registry-remove` - Remove registry entry by alias.

```bash
context-cache registry-remove service-a
```

`cross-repo-search` - Semantic search across registered repositories.

```bash
context-cache cross-repo-search "auth middleware" --limit 30
context-cache cross-repo-search "retry logic" --aliases service-a,service-b --model hash-v1
```

`cross-repo-impact` - Compute cross-repository impact.

```bash
context-cache cross-repo-impact --base HEAD~1 --depth 2
context-cache cross-repo-impact --changed src/index.ts --aliases service-a,service-b
```

### Parity Audit

`parity` - Report CLI-to-MCP command/tool parity.

```bash
context-cache parity
```

## Notes

- Most analysis commands assume you already ran `context-cache refresh` and `context-cache graph-build`.
- Commands using `--base` depend on `git diff` relative to that ref (default often `HEAD~1`).
- For command-specific flags, run:

```bash
context-cache <command> --help
```
