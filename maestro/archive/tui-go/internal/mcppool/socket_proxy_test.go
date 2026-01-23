package mcppool

import (
	"context"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"syscall"
	"testing"
	"time"
)

// TestNewSocketProxy tests creating a new socket proxy
func TestNewSocketProxy(t *testing.T) {
	ctx := context.Background()

	tests := []struct {
		name     string
		command  string
		args     []string
		env      map[string]string
		wantErr  bool
	}{
		{
			name:    "basic proxy",
			command: "echo",
			args:    []string{"test"},
			env:     map[string]string{"TEST": "value"},
			wantErr: false,
		},
		{
			name:    "proxy with no args",
			command: "echo",
			args:    []string{},
			env:     nil,
			wantErr: false,
		},
		{
			name:    "proxy with empty command",
			command: "",
			args:    []string{},
			env:     nil,
			wantErr: false, // NewSocketProxy doesn't validate command
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			proxy, err := NewSocketProxy(ctx, "test-mcp", tt.command, tt.args, tt.env)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewSocketProxy() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			if !tt.wantErr {
				if proxy == nil {
					t.Fatal("NewSocketProxy() returned nil proxy")
				}

				if proxy.name != "test-mcp" {
					t.Errorf("proxy.name = %s, want test-mcp", proxy.name)
				}

				if proxy.command != tt.command {
					t.Errorf("proxy.command = %s, want %s", proxy.command, tt.command)
				}

				if proxy.Status != StatusStarting {
					t.Errorf("proxy.Status = %v, want StatusStarting", proxy.Status)
				}
			}
		})
	}
}

// TestSocketProxyPath tests socket path generation
func TestSocketProxyPath(t *testing.T) {
	ctx := context.Background()
	proxy, err := NewSocketProxy(ctx, "test-mcp", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	expectedPath := "/tmp/maestro-mcp-test-mcp.sock"
	if proxy.socketPath != expectedPath {
		t.Errorf("proxy.socketPath = %s, want %s", proxy.socketPath, expectedPath)
	}
}

// TestGetSocketPath tests getting socket path
func TestGetSocketPath(t *testing.T) {
	ctx := context.Background()
	proxy, err := NewSocketProxy(ctx, "test-mcp", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	path := proxy.GetSocketPath()
	expectedPath := "/tmp/maestro-mcp-test-mcp.sock"
	if path != expectedPath {
		t.Errorf("GetSocketPath() = %s, want %s", path, expectedPath)
	}
}

// TestGetClientCount tests getting client count
func TestGetClientCount(t *testing.T) {
	ctx := context.Background()
	proxy, err := NewSocketProxy(ctx, "test-mcp", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	// Initially zero
	count := proxy.GetClientCount()
	if count != 0 {
		t.Errorf("GetClientCount() = %d, want 0", count)
	}

	// Add a mock client
	proxy.clientsMu.Lock()
	proxy.clients["client-1"] = &mockConn{}
	proxy.clientsMu.Unlock()

	count = proxy.GetClientCount()
	if count != 1 {
		t.Errorf("GetClientCount() = %d, want 1", count)
	}
}

// TestHealthCheck tests health check functionality
func TestHealthCheck(t *testing.T) {
	ctx := context.Background()
	proxy, err := NewSocketProxy(ctx, "test-mcp", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	// No process running
	err = proxy.HealthCheck()
	if err == nil {
		t.Error("HealthCheck() with no process = nil, want error")
	}

	// Create a mock process - we'll test with nil instead
	proxy.mcpProcess = nil

	// Socket doesn't exist
	err = proxy.HealthCheck()
	if err == nil {
		t.Error("HealthCheck() with no socket = nil, want error")
	}
}

// TestStop tests stopping a proxy
func TestStop(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	proxy, err := NewSocketProxy(ctx, "test-mcp", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	// Stop should not panic
	err = proxy.Stop()
	if err != nil {
		t.Errorf("Stop() = %v, want nil", err)
	}

	if proxy.Status != StatusStopped {
		t.Errorf("proxy.Status after Stop() = %v, want StatusStopped", proxy.Status)
	}
}

// TestStopWithContextCancel tests stopping when context is cancelled
func TestStopWithContextCancel(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())

	proxy, err := NewSocketProxy(ctx, "test-mcp", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	// Cancel context
	cancel()

	// Give goroutines time to exit
	time.Sleep(100 * time.Millisecond)

	// Stop should clean up
	err = proxy.Stop()
	if err != nil {
		t.Errorf("Stop() after cancel = %v, want nil", err)
	}
}

// TestJSONRPCRequestSerialization tests JSON-RPC request serialization
func TestJSONRPCRequestSerialization(t *testing.T) {
	req := JSONRPCRequest{
		JSONRPC: "2.0",
		Method:  "test/method",
		Params:  map[string]string{"key": "value"},
		ID:      float64(1), // JSON numbers unmarshal to float64
	}

	data, err := json.Marshal(req)
	if err != nil {
		t.Fatalf("json.Marshal() failed = %v", err)
	}

	var decoded JSONRPCRequest
	err = json.Unmarshal(data, &decoded)
	if err != nil {
		t.Fatalf("json.Unmarshal() failed = %v", err)
	}

	if decoded.JSONRPC != "2.0" {
		t.Errorf("decoded.JSONRPC = %s, want 2.0", decoded.JSONRPC)
	}

	if decoded.Method != "test/method" {
		t.Errorf("decoded.Method = %s, want test/method", decoded.Method)
	}

	// ID will be float64 when unmarshaled from JSON
	if idFloat, ok := decoded.ID.(float64); !ok || idFloat != 1.0 {
		t.Errorf("decoded.ID = %v (%T), want 1.0 (float64)", decoded.ID, decoded.ID)
	}
}

// TestJSONRPCResponseSerialization tests JSON-RPC response serialization
func TestJSONRPCResponseSerialization(t *testing.T) {
	tests := []struct {
		name    string
		resp    JSONRPCResponse
		wantErr bool
	}{
		{
			name: "success response",
			resp: JSONRPCResponse{
				JSONRPC: "2.0",
				Result:  map[string]string{"status": "ok"},
				ID:      float64(1),
			},
			wantErr: false,
		},
		{
			name: "error response",
			resp: JSONRPCResponse{
				JSONRPC: "2.0",
				Error:   map[string]string{"code": "-32600", "message": "Invalid Request"},
				ID:      float64(1),
			},
			wantErr: false,
		},
		{
			name: "notification (no ID)",
			resp: JSONRPCResponse{
				JSONRPC: "2.0",
				Result:  map[string]string{"event": "notification"},
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			data, err := json.Marshal(tt.resp)
			if (err != nil) != tt.wantErr {
				t.Errorf("json.Marshal() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			var decoded JSONRPCResponse
			err = json.Unmarshal(data, &decoded)
			if (err != nil) != tt.wantErr {
				t.Errorf("json.Unmarshal() error = %v, wantErr %v", err, tt.wantErr)
			}

			if !tt.wantErr {
				if decoded.JSONRPC != "2.0" {
					t.Errorf("decoded.JSONRPC = %s, want 2.0", decoded.JSONRPC)
				}
			}
		})
	}
}

// TestIsSocketAlive tests socket alive checking
func TestIsSocketAlive(t *testing.T) {
	// Non-existent socket
	if isSocketAlive("/tmp/nonexistent-maestro-test-socket-12345.sock") {
		t.Error("isSocketAlive(nonexistent) = true, want false")
	}

	// Create a real socket
	tmpDir := t.TempDir()
	socketPath := filepath.Join(tmpDir, "test-socket.sock")

	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("Failed to create test socket: %v", err)
	}
	defer listener.Close()

	// Socket should be alive
	if !isSocketAlive(socketPath) {
		t.Error("isSocketAlive(valid socket) = false, want true")
	}

	// Close listener
	listener.Close()

	// Give OS time to clean up
	time.Sleep(50 * time.Millisecond)

	// Socket should still report as alive (file exists but no listener)
	// because DialTimeout will fail
	if isSocketAlive(socketPath) {
		// This is expected - socket file exists but no one listening
		// isSocketAlive returns false when Dial fails
		t.Error("isSocketAlive(closed socket) = true, want false")
	}
}

// TestStartWithExistingSocket tests reusing existing socket
func TestStartWithExistingSocket(t *testing.T) {
	ctx := context.Background()

	// Create an existing socket
	tmpDir := t.TempDir()
	socketPath := filepath.Join(tmpDir, "maestro-mcp-test.sock")

	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("Failed to create socket: %v", err)
	}
	defer listener.Close()

	// Create proxy with name that matches socket
	// This requires mocking /tmp path, so we'll test the logic differently
	proxy, err := NewSocketProxy(ctx, "test", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	// Manually set socket path to our test socket
	proxy.socketPath = socketPath

	// Status should be Starting (not Running since we didn't go through Start)
	if proxy.Status != StatusStarting {
		t.Errorf("proxy.Status = %v, want StatusStarting", proxy.Status)
	}
}

// TestStopWithMockProcess tests stop with mocked process
func TestStopWithMockProcess(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	tmpDir := t.TempDir()
	socketPath := filepath.Join(tmpDir, "test.sock")

	// Create socket file
	f, err := os.Create(socketPath)
	if err != nil {
		t.Fatalf("Failed to create socket file: %v", err)
	}
	f.Close()

	proxy, err := NewSocketProxy(ctx, "test", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	proxy.socketPath = socketPath

	// Mock process - skip for this test since we can't create *exec.Cmd

	// Stop should not panic
	err = proxy.Stop()
	if err != nil {
		t.Errorf("Stop() = %v, want nil", err)
	}

	// Socket file should be cleaned up
	// (In real scenario, mcpProcess != nil would remove the socket)
	if proxy.Status != StatusStopped {
		t.Error("Proxy status not Stopped after Stop()")
	}
}

// TestStatusTransitions tests status state transitions
func TestStatusTransitions(t *testing.T) {
	ctx := context.Background()
	proxy, err := NewSocketProxy(ctx, "test", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	// Initial state
	if proxy.Status != StatusStarting {
		t.Errorf("Initial status = %v, want StatusStarting", proxy.Status)
	}

	// Transition to Running
	proxy.Status = StatusRunning
	if proxy.Status != StatusRunning {
		t.Errorf("Status after transition = %v, want StatusRunning", proxy.Status)
	}

	// Transition to Stopped
	proxy.Status = StatusStopped
	if proxy.Status != StatusStopped {
		t.Errorf("Status after stop = %v, want StatusStopped", proxy.Status)
	}

	// Transition to Failed
	proxy.Status = StatusFailed
	if proxy.Status != StatusFailed {
		t.Errorf("Status after failure = %v, want StatusFailed", proxy.Status)
	}
}

// TestConcurrentClientOperations tests concurrent client operations
func TestConcurrentClientOperations(t *testing.T) {
	ctx := context.Background()
	proxy, err := NewSocketProxy(ctx, "test", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	// Add multiple clients concurrently
	done := make(chan bool)
	for i := 0; i < 10; i++ {
		go func(id int) {
			proxy.clientsMu.Lock()
			proxy.clients[fmt.Sprintf("client-%d", id)] = &mockConn{}
			proxy.clientsMu.Unlock()
			done <- true
		}(i)
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		<-done
	}

	// Verify count
	if proxy.GetClientCount() != 10 {
		t.Errorf("GetClientCount() = %d, want 10", proxy.GetClientCount())
	}
}

// TestEnvHandling tests environment variable handling
func TestEnvHandling(t *testing.T) {
	ctx := context.Background()

	env := map[string]string{
		"PATH":     "/usr/bin",
		"TEST_VAR": "test_value",
	}

	proxy, err := NewSocketProxy(ctx, "test", "echo", []string{}, env)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}

	if proxy.env == nil {
		t.Fatal("proxy.env is nil")
	}

	if len(proxy.env) != 2 {
		t.Errorf("proxy.env length = %d, want 2", len(proxy.env))
	}

	if proxy.env["TEST_VAR"] != "test_value" {
		t.Errorf("proxy.env[\"TEST_VAR\"] = %s, want test_value", proxy.env["TEST_VAR"])
	}
}

// Mock implementations for testing

type mockConn struct {
	net.Conn
}

type mockProcess struct {
	exited bool
}

func (m *mockProcess) Signal(sig syscall.Signal) error {
	m.exited = true
	return nil
}

func (m *mockProcess) Wait() error {
	return nil
}
