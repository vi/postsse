# postsse
HTTP POST and Server-sent events interconnector, allowing simple publish-subsribe scheme using GET and POST requests.

# Features

* Listening for HTTP 1.1 and HTTP 2 connections and handling GET or POST requests on arbitrary URL.
* Broadcasting each POSTed message to each ongoing GET request as a SSE event.
* Each URL path gets a separate broadcast domain.

This is intended to be used as a building block, especially during development or prototyping.

# Limitations

* If receiver reads messages slowly, messages get dropped. If you want adjustable buffer size or other stategies (buffer endlessly or slow down senders), open a Github issue with a feature request.
* No DoS resistance. Each encountered URL path is remembered forever, there are no limitations for message size or number of paths or receivers. 

# Installation

Download pre-built executables from [Github releases](https://github.com/vi/postsse/releases/), install it from source code with `cargo install --path .` or from crates.io with `cargo install postsse`.

# Example

```
$ postsse 127.0.0.1:1234

$ curl -v http://127.0.0.1:1234/
*   Trying 127.0.0.1:1234...
* Connected to 127.0.0.1 (127.0.0.1) port 1234 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:1234
> User-Agent: curl/7.74.0
> Accept: */*
>
* Mark bundle as not supporting multiuse
< HTTP/1.1 200 OK
< content-type: text/event-stream
< access-control-allow-origin: *
< transfer-encoding: chunked
< date: Mon, 24 Oct 2022 21:58:36 GMT
<
data: 123        | $ curl http://127.0.0.1:1234/ -d 123
                 |
data: qwerty     | $ curl http://127.0.0.1:1234/ -d qwerty
                 |
data: ABC        | $ curl http://127.0.0.1:1234/ \
data: DEF        |     --data-binary $'ABC\nDEF\n'
```

# See also

* https://github.com/vi/wsbroad - WebSocket analogue. If needed, it is not hard to create a combined publish-subscriber with both POST/SSE and WebSockets.
