#!/usr/bin/env vrsctl
# youtube.ll - Youtube Extensions
#

(bind_srv :os_notify)
(bind_srv :os_browser)

(def ytdlp_download_dir (shell_expand "~/Downloads/yt-dlp"))
(def ytdlp_download_filename_template "%(uploader)s/%(title)s/%(title)s.%(ext)s")

(defn download_video (url)
  "(download_video URL) - Downloads video at URL"
  (if (not? (contains? url "youtube"))
    (notify "Error" (format "{} is not a Youtube URL" url))
    (begin
     (spawn (fn ()
              (notify "yt-dlp" (format "Downloading \n{}" url))
              (def result (exec "yt-dlp"
                                "--quiet"
                                "--write-webloc-link"
                                "-o" (format "{}/{}" ytdlp_download_dir ytdlp_download_filename_template)
                                url))
              (if (eq? (get result :exit) 0)
                (notify "yt-dlp" (format "Downloaded \n{}" url))
                (notify "yt-dlp" (format "Error \n{}" (get result :stderr))))))
    :ok)))

(defn download_video_active_tab ()
  "(download_video_active_tab) - Downloads video at current active tab"
  (download_video (get (active_tab) :url)))

(defn list_videos ()
  "(list_videos) - List available youtube videos"
  (def result (exec "find" ytdlp_download_dir "-type" "f" "-name" "*.webm"))
  (if (eq? (get result :exit) 0)
    (map (decode :lines (get result :stdout))
         (fn (path)
           (begin
            (def parts (split "/" path))
            (list :title (format "{} - {}" (get parts -3) (get parts -2))
                  :path path))))
    '()))

# TODO: Write about iterative dev experience? Took ~15m?
# - Inspiration - flying to japan
# - Install yt-dlp
# - Use cogni to spit out shell cmd from doc
# - Iterate to impl of (download_video xxx) in buffer itself. Source URL was sourced via os_browser
# - Iterate on (list_videos)
# - Add safeguard - (not? (contains? url "youtube"))
# - Spin up youtube servic
# - Integrate with vrsjmp via service

(spawn_srv :youtube :interface '(download_video download_video_active_tab list_videos))
