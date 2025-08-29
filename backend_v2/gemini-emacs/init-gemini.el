;;; init-gemini.el --- Load the canonical gemini-cli.el

;; Reset gemini buffer variable
(setq gemini-buffer nil)

;; Load canonical gemini-cli
(load-file "/Users/daws/alphapulse/backend_v2/gemini-emacs/gemini-cli.el")

(message "Gemini CLI loaded successfully!")
(message "Commands: C-c g g (toggle), C-c g s (send), C-c g x (context), C-c g k (kill)")

;;; init-gemini.el ends here