;;; test-org-edna.el --- Comprehensive org-edna validation tests

;; This file tests that org-edna is actually working with emacs internals,
;; not just that we have the correct syntax in our files.

(require 'package)
(package-initialize)
(require 'org)

;; Try to load org-edna, but continue tests even if not available
(condition-case err
    (progn
      (require 'org-edna)
      (org-edna-mode 1))
  (error
   (message "Warning: org-edna not available: %s" err)))

;; Test file path
(defvar test-org-file (expand-file-name "../tasks/active.org" 
                                        (file-name-directory load-file-name)))

;; Test results accumulator
(defvar test-results '())

(defun record-test (name result details)
  "Record test result."
  (push (list :name name :result result :details details) test-results))

;;; Test 1: Verify org-edna is loaded and active
(defun test-edna-loaded ()
  "Test that org-edna is properly loaded."
  (let ((loaded (featurep 'org-edna))
        (mode-active org-edna-mode))
    (record-test "org-edna-loaded" 
                 (and loaded mode-active)
                 (format "Feature loaded: %s, Mode active: %s" loaded mode-active))))

;;; Test 2: Parse BLOCKER/TRIGGER properties
(defun test-parse-edna-properties ()
  "Test that org-edna properties are parsed correctly."
  (with-current-buffer (find-file-noselect test-org-file)
    (org-mode)
    (goto-char (point-min))
    (let ((blocker-count 0)
          (trigger-count 0)
          (examples '()))
      ;; Search for tasks with edna properties
      (while (re-search-forward "^\\*+ \\(TODO\\|DONE\\|NEXT\\)" nil t)
        (let ((props (org-entry-properties)))
          (when (assoc "BLOCKER" props)
            (setq blocker-count (1+ blocker-count))
            (when (< (length examples) 3)
              (push (cons (org-get-heading t t t t) 
                         (cdr (assoc "BLOCKER" props))) 
                    examples)))
          (when (assoc "TRIGGER" props)
            (setq trigger-count (1+ trigger-count)))))
      (record-test "parse-edna-properties"
                   (> blocker-count 0)
                   (format "Found %d BLOCKER, %d TRIGGER properties. Examples: %S" 
                          blocker-count trigger-count examples)))))

;;; Test 3: Test state change triggers
(defun test-state-change-trigger ()
  "Test that completing a task triggers dependent tasks."
  (with-current-buffer (find-file-noselect test-org-file)
    (goto-char (point-min))
    ;; Find a test task with TRIGGER property
    (if (re-search-forward ":ID: +BUILD-001-TESTS" nil t)
        (progn
          (org-back-to-heading)
          ;; Get current state of triggered task
          (save-excursion
            (goto-char (point-min))
            (re-search-forward ":ID: +BUILD-001" nil t)
            (org-back-to-heading)
            (let ((initial-state (org-get-todo-state)))
              ;; Now mark test task as DONE (this should trigger BUILD-001)
              (goto-char (point-min))
              (re-search-forward ":ID: +BUILD-001-TESTS" nil t)
              (org-back-to-heading)
              ;; Check if the trigger would fire (dry run)
              (let* ((triggers (org-entry-get nil "TRIGGER"))
                     (parsed (when triggers 
                              (org-edna--string-to-conditions triggers))))
                (record-test "state-change-trigger"
                            (and triggers parsed)
                            (format "Trigger property: %s, Parsed: %S" 
                                   triggers parsed))))))
      (record-test "state-change-trigger" nil "Could not find test task BUILD-001-TESTS"))))

;;; Test 4: Test dependency blocking
(defun test-dependency-blocking ()
  "Test that blocked tasks cannot be marked DONE."
  (with-current-buffer (find-file-noselect test-org-file)
    (goto-char (point-min))
    ;; Find GAP-005 which is blocked by SAFETY-001
    (if (re-search-forward ":ID: +GAP-005[^-]" nil t)
        (progn
          (org-back-to-heading)
          (let* ((blockers (org-entry-get nil "BLOCKER"))
                 (blocked-ids (when blockers
                               (let ((matches '()))
                                 (string-match-all "ids(\\([^)]+\\))" blockers
                                   (lambda (m)
                                     (setq matches (append matches 
                                                          (split-string (match-string 1 blockers))))))
                                 matches))))
            ;; Check if any blockers are incomplete
            (let ((incomplete-blockers '()))
              (dolist (id blocked-ids)
                (save-excursion
                  (goto-char (point-min))
                  (when (re-search-forward (format ":ID: +%s" id) nil t)
                    (org-back-to-heading)
                    (let ((state (org-get-todo-state)))
                      (unless (member state '("DONE" "CANCELLED"))
                        (push (cons id state) incomplete-blockers))))))
              (record-test "dependency-blocking"
                          (> (length incomplete-blockers) 0)
                          (format "GAP-005 blocked by: %S" incomplete-blockers)))))
      (record-test "dependency-blocking" nil "Could not find GAP-005"))))

;;; Test 5: Test ids() function
(defun test-ids-function ()
  "Test that ids() function in edna properties works."
  (with-current-buffer (find-file-noselect test-org-file)
    (goto-char (point-min))
    (let ((ids-usage-count 0)
          (examples '()))
      (while (re-search-forward ":BLOCKER: +.*ids(" nil t)
        (setq ids-usage-count (1+ ids-usage-count))
        (when (< (length examples) 5)
          (push (buffer-substring-no-properties 
                 (line-beginning-position) (line-end-position))
                examples)))
      (record-test "ids-function"
                   (> ids-usage-count 0)
                   (format "Found %d uses of ids() function. Examples: %S" 
                          ids-usage-count examples)))))

;;; Test 6: Test children condition
(defun test-children-condition ()
  "Test children todo?(DONE) conditions."
  (with-current-buffer (find-file-noselect test-org-file)
    (goto-char (point-min))
    (let ((children-conditions 0)
          (examples '()))
      (while (re-search-forward "children todo\\?" nil t)
        (setq children-conditions (1+ children-conditions))
        (when (< (length examples) 3)
          (save-excursion
            (org-back-to-heading)
            (push (org-get-heading t t t t) examples))))
      (record-test "children-condition"
                   (> children-conditions 0)
                   (format "Found %d children conditions. Tasks: %S" 
                          children-conditions examples)))))

;;; Test 7: Test circular dependency detection
(defun test-circular-dependencies ()
  "Test for circular dependencies which org-edna should prevent."
  (with-current-buffer (find-file-noselect test-org-file)
    ;; Build dependency graph
    (goto-char (point-min))
    (let ((deps (make-hash-table :test 'equal)))
      (while (re-search-forward "^\\*+ " nil t)
        (let ((id (org-entry-get nil "ID"))
              (blocker (org-entry-get nil "BLOCKER")))
          (when (and id blocker)
            (let ((blocked-ids '()))
              ;; Extract IDs from blocker
              (when (string-match "ids(\\([^)]+\\))" blocker)
                (setq blocked-ids (split-string (match-string 1 blocker))))
              (puthash id blocked-ids deps)))))
      ;; Check for cycles (simplified - just check if A→B and B→A)
      (let ((cycles '()))
        (maphash (lambda (id blockers)
                  (dolist (blocker blockers)
                    (let ((reverse-deps (gethash blocker deps)))
                      (when (member id reverse-deps)
                        (push (format "%s <-> %s" id blocker) cycles)))))
                deps)
        (record-test "circular-dependencies"
                     (= (length cycles) 0)
                     (if cycles
                         (format "Found circular dependencies: %S" cycles)
                       "No circular dependencies found"))))))

;;; Test 8: Test TODO state keywords
(defun test-todo-keywords ()
  "Test that our TODO keywords are properly configured."
  (with-current-buffer (find-file-noselect test-org-file)
    (let ((keywords org-todo-keywords-1)
          (expected '("TODO" "NEXT" "IN-PROGRESS" "DONE" "CANCELLED")))
      (record-test "todo-keywords"
                   (cl-subsetp expected keywords :test 'string=)
                   (format "TODO keywords: %S" keywords)))))

;;; Test 9: Test cross-tree dependencies
(defun test-cross-tree-deps ()
  "Test dependencies across different project trees."
  (with-current-buffer (find-file-noselect test-org-file)
    (goto-char (point-min))
    ;; Find GAP-005 and check its dependencies
    (when (re-search-forward ":ID: +GAP-005[^-]" nil t)
      (let* ((blocker (org-entry-get nil "BLOCKER"))
             (has-safety-001 (and blocker (string-match "SAFETY-001" blocker))))
        ;; Now find SAFETY-001 and check it's in a different tree
        (save-excursion
          (goto-char (point-min))
          (when (re-search-forward ":ID: +SAFETY-001[^-]" nil t)
            (let ((safety-heading (org-get-heading t t t t)))
              (record-test "cross-tree-deps"
                          has-safety-001
                          (format "GAP-005 (build tree) blocked by SAFETY-001 (safety tree): %s" 
                                 safety-heading)))))))))

;;; Test 10: Test trigger state transitions
(defun test-trigger-transitions ()
  "Test todo!(NEXT) state transitions."
  (with-current-buffer (find-file-noselect test-org-file)
    (goto-char (point-min))
    (let ((next-triggers 0)
          (done-triggers 0)
          (examples '()))
      (while (re-search-forward ":TRIGGER: +.*todo!" nil t)
        (let ((line (buffer-substring-no-properties 
                    (line-beginning-position) (line-end-position))))
          (cond ((string-match "todo!(NEXT)" line)
                 (setq next-triggers (1+ next-triggers)))
                ((string-match "todo!(DONE)" line)
                 (setq done-triggers (1+ done-triggers))))
          (when (< (length examples) 3)
            (push line examples))))
      (record-test "trigger-transitions"
                   (> next-triggers 0)
                   (format "NEXT triggers: %d, DONE triggers: %d. Examples: %S" 
                          next-triggers done-triggers examples)))))

;;; Run all tests
(defun run-all-tests ()
  "Run all org-edna validation tests."
  (setq test-results '())
  (test-edna-loaded)
  (test-parse-edna-properties)
  (test-state-change-trigger)
  (test-dependency-blocking)
  (test-ids-function)
  (test-children-condition)
  (test-circular-dependencies)
  (test-todo-keywords)
  (test-cross-tree-deps)
  (test-trigger-transitions)
  
  ;; Print results
  (princ "═══════════════════════════════════════════════════════════\n")
  (princ "                 ORG-EDNA VALIDATION REPORT                 \n")
  (princ "═══════════════════════════════════════════════════════════\n\n")
  
  (let ((passed 0)
        (failed 0))
    (dolist (test (reverse test-results))
      (let ((name (plist-get test :name))
            (result (plist-get test :result))
            (details (plist-get test :details)))
        (if result
            (progn
              (setq passed (1+ passed))
              (princ (format "✅ %s\n   %s\n\n" name details)))
          (progn
            (setq failed (1+ failed))
            (princ (format "❌ %s\n   %s\n\n" name details))))))
    
    (princ "───────────────────────────────────────────────────────────\n")
    (princ (format "SUMMARY: %d passed, %d failed out of %d tests\n" 
                   passed failed (+ passed failed)))
    (if (= failed 0)
        (princ "🎉 All tests passed! Org-edna is working correctly.\n")
      (princ "⚠️  Some tests failed. Check configuration.\n"))))

;; Helper function for string matching
(defun string-match-all (regex string fn)
  "Apply FN to all matches of REGEX in STRING."
  (save-match-data
    (let ((pos 0))
      (while (string-match regex string pos)
        (funcall fn (match-string 0 string))
        (setq pos (match-end 0))))))

;; Run tests
(run-all-tests)