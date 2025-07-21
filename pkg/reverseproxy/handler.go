// Package `reverseproxy` forwards the requests to backends. It gets
// additional request headers from `header_injectors`, and adds to the
// forwarding request to downstream.
package reverseproxy

import (
	"fmt"
	"bytes"
	"net"
	"io"
	"log"
	"net/http"
	"net/http/httputil"
	"net/url"
	"strings"

	"github.com/google/uuid"
)

type HTTPHandler struct {
	// required, internal reverse proxy that forwards the requests
	reverseProxy *httputil.ReverseProxy

	// required, the URL that requests will be forwarding to
	To *url.URL

	// optional, preserve the host in outbound requests
	PreserveHost bool

	// optional, but in fact required, injecting fingerprint headers to outbound requests
	HeaderInjectors []HeaderInjector

	// optional, if IsProbeRequest returns true, handler will respond with
	// a HTTP 200 OK instead of forwarding requests, useful for kubernetes
	// liveness/readiness probes. defaults to nil, which disables this behavior
	IsProbeRequest func(*http.Request) bool
}

const (
	ProbeStatusCode = http.StatusOK
	ProbeResponse   = "OK"
)

// NewHTTPHandler creates an HTTP handler, changes `reverseProxy.Rewrite` to support request
// header injection, then assigns `reverseProxy` to the handler which proxies requests to backend
func NewHTTPHandler(to *url.URL, reverseProxy *httputil.ReverseProxy, headerInjectors []HeaderInjector) *HTTPHandler {
	f := &HTTPHandler{
		To:              to,
		reverseProxy:    reverseProxy,
		HeaderInjectors: headerInjectors,	
	} 

	f.reverseProxy.Rewrite = f.rewriteFunc
	return f
}

// getClientIP returns the client's real IP from X-Forwarded-For or RemoteAddr.
// Returns the first valid IP in X-Forwarded-For (comma-separated list) or RemoteAddr if none found.
func getClientIP(r *http.Request) string {
    // 1. Check X-Forwarded-For (could be comma-separated list)
    xff := r.Header.Get("X-Forwarded-For")
    if xff != "" {
        // Split into potential IPs (e.g., "client, proxy1, proxy2")
        ips := strings.Split(xff, ",")
        for _, ip := range ips {
            ip = strings.TrimSpace(ip)
            if net.ParseIP(ip) != nil { // Validate it's a real IP
                return ip
            }
        }
    }

    // 2. Fall back to RemoteAddr (format: "IP:port" or "[IPv6]:port")
    ip, _, err := net.SplitHostPort(r.RemoteAddr)
    if err != nil {
        // Handle cases where RemoteAddr has no port (unlikely in HTTP servers)
        return r.RemoteAddr
    }
    return ip
}

func (f *HTTPHandler) rewriteFunc(r *httputil.ProxyRequest) {
	r.SetURL(f.To)
	
	requestId := uuid.New().String()

	r.Out.Header.Set("X-Request-ID", requestId)
	r.Out.Header.Set("X-Forwarded-For", getClientIP(r.In))

	if f.PreserveHost {
		r.Out.Host = r.In.Host
	}

	for _, hj := range f.HeaderInjectors {
		k := hj.GetHeaderName()
		if v, err := hj.GetHeaderValue(r.In); err != nil {
			f.logf("get header %s value for %s failed: %s", k, r.In.RemoteAddr, err)
		} else if v != "" { // skip empty header values
			r.Out.Header.Set(k, v)
		}
	}

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
}

func (f *HTTPHandler) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	if f.IsProbeRequest != nil && f.IsProbeRequest(req) {
		w.WriteHeader(ProbeStatusCode)
		w.Write([]byte(ProbeResponse))
		return
	}
	f.reverseProxy.ServeHTTP(w, req)
}

func IsKubernetesProbeRequest(r *http.Request) bool {
	// https://github.com/kubernetes/kubernetes/blob/656cb1028ea5af837e69b5c9c614b008d747ab63/pkg/probe/http/request.go#L91
	return strings.HasPrefix(r.UserAgent(), "kube-probe/")
}

func (f *HTTPHandler) logf(format string, args ...any) {
	if f.reverseProxy.ErrorLog != nil {
		f.reverseProxy.ErrorLog.Printf(format, args...)
	} else {
		log.Printf(format, args...)
	}
}
