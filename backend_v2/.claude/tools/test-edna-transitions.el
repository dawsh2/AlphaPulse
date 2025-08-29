;;; test-edna-transitions.el --- Test org-edna state transitions

(require 'package)
(package-initialize)
(require 'org)
(require 'org-edna)

;; Enable org-edna
(org-edna-mode 1)

(defvar test-file (expand-file-name "edna-stress-test.org" 
                                    (file-name-directory load-file-name)))

(princ "════════════════════════════════════════════════════════════════\n")
(princ "              ORG-EDNA STATE TRANSITION TEST                   \n")
(princ "════════════════════════════════════════════════════════════════\n\n")

;; Test automatic state transitions
(with-current-buffer (find-file-noselect test-file)
  (org-mode)
  
  ;; Test 1: Simple transition
  (princ "TEST 1: Simple Dependency (A1 -> A2)\n")
  (princ "─────────────────────────────────────────────────────────────\n")
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-A1" nil t)
  (org-back-to-heading)
  (princ (format "  Initial A1 state: %s\n" (org-get-todo-state)))
  
  ;; Check A2 initial state
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-A2" nil t)
  (org-back-to-heading)
  (princ (format "  Initial A2 state: %s\n" (org-get-todo-state)))
  
  ;; Mark A1 as DONE
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-A1" nil t)
  (org-back-to-heading)
  (org-todo "DONE")
  (princ (format "  Marked A1 as: %s\n" (org-get-todo-state)))
  
  ;; Check if A2 advanced
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-A2" nil t)
  (org-back-to-heading)
  (let ((a2-state (org-get-todo-state)))
    (princ (format "  A2 state after trigger: %s\n" a2-state))
    (if (string= a2-state "NEXT")
        (princ "  ✅ Trigger worked!\n\n")
      (princ "  ❌ Trigger did not fire\n\n")))
  
  ;; Test 2: Children blocking
  (princ "TEST 2: Children Blocking\n")
  (princ "─────────────────────────────────────────────────────────────\n")
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-C-PARENT" nil t)
  (org-back-to-heading)
  (let ((parent-state (org-get-todo-state)))
    (princ (format "  Parent initial state: %s\n" parent-state))
    
    ;; Try to mark parent as DONE (should be blocked)
    (condition-case err
        (progn
          (org-todo "DONE")
          (let ((new-state (org-get-todo-state)))
            (if (string= new-state "DONE")
                (princ "  ❌ Parent was marked DONE despite children (blocking failed)\n")
              (princ (format "  ✅ Parent blocked (state: %s)\n" new-state)))))
      (error
       (princ (format "  ✅ Blocking worked! Error: %s\n" (error-message-string err))))))
  
  ;; Mark children as DONE
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-C1" nil t)
  (org-back-to-heading)
  (org-todo "DONE")
  (princ (format "  Marked C1 as: %s\n" (org-get-todo-state)))
  
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-C2" nil t)
  (org-back-to-heading)
  (org-todo "DONE")
  (princ (format "  Marked C2 as: %s\n" (org-get-todo-state)))
  
  ;; Now try parent again
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-C-PARENT" nil t)
  (org-back-to-heading)
  (org-todo "DONE")
  (let ((final-state (org-get-todo-state)))
    (princ (format "  Parent after children done: %s\n" final-state))
    (if (string= final-state "DONE")
        (princ "  ✅ Parent can now be completed!\n\n")
      (princ "  ❌ Parent still blocked\n\n")))
  
  ;; Test 3: Multiple IDs
  (princ "TEST 3: Multiple ID Triggers\n")
  (princ "─────────────────────────────────────────────────────────────\n")
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-B1" nil t)
  (org-back-to-heading)
  (org-todo "DONE")
  (princ (format "  Marked B1 as: %s\n" (org-get-todo-state)))
  
  ;; Check both B2 and B3
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-B2" nil t)
  (org-back-to-heading)
  (princ (format "  B2 state: %s\n" (org-get-todo-state)))
  
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-B3" nil t)
  (org-back-to-heading)
  (princ (format "  B3 state: %s\n\n" (org-get-todo-state)))
  
  ;; Test 4: Cross-tree dependencies
  (princ "TEST 4: Cross-Tree Dependencies\n")
  (princ "─────────────────────────────────────────────────────────────\n")
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-E1" nil t)
  (org-back-to-heading)
  (org-todo "DONE")
  (princ (format "  Marked E1 as: %s\n" (org-get-todo-state)))
  
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-E2" nil t)
  (org-back-to-heading)
  (princ (format "  E2 state (should be NEXT): %s\n" (org-get-todo-state)))
  
  (org-todo "DONE")
  (princ (format "  Marked E2 as: %s\n" (org-get-todo-state)))
  
  (goto-char (point-min))
  (re-search-forward ":ID: *TEST-D1" nil t)
  (org-back-to-heading)
  (princ (format "  D1 state (cross-tree trigger): %s\n\n" (org-get-todo-state))))

;; Summary
(princ "════════════════════════════════════════════════════════════════\n")
(princ "                      TEST COMPLETE                            \n")
(princ "════════════════════════════════════════════════════════════════\n")
(princ "\nIf you see NEXT states and successful blocks above,\n")
(princ "org-edna is working correctly with automatic state transitions!\n")