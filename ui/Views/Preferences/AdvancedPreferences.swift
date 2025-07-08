import SwiftUI

struct AdvancedPreferences: View {
    @EnvironmentObject var appState: AppState
    @AppStorage("ringBufferSize") private var ringBufferSize = 10000000
    @AppStorage("compressionLevel") private var compressionLevel = 6
    @AppStorage("encryptionEnabled") private var encryptionEnabled = true
    @AppStorage("backupInterval") private var backupInterval = 86400.0 // 24 hours
    @AppStorage("retentionDays") private var retentionDays = 30
    @AppStorage("debugMode") private var debugMode = false
    @AppStorage("enableMetrics") private var enableMetrics = true
    @AppStorage("apiPort") private var apiPort = 8080
    
    @State private var showingWipeConfirmation = false
    @State private var showingResetConfirmation = false
    @State private var wipeDays = 30
    
    private let bufferSizes = [1000000, 5000000, 10000000, 20000000, 50000000]
    private let compressionLevels = Array(1...9)
    private let backupIntervals = [3600.0, 21600.0, 43200.0, 86400.0, 604800.0] // 1h, 6h, 12h, 1d, 1w
    private let retentionOptions = [7, 14, 30, 60, 90, 180, 365]
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            SectionHeader(title: "Advanced Settings", systemImage: "wrench.and.screwdriver")
            
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    // Ring Buffer Settings
                    ringBufferSection
                    
                    Divider()
                    
                    // Storage Settings
                    storageSection
                    
                    Divider()
                    
                    // Backup Settings
                    backupSection
                    
                    Divider()
                    
                    // API Settings
                    apiSection
                    
                    Divider()
                    
                    // Debug Settings
                    debugSection
                    
                    Divider()
                    
