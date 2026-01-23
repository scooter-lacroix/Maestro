package mcppool

import (
	"context"
	"net"
	"path/filepath"
	"sync"
	"testing"
	"time"
)

// TestPoolConfig tests pool configuration
func TestPoolConfig(t *testing.T) {
	tests := []struct {
		name   string
		config *PoolConfig
		mcp    string
		want   bool
	}{
		{
			name: "disabled pool",
			config: &PoolConfig{
				Enabled: false,
			},
			mcp:  "test-mcp",
			want: false,
		},
		{
			name: "pool all with no exclusions",
			config: &PoolConfig{
				Enabled:     true,
				PoolAll:     true,
				ExcludeMCPs: []string{},
			},
			mcp:  "test-mcp",
			want: true,
		},
		{
			name: "pool all with exclusion",
			config: &PoolConfig{
				Enabled:     true,
				PoolAll:     true,
				ExcludeMCPs: []string{"test-mcp"},
			},
			mcp:  "test-mcp",
			want: false,
		},
		{
			name: "pool all with different exclusion",
			config: &PoolConfig{
				Enabled:     true,
				PoolAll:     true,
				ExcludeMCPs: []string{"other-mcp"},
			},
			mcp:  "test-mcp",
			want: true,
		},
		{
			name: "specific pool list - included",
			config: &PoolConfig{
				Enabled:  true,
				PoolAll:  false,
				PoolMCPs: []string{"test-mcp", "other-mcp"},
			},
			mcp:  "test-mcp",
			want: true,
		},
		{
			name: "specific pool list - not included",
			config: &PoolConfig{
				Enabled:  true,
				PoolAll:  false,
				PoolMCPs: []string{"other-mcp"},
			},
			mcp:  "test-mcp",
			want: false,
		},
		{
			name: "fallback enabled",
			config: &PoolConfig{
				Enabled:       true,
				PoolAll:       true,
				FallbackStdio: true,
			},
			mcp:  "test-mcp",
			want: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx := context.Background()
			pool, err := NewPool(ctx, tt.config)
			if err != nil {
				t.Fatalf("NewPool() failed = %v", err)
			}

			got := pool.ShouldPool(tt.mcp)
			if got != tt.want {
				t.Errorf("Pool.ShouldPool() = %v, want %v", got, tt.want)
			}

			// Test FallbackEnabled
			if tt.config.FallbackStdio && !pool.FallbackEnabled() {
				t.Errorf("Pool.FallbackEnabled() = false, want true")
			}
		})
	}
}

// TestNewPool tests pool creation
func TestNewPool(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{
		Enabled: true,
		PoolAll: true,
	}

	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	if pool == nil {
		t.Fatal("NewPool() returned nil pool")
	}

	if pool.config != config {
		t.Errorf("Pool.config = %p, want %p", pool.config, config)
	}

	if pool.proxies == nil {
		t.Error("Pool.proxies is nil")
	}

	if len(pool.proxies) != 0 {
		t.Errorf("Pool.proxies length = %d, want 0", len(pool.proxies))
	}
}

// TestListServers tests listing servers
func TestListServers(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	// Initially empty
	servers := pool.ListServers()
	if len(servers) != 0 {
		t.Errorf("ListServers() length = %d, want 0", len(servers))
	}

	// Add a mock proxy
	pool.mu.Lock()
	pool.proxies["test-mcp"] = &SocketProxy{
		name:       "test-mcp",
		socketPath: "/tmp/test.sock",
		Status:     StatusRunning,
		clients:    make(map[string]net.Conn),
	}
	pool.mu.Unlock()

	servers = pool.ListServers()
	if len(servers) != 1 {
		t.Fatalf("ListServers() length = %d, want 1", len(servers))
	}

	if servers[0].Name != "test-mcp" {
		t.Errorf("Server[0].Name = %s, want test-mcp", servers[0].Name)
	}

	if servers[0].SocketPath != "/tmp/test.sock" {
		t.Errorf("Server[0].SocketPath = %s, want /tmp/test.sock", servers[0].SocketPath)
	}

	if servers[0].Status != "running" {
		t.Errorf("Server[0].Status = %s, want running", servers[0].Status)
	}
}

// TestGetURL tests getting socket URLs
func TestGetURL(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	// Non-existent proxy
	url := pool.GetURL("nonexistent")
	if url != "" {
		t.Errorf("GetURL(nonexistent) = %s, want empty string", url)
	}

	// Add a proxy
	expectedPath := "/tmp/test-mcp.sock"
	pool.mu.Lock()
	pool.proxies["test-mcp"] = &SocketProxy{
		name:       "test-mcp",
		socketPath: expectedPath,
		Status:     StatusRunning,
		clients:    make(map[string]net.Conn),
	}
	pool.mu.Unlock()

	url = pool.GetURL("test-mcp")
	if url != expectedPath {
		t.Errorf("GetURL(test-mcp) = %s, want %s", url, expectedPath)
	}

	// Test GetSocketPath alias
	path := pool.GetSocketPath("test-mcp")
	if path != expectedPath {
		t.Errorf("GetSocketPath(test-mcp) = %s, want %s", path, expectedPath)
	}
}

