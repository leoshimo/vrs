#!/usr/bin/env vrsctl
# Device addresses and reported ports come from Tailscale; web titles from HTTP.

(defn! tailscale_exec (args)
  (def result (apply exec (+ (list "bash" "-euo" "pipefail" "-c" """
    ts="$(command -v tailscale || true)"
    if [ -z "$ts" ]; then ts=/Applications/Tailscale.app/Contents/MacOS/Tailscale; fi
    if [ ! -x "$ts" ]; then echo 'Tailscale CLI not found' >&2; exit 1; fi
    "$ts" "$@" &
    command_pid=$!
    (sleep 2.5; kill "$command_pid" 2>/dev/null) </dev/null >/dev/null 2>&1 &
    timer_pid=$!
    status=0
    wait "$command_pid" || status=$?
    kill "$timer_pid" 2>/dev/null || true
    wait "$timer_pid" 2>/dev/null || true
    if [ "$status" -gt 128 ]; then echo 'Tailscale command timed out' >&2; fi
    exit "$status"
    """ "tailscale") args)))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Tailscale: " (get result :stderr))))
  (get result :stdout))

(defn! tailscale_parallel (items operation)
  # Each command has its own timeout; one slow endpoint does not block the rest.
  (def parent (self))
  (def batch (ref))
  (def jobs (map items (fn (item)
    (spawn (fn () (send parent (list batch (self) (try (operation item)))))))))
  (map jobs (fn (job) (get (recv (list batch job '_)) 2))))

(defn! tailscale_data (status netmap)
  (def result (exec "jq" "-sc" """
    .[0] as $status | .[1] as $netmap |
    ([($status.Self | select(. != null) | . + {self: true}),
      (($status.Peer // {})[] | . + {self: false})] | map(
        ((.DNSName // "") | rtrimstr(".")) as $dns |
        ([.TailscaleIPs[]? | select(contains(":") | not)][0]) as $ipv4 |
        ([.TailscaleIPs[]? | select(contains(":"))][0]) as $ipv6 |
        {id: (.ID // $dns), title: (if $dns != "" then $dns | split(".")[0]
                                  else .HostName // $ipv4 // $ipv6 end),
         hostname: (.HostName // ""), dns: (if $dns == "" then null else $dns end),
         ipv4: $ipv4, ipv6: $ipv6, os: (.OS // ""), self: .self,
         online: (.Online == true and $status.BackendState == "Running"), last_seen: .LastSeen}) |
      sort_by((.online | not), (.self | not), (.title | ascii_downcase))) as $devices |
    {devices: $devices, running: ($status.BackendState == "Running"),
     collection_enabled: $netmap.CollectServices,
     endpoints: ([$netmap.SelfNode, $netmap.Peers[]?] | map(select(. != null) as $node |
       $devices[] | select(.online and (.ipv4 != null or .ipv6 != null)) |
       select(.id == $node.StableID or
              (.dns != null and .dns == (($node.Name // "") | rtrimstr(".")))) as $device |
       $node.Hostinfo.Services[]? |
       select(.Proto == "tcp" and (.Port | type) == "number") |
       select(.Port > 0 and .Port < 65536 and .Port == (.Port | floor)) |
       {device_id: $device.id, device: $device.title, dns: $device.dns,
        address: ($device.ipv4 // $device.ipv6),
        host: (if $status.CurrentTailnet.MagicDNSEnabled == false
               then $device.ipv4 // $device.ipv6 else $device.dns // $device.ipv4 // $device.ipv6 end),
        port: .Port, description: (.Description // "")}) | unique_by([.device_id, .port]))}
    """ :stdin (str status "\n" netmap)))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not read Tailscale data: " (get result :stderr))))
  (decode :json (get result :stdout)))

(defn! tailscale_web_page (endpoint)
  (def result (exec "bash" "-euo" "pipefail" "-c" """
    host=$1 address=$2 port=$3 device=$4
    authority=$host
    case "$host" in *:*) authority="[$host]" ;; esac
    case "$address" in *:*) address="[$address]" ;; esac
    command -v curl >/dev/null
    for scheme in https http; do
      url="$scheme://$authority:$port/"
      set -- -q --silent --globoff --noproxy '*' --proto '=http,https'
      set -- "$@" --max-time 0.9 --max-filesize 65536 --range 0-65535
      if [ "$host" != "$address" ] && [ "$authority" != "$address" ]; then
        set -- "$@" --resolve "$host:$port:$address"
      fi
      # Leave redirects to the browser. TLS verification stays enabled.
      code=0
      response=$(curl "$@" --write-out '\n%{http_code}' --url "$url") || code=$?
      case "$code" in 0|28|63) ;; *) continue ;; esac
      status=${response##*$'\n'}
      case "$status" in [2-5][0-9][0-9]) ;; *) continue ;; esac
      printf '%s' "${response%$'\n'*}" | jq -Rs --arg url "$url" \
        --arg fallback "$device:$port" --argjson status "$status" '
        (if $status < 300 then
           ([capture("<title[^>]*>(?<title>.*?)</title>"; "is").title][0] // "") |
           gsub("&quot;"; "\"") | gsub("&apos;|&#39;"; "\u0027") |
           gsub("&lt;"; "<") | gsub("&gt;"; ">") | gsub("&nbsp;"; " ") |
           gsub("&#(?<code>[0-9]+);"; (try ([.code | tonumber] | implode) catch "")) |
           gsub("&amp;"; "&") | gsub("\\s+"; " ") | sub("^ "; "") | sub(" $"; "") | .[:160]
         else "" end) as $title |
        {title: (if $title == "" then $fallback else $title end), url: $url, status: $status}'
      exit 0
    done
    """ "tailscale-web" (get endpoint :host) (get endpoint :address)
        (str (get endpoint :port)) (get endpoint :device)))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not check web endpoint: " (get result :stderr))))
  (if (eq? (get result :stdout) "") nil
    (+ (decode :json (get result :stdout)) endpoint)))

(defn! get_tailscale_snapshot ()
  "Get devices, discovered web pages, and endpoint collection status"
  (def reports (tailscale_parallel '(("status" "--json") ("debug" "netmap")) tailscale_exec))
  (if (err? (get reports 0)) (error (display (get reports 0))))
  (def warning (if (err? (get reports 1)) (display (get reports 1)) nil))
  (def data (tailscale_data (get reports 0) (if warning "{}" (get reports 1))))
  (def endpoints (get data :endpoints))
  (def checked (tailscale_parallel endpoints tailscale_web_page))
  (def failures (filter checked err?))
  (if (and! (eq? warning nil) (not? (empty? failures)))
    (set warning (display (get failures 0))))
  (def pages (filter checked list?))
  (def summary (cond
    ((not? (get data :running)) "Tailscale disconnected")
    (warning "Endpoint discovery unavailable")
    ((eq? (get data :collection_enabled) false) "Endpoint collection disabled")
    ((empty? endpoints) "No web endpoints reported")
    ((empty? pages) "No web endpoints responded")
    (true (str (len pages) " web endpoint" (if (eq? (len pages) 1) "" "s")))))
  `(:devices ,(get data :devices) :pages ,pages :summary ,summary
    :collection_enabled ,(get data :collection_enabled)
    :reported_tcp_endpoints ,(len endpoints) :warning ,warning))

(defn! get_tailscale_devices ()
  "Get Tailscale devices and their IPv4, IPv6, MagicDNS, and online status"
  (map (get (tailscale_data (tailscale_exec '("status" "--json")) "{}") :devices)
       (fn (device) (+ '(:tailscale/device) device))))

(defn! ping_tailscale_device (device)
  "Ping a Tailscale device once and return its route and latency"
  (def address (or! (get device :ipv4) (get device :ipv6)))
  (if (eq? address nil) (error "This device has no Tailscale IP address"))
  (tailscale_exec (list "ping" "--c=1" "--timeout=2s" "--until-direct=false" "--" address)))

(set_entity_completions :tailscale/device 'get_tailscale_devices)
(spawn_srv! :tailscale
  :interface '(get_tailscale_snapshot get_tailscale_devices ping_tailscale_device))
