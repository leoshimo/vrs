#!/usr/bin/env vrsctl
# github.ll - GitHub
#

# TODO: gh.ll and safari_history.ll have similar refresh + fetch pattern, which could be addressed by hypermedia client

(def pull_requests '())

(defn get_pull_requests ()
  "(get_pull_requests) - Return open pull requests"
  pull_requests)

(defn refresh_pull_requests ()
  "(refresh_pull_requests) - Fetch open pull requests"
  (def repo_result (exec "printenv" "WORK_REPO"))
  (if (not? (eq? (get repo_result :exit) 0))
    (error "WORK_REPO is not set"))
  (def repo (get (decode :lines (get repo_result :stdout)) 0))
  (def result (exec "gh" "pr" "list" "-R" repo "--json" "title,url"))
  (if (eq? (get result :exit) 0)
    (set pull_requests (decode :json (get result :stdout)))
    (error (get result :stderr)))
  :ok)

(spawn_srv :github :interface '(get_pull_requests refresh_pull_requests))
