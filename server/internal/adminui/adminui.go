// Package adminui embeds the single-file admin console.
package adminui

import _ "embed"

//go:embed index.html
var HTML []byte

// Page returns the console HTML.
func Page() []byte { return HTML }
