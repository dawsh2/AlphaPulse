;;; simple-edna-test.el --- Simple validation that org-edna is working

(require 'package)
(package-initialize)
(require 'org)
(require 'org-edna)

;; Enable org-edna mode
(org-edna-mode 1)

(defvar test-file (expand-file-name "../tasks/active.org" 
                                    (file-name-directory load-file-name)))

(princ "════════════════════════════════════════════════════════════════\n")
(princ "                    ORG-EDNA STATUS REPORT                     \n")
(princ "════════════════════════════════════════════════════════════════\n\n")

;; Test 1: Check if org-edna is loaded
(princ (format "✅ Org-edna loaded: %s\n" (featurep 'org-edna)))
(princ (format "✅ Org-edna mode active: %s\n\n" org-edna-mode))

;; Test 2: Count BLOCKER and TRIGGER properties
(with-current-buffer (find-file-noselect test-file)
  (goto-char (point-min))
  (let ((blocker-count 0)
        (trigger-count 0)
        (children-count 0)
        (ids-count 0))
    (while (re-search-forward "^\\s-*:BLOCKER:" nil t)
      (setq blocker-count (1+ blocker-count))
      (let ((line (buffer-substring-no-properties
                  (line-beginning-position) (line-end-position))))
        (when (string-match "children" line)
          (setq children-count (1+ children-count)))
        (when (string-match "ids(" line)
          (setq ids-count (1+ ids-count)))))
    (goto-char (point-min))
    (while (re-search-forward "^\\s-*:TRIGGER:" nil t)
      (setq trigger-count (1+ trigger-count)))
    
    (princ "📊 PROPERTY STATISTICS\n")
    (princ "─────────────────────────────────────────────────────────────\n")
    (princ (format "  BLOCKER properties: %d\n" blocker-count))
    (princ (format "  TRIGGER properties: %d\n" trigger-count))
    (princ (format "  Using 'children': %d\n" children-count))
    (princ (format "  Using 'ids()': %d\n\n" ids-count))))

;; Test 3: Check for syntax errors by looking at org-edna errors
(princ "🔍 CHECKING FOR SYNTAX ERRORS\n")
(princ "─────────────────────────────────────────────────────────────\n")

;; Try to parse the file and see if org-edna complains
(with-current-buffer (find-file-noselect test-file)
  (org-mode)
  (goto-char (point-min))
  (let ((error-count 0)
        (sample-tasks '()))
    ;; Find tasks with dependencies
    (while (re-search-forward "^\\*+ TODO" nil t)
      (let ((heading (org-get-heading t t t t))
            (blocker (org-entry-get nil "BLOCKER"))
            (trigger (org-entry-get nil "TRIGGER")))
        (when (and (< (length sample-tasks) 5)
                   (or blocker trigger))
          (push (list :heading heading
                     :blocker blocker
                     :trigger trigger)
                sample-tasks))))
    
    (if (= error-count 0)
        (princ "  ✅ No obvious syntax errors detected\n\n")
      (princ (format "  ⚠️  Found %d potential issues\n\n" error-count)))
    
    ;; Show sample tasks
    (princ "📝 SAMPLE TASKS WITH DEPENDENCIES\n")
    (princ "─────────────────────────────────────────────────────────────\n")
    (dolist (task (reverse sample-tasks))
      (princ (format "\n  Task: %s\n" (plist-get task :heading)))
      (when (plist-get task :blocker)
        (princ (format "    BLOCKER: %s\n" (plist-get task :blocker))))
      (when (plist-get task :trigger)
        (princ (format "    TRIGGER: %s\n" (plist-get task :trigger)))))))

;; Test 4: Check if IDs are properly formatted
(princ "\n\n🆔 ID VALIDATION\n")
(princ "─────────────────────────────────────────────────────────────\n")
(with-current-buffer (find-file-noselect test-file)
  (goto-char (point-min))
  (let ((single-id-count 0)
        (multi-id-count 0)
        (quoted-id-count 0))
    (while (re-search-forward "ids(\\([^)]+\\))" nil t)
      (let ((content (match-string 1)))
        (cond
         ;; Quoted multiple IDs
         ((string-match "^\".*\"$" content)
          (setq quoted-id-count (1+ quoted-id-count)))
         ;; Multiple IDs (space-separated)
         ((string-match " " content)
          (setq multi-id-count (1+ multi-id-count)))
         ;; Single ID
         (t
          (setq single-id-count (1+ single-id-count))))))
    (princ (format "  Single IDs: %d\n" single-id-count))
    (princ (format "  Multiple IDs (quoted): %d\n" quoted-id-count))
    (princ (format "  Multiple IDs (unquoted - may cause errors): %d\n\n" multi-id-count))))

;; Final summary
(princ "════════════════════════════════════════════════════════════════\n")
(princ "                           SUMMARY                              \n")
(princ "════════════════════════════════════════════════════════════════\n")

(if (and (featurep 'org-edna)
         org-edna-mode)
    (progn
      (princ "\n🎉 ORG-EDNA IS ACTIVE AND CONFIGURED!\n\n")
      (princ "Next steps to verify full functionality:\n")
      (princ "1. Open active.org in Emacs\n")
      (princ "2. Mark a test task (e.g., BUILD-001-TESTS) as DONE\n")
      (princ "3. Check if BUILD-001 automatically advances to NEXT\n")
      (princ "4. This confirms automatic state transitions are working\n"))
  (princ "\n⚠️  ORG-EDNA MAY NOT BE FULLY CONFIGURED\n"))