// TestIsRunning tests checking if proxy is running
func TestIsRunning(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	// Non-existent proxy
	if pool.IsRunning("nonexistent") {
		t.Error("IsRunning(nonexistent) = true, want false")
	}

	// Proxy with status not running
	proxy, err := NewSocketProxy(ctx, "test-mcp", "echo", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}
	proxy.Status = StatusStarting

	pool.mu.Lock()
	pool.proxies["test-mcp"] = proxy
	pool.mu.Unlock()

	if pool.IsRunning("test-mcp") {
		t.Error("IsRunning(test-mcp) = true (starting), want false")
	}

	// Test with a real socket
	tmpDir := t.TempDir()
	socketPath := filepath.Join(tmpDir, "test-socket.sock")

	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("Failed to create test socket: %v", err)
	}
	defer listener.Close()

	// Register the socket
	err = pool.RegisterExternalSocket("test-real", socketPath)
	if err != nil {
		t.Fatalf("RegisterExternalSocket() = %v", err)
	}

	// Should return true for live socket
	if !pool.IsRunning("test-real") {
		t.Error("IsRunning(test-real) with live socket = false, want true")
	}
}

// TestRestartProxy tests restarting a proxy
func TestRestartProxy(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	// Restart non-existent proxy
	err = pool.RestartProxy("nonexistent")
	if err == nil {
		t.Error("RestartProxy(nonexistent) = nil, want error")
	}

	// Add a proxy using NewSocketProxy (properly initialized)
	proxy, err := NewSocketProxy(ctx, "test-mcp", "", []string{}, nil)
	if err != nil {
		t.Fatalf("NewSocketProxy() failed = %v", err)
	}
	proxy.Status = StatusRunning

	pool.mu.Lock()
	pool.proxies["test-mcp"] = proxy
	pool.mu.Unlock()

	// Should fail to restart (no command)
	err = pool.RestartProxy("test-mcp")
	if err == nil {
		t.Error("RestartProxy(test-mcp without command) = nil, want error")
	}
}

// TestDiscoverExistingSockets tests socket discovery
func TestDiscoverExistingSockets(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	// Create a temporary socket
	tmpDir := t.TempDir()
	socketPath := filepath.Join(tmpDir, "maestro-mcp-test-mcp.sock")

	// Create a Unix socket listener
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("Failed to create test socket: %v", err)
	}
	defer listener.Close()

	// Mock the pattern to use our temp dir
	// We'll test RegisterExternalSocket directly instead
	err = pool.RegisterExternalSocket("test-mcp", socketPath)
	if err != nil {
		t.Fatalf("RegisterExternalSocket() failed = %v", err)
	}

	// Verify it was registered
	if !pool.IsRunning("test-mcp") {
		t.Error("IsRunning(test-mcp) = false after registration, want true")
	}

	// Try to register again (should be idempotent)
	err = pool.RegisterExternalSocket("test-mcp", socketPath)
	if err != nil {
		t.Errorf("RegisterExternalSocket() duplicate = %v, want nil", err)
	}
}

// TestShutdown tests pool shutdown
func TestShutdown(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	// Add some properly initialized proxies
	for i := 0; i < 3; i++ {
		name := filepath.Join("test-mcp", string(rune('a'+i)))
		proxy, err := NewSocketProxy(ctx, name, "", []string{}, nil)
		if err != nil {
			t.Fatalf("NewSocketProxy() failed = %v", err)
		}
		proxy.Status = StatusRunning

		pool.mu.Lock()
		pool.proxies[name] = proxy
		pool.mu.Unlock()
	}

	// Shutdown should not panic
	err = pool.Shutdown()
	if err != nil {
		t.Errorf("Shutdown() = %v, want nil", err)
	}

	// Context should be cancelled
	select {
	case <-pool.ctx.Done():
		// Expected
	default:
		t.Error("Pool context not cancelled after Shutdown()")
	}
}

