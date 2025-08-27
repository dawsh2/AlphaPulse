;;; org_parser_simple.el --- Simple working org-mode parser

(require 'org)
(require 'json)

(defun org-parse-to-json (file)
  "Parse org file and output JSON"
  (with-temp-buffer
    (insert-file-contents file)
    (org-mode)
    (let ((tasks '())
          (task-count 0)
          (todo-count 0)
          (done-count 0))
      
      (org-map-entries
       (lambda ()
         (let* ((id (org-entry-get nil "ID"))
                (state (org-get-todo-state))
                (heading (org-get-heading t t t t))
                (tags (org-get-tags))
                (priority (org-entry-get nil "PRIORITY"))
                (level (org-current-level))
                (depends (org-entry-get nil "DEPENDS"))
                (effort (org-entry-get nil "EFFORT"))
                (is-goal (and (= level 1) (null state)))
                (is-actionable (and state 
                                   (member state '("TODO" "NEXT"))
                                   (save-excursion (not (org-goto-first-child))))))
           
           (when (or state is-goal)
             (when state
               (setq task-count (1+ task-count))
               (cond
                ((string= state "TODO") (setq todo-count (1+ todo-count)))
                ((string= state "DONE") (setq done-count (1+ done-count)))))
             
             (push `((id . ,id)
                     (heading . ,heading)
                     (state . ,state)
                     (priority . ,priority)
                     (tags . ,(vconcat (or tags [])))
                     (level . ,level)
                     (depends . ,depends)
                     (effort . ,effort)
                     (is_goal . ,(if is-goal t json-false))
                     (is_actionable . ,(if is-actionable t json-false)))
                   tasks)))))
      
      (let ((output `((tasks . ,(vconcat (reverse tasks)))
                      (metadata . ((total_tasks . ,task-count)
                                   (todo_count . ,todo-count)
                                   (done_count . ,done-count)
                                   (parse_timestamp . ,(format-time-string "%Y-%m-%dT%H:%M:%S")))))))
        (princ (json-encode output))))))

(defun org-update-task-state (file task-id new-state)
  "Update task state"
  (find-file file)
  (org-mode)
  (goto-char (point-min))
  
  (let ((found nil))
    (while (and (not found) (re-search-forward ":ID:\\s-+\\(.*\\)" nil t))
      (when (string= (match-string 1) task-id)
        (setq found t)
        (org-back-to-heading)
        (org-todo new-state)))
    
    (if found
        (progn
          (save-buffer)
          (princ (format "Task %s updated to %s" task-id new-state)))
      (error "Task %s not found" task-id))))

;; Global variable for command args
(defvar org-command-args nil)

;; Main CLI function
(defun org-cli-main ()
  "Main CLI entry point"
  (when org-command-args
    (let ((command (car org-command-args))
          (file (cadr org-command-args)))
      (cond
       ((string= command "parse")
        (org-parse-to-json file))
       
       ((string= command "update")
        (let ((task-id (nth 2 org-command-args))
              (new-state (nth 3 org-command-args)))
          (org-update-task-state file task-id new-state)))
       
       (t
        (error "Unknown command: %s" command))))))

;; Run if in batch mode
(when noninteractive
  (org-cli-main))

(provide 'org_parser_simple)
;;; org_parser_simple.el ends here