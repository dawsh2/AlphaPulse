;;; gemini-simple.el --- Ultra-simple Gemini CLI integration -*- lexical-binding: t; -*-

;;; Commentary:
;; Minimal, robust Gemini CLI integration - no complexity

;;; Code:

(defvar gemini-buffer nil "The Gemini terminal buffer.")

(defun gemini-start ()
  "Start Gemini CLI in terminal."
  (interactive)
  (when (and gemini-buffer (buffer-live-p gemini-buffer))
    (kill-buffer gemini-buffer))
  
  (setq gemini-buffer
        (if (require 'vterm nil t)
            ;; Use vterm with direct command
            (let ((vterm-shell "/opt/homebrew/bin/gemini"))
              (vterm "*gemini*"))
          ;; Fallback to term
          (make-term "gemini" "/opt/homebrew/bin/gemini")))
  
  (with-current-buffer gemini-buffer
    (unless (fboundp 'vterm-mode)
      (term-mode)
      (term-char-mode)))
  
  (display-buffer gemini-buffer '((display-buffer-in-side-window)
                                  (side . right)
                                  (window-width . 0.4)))
  (message "Gemini started"))

(defun gemini-send (text)
  "Send TEXT to Gemini."
  (unless (and gemini-buffer (buffer-live-p gemini-buffer))
    (gemini-start))
  
  (with-current-buffer gemini-buffer
    (if (fboundp 'vterm-send-string)
        (progn (vterm-send-string text) (vterm-send-return))
      (progn (term-send-raw-string text) (term-send-raw-string "\n")))))

(defun gemini-command (cmd)
  "Send command CMD to Gemini."
  (interactive "sGemini: ")
  (gemini-send cmd))

(defun gemini-context (cmd)
  "Send CMD with file context."
  (interactive "sGemini (with context): ")
  (let* ((file (buffer-file-name))
         (line (line-number-at-pos))
         (context (when file (format "@%s:%d" file line)))
         (full (if context (format "%s\n\n%s" cmd context) cmd)))
    (gemini-send full)))

(defun gemini-toggle ()
  "Toggle Gemini window."
  (interactive)
  (if (and gemini-buffer (get-buffer-window gemini-buffer))
      (delete-window (get-buffer-window gemini-buffer))
    (gemini-start)))

(defun gemini-kill ()
  "Kill Gemini."
  (interactive)
  (when gemini-buffer
    (kill-buffer gemini-buffer)
    (setq gemini-buffer nil)))

;; Keybindings
(global-set-key (kbd "C-c g g") #'gemini-toggle)
(global-set-key (kbd "C-c g s") #'gemini-command)
(global-set-key (kbd "C-c g x") #'gemini-context)
(global-set-key (kbd "C-c g k") #'gemini-kill)

(provide 'gemini-simple)
;;; gemini-simple.el ends here