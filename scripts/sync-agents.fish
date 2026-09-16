#!/usr/bin/env fish
# Sync @tasks/agents/*.org → .claude/agents/<name>.md
# Run via direnv (.envrc) on directory entry, or manually.

set REPO_ROOT (cd (dirname (status filename))/.. && pwd)
set AGENTS_SRC "$REPO_ROOT/@tasks/agents"
set AGENTS_OUT "$REPO_ROOT/.claude/agents"
set SKILLS_DIR "$REPO_ROOT/.claude/skills"

# Guard: pandoc is required for org→markdown conversion
if not command -q pandoc
    exit 0
end

# ---------- Migration: remove old agent-generated skill dirs ----------
for marker in $SKILLS_DIR/*/.agent-generated
    test -f "$marker"; or continue
    set skill_dir (dirname "$marker")
    rm -rf "$skill_dir"
    echo "Migrated (removed old skill dir): "(basename "$skill_dir")
end

# ---------- Cleanup stale auto-generated agent .md files ----------
mkdir -p "$AGENTS_OUT"

for md_file in $AGENTS_OUT/*.md
    test -f "$md_file"; or continue
    # Only touch files we generated (contain the marker comment)
    grep -q '^<!-- auto-generated ' "$md_file"; or continue
    set name (basename "$md_file" .md)
    if not test -f "$AGENTS_SRC/$name.org"
        rm -f "$md_file"
        echo "Removed stale agent: $name"
    end
end

# ---------- Agent tool/model mapping ----------
function agent_model -a name
    switch $name
        case parser-specialist metasystem-architect pipeline-engineer tooling-dx-engineer
            echo opus
        case codegen-developer site-designer testing-engineer code-quality-guardian
            echo sonnet
        case '*'
            echo sonnet
    end
end

function agent_tools -a name
    switch $name
        case parser-specialist metasystem-architect pipeline-engineer code-quality-guardian tooling-dx-engineer
            echo "Read, Grep, Glob, Bash, Edit, Write, LSP"
        case codegen-developer testing-engineer
            echo "Read, Grep, Glob, Bash, Edit, Write"
        case site-designer
            echo "Read, Grep, Glob, Bash, Edit, Write, WebFetch"
        case '*'
            echo "Read, Grep, Glob, Bash, Edit, Write"
    end
end

# ---------- Sync each agent ----------
for org_file in $AGENTS_SRC/*.org
    test -f "$org_file"; or continue
    set name (basename "$org_file" .org)

    # Skip index.org
    test "$name" = index; and continue

    set out_file "$AGENTS_OUT/$name.md"

    # Timestamp skip: if output exists, is auto-generated, and org is older → skip
    if test -f "$out_file"
        and grep -q '^<!-- auto-generated ' "$out_file"
        and test "$org_file" -ot "$out_file"
        continue
    end

    # Safety: if output exists but is NOT auto-generated, don't clobber
    if test -f "$out_file"
        and not grep -q '^<!-- auto-generated ' "$out_file"
        echo "Warning: $out_file exists without auto-generated marker — skipping (hand-crafted?)"
        continue
    end

    # Extract #+TITLE:
    set title (grep '^#+TITLE:' "$org_file" | head -1 | sed 's/^#+TITLE: *//')

    # Extract first sentence of Apex Expertise section
    set apex_first_sentence (
        awk '/^\*\* Apex Expertise$/{found=1; next} found && /^\*/{exit} found && NF{printf "%s ", $0}' "$org_file" \
        | sed 's/  */ /g; s/^ *//; s/ *$//' \
        | sed 's/\. .*/\./'
    )
    # Ensure it ends with exactly one period
    set apex_first_sentence (string trim --right --chars='.' -- "$apex_first_sentence")
    set description "$title agent. $apex_first_sentence."

    set model (agent_model "$name")
    set tools (agent_tools "$name")

    # Convert org → GitHub-flavored markdown via pandoc
    set body (pandoc -f org -t gfm --wrap=none "$org_file")

    # Write agent .md with marker comment and YAML frontmatter
    printf '%s\n' \
        "<!-- auto-generated from @tasks/agents/$name.org — do not edit -->" \
        "---" \
        "name: $name" \
        "description: \"$description\"" \
        "tools: $tools" \
        "model: $model" \
        "---" \
        "" \
        "$body" \
        > "$out_file"

    echo "Synced agent: $name"
end
