#!/bin/bash

# Sync .claude/agents/ to .claude/commands/ via file copying
# Copies agent files to commands directory, overwriting if they exist

AGENTS_DIR=".claude/agents"
COMMANDS_DIR=".claude/commands"

# Ensure commands directory exists
mkdir -p "$COMMANDS_DIR"

# Loop through all .md files in agents directory
for agent_file in "$AGENTS_DIR"/*.md; do
    if [ -f "$agent_file" ]; then
        # Get just the filename (e.g., "rust-devops-specialist.md")
        agent_name=$(basename "$agent_file")
        command_path="$COMMANDS_DIR/$agent_name"
        
        # Copy the file, removing YAML frontmatter if present
        echo "Copying: $agent_file -> $command_path"
        
        # Remove YAML frontmatter (lines between --- at the beginning)
        if head -1 "$agent_file" | grep -q "^---$"; then
            # File starts with frontmatter, skip until after the closing ---
            awk '/^---$/ && NR==1 {in_frontmatter=1; next} 
                 /^---$/ && in_frontmatter {in_frontmatter=0; next} 
                 !in_frontmatter {print}' "$agent_file" > "$command_path"
        else
            # No frontmatter, just copy normally
            cp "$agent_file" "$command_path"
        fi
    fi
done

echo "Agent sync complete!"