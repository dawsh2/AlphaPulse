;;; load-claude-style.el --- Load claude-code.el style implementation

;; Reset gemini buffer variable
(setq gemini-buffer nil)

;; Load claude-style implementation
(load-file "/Users/daws/alphapulse/backend_v2/gemini-emacs/gemini-claude-style.el")

(message "Claude-style Gemini loaded - FIXED!")
(message "Uses exact claude-code.el buffer creation and command sending")
(message "Commands: C-c g g (toggle), C-c g s (send), C-c g x (context), C-c g k (kill)")

;;; load-claude-style.el ends here