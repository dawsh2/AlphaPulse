;;; test-simple.el --- Test the ultra-simple version

;; Clean slate
(when (get-buffer "*gemini*") (kill-buffer "*gemini*"))
(setq gemini-buffer nil)

;; Load simple version
(load-file "/Users/daws/alphapulse/backend_v2/gemini-emacs/gemini-simple.el")

(message "Ultra-simple Gemini loaded!")
(message "Try: C-c g g (toggle), C-c g s (send), C-c g x (context)")

;;; test-simple.el ends here