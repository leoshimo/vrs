(defmacro defn (name params & body)
  "Define a named function, with optional documentation and interactive metadata."
  `(def ,name (fn ,params ,@body)))

(defmacro when (test & body)
  "Evaluate BODY when TEST is true."
  `(if ,test (begin ,@body) nil))

(defmacro and (& expressions)
  "Evaluate left to right until a false value; return the last evaluated value."
  (if (empty? expressions) true
    (if (eq? (len expressions) 1) (get expressions 0)
      (let ((value (gensym "value")))
        `(let ((,value ,(get expressions 0)))
           (if ,value (and! ,@(slice expressions 1)) ,value))))))

(defmacro or (& expressions)
  "Evaluate left to right until a true value; return the last evaluated value."
  (if (empty? expressions) nil
    (if (eq? (len expressions) 1) (get expressions 0)
      (let ((value (gensym "value")))
        `(let ((,value ,(get expressions 0)))
           (if ,value ,value (or! ,@(slice expressions 1))))))))
