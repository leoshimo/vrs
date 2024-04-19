#!/usr/bin/env vrsctl
# nl_scheduler.ll - Schedule my day in natural language
#

(bind_srv :os_cal)
(bind_srv :os_notify)
(bind_srv :nl_shell)

(defn scheduler_codegen (day)
  "(scheduler_codegen DAY) - Generate code for scheduler. DAY can be a date specifier like \"today\", \"tomorrow\", \"August 5th\""

  (def day_schedule (get (get_events (format "{} at 0h" day)
                                     (format "{} at 24h" day)) 1))

  (codegen (format "The following events are my calendar events {}:
{}

Schedule one or more events for {} in line with the preferences below:

Do not schedule over existing events.

Schedule my commute to and from the office.
I want to be at the office before 9:45 AM. My commute is roughly 20 minutes.
I prefer to be home by 6 PM if possible.

Schedule one or more focus blocks on my calendar. The ideal length is 2 hours, although it can be flex-ed down to 1 hour if needed around other events.
Do not schedule focus blocks shorter than 1 hour.
Focus blocks can occur anytime after my commute to office, and before my commute back home

Expect a 30 minute lunch break anytime after noon.
" day day_schedule day)))

(defn schedule_the_day (day)
  "(schedule_the_day DAY) - Schedules my day. DAY can be a date specifier like \"today\", \"tomorrow\", \"August 5th\""
  (notify (format "Scheduling your day for {}" day) "thinking...")
  (spawn (fn ()
           (def code (try (scheduler_codegen day)))
           (if (ok? code)
             (begin
              (publish :code code) 
              (eval code))
             (notify "Encountered error :(" (format "{}" code)))))
  :ok)

(spawn_srv :nl_scheduler :interface '(schedule_the_day))