                    // Dangerous Actions
                    dangerZone
                }
            }
        }
        .wipeDataConfirmationModal(
            isPresented: $showingWipeConfirmation,
            daysToWipe: wipeDays,
            onConfirm: wipeOldData
        )
        .confirmationModal(
            isPresented: $showingResetConfirmation,
            title: "Reset All Settings?",
            message: "This will reset all Chronicle settings to their default values. You will need to reconfigure your preferences.",
            confirmText: "Reset",
            cancelText: "Cancel",
            isDestructive: true,
            onConfirm: resetAllSettings
        )
    }
    
    private var ringBufferSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Ring Buffer")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
            
            VStack(spacing: 8) {
                PreferenceRow(
                    title: "Buffer Size",
                    description: "Maximum number of events in memory buffer"
                ) {
                    Picker("Buffer Size", selection: $ringBufferSize) {
                        ForEach(bufferSizes, id: \.self) { size in
                            Text(formatBufferSize(size)).tag(size)
                        }
                    }
                    .pickerStyle(MenuPickerStyle())
                    .frame(width: 120)
                }
                
                // Current buffer usage
                if appState.ringBufferStats.totalCapacity > 0 {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text("Current Usage")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            
                            Spacer()
                            
                            Text("\(appState.ringBufferStats.currentUsage) / \(appState.ringBufferStats.totalCapacity)")
                                .font(.caption2)
                                .fontWeight(.medium)
                        }
                        
                        ProgressView(value: appState.ringBufferStats.usagePercentage / 100.0)
                            .progressViewStyle(LinearProgressViewStyle(tint: colorForBufferUsage(appState.ringBufferStats.usagePercentage)))
                            .scaleEffect(y: 0.5)
                    }
                }
            }
        }
    }
    
    private var storageSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Storage")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
            
            VStack(spacing: 8) {
                PreferenceRow(
                    title: "Compression Level",
                    description: "Higher levels save space but use more CPU"
                ) {
                    Picker("Compression", selection: $compressionLevel) {
                        ForEach(compressionLevels, id: \.self) { level in
                            Text("\(level)").tag(level)
                        }
                    }
                    .pickerStyle(MenuPickerStyle())
                    .frame(width: 80)
                }
                
                PreferenceRow(
                    title: "Encryption",
                    description: "Encrypt stored data (recommended)"
                ) {
                    Toggle("", isOn: $encryptionEnabled)
                        .toggleStyle(SwitchToggleStyle())
                }
                
                PreferenceRow(
                    title: "Data Retention",
                    description: "How long to keep data before automatic cleanup"
                ) {
                    Picker("Retention", selection: $retentionDays) {
                        ForEach(retentionOptions, id: \.self) { days in
                            Text(formatRetentionDays(days)).tag(days)
                        }
                    }
                    .pickerStyle(MenuPickerStyle())
                    .frame(width: 100)
                }
            }
        }
    }
    
    private var backupSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Backup")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
            
            VStack(spacing: 8) {
                PreferenceRow(
                    title: "Backup Interval",
                    description: "How often to automatically create backups"
                ) {
                    Picker("Interval", selection: $backupInterval) {
                        ForEach(backupIntervals, id: \.self) { interval in
                            Text(formatBackupInterval(interval)).tag(interval)
                        }
                    }
                    .pickerStyle(MenuPickerStyle())
                    .frame(width: 100)
                }
                
                if let lastBackup = appState.backupStatus.lastBackup {
                    HStack {
                        Text("Last Backup")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        
                        Spacer()
                        
                        Text(formatRelativeTime(lastBackup))
                            .font(.caption2)
                            .fontWeight(.medium)
                    }
                }
            }
        }
    }
    
    private var apiSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("API")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
            
            VStack(spacing: 8) {
                PreferenceRow(
                    title: "API Port",
                    description: "Port for Chronicle API server"
                ) {
                    TextField("Port", value: $apiPort, format: .number)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .frame(width: 80)
                }
                
                PreferenceRow(
                    title: "Enable Metrics",
                    description: "Expose metrics endpoint for monitoring"
                ) {
                    Toggle("", isOn: $enableMetrics)
                        .toggleStyle(SwitchToggleStyle())
                }
            }
        }
    }
    
    private var debugSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Debug")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
            
            VStack(spacing: 8) {
                PreferenceRow(
                    title: "Debug Mode",
                    description: "Enable verbose logging and debug features"
                ) {
                    Toggle("", isOn: $debugMode)
                        .toggleStyle(SwitchToggleStyle())
                }
                
                HStack(spacing: 8) {
                    Button(action: openDataDirectory) {
                        HStack {
                            Image(systemName: "folder")
                            Text("Data Folder")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(ActionButtonStyle())
                    
                    Button(action: openLogDirectory) {
                        HStack {
                            Image(systemName: "doc.text")
                            Text("Logs")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(ActionButtonStyle())
                }
            }
        }
    }
    
    private var dangerZone: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Danger Zone")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.red)
            
            VStack(spacing: 8) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Wipe Old Data")
                        .font(.caption)
                        .fontWeight(.medium)
                    
                    Text("Permanently delete Chronicle data older than specified days")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    
                    HStack {
                        TextField("Days", value: $wipeDays, format: .number)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .frame(width: 60)
                        
                        Text("days")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        
                        Spacer()
                        
                        Button("Wipe Data") {
                            showingWipeConfirmation = true
                        }
                        .buttonStyle(DangerButtonStyle())
                    }
                }
                
                Button(action: {
                    showingResetConfirmation = true
                }) {
                    HStack {
                        Image(systemName: "arrow.counterclockwise")
                        Text("Reset All Settings")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(DangerButtonStyle())
            }
        }
        .padding(8)
        .background(Color.red.opacity(0.05))
        .cornerRadius(6)
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .stroke(Color.red.opacity(0.2), lineWidth: 1)
        )
    }
    
    // MARK: - Helper Methods
    
    private func formatBufferSize(_ size: Int) -> String {
        if size >= 1000000 {
            return "\(size / 1000000)M"
        } else {
            return "\(size / 1000)K"
        }
    }
    
    private func formatRetentionDays(_ days: Int) -> String {
        if days >= 365 {
            return "\(days / 365) year"
        } else if days >= 30 {
            return "\(days / 30) months"
        } else {
            return "\(days) days"
        }
    }
    
    private func formatBackupInterval(_ interval: TimeInterval) -> String {
        let hours = Int(interval) / 3600
        if hours >= 24 {
            let days = hours / 24
            return "\(days) day\(days > 1 ? "s" : "")"
        } else {
            return "\(hours) hour\(hours > 1 ? "s" : "")"
        }
    }
    
    private func formatRelativeTime(_ date: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.dateTimeStyle = .named
        return formatter.localizedString(for: date, relativeTo: Date())
    }
    
    private func colorForBufferUsage(_ usage: Double) -> Color {
        if usage > 95 {
            return .red
        } else if usage > 80 {
            return .orange
        } else {
            return .green
        }
    }
    
    private func openDataDirectory() {
        let url = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library")
            .appendingPathComponent("Application Support")
            .appendingPathComponent("Chronicle")
        
        NSWorkspace.shared.open(url)
    }
    
    private func openLogDirectory() {
        let url = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library")
            .appendingPathComponent("Logs")
            .appendingPathComponent("Chronicle")
        
        NSWorkspace.shared.open(url)
    }
    
    private func wipeOldData() {
        Task {
            do {
                try await appState.apiClient.wipeDatabaseOlderThan(days: wipeDays)
                await appState.refreshAllData()
            } catch {
                // Handle error
            }
        }
    }
    
    private func resetAllSettings() {
        ringBufferSize = 10000000
        compressionLevel = 6
        encryptionEnabled = true
        backupInterval = 86400.0
        retentionDays = 30
        debugMode = false
        enableMetrics = true
        apiPort = 8080
    }
}

// MARK: - Danger Button Style

struct DangerButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.caption)
            .fontWeight(.medium)
            .foregroundColor(.white)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(Color.red)
            .cornerRadius(6)
            .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}

// MARK: - Preview

struct AdvancedPreferences_Previews: PreviewProvider {
    static var previews: some View {
        AdvancedPreferences()
            .environmentObject(AppState())
            .frame(width: 350, height: 400)
            .padding()
    }
}