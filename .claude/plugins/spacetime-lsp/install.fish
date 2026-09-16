#!/usr/bin/env fish

set plugin_src (dirname (status filename))
set plugin_dest ~/.claude/plugins/spacetime-lsp

# Create destination directory
mkdir -p $plugin_dest

# Copy plugin metadata to root (not nested in .claude-plugin/)
cp $plugin_src/.claude-plugin/plugin.json $plugin_dest/plugin.json

# Create .lsp.json with INSTALLED as default (flip from project-local version)
echo '{
  "spacetime": {
    "command": "sh",
    "args": ["-c", "if [ \"$SPACETIME_DEV\" = \"1\" ]; then cargo run --manifest-path /home/user/code/ora/spacetime-testing/Cargo.toml --release -- lsp; else spacetime lsp; fi"],
    "extensionToLanguage": {
      ".st": "spacetime"
    },
    "transport": "stdio",
    "initializationOptions": {},
    "settings": {},
    "maxRestarts": 3
  }
}' > $plugin_dest/.lsp.json

echo ""
echo "Spacetime LSP plugin installed to $plugin_dest"
echo ""
echo "Add to ~/.claude/settings.json:"
echo '  "enabledPlugins": { "spacetime-lsp": true }'
echo ""
echo "Then restart Claude Code to pick up the plugin."
echo ""
echo "To use dev mode (cargo run), set: SPACETIME_DEV=1"
