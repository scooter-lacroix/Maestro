package session

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

const (
	// DefaultProfile is the name of the default profile
	DefaultProfile = "default"

	// ProfilesDirName is the directory containing all profiles
	ProfilesDirName = "profiles"

	// ConfigFileName is the global config file name
	ConfigFileName = "config.json"
)

// Config represents the global maestro-tui configuration
type Config struct {
	// DefaultProfile is the profile to use when none is specified
	DefaultProfile string `json:"default_profile"`

	// LastUsed is the most recently used profile (for future use)
	LastUsed string `json:"last_used,omitempty"`

	// Version tracks config format for future migrations
	Version int `json:"version"`
}

// GetMaestroDir returns the base maestro directory (~/.maestro)
// STANDARDIZED: Changed from .maestro-tui to .maestro for consistency with Python migration
func GetMaestroDir() (string, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}
	return filepath.Join(homeDir, ".maestro"), nil
}

// GetConfigPath returns the path to the global config file
func GetConfigPath() (string, error) {
	dir, err := GetMaestroDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, ConfigFileName), nil
}

// GetProfilesDir returns the path to the profiles directory
func GetProfilesDir() (string, error) {
	dir, err := GetMaestroDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, ProfilesDirName), nil
}

// GetProfileDir returns the path to a specific profile's directory
// SECURITY: Validates profile name to prevent path traversal attacks
func GetProfileDir(profile string) (string, error) {
	if profile == "" {
		profile = DefaultProfile
	}

	// SECURITY: Prevent path traversal in profile names
	// Check for path separators and dangerous patterns
	if strings.ContainsAny(profile, "/\\") {
		return "", fmt.Errorf("invalid profile name: contains path separators: %s", profile)
	}

	// Check for absolute paths
	if filepath.IsAbs(profile) {
		return "", fmt.Errorf("invalid profile name: absolute path not allowed: %s", profile)
	}

	// Check for directory traversal components
	if strings.Contains(profile, "..") {
		return "", fmt.Errorf("invalid profile name: contains '..' component: %s", profile)
	}

	// Use filepath.Base as final safety measure
	profile = filepath.Base(profile)
	if profile == "." || profile == ".." {
		return "", fmt.Errorf("invalid profile name: %s", profile)
	}

	// Ensure profile name is not empty after sanitization
	if profile == "" {
		return "", fmt.Errorf("invalid profile name: empty after sanitization")
	}

	profilesDir, err := GetProfilesDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(profilesDir, profile), nil
}

// LoadConfig loads the global configuration
func LoadConfig() (*Config, error) {
	configPath, err := GetConfigPath()
	if err != nil {
		return nil, err
	}

	// Check if config exists
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		// Return default config
		return &Config{
			DefaultProfile: DefaultProfile,
			Version:        1,
		}, nil
	}

	data, err := os.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config: %w", err)
	}

	var config Config
	if err := json.Unmarshal(data, &config); err != nil {
		return nil, fmt.Errorf("failed to parse config: %w", err)
	}

	// Ensure default profile is set
	if config.DefaultProfile == "" {
		config.DefaultProfile = DefaultProfile
	}

	return &config, nil
}

// SaveConfig saves the global configuration
func SaveConfig(config *Config) error {
	configPath, err := GetConfigPath()
	if err != nil {
		return err
	}

	// Ensure directory exists
	dir := filepath.Dir(configPath)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return fmt.Errorf("failed to create config directory: %w", err)
	}

	data, err := json.MarshalIndent(config, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	if err := os.WriteFile(configPath, data, 0600); err != nil {
		return fmt.Errorf("failed to write config: %w", err)
	}

	return nil
}

// ListProfiles returns all available profile names
func ListProfiles() ([]string, error) {
	profilesDir, err := GetProfilesDir()
	if err != nil {
		return nil, err
	}

	// Check if profiles directory exists
	if _, err := os.Stat(profilesDir); os.IsNotExist(err) {
		// No profiles yet - check if we need migration
		return []string{}, nil
	}

	entries, err := os.ReadDir(profilesDir)
	if err != nil {
		return nil, fmt.Errorf("failed to read profiles directory: %w", err)
	}

	var profiles []string
	for _, entry := range entries {
		if entry.IsDir() {
			// Verify it has a sessions.json file (valid profile)
			sessionsPath := filepath.Join(profilesDir, entry.Name(), "sessions.json")
			if _, err := os.Stat(sessionsPath); err == nil {
				profiles = append(profiles, entry.Name())
			}
		}
	}

	sort.Strings(profiles)
	return profiles, nil
}

