;;; load-rock-solid.el --- Load the rock solid version

;; Just reset, don't kill buffers that might have vterm processes
(setq gemini-buffer nil)

;; Load rock solid version
(load-file "/Users/daws/alphapulse/backend_v2/gemini-emacs/gemini-rock-solid.el")

(message "Rock solid Gemini loaded - uses ansi-term, no vterm issues!")
(message "Commands: C-c g g (toggle), C-c g s (send), C-c g x (context), C-c g k (kill)")

;;; load-rock-solid.el ends here