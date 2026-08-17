#!/usr/bin/env vrsctl
# AppKit exposes user-facing applications without scraping System Events.

(defn! get_running_apps ()
  "List running applications as :os/app objects"
  (def result (exec "osascript" "-l" "JavaScript" :stdin """
    ObjC.import('AppKit');
    const apps = $.NSWorkspace.sharedWorkspace.runningApplications;
    const result = [];
    for (let i = 0; i < apps.count; i++) {
      const app = apps.objectAtIndex(i);
      if (Number(app.activationPolicy) !== 0 || app.terminated || app.processIdentifier < 1) continue;
      const launched = ObjC.unwrap(app.launchDate);
      if (!launched) continue;
      const title = ObjC.unwrap(app.localizedName) || '';
      const bundle = ObjC.unwrap(app.bundleIdentifier) || '';
      if (title.toLowerCase() === 'vrsjmp') continue;
      result.push({title, pid: app.processIdentifier, bundle_id: bundle,
                   started_at: String(launched.getTime())});
    }
    JSON.stringify(result.sort((a, b) => a.title.localeCompare(b.title)));
    """))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not list running apps: " (get result :stderr))))
  (map (decode :json (get result :stdout)) (fn (app) (+ '(:os/app) app))))

(defn! force_quit_app (app)
  "Force Quit"
  (interactive :os/app)
  (def result (exec "osascript" "-l" "JavaScript" "-"
    (str (get app :pid)) (get app :bundle_id) (get app :started_at) :stdin """
    ObjC.import('AppKit');
    function run(argv) {
      const app = $.NSRunningApplication.runningApplicationWithProcessIdentifier(Number(argv[0]));
      if (!app || app.isNil() || app.terminated) throw Error('This app is no longer running');
      const launched = ObjC.unwrap(app.launchDate);
      if ((ObjC.unwrap(app.bundleIdentifier) || '') !== argv[1] || !launched ||
          String(launched.getTime()) !== argv[2])
        throw Error('The selected app has restarted; choose it again');
      if (Number(app.activationPolicy) !== 0 || String(ObjC.unwrap(app.localizedName)).toLowerCase() === 'vrsjmp')
        throw Error('Not a selectable application');
      if (!app.forceTerminate) throw Error('macOS refused to force quit this app');
      return 'ok';
    }
    """))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not force quit app: " (get result :stderr))))
  result)

(set_entity_completions :os/app 'get_running_apps)
(spawn_srv! :os_apps :interface '(get_running_apps force_quit_app))
