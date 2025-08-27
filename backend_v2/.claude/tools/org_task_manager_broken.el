;;; org_task_manager.el --- AlphaPulse Org-mode Task Management Functions

;;; Commentary:
;; Elisp functions for reading and writing org-mode tasks
;; Designed to be called from batch mode

;;; Code:

(require 'org)
(require 'org-element)
(require 'json)

(defun alphapulse/parse-tasks-to-json (file)
  "Parse all tasks from FILE and output as JSON."
  (with-temp-buffer
    (insert-file-contents file)
    (org-mode)
    (let ((tasks '())
          (task-count 0)
          (todo-count 0)
          (done-count 0)
          (in-progress-count 0))
      
      ;; Parse all headings (goals and tasks)
      (org-map-entries
       (lambda ()
         (let* ((id (org-entry-get nil "ID"))
                (state (org-get-todo-state))
                (heading (org-get-heading t t t t))
                (tags (org-get-tags))
                (priority (org-entry-get nil "PRIORITY"))
                (level (org-current-level))
                (props (org-entry-properties))
                (scheduled (org-entry-get nil "SCHEDULED"))
                (deadline (org-entry-get nil "DEADLINE"))
                (body (org-get-entry))
                (is-goal (and (= level 1) (null state)))
                (is-actionable nil)
                (effective-status state))
           
           ;; Check if actionable (has state, is TODO/NEXT, no children)
           (when (and state (member state '("TODO" "NEXT")))
             (save-excursion
               (setq is-actionable (not (org-goto-first-child)))))
           
           ;; Compute effective status (READY vs WAITING for TODO tasks)
           (when (and (string= state "TODO") is-actionable)
             ;; TODO task that's actionable - check if dependencies are met
             (let ((depends (org-entry-get nil "DEPENDS")))
               (setq effective-status 
                     (if (and depends (not (string-empty-p depends)))
                         "WAITING"  ; Has dependencies, assumed not ready for now
                       "READY")))) ; No dependencies, ready to start
           
           ;; Count only items with states
           (when state
             (setq task-count (1+ task-count))
             (cond
              ((string= state "TODO") (setq todo-count (1+ todo-count)))
              ((string= state "DONE") (setq done-count (1+ done-count)))
              ((string= state "IN-PROGRESS") (setq in-progress-count (1+ in-progress-count)))))
           
           ;; Include all headings (goals and tasks)
           (when (or state is-goal)
             (push `((id . ,id)
                     (heading . ,heading)
                     (state . ,state)
                     (effective_status . ,effective-status)
                     (priority . ,priority)
                     (tags . ,(vconcat tags))
                     (level . ,level)
                     (scheduled . ,scheduled)
                     (deadline . ,deadline)
                     (properties . ,props)
                     (body . ,body)
                     (is_goal . ,(if is-goal t json-false))
                     (is_actionable . ,(if is-actionable t json-false)))
                   tasks))))))
      
      ;; Output JSON
      (let ((output `((tasks . ,(vconcat (reverse tasks)))
                      (metadata . ((total_tasks . ,task-count)
                                   (todo_count . ,todo-count)
                                   (done_count . ,done-count)
                                   (in_progress_count . ,in-progress-count)
                                   (parse_timestamp . ,(format-time-string "%Y-%m-%dT%H:%M:%S")))))))
        (princ (json-encode output))))

(defun alphapulse/update-task-state (file task-id new-state)
  "Update the state of TASK-ID in FILE to NEW-STATE."
  (find-file file)
  (org-mode)
  (goto-char (point-min))
  
  ;; Find task by ID
  (let ((task-found nil))
    (while (and (not task-found) (re-search-forward ":ID:\\s-+\\(.*\\)" nil t))
      (when (string= (match-string 1) task-id)
        (setq task-found t)
        (org-back-to-heading)
        (org-todo new-state)))
  
    (if task-found
        (progn
          (save-buffer)
          (princ (format "Task %s updated to %s" task-id new-state)))
      (error "Task %s not found" task-id))))

(defun alphapulse/add-task (file heading &optional state priority tags properties body parent-id)
  "Add a new task to FILE."
  (find-file file)
  (org-mode)
  
  ;; Navigate to parent if specified
  (if parent-id
      (progn
        (goto-char (point-min))
        (let ((found nil))
          (while (and (not found) (re-search-forward ":ID:\\s-+\\(.*\\)" nil t))
            (when (string= (match-string 1) parent-id)
              (setq found t)
              (org-end-of-subtree t t)))
          (unless found
            (error "Parent ID %s not found" parent-id))))
    (goto-char (point-max)))
  
  ;; Insert new heading
  (org-insert-heading-respect-content)
  (insert (or state "TODO") " " heading)
  
  ;; Add tags
  (when tags
    (org-set-tags (split-string tags ":")))
  
  ;; Set priority
  (when priority
    (org-priority (string-to-char priority)))
  
  ;; Add properties
  (when properties
    (let ((props (json-read-from-string properties)))
      (dolist (prop props)
        (org-set-property (symbol-name (car prop)) (cdr prop)))))
  
  ;; Add body
  (when body
    (org-end-of-meta-data)
    (insert "\n" body))
  
  (save-buffer)
  (princ (format "Task '%s' added successfully" heading)))

(defun alphapulse/get-ready-tasks (file)
  "Get all tasks ready for execution from FILE."
  (with-temp-buffer
    (insert-file-contents file)
    (org-mode)
    (let ((ready-tasks '())
          (all-tasks (make-hash-table :test 'equal)))
      
      ;; First pass: collect all tasks
      (org-map-entries
       (lambda ()
         (when (org-get-todo-state)
           (let* ((id (org-entry-get nil "ID"))
                  (state (org-get-todo-state))
                  (depends (org-entry-get nil "DEPENDS"))
                  (is-actionable nil))
             
             ;; Check if actionable (has state, is TODO/NEXT, no children)
             (when (and state (member state '("TODO" "NEXT")))
               (save-excursion
                 (setq is-actionable (not (org-goto-first-child)))))
             
             (when id
               (let ((effective-status (if (and (string= state "TODO") 
                                               is-actionable 
                                               depends 
                                               (not (string-empty-p depends)))
                                          "WAITING"
                                        (if (and (string= state "TODO") is-actionable)
                                            "READY"
                                          state))))
                 (puthash id `((state . ,state)
                              (effective_status . ,effective-status)
                              (depends . ,(when depends (split-string depends)))
                              (heading . ,(org-get-heading t t t t))
                              (priority . ,(org-entry-get nil "PRIORITY"))
                              (is_actionable . ,is-actionable))
                         all-tasks)))))))
      
      ;; Second pass: find ready tasks (actionable + dependencies complete)
      (maphash
       (lambda (id task-data)
         (let ((state (cdr (assoc 'state task-data)))
               (depends (cdr (assoc 'depends task-data)))
               (is-actionable (cdr (assoc 'is_actionable task-data))))
           ;; Task is ready if actionable and effective status is READY
           (when (and is-actionable
                      (string= (cdr (assoc 'effective_status task-data)) "READY"))
             (push `((id . ,id) ,@task-data) ready-tasks))))
       all-tasks)
      
      ;; Sort by priority
      (setq ready-tasks (sort ready-tasks 'alphapulse/task-priority-less))
      
      ;; Output JSON
      (princ (json-encode `((ready_tasks . ,(vconcat ready-tasks))))))))

(defun alphapulse/all-dependencies-done (depends all-tasks)
  "Check if all DEPENDS tasks are done in ALL-TASKS."
  (catch 'not-done
    (dolist (dep-id depends t)
      (let ((dep-task (gethash dep-id all-tasks)))
        (when (and dep-task
                   (not (string= (cdr (assoc 'state dep-task)) "DONE")))
          (throw 'not-done nil))))))

(defun alphapulse/task-priority-less (a b)
  "Compare tasks A and B by priority."
  (let ((pa (cdr (assoc 'priority a)))
        (pb (cdr (assoc 'priority b))))
    (cond
     ((and pa pb) (string< pa pb))
     (pa t)
     (pb nil)
     (t (string< (cdr (assoc 'id a)) (cdr (assoc 'id b)))))))

;; Global variable to store command arguments
(defvar alphapulse/command-args nil
  "Command arguments for alphapulse CLI.")

;; Command-line interface
(defun alphapulse/cli-main ()
  "Main entry point for batch mode."
  (when alphapulse/command-args
    (let ((command (car alphapulse/command-args))
          (file (cadr alphapulse/command-args)))
      (cond
       ((string= command "parse")
        (alphapulse/parse-tasks-to-json file))
       
       ((string= command "ready")
        (alphapulse/get-ready-tasks file))
       
       ((string= command "update")
        (let ((task-id (nth 2 alphapulse/command-args))
              (new-state (nth 3 alphapulse/command-args)))
          (alphapulse/update-task-state file task-id new-state)))
       
       ((string= command "add")
        (let ((heading (nth 2 alphapulse/command-args))
              (state (or (nth 3 alphapulse/command-args) "TODO"))
              (priority (nth 4 alphapulse/command-args))
              (tags (nth 5 alphapulse/command-args))
              (properties (nth 6 alphapulse/command-args))
              (body (nth 7 alphapulse/command-args))
              (parent-id (nth 8 alphapulse/command-args)))
          (alphapulse/add-task file heading state priority tags properties body parent-id)))
       
       (t
        (error "Unknown command: %s" command))))))

(provide 'org_task_manager)
;;; org_task_manager.el ends here