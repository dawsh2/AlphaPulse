;;; gemini-final.el --- Rock solid Gemini CLI integration -*- lexical-binding: t; -*-

;;; Commentary:
;; Absolutely minimal, bulletproof implementation using basic term-mode

;;; Code:

(defvar gemini-buffer nil "The Gemini terminal buffer.")
(defvar gemini-program "/opt/homebrew/bin/gemini" "Path to Gemini CLI.")

(defun gemini-start ()
  "Start Gemini CLI."
  (interactive)
  ;; Force clean start
  (when (get-buffer "*gemini*")
    (let ((kill-buffer-query-functions nil))
      (kill-buffer "*gemini*")))
  (setq gemini-buffer nil)
  
  ;; Use vterm if available, fallback to ansi-term
  (if (require 'vterm nil t)
      (progn
        ;; Start vterm directly with gemini command
        (let ((vterm-shell gemini-program))
          (setq gemini-buffer (vterm "*gemini*"))))
    ;; Fallback to ansi-term
    (progn
      (setq gemini-buffer (ansi-term gemini-program "gemini"))
      (with-current-buffer gemini-buffer
        (rename-buffer "*gemini*" t))))
  
  ;; Show it
  (display-buffer gemini-buffer '((display-buffer-in-side-window)
                                  (side . right)
                                  (window-width . 80)))
  (message "Gemini started"))

(defun gemini-send-text (text)
  "Send TEXT to Gemini."
  (unless (and gemini-buffer (buffer-live-p gemini-buffer))
    (gemini-start))
  
  (with-current-buffer gemini-buffer
    (if (fboundp 'vterm-send-string)
        (progn
          ;; Clear any existing input first
          (vterm-send-key "C-c")
          (sit-for 0.1)
          (vterm-send-key "C-u") 
          (sit-for 0.1)
          ;; Send the text all at once
          (vterm-send-string text)
          (vterm-send-return))
      (progn
        (term-send-raw-string text)
        (term-send-raw-string "\n")))))

(defun gemini-command (prompt)
  "Send PROMPT to Gemini."
  (interactive "sGemini: ")
  (gemini-send-text prompt))

(defun gemini-context (prompt)
  "Send PROMPT with file context to Gemini."
  (interactive "sGemini with context: ")
  (let* ((file (buffer-file-name))
         (line (line-number-at-pos))
         (region-text (when (use-region-p)
                        (buffer-substring-no-properties 
                         (region-beginning) (region-end))))
         (context-parts (list prompt))
         (full-prompt ""))
    
    ;; Add file reference
    (when file
      (if (use-region-p)
          (push (format "@%s:%d-%d" file 
                       (line-number-at-pos (region-beginning))
                       (line-number-at-pos (region-end))) 
                context-parts)
        (push (format "@%s:%d" file line) context-parts)))
    
    ;; Add region content if present
    (when region-text
      (push "\nRegion content:" context-parts)
      (push region-text context-parts))
    
    (setq full-prompt (string-join (reverse context-parts) "\n"))
    (gemini-send-text full-prompt)))

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
  "Kill Gemini session."
  (interactive)
  (when (get-buffer "*gemini*")
    (let ((kill-buffer-query-functions nil))
      (kill-buffer "*gemini*")))
  (setq gemini-buffer nil)
  (message "Gemini killed"))

;; Keybindings
(global-set-key (kbd "C-c g g") #'gemini-toggle)
(global-set-key (kbd "C-c g s") #'gemini-command) 
(global-set-key (kbd "C-c g x") #'gemini-context)
(global-set-key (kbd "C-c g k") #'gemini-kill)

(provide 'gemini-final)
;;; gemini-final.el ends here