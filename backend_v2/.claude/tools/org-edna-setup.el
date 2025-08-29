;;; org-edna-setup.el --- Torq Org-Edna Configuration -*- lexical-binding: t; -*-

;;; Commentary:
;; 
;; Org-edna configuration for advanced task dependency management in Torq.
;; This replaces the simple :DEPENDS: property with powerful TRIGGER/BLOCKER system.
;;
;; Key Features:
;; - Automatic state transitions (TODO → NEXT → DONE)
;; - Bidirectional dependencies without manual sync
;; - Complex conditional logic for task progression
;; - Integration with NEXT keyword for actionable tasks

;;; Installation:
;;
;; 1. Install org-edna from ELPA:
;;    M-x package-install RET org-edna RET
;;
;; 2. Add to your init.el or .emacs:
;;    (load-file "~/.claude/tools/org-edna-setup.el")
;;
;; 3. Restart Emacs or eval the buffer

;;; Code:

(require 'org)
(require 'org-edna)

;; Enable org-edna mode
(org-edna-mode 1)

;; Configure TODO keywords with NEXT state
(setq org-todo-keywords
      '((sequence "TODO(t)" "NEXT(n)" "IN-PROGRESS(i)" "|" "DONE(d)" "CANCELLED(c)")
        (sequence "WAITING(w@/!)" "BLOCKED(b@/!)" "|" "DELEGATED(g@/!)")))

;; Set keyword faces for better visibility
(setq org-todo-keyword-faces
      '(("TODO" . (:foreground "red" :weight bold))
        ("NEXT" . (:foreground "orange" :weight bold))
        ("IN-PROGRESS" . (:foreground "yellow" :weight bold))
        ("DONE" . (:foreground "green" :weight bold))
        ("CANCELLED" . (:foreground "gray" :weight bold))
        ("WAITING" . (:foreground "purple" :weight bold))
        ("BLOCKED" . (:foreground "dark red" :weight bold))))

;; Custom edna finder for Torq task IDs
(defun org-edna-finder/torq-id (id)
  "Find a task by its Torq ID property."
  (org-map-entries
   (lambda ()
     (when (string= (org-entry-get nil "ID") id)
       (point-marker)))
   nil 'agenda))

;; Helper function to check if dependencies are met
(defun torq/check-dependencies ()
  "Check if current task's dependencies are satisfied."
  (let ((blocker (org-entry-get nil "BLOCKER")))
    (if blocker
        (condition-case err
            (not (org-edna-blocker-function blocker))
          (error
           (message "Error checking blocker: %s" err)
           nil))
      t)))

;; Function to find NEXT actionable tasks
(defun torq/find-next-tasks ()
  "Find all tasks that are ready to be marked NEXT."
  (interactive)
  (let ((next-tasks '()))
    (org-map-entries
     (lambda ()
       (when (and (member (org-get-todo-state) '("TODO"))
                  (torq/check-dependencies))
         (push (list (org-get-heading t t t t)
                    (org-entry-get nil "ID")
                    (org-entry-get nil "EFFORT"))
               next-tasks)))
     nil 'agenda)
    (if next-tasks
        (progn
          (switch-to-buffer "*Next Tasks*")
          (erase-buffer)
          (insert "Tasks Ready to Start (Dependencies Met):\n")
          (insert "=========================================\n\n")
          (dolist (task next-tasks)
            (insert (format "- %s\n  ID: %s | Effort: %s\n\n" 
                           (nth 0 task) (nth 1 task) (nth 2 task))))
          (goto-char (point-min)))
      (message "No tasks ready to start"))))

;; Function to visualize task dependencies
(defun torq/show-task-dependencies ()
  "Show dependencies for task at point."
  (interactive)
  (let ((id (org-entry-get nil "ID"))
        (blocker (org-entry-get nil "BLOCKER"))
        (trigger (org-entry-get nil "TRIGGER")))
    (message "Task %s:\n  Blocked by: %s\n  Triggers: %s"
             (or id "NO-ID")
             (or blocker "nothing")
             (or trigger "nothing"))))

;; Automatic NEXT promotion when dependencies are met
(defun torq/auto-promote-to-next ()
  "Automatically promote TODO tasks to NEXT when dependencies are met."
  (interactive)
  (org-map-entries
   (lambda ()
     (when (and (string= (org-get-todo-state) "TODO")
                (torq/check-dependencies))
       (org-todo "NEXT")
       (message "Promoted %s to NEXT" (org-get-heading t t t t))))
   nil 'agenda))

;; Hook to check for stuck projects
(defun torq/find-stuck-projects ()
  "Find projects with no NEXT tasks."
  (interactive)
  (let ((stuck-projects '()))
    (org-map-entries
     (lambda ()
       (when (and (= (org-current-level) 1) ; Top-level projects
                  (member (org-get-todo-state) '("TODO")))
         (let ((has-next nil))
           (org-map-entries
            (lambda ()
              (when (string= (org-get-todo-state) "NEXT")
                (setq has-next t)))
            nil 'tree)
           (unless has-next
             (push (org-get-heading t t t t) stuck-projects)))))
     nil 'agenda)
    (if stuck-projects
        (message "Stuck projects: %s" (string-join stuck-projects ", "))
      (message "No stuck projects found"))))

;; Common org-edna patterns for Torq

;; TDD Pattern: Test task triggers implementation task
(defconst torq/edna-tdd-test
  "ids(IMPL-TASK-ID) todo!(NEXT)"
  "TRIGGER for test task to activate implementation task.")

(defconst torq/edna-tdd-impl
  "ids(TEST-TASK-ID) todo?(DONE)"
  "BLOCKER for implementation task waiting on test task.")

;; Sequential workflow pattern
(defconst torq/edna-sequential
  "next-sibling todo!(NEXT)"
  "TRIGGER to activate next sibling task in sequence.")

;; Parallel completion pattern  
(defconst torq/edna-parallel-parent
  "children todo?(DONE)"
  "BLOCKER for parent waiting on all children.")

;; Sprint completion pattern
(defconst torq/edna-sprint-complete
  "parent todo!(DONE)"
  "TRIGGER when last task completes sprint.")

;; Key bindings for Torq org-edna functions
(with-eval-after-load 'org
  (define-key org-mode-map (kbd "C-c C-x n") 'torq/find-next-tasks)
  (define-key org-mode-map (kbd "C-c C-x d") 'torq/show-task-dependencies)  
  (define-key org-mode-map (kbd "C-c C-x p") 'torq/auto-promote-to-next)
  (define-key org-mode-map (kbd "C-c C-x s") 'torq/find-stuck-projects))

(provide 'org-edna-setup)

;;; org-edna-setup.el ends here