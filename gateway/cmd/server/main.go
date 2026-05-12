package main

import (
	"context"
	"fmt"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"gateway/internal/hub"
)

func main() {
	level := parseGatewayLogLevel(os.Getenv("GATEWAY_LOG_LEVEL"))
	logger := slog.New(slog.NewJSONHandler(os.Stderr, &slog.HandlerOptions{
		Level: level,
	}))
	slog.SetDefault(logger)
	slog.Info("gateway logger initialized", "level", level.String())

	ln, err := net.Listen("tcp", ":0")
	if err != nil {
		slog.Error("failed to bind listener", "error", err)
		os.Exit(1)
	}

	port := ln.Addr().(*net.TCPAddr).Port

	h := hub.New()

	mux := http.NewServeMux()
	mux.HandleFunc("/ws", h.HandleWS)
	mux.HandleFunc("/rooms", h.HandleCreateRoom)
	mux.HandleFunc("/end-session", h.HandleEndSession)
	mux.HandleFunc("/health", func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
	})

	srv := &http.Server{Handler: mux}
	serveErr := make(chan error, 1)
	go func() {
		serveErr <- srv.Serve(ln)
	}()

	slog.Info("gateway started", "port", port)
	fmt.Printf("{\"port\":%d}\n", port)

	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGTERM, syscall.SIGINT)

	select {
	case err := <-serveErr:
		if err != nil && err != http.ErrServerClosed {
			slog.Error("gateway serve failed", "error", err)
			os.Exit(1)
		}
	case <-sig:
		slog.Info("shutdown signal received")
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := srv.Shutdown(ctx); err != nil {
			slog.Error("graceful shutdown failed", "error", err)
		}
	}
}
func parseGatewayLogLevel(raw string) slog.Level {
	switch strings.ToLower(strings.TrimSpace(raw)) {
	case "debug", "trace":
		return slog.LevelDebug
	case "info", "":
		return slog.LevelInfo
	case "warn", "warning":
		return slog.LevelWarn
	case "error", "off":
		return slog.LevelError
	default:
		return slog.LevelInfo
	}
}