// ProfileExists checks if a profile exists
func ProfileExists(profile string) (bool, error) {
	profileDir, err := GetProfileDir(profile)
	if err != nil {
		return false, err
	}

	sessionsPath := filepath.Join(profileDir, "sessions.json")
	_, err = os.Stat(sessionsPath)
	if os.IsNotExist(err) {
		return false, nil
	}
	if err != nil {
		return false, err
	}
	return true, nil
}

// CreateProfile creates a new empty profile
func CreateProfile(profile string) error {
	// Validate profile name
	if profile == "" {
		return fmt.Errorf("profile name cannot be empty")
	}

	// Check if already exists
	exists, err := ProfileExists(profile)
	if err != nil {
		return err
	}
	if exists {
		return fmt.Errorf("profile '%s' already exists", profile)
	}

	profileDir, err := GetProfileDir(profile)
	if err != nil {
		return err
	}

	// Create profile directory
	if err := os.MkdirAll(profileDir, 0700); err != nil {
		return fmt.Errorf("failed to create profile directory: %w", err)
	}

	// Create empty sessions.json
	emptyData := StorageData{
		Instances: []*InstanceData{},
		Groups:    []*GroupData{},
	}

	data, err := json.MarshalIndent(emptyData, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal empty sessions: %w", err)
	}

	sessionsPath := filepath.Join(profileDir, "sessions.json")
	if err := os.WriteFile(sessionsPath, data, 0600); err != nil {
		return fmt.Errorf("failed to create sessions file: %w", err)
	}

	return nil
}

// DeleteProfile deletes a profile and all its data
func DeleteProfile(profile string) error {
	// Prevent deleting the default profile if it's the only one
	if profile == DefaultProfile {
		profiles, err := ListProfiles()
		if err != nil {
			return err
		}
		if len(profiles) <= 1 {
			return fmt.Errorf("cannot delete the only remaining profile")
		}
	}

	profileDir, err := GetProfileDir(profile)
	if err != nil {
		return err
	}

	// Check if profile exists
	exists, err := ProfileExists(profile)
	if err != nil {
		return err
	}
	if !exists {
		return fmt.Errorf("profile '%s' does not exist", profile)
	}

	// Remove the profile directory
	if err := os.RemoveAll(profileDir); err != nil {
		return fmt.Errorf("failed to delete profile: %w", err)
	}

	// Update config if this was the default profile
	config, err := LoadConfig()
	if err != nil {
		return err
	}
	if config.DefaultProfile == profile {
		config.DefaultProfile = DefaultProfile
		if err := SaveConfig(config); err != nil {
			return fmt.Errorf("profile deleted but failed to update config: %w", err)
		}
	}

	return nil
}

// SetDefaultProfile sets the default profile in the config
func SetDefaultProfile(profile string) error {
	// Verify profile exists
	exists, err := ProfileExists(profile)
	if err != nil {
		return err
	}
	if !exists {
		return fmt.Errorf("profile '%s' does not exist", profile)
	}

	config, err := LoadConfig()
	if err != nil {
		return err
	}

	config.DefaultProfile = profile
	return SaveConfig(config)
}

// GetEffectiveProfile returns the profile to use, considering:
// 1. Explicitly provided profile (from -p flag)
// 2. Environment variable AGENTDECK_PROFILE
// 3. Config default profile
// 4. Fallback to "default"
func GetEffectiveProfile(explicit string) string {
	if explicit != "" {
		return explicit
	}

	if envProfile := os.Getenv("AGENTDECK_PROFILE"); envProfile != "" {
		return envProfile
	}

	config, err := LoadConfig()
	if err != nil {
		return DefaultProfile
	}

	if config.DefaultProfile != "" {
		return config.DefaultProfile
	}

	return DefaultProfile
}
