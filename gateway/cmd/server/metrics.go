package main

import (
	"encoding/json"
	"log/slog"
	"net/http"

	gatewaymetrics "gateway/internal/metrics"
)

func metricsHandler(registry *gatewaymetrics.Registry) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		if err := json.NewEncoder(w).Encode(registry.Response()); err != nil {
			slog.Warn("failed to encode gateway metrics", "error", err)
		}
	}
}
