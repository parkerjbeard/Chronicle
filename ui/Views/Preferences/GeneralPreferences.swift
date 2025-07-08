import SwiftUI

struct GeneralPreferences: View {
    @AppStorage("autoStart") private var autoStart = true
    @AppStorage("showNotifications") private var showNotifications = true
    @AppStorage("logLevel") private var logLevel = "info"
    @AppStorage("updateInterval") private var updateInterval = 5.0
    @AppStorage("showMenuBarIcon") private var showMenuBarIcon = true
    @AppStorage("startMinimized") private var startMinimized = false
    @AppStorage("checkForUpdates") private var checkForUpdates = true
    
    private let logLevels = ["debug", "info", "warn", "error"]
    private let updateIntervals = [1.0, 2.0, 5.0, 10.0, 30.0]
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            SectionHeader(title: "General Settings", systemImage: "gear")
            
            VStack(alignment: .leading, spacing: 12) {
                // Auto Start
                PreferenceRow(
                    title: "Auto Start",
                    description: "Launch Chronicle automatically when you log in"
                ) {
                    Toggle("", isOn: $autoStart)
                        .toggleStyle(SwitchToggleStyle())
                }
                
                // Show Notifications
                PreferenceRow(
                    title: "Show Notifications",
                    description: "Display system notifications for important events"
                ) {
                    Toggle("", isOn: $showNotifications)
                        .toggleStyle(SwitchToggleStyle())
                }
                
                // Menu Bar Icon
                PreferenceRow(
                    title: "Show Menu Bar Icon",
                    description: "Display Chronicle icon in the menu bar"
                ) {
                    Toggle("", isOn: $showMenuBarIcon)
                        .toggleStyle(SwitchToggleStyle())
                }
                
                // Start Minimized
                PreferenceRow(
                    title: "Start Minimized",
                    description: "Launch Chronicle in the background without opening the menu"
                ) {
                    Toggle("", isOn: $startMinimized)
                        .toggleStyle(SwitchToggleStyle())
                }
                
                // Check for Updates
                PreferenceRow(
                    title: "Check for Updates",
                    description: "Automatically check for Chronicle updates"
                ) {
                    Toggle("", isOn: $checkForUpdates)
                        .toggleStyle(SwitchToggleStyle())
                }
                
                Divider()
                
                // Log Level
                PreferenceRow(
                    title: "Log Level",
                    description: "Set the verbosity of Chronicle logging"
                ) {
                    Picker("Log Level", selection: $logLevel) {
                        ForEach(logLevels, id: \.self) { level in
                            Text(level.capitalized).tag(level)
                        }
                    }
                    .pickerStyle(MenuPickerStyle())
                    .frame(width: 100)
                }
                
                // Update Interval
                PreferenceRow(
                    title: "Update Interval",
                    description: "How often to refresh status information (seconds)"
                ) {
                    Picker("Update Interval", selection: $updateInterval) {
                        ForEach(updateIntervals, id: \.self) { interval in
                            Text(formatInterval(interval)).tag(interval)
                        }
                    }
                    .pickerStyle(MenuPickerStyle())
                    .frame(width: 100)
                }
            }
            
            Spacer()
            
            // Action Buttons
            VStack(spacing: 8) {
                Button(action: resetToDefaults) {
                    HStack {
                        Image(systemName: "arrow.counterclockwise")
                        Text("Reset to Defaults")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(ActionButtonStyle())
                
                Button(action: openLogDirectory) {
                    HStack {
                        Image(systemName: "folder")
                        Text("Open Log Directory")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(ActionButtonStyle())
            }
        }
    }
    
    private func formatInterval(_ interval: Double) -> String {
        if interval < 60 {
            return "\(Int(interval))s"
        } else {
            return "\(Int(interval / 60))m"
        }
    }
    
    private func resetToDefaults() {
        autoStart = true
        showNotifications = true
        logLevel = "info"
        updateInterval = 5.0
        showMenuBarIcon = true
        startMinimized = false
        checkForUpdates = true
    }
    
    private func openLogDirectory() {
        let url = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library")
            .appendingPathComponent("Logs")
            .appendingPathComponent("Chronicle")
        
        NSWorkspace.shared.open(url)
    }
}

// MARK: - Preference Row Component

struct PreferenceRow<Content: View>: View {
    let title: String
    let description: String
    let content: Content
    
    init(title: String, description: String, @ViewBuilder content: () -> Content) {
        self.title = title
        self.description = description
        self.content = content()
    }
    
    var body: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.caption)
                    .fontWeight(.medium)
                
                Text(description)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            
            Spacer()
            
            content
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Preview

struct GeneralPreferences_Previews: PreviewProvider {
    static var previews: some View {
        GeneralPreferences()
            .frame(width: 350, height: 400)
            .padding()
    }
}