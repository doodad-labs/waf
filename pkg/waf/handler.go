package waf

import (
    "bytes"
    "fmt"
    "io"
    "net/http/httputil"
)

// RequestScreening determines whether a request should be forwarded to the backend.
func RequestScreening(r *httputil.ProxyRequest) (bool, error) {
    
    	bodyBytes, _ := io.ReadAll(r.In.Body)
	r.In.Body = io.NopCloser(bytes.NewBuffer(bodyBytes))

	fmt.Printf(`
		[WAF INSPECTION] Incoming Request:
		- Source IP: %s
		- Method: %s
		- URL: %s
		- Headers: %#v
		- Cookies: %#v
		- Query: %#v
		- Body: %q
		- JA3: %s
		- JA4: %s
		- HTTP2 FP: %s
	`,
		r.In.RemoteAddr,
		r.In.Method,
		r.In.URL.String(),
		r.In.Header,
		r.In.Cookies(),
		r.In.URL.Query(),
		string(bodyBytes),
		r.Out.Header.Get("X-JA3-Fingerprint"),
		r.Out.Header.Get("X-JA4-Fingerprint"),
		r.Out.Header.Get("X-HTTP2-Fingerprint"),
	)


    return true, nil
}