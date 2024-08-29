#!/usr/bin/env bash
# youtube_ytdlp_shim - Shim Bash Script
#

set -euo pipefail

# TODO: Improve ergonomics around defining shell scripts directly in lyric
echo "("
for channel_path in ~/Downloads/yt-dlp/*; do
    channel_name=$(basename "$channel_path")
    for video_path in "${channel_path}"/*; do
        video_name=$(basename "$video_path")
        video_webm_path="${video_path}/${video_name}.webm"
        echo "(:title \"${channel_name} - ${video_name}\" :path \"$video_webm_path\")"
    done
done
echo ")"
