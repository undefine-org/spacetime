package tree_sitter_spacetime_test

import (
	"testing"

	tree_sitter "github.com/smacker/go-tree-sitter"
	"github.com/tree-sitter/tree-sitter-spacetime"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_spacetime.Language())
	if language == nil {
		t.Errorf("Error loading Spacetime grammar")
	}
}
