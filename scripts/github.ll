#!/usr/bin/env vrsctl
# github.ll - GitHub
#

# TODO: github.ll and safari_history.ll have similar refresh + fetch pattern, which could be addressed by hypermedia client

(def pull_requests '())

(defn get_pull_requests ()
  "(get_pull_requests) - Return your cached open pull requests across repositories"
  pull_requests)

(defn refresh_pull_requests ()
  "(refresh_pull_requests) - Fetch up to 100 open PRs authored by the signed-in gh user, most recently updated first"
  (def result (exec "gh" "search" "prs" "--author" "@me" "--state" "open"
                   "--sort" "updated" "--order" "desc" "--limit" "100"
                   "--json" "title,url"))
  (if (eq? (get result :exit) 0)
    (set pull_requests (decode :json (get result :stdout)))
    (error (get result :stderr)))
  :ok)

(spawn_srv :github :interface '(get_pull_requests refresh_pull_requests))
