package main

import (
	"log/slog"
	"net"
	"net/http"
	"os"
	"strconv"
	"strings"

	"github.com/didip/tollbooth/v8"
	"github.com/didip/tollbooth/v8/libstring"
	"github.com/didip/tollbooth/v8/limiter"
)

func clientIPForWSRateLimit(r *http.Request) string {
	if s := strings.TrimSpace(r.Header.Get("CF-Connecting-IP")); s != "" {
		return libstring.CanonicalizeIP(s)
	}
	if s := strings.TrimSpace(r.Header.Get("X-Forwarded-For")); s != "" {
		parts := strings.Split(s, ",")
		for i := range parts {
			parts[i] = strings.TrimSpace(parts[i])
		}
		if len(parts) > 0 && parts[0] != "" {
			return libstring.CanonicalizeIP(parts[0])
		}
	}
	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil {
		return libstring.CanonicalizeIP(strings.TrimSpace(r.RemoteAddr))
	}
	return libstring.CanonicalizeIP(host)
}

func newWSRateLimiter(rpm int) *limiter.Limiter {
	max := float64(rpm) / 60.0
	return tollbooth.NewLimiter(max, nil)
}

func wsRateLimitMiddleware(lmt *limiter.Limiter, next http.HandlerFunc) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		ip := clientIPForWSRateLimit(r)
		if ip == "" {
			ip = "unknown"
		}
		keys := []string{ip, r.URL.Path}
		if httpErr := tollbooth.LimitByKeys(lmt, keys); httpErr != nil {
			slog.Warn("websocket rate limit exceeded", "remote_ip", ip, "path", r.URL.Path)
			lmt.ExecOnLimitReached(w, r)
			if lmt.GetOverrideDefaultResponseWriter() {
				return
			}
			w.Header().Add("Content-Type", lmt.GetMessageContentType())
			w.WriteHeader(httpErr.StatusCode)
			_, _ = w.Write([]byte(httpErr.Message))
			return
		}
		next.ServeHTTP(w, r)
	})
}

func parseWSRateLimitRPM() int {
	raw := strings.TrimSpace(os.Getenv("GATEWAY_WS_RATE_LIMIT_RPM"))
	if raw == "" {
		return 5
	}
	n, err := strconv.Atoi(raw)
	if err != nil {
		slog.Warn("invalid GATEWAY_WS_RATE_LIMIT_RPM; using default", "value", raw, "default", 5)
		return 5
	}
	return n
}
