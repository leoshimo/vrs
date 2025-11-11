#!/usr/bin/env vrsctl
# os_cal.ll - OS Specific Calendarr

# personal calendar
(def default_calendar "610425FC-858D-412F-81AA-1E5235A3C964")

# TODO: Allow specifying calendars
(defn create_event (title start_date end_date)
  "(create_event TITLE START_DATE END_DATE) - Creates a new calendar event named TITLE.
   START_DATE and END_DATE are both quoted strings that specify start and end date.
   Valid date formats are \"1/1\", \"1/1/2024\", \"now\", \"today at 8am\", \"in one hour\", and other standard date formats."
  (exec "eventkitcli" "events" "create"
        "--calendar" default_calendar
        "--title" (str title)
        "--start-date" (str start_date)
        "--end-date" (str end_date)))

(defn get_events (start_date end_date)
  "(get_events START_DATE END_DATE) - Search for calendar events that occur between START_DATE and END_DATE
   START_DATE and END_DATE are both quoted strings that specify start and end date.
   Valid date formats are \"1/1\", \"1/1/2024\", \"now\", \"today at 8am\", \"in one hour\", and other standard date formats."
  (exec "eventkitcli" "events"
        "--start-date" (str start_date)
        "--end-date" (str end_date)))

(spawn_srv :os_cal :interface '(create_event get_events))
