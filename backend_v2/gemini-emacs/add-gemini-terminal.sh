#!/bin/bash

# Add Gemini terminal mode to Emacs config

EMACS_CONFIG="$HOME/.emacs.d/init.el"
GEMINI_CONFIG='
;; Gemini Terminal Mode (claude-code.el equivalent for Gemini)
(add-to-list '"'"'load-path "/Users/daws/alphapulse/backend_v2/gemini-emacs/elisp")
(require '"'"'gemini-native)

;; Optional: Install eat for better terminal emulation
;; (use-package eat
;;   :ensure t
;;   :config
;;   (setq gemini-native-use-eat t))

;; Gemini terminal configuration
(setq gemini-native-window-position '"'"'right)  ; Window appears on right
(setq gemini-native-window-size 0.4)         ; Takes 40% of frame width

;; Keybindings:
;; C-c G G - Toggle Gemini terminal
;; C-c G s - Send prompt
;; C-c G r - Send region
;; C-c G b - Send buffer
;; C-c G k - Kill session
'

# Check if already configured
if grep -q "gemini-native" "$EMACS_CONFIG" 2>/dev/null; then
    echo "Gemini terminal mode already configured in init.el"
    exit 0
fi

# Backup
cp "$EMACS_CONFIG" "$EMACS_CONFIG.backup.$(date +%Y%m%d_%H%M%S)"

# Add configuration
echo "" >> "$EMACS_CONFIG"
echo "$GEMINI_CONFIG" >> "$EMACS_CONFIG"

echo "✓ Gemini terminal mode added to ~/.emacs.d/init.el"
echo "✓ Backup created"
echo ""
echo "To use:"
echo "  1. Restart Emacs or M-x load-file RET ~/.emacs.d/init.el"
echo "  2. Press C-c G G to open Gemini terminal"
echo ""
echo "This gives you the same experience as claude-code.el but with Gemini!"