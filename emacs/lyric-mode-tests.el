;;; lyric-mode-tests.el --- Tests for lyric-mode -*- lexical-binding: t; -*-

(require 'ert)
(require 'lyric-mode)

(defconst lyric-test--block-expression
  (concat
   "(exec \"bash\" \"-s\"\n"
   "      :stdin \"\"\"\n"
   "      printf \"%s\\n\" \"$1\"\n"
   "      # Shell parens are not Lyric forms: (hello)\n"
   "      path='C:\\tmp'\n"
   "      \"\"\")"))

(ert-deftest lyric-mode-block-string-is-one-sexp ()
  (with-temp-buffer
    (insert lyric-test--block-expression)
    (lyric-mode)
    (syntax-propertize (point-max))
    (goto-char (point-max))
    (should (equal (lyric--last-sexp-source)
                   lyric-test--block-expression))))

(ert-deftest lyric-mode-block-content-has-string-syntax ()
  (with-temp-buffer
    (insert lyric-test--block-expression)
    (lyric-mode)
    (syntax-propertize (point-max))
    (goto-char (point-min))
    (search-forward "(hello)")
    (should (nth 3 (syntax-ppss)))))

(ert-deftest lyric-mode-last-sexp-preserves-multiline-source ()
  (with-temp-buffer
    (insert lyric-test--block-expression "\n\n")
    (lyric-mode)
    (syntax-propertize (point-max))
    (goto-char (point-max))
    (should (equal (lyric--last-sexp-source)
                   lyric-test--block-expression))))

(ert-deftest lyric-mode-inline-block-may-end-with-a-quote ()
  (let ((source "(list \"\"\"inline \"quoted\"\"\"\")"))
    (with-temp-buffer
      (insert source)
      (lyric-mode)
      (syntax-propertize (point-max))
      (goto-char (point-max))
      (should (equal (lyric--last-sexp-source) source)))))

(provide 'lyric-mode-tests)

;;; lyric-mode-tests.el ends here
