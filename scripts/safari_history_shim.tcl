#!/usr/bin/env tclsh
# safari_history_shim.tcl - Shim for Safari History SQLite and VRS
#
# TODO: Embed sqlite bindings in VRS?
# TODO: Ergonomics for embedded scripts within lyric lang?

package require sqlite3

set output "(\n"

catch {
    sqlite3 historydb "$env(HOME)/Library/Safari/History.db"
    historydb eval {
        SELECT datetime(visit_time + 978307200, 'unixepoch', 'localtime') as local_visit_time, TRIM(title) as trim_title, url, domain_expansion
        FROM history_visits
        JOIN history_items ON history_visits.history_item = history_items.id
        WHERE LENGTH(TRIM(COALESCE(title, ''))) > 0
            AND LENGTH(TRIM(COALESCE(url, ''))) > 0
            AND LENGTH(url) < 500
            AND url NOT LIKE "%/search%"
            AND url NOT LIKE "%read.amazon.co.jp%"
        GROUP BY history_item
        ORDER BY visit_time DESC, visit_count_score DESC
        LIMIT 250
    } {
        append output "(:safari_history_item :title \"$trim_title\" :url \"$url\" :domain_expansion \"$domain_expansion\")"
    }
}

append output ")"

historydb close

puts "$output"
