;;; load-gemini.el --- Simple loader for gemini-cli.el

;; Kill any existing gemini buffer first
(when (get-buffer "*gemini*")
  (kill-buffer "*gemini*"))
;; Reset the buffer variable
(setq gemini-buffer nil)
;; Load the clean gemini-cli.el implementation
(load-file "/Users/daws/alphapulse/backend_v2/gemini-emacs/gemini-cli.el")

(message "Gemini-cli.el loaded successfully!")
(message "Available commands:")
(message "  C-c g g - Toggle Gemini window")
(message "  C-c g s - Send command")
(message "  C-c g x - Send command with context")
(message "  C-c g r - Send region")
(message "  C-c g k - Kill Gemini")

;;; load-gemini.el ends here