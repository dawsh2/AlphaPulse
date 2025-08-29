;;; gemini-rock-solid.el --- Absolutely bulletproof Gemini CLI -*- lexical-binding: t; -*-

;;; Code:

(defvar gemini-buffer nil "The Gemini terminal buffer.")
(defvar gemini-program "/opt/homebrew/bin/gemini" "Path to Gemini CLI.")

(defun gemini-cleanup ()
  "Clean up gemini buffers safely."
  (when gemini-buffer
    (when (buffer-live-p gemini-buffer)
      ;; Don't kill if vterm process is running - just hide
      (when (get-buffer-window gemini-buffer)
        (delete-window (get-buffer-window gemini-buffer))))
    (setq gemini-buffer nil)))

(defun gemini-start ()
  "Start Gemini CLI."
  (interactive)
  (gemini-cleanup)
  
  ;; Always use ansi-term - it's bulletproof
  (setq gemini-buffer (ansi-term gemini-program "gemini"))
  (with-current-buffer gemini-buffer
    (rename-buffer "*gemini*" t))
  
  (display-buffer gemini-buffer '((display-buffer-in-side-window)
                                  (side . right)
                                  (window-width . 80)))
  (message "Gemini started"))

(defun gemini-send-text (text)
  "Send TEXT to Gemini."
  (unless (and gemini-buffer (buffer-live-p gemini-buffer))
    (gemini-start))
  
  (with-current-buffer gemini-buffer
    (goto-char (point-max))
    (term-send-raw-string text)
    (term-send-raw-string "\n")))

(defun gemini-command (prompt)
  "Send PROMPT to Gemini."
  (interactive "sGemini: ")
  (gemini-send-text prompt))

(defun gemini-context (prompt)
  "Send PROMPT with file context to Gemini."
  (interactive "sGemini with context: ")
  (let* ((file (buffer-file-name))
         (line (line-number-at-pos))
         (context (when file (format "@%s:%d" file line)))
         (full (if context (format "%s\n\n%s" prompt context) prompt)))
    (gemini-send-text full)))

(defun gemini-toggle ()
  "Toggle Gemini window visibility."
  (interactive)
  (if (and gemini-buffer (get-buffer-window gemini-buffer))
      (delete-window (get-buffer-window gemini-buffer))
    (if (and gemini-buffer (buffer-live-p gemini-buffer))
        (display-buffer gemini-buffer '((display-buffer-in-side-window)
                                        (side . right)
                                        (window-width . 80)))
      (gemini-start))))

(defun gemini-kill ()
  "Kill Gemini session safely."
  (interactive)
  (when (and gemini-buffer (buffer-live-p gemini-buffer))
    (kill-buffer gemini-buffer))
  (setq gemini-buffer nil)
  (message "Gemini killed"))

;; Keybindings
(global-set-key (kbd "C-c g g") #'gemini-toggle)
(global-set-key (kbd "C-c g s") #'gemini-command) 
(global-set-key (kbd "C-c g x") #'gemini-context)
(global-set-key (kbd "C-c g k") #'gemini-kill)

(provide 'gemini-rock-solid)
;;; gemini-rock-solid.el ends here