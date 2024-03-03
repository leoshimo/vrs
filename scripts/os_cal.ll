#!/usr/bin/env vrsctl
# os_cal.ll - OS Specific Calendarr

(defn add_event (title start_date end_date)
  "(add_event TITLE START_DATE END_DATE) - Add an event to calendar named TITLE.
   START_DATE and END_DATE are both strings that specify start and end date.
   Valid date formats are \"1/1\", \"1/1/2024\", \"now\", \"today at 8am\", \"in one hour\", and other standard date formats."
  (exec "eventkitcli" "add-event"
        "--title" title
        "--start-date" start_date
        "--end-date" end_date))

(spawn_srv :os_cal :interface '(add_event))

# DEMO: Run + Test in Editor
