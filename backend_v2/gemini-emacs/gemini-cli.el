;;; gemini-cli.el --- Gemini CLI integration for Emacs -*- lexical-binding: t; -*-

;;; Commentary:
;; Robust Gemini CLI integration with vterm, inspired by claude-code.el

;;; Code:

(defvar gemini-buffer nil "The Gemini terminal buffer.")
(defvar gemini-program "/opt/homebrew/bin/gemini" "Path to Gemini CLI.")

(defun gemini--ensure-vterm ()
  "Ensure vterm package is loaded."
  (unless (and (require 'vterm nil t) (featurep 'vterm))
    (error "The vterm package is required")))

(defun gemini--term-make (buffer-name program &optional switches)
  "Create a vterm terminal exactly like claude-code.el does.
BUFFER-NAME is the name for the new terminal buffer.
PROGRAM is the program to run in the terminal.
SWITCHES are optional command-line arguments for PROGRAM."
  (gemini--ensure-vterm)
  (let* ((vterm-shell (if switches
                          (concat program " " (mapconcat #'identity switches " "))
                        program))
         (buffer (get-buffer-create buffer-name)))
    (with-current-buffer buffer
      ;; Exactly like claude-code.el: pop-to-buffer, vterm-mode, then delete-window
      (pop-to-buffer buffer)
      (vterm-mode)
      (delete-window (get-buffer-window buffer))
      buffer)))

(defun gemini--send-command (cmd)
  "Send a command CMD to Gemini using claude-code.el approach."
  (when (and gemini-buffer (buffer-live-p gemini-buffer))
    (with-current-buffer gemini-buffer
      ;; Send the command string, without return
      (vterm-send-string cmd)
      ;; Give the terminal time to process the full command
      (sit-for 0.1)
      ;; Send Return - use vterm-send-return, not kbd
      (vterm-send-return)
      ;; Display buffer
      (display-buffer gemini-buffer '((display-buffer-in-side-window)
                                      (side . right)
                                      (window-width . 80))))
    gemini-buffer))

(defun gemini-start ()
  "Start Gemini CLI using claude-code.el approach."
  (interactive)
  ;; Clean up any existing buffer
  (when (and gemini-buffer (buffer-live-p gemini-buffer))
    (if (y-or-n-p "Gemini is running. Restart? ")
        (kill-buffer gemini-buffer)
      (user-error "Gemini already running")))
  
  (setq gemini-buffer (gemini--term-make "*gemini*" gemini-program))
  
  ;; Brief delay to let terminal initialize
  (sit-for 0.1)
  
  ;; Show the buffer
  (display-buffer gemini-buffer '((display-buffer-in-side-window)
                                  (side . right)
                                  (window-width . 80)))
  (message "Gemini started"))

(defun gemini-command (prompt)
  "Send PROMPT to Gemini using claude-code.el approach."
  (interactive "sGemini: ")
  (unless (and gemini-buffer (buffer-live-p gemini-buffer))
    (gemini-start))
  (gemini--send-command prompt))

(defun gemini-context (prompt)
  "Send PROMPT with file context using claude-code.el approach."
  (interactive "sGemini with context: ")
  (unless (and gemini-buffer (buffer-live-p gemini-buffer))
    (gemini-start))
  (let* ((file (buffer-file-name))
         (line (line-number-at-pos))
         (file-ref (when file
                     (if (use-region-p)
                         (format "@%s:%d-%d" file 
                                (line-number-at-pos (region-beginning))
                                (line-number-at-pos (region-end)))
                       (format "@%s:%d" file line))))
         ;; Use claude-code.el format: single newline
         (cmd-with-context (if file-ref
                               (format "%s\n%s" prompt file-ref)
                             prompt)))
    (gemini--send-command cmd-with-context)))

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
  (when (and gemini-buffer (buffer-live-p gemini-buffer))
    (kill-buffer gemini-buffer))
  (setq gemini-buffer nil)
  (message "Gemini killed"))

;; Keybindings
(global-set-key (kbd "C-c g g") #'gemini-toggle)
(global-set-key (kbd "C-c g s") #'gemini-command) 
(global-set-key (kbd "C-c g x") #'gemini-context)
(global-set-key (kbd "C-c g k") #'gemini-kill)

(provide 'gemini-cli)
;;; gemini-cli.el ends here