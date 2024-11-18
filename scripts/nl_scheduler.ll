#!/usr/bin/env vrsctl
# nl_scheduler.ll - Schedule my day in natural language
#

(bind_srv :os_cal)
(bind_srv :os_notify)
(bind_srv :nl_shell)

(defn scheduler_prompt (day)
  "(scheduler_prompt DAY) - Generate prompt for scheduler. DAY can be a date specifier like \"today\", \"tomorrow\", \"August 5th\""

  (def day_schedule (get (get_events (format "{} at 0h" day)
                                     (format "{} at 24h" day)) 1))
  (if (eq? day_schedule "")
    (set day_schedule "NO EVENTS"))

  (def date (get (exec "date") -1))

  (format "Today is {}

The following events are my calendar events {}:
{}

Schedule one or more events for {} in line with the preferences below:

Do not schedule over existing events.
Do not schedule events if it already exists on the calendar.

Anytime after 1pm, add a 20 minute block to read emails.

Avoid scheduling any events during 12:00 to 1PM to set aside time for lunch.

If I have a one-on-one meeting, which may have titles like \"1:1\", \"PersonA / Leo\", \"PersonB & Leo\", \"1on1\", \"PersonC // Leo\", set aside 10 minutes earlier in the day to prepare for it.

If I have a interview with candidate, which may have titles like \"Intro Chat\", set aside 15 minutes before event to prepare for it.

" date day day_schedule day))

(defn schedule_the_day (day)
  "(schedule_the_day DAY) - Schedules my day. DAY can be a date specifier like \"today\", \"tomorrow\", \"August 5th\""
  (notify (format "Scheduling your day for {}" day) "thinking...")
  (spawn (fn ()
           (def code (try (codegen (scheduler_prompt day))))
           (if (ok? code)
             (begin
              (publish :code code) 
              (eval code))
             (notify "Encountered error :(" (format "{}" code)))))
  :ok)

(spawn_srv :nl_scheduler :interface '(schedule_the_day))