// TestConcurrentAccess tests concurrent pool operations
func TestConcurrentAccess(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true, PoolAll: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	var wg sync.WaitGroup
	iterations := 100

	// Concurrent reads
	for i := 0; i < iterations; i++ {
		wg.Add(1)
		go func(n int) {
			defer wg.Done()
			mcpName := filepath.Join("mcp", string(rune('a'+(n%26))))
			_ = pool.ShouldPool(mcpName)
			_ = pool.GetURL(mcpName)
			_ = pool.IsRunning(mcpName)
		}(i)
	}

	// Concurrent writes
	for i := 0; i < iterations; i++ {
		wg.Add(1)
		go func(n int) {
			defer wg.Done()
			name := filepath.Join("test-mcp", string(rune('a'+(n%10))))
			proxy, _ := NewSocketProxy(ctx, name, "", []string{}, nil)
			proxy.Status = StatusRunning

			pool.mu.Lock()
			pool.proxies[name] = proxy
			pool.mu.Unlock()
		}(i)
	}

	wg.Wait()

	// Verify no race conditions occurred
	servers := pool.ListServers()
	if len(servers) < 10 {
		t.Errorf("After concurrent operations, expected at least 10 servers, got %d", len(servers))
	}
}

// TestServerStatusString tests ServerStatus string representation
func TestServerStatusString(t *testing.T) {
	tests := []struct {
		status ServerStatus
		want   string
	}{
		{StatusStopped, "stopped"},
		{StatusStarting, "starting"},
		{StatusRunning, "running"},
		{StatusFailed, "failed"},
		{ServerStatus(999), "unknown"},
	}

	for _, tt := range tests {
		t.Run(tt.want, func(t *testing.T) {
			got := tt.status.String()
			if got != tt.want {
				t.Errorf("ServerStatus(%d).String() = %s, want %s", tt.status, got, tt.want)
			}
		})
	}
}

// TestIsSocketAliveCheck tests socket alive check helper
func TestIsSocketAliveCheck(t *testing.T) {
	// Non-existent socket
	if isSocketAliveCheck("/tmp/nonexistent-maestro-test-socket-12345.sock") {
		t.Error("isSocketAliveCheck(nonexistent) = true, want false")
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
	if !isSocketAliveCheck(socketPath) {
		t.Error("isSocketAliveCheck(valid socket) = false, want true")
	}

	// Close the socket
	listener.Close()

	// Socket should no longer be alive after a short delay
	time.Sleep(100 * time.Millisecond)
	if isSocketAliveCheck(socketPath) {
		t.Error("isSocketAliveCheck(closed socket) = true, want false")
	}
}

// TestStartIdempotent tests that starting the same proxy twice is idempotent
func TestStartIdempotent(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	// Use a mock command that exits immediately
	pool.mu.Lock()
	pool.proxies["test-mcp"] = &SocketProxy{
		name:       "test-mcp",
		socketPath: filepath.Join(t.TempDir(), "test-mcp.sock"),
		command:    "echo",
		args:       []string{"test"},
		env:        make(map[string]string),
		Status:     StatusRunning,
		clients:    make(map[string]net.Conn),
	}
	pool.mu.Unlock()

	// Starting again should return nil (already exists)
	err = pool.Start("test-mcp", "echo", []string{"test"}, nil)
	if err != nil {
		t.Errorf("Start(already running) = %v, want nil", err)
	}
}

// TestProxyInfo tests ProxyInfo struct
func TestProxyInfo(t *testing.T) {
	info := ProxyInfo{
		Name:       "test-mcp",
		SocketPath: "/tmp/test.sock",
		Status:     "running",
		Clients:    5,
	}

	if info.Name != "test-mcp" {
		t.Errorf("ProxyInfo.Name = %s, want test-mcp", info.Name)
	}
	if info.SocketPath != "/tmp/test.sock" {
		t.Errorf("ProxyInfo.SocketPath = %s, want /tmp/test.sock", info.SocketPath)
	}
	if info.Status != "running" {
		t.Errorf("ProxyInfo.Status = %s, want running", info.Status)
	}
	if info.Clients != 5 {
		t.Errorf("ProxyInfo.Clients = %d, want 5", info.Clients)
	}
}

// TestPoolWithTempSocket tests pool operations with temporary sockets
func TestPoolWithTempSocket(t *testing.T) {
	ctx := context.Background()
	config := &PoolConfig{Enabled: true}
	pool, err := NewPool(ctx, config)
	if err != nil {
		t.Fatalf("NewPool() failed = %v", err)
	}

	tmpDir := t.TempDir()
	socketPath := filepath.Join(tmpDir, "maestro-mcp-test.sock")

	// Create a socket
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("Failed to create socket: %v", err)
	}
	defer listener.Close()

	// Register it
	err = pool.RegisterExternalSocket("test", socketPath)
	if err != nil {
		t.Fatalf("RegisterExternalSocket() = %v", err)
	}

	// Verify we can get the path
	path := pool.GetSocketPath("test")
	if path != socketPath {
		t.Errorf("GetSocketPath() = %s, want %s", path, socketPath)
	}

	// List servers should include it
	servers := pool.ListServers()
	if len(servers) != 1 {
		t.Fatalf("ListServers() length = %d, want 1", len(servers))
	}

	if servers[0].Name != "test" {
		t.Errorf("Server[0].Name = %s, want test", servers[0].Name)
	}
}
