;;; load-final.el --- Load the bulletproof version

;; Kill ALL gemini-related buffers first
(dolist (buf (buffer-list))
  (when (string-match-p "gemini\\|\\*gemini\\*" (buffer-name buf))
    (let ((kill-buffer-query-functions nil))
      (kill-buffer buf))))

;; Clear variables
(setq gemini-buffer nil)

;; Load final version
(load-file "/Users/daws/alphapulse/backend_v2/gemini-emacs/gemini-final.el")

(message "Final Gemini version loaded with DIRECT vterm!")
(message "Commands: C-c g g (toggle), C-c g s (send), C-c g x (context), C-c g k (kill)")
(message "vterm now runs gemini directly + input clearing - should fix cutoffs!")

;;; load-final.el ends here