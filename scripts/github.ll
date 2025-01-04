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
  (def (:ok res) (exec "./scripts/gh_pr_list.sh"))
  (set pull_requests (read res))
  :ok)

(spawn_srv :github :interface '(get_pull_requests refresh_pull_requests))
