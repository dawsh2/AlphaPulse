;;; debug-input.el --- Debug the input issue

(defun debug-gemini-command ()
  "Debug version of gemini command."
  (interactive)
  (let ((prompt (read-string "Debug Gemini: ")))
    (message "You typed: '%s'" prompt)
    (message "Length: %d characters" (length prompt))
    ;; Don't send to gemini yet, just show what we got
    (with-current-buffer (get-buffer-create "*debug*")
      (insert (format "Input: '%s' (length: %d)\n" prompt (length prompt))))))

(global-set-key (kbd "C-c g d") #'debug-gemini-command)

(message "Debug command loaded: C-c g d")

;;; debug-input.el ends here