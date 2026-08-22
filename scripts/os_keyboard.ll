#!/usr/bin/env vrsctl
# Built-in keyboard backlight on this node's MacBook.
# Uses macOS's built-in osascript and its Objective-C bridge; no helper install.

(def keyboard_script """
function run(args) {
  var setting = args[0] === 'set';
  var percent = setting ? JSON.parse(args[1]) : null;
  if (setting && (!Number.isInteger(percent) || percent < 0 || percent > 100)) {
    throw Error('Keyboard brightness must be an integer 0–100');
  }
  ObjC.import('Foundation');
  $.NSBundle.bundleWithPath('/System/Library/PrivateFrameworks/CoreBrightness.framework').load;
  var client = $.NSClassFromString('KeyboardBrightnessClient').alloc.init;
  var keyboards = ObjC.unwrap(client.copyKeyboardBacklightIDs);
  for (var i = 0; i < keyboards.length; i++) {
    var keyboard = ObjC.unwrap(keyboards[i]);
    if (!client.isKeyboardBuiltIn(keyboard)) continue;
    if (setting) {
      if (!client.setBrightnessFadeSpeedCommitForKeyboard(percent / 100, 0, true, keyboard)) {
        throw Error('Keyboard brightness change rejected');
      }
      $.NSThread.sleepForTimeInterval(0.25);
    }
    var brightness = client.brightnessForKeyboard(keyboard);
    if (!Number.isFinite(brightness) || brightness < 0 || brightness > 1) {
      throw Error('Invalid keyboard brightness readback');
    }
    return Math.round(brightness * 100);
  }
  throw Error('No built-in backlit keyboard found');
}
""")

(defn! keyboard_invoke (arguments)
  (def result (apply exec
    (+ '("/usr/bin/osascript" "-l" "JavaScript" "-") arguments
       (list :stdin keyboard_script))))
  (if (eq? (get result :exit) 0)
    (decode :json (get result :stdout))
    (error (get result :stderr))))

(defn! get_keyboard_brightness ()
  "Read the built-in MacBook keyboard brightness as an integer 0–100."
  (keyboard_invoke '("get")))

(defn! set_keyboard_brightness (percent)
  "Set the built-in MacBook keyboard brightness to an integer 0–100; 0 is off. Returns macOS readback."
  # Display preserves types: a string such as "5" is decoded and rejected.
  (keyboard_invoke (list "set" (display percent))))

(defn! toggle_keyboard_backlight ()
  "Toggle the MacBook keyboard backlight: off becomes 5%; any positive brightness becomes off."
  (set_keyboard_brightness (if (eq? (get_keyboard_brightness) 0) 5 0)))

(spawn_srv! :os_keyboard :interface
  '(get_keyboard_brightness set_keyboard_brightness toggle_keyboard_backlight))
