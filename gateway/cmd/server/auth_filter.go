package main

import (
	"crypto/subtle"
	"net/http"
)

func bearerAuthFilter(token string, next http.Handler) http.Handler {
	expected := "Bearer " + token
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/health", "/ws":
			next.ServeHTTP(w, r)
			return
		}
		if subtle.ConstantTimeCompare(
			[]byte(r.Header.Get("Authorization")),
			[]byte(expected),
		) != 1 {
			http.Error(w, "unauthorized", http.StatusUnauthorized)
			return
		}
		next.ServeHTTP(w, r)
	})
}
