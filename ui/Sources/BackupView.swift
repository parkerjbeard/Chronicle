import SwiftUI

struct BackupView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingExportDialog = false
    @State private var exportFormat: ExportFormat = .json
    @State private var exportDateRange = DateInterval(start: Calendar.current.date(byAdding: .month, value: -1, to: Date()) ?? Date(), end: Date())
    @State private var isExporting = false
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                // Backup Status Section
                backupStatusSection
                
                // Manual Backup Section
                manualBackupSection
                
                // Export Section
                exportSection
                
                // Backup History Section
                backupHistorySection
            }
        }
    }
    
    private var backupStatusSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Backup Status", systemImage: "externaldrive")
            
            VStack(spacing: 8) {
                HStack {
                    StatusIndicatorComponent(
                        status: appState.backupStatus.isRunning ? .warning : .good,
                        size: 16
                    )
                    
                    Text(appState.backupStatus.isRunning ? "Backup in progress..." : "Ready")
                        .font(.caption)
                        .fontWeight(.medium)
                    
                    Spacer()
                    
                    if appState.backupStatus.isRunning {
                        ProgressView()
                            .scaleEffect(0.5)
                            .frame(width: 12, height: 12)
                    }
                }
                
                if let lastBackup = appState.backupStatus.lastBackup {
                    BackupInfoRow(
                        title: "Last Backup",
                        value: formatBackupDate(lastBackup),
                        systemImage: "clock"
                    )
                }
                
                if let nextBackup = appState.backupStatus.nextScheduledBackup {
                    BackupInfoRow(
                        title: "Next Scheduled",
                        value: formatBackupDate(nextBackup),
                        systemImage: "clock.arrow.circlepath"
                    )
                }
                
                BackupInfoRow(
                    title: "Total Backups",
                    value: "\(appState.backupStatus.totalBackups)",
                    systemImage: "doc.on.doc"
                )
                
                if appState.backupStatus.lastBackupSize > 0 {
                    BackupInfoRow(
                        title: "Last Backup Size",
                        value: ByteCountFormatter.string(fromByteCount: appState.backupStatus.lastBackupSize, countStyle: .binary),
                        systemImage: "internaldrive"
                    )
                }
                
                if appState.backupStatus.averageBackupTime > 0 {
                    BackupInfoRow(
                        title: "Average Duration",
                        value: String(format: "%.1f seconds", appState.backupStatus.averageBackupTime),
                        systemImage: "timer"
                    )
                }
            }
            .padding(.horizontal, 8)
        }
    }
    
    private var manualBackupSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Manual Backup", systemImage: "play.fill")
            
            VStack(spacing: 8) {
                Text("Create an immediate backup of current data")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                Button(action: {
                    Task {
                        await appState.startBackup()
                    }
                }) {
                    HStack {
                        Image(systemName: "play.fill")
                        Text("Start Backup Now")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(ActionButtonStyle(isPrimary: true))
                .disabled(appState.backupStatus.isRunning)
            }
            .padding(.horizontal, 8)
        }
    }
    
    private var exportSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Export Data", systemImage: "square.and.arrow.up")
            
            VStack(spacing: 8) {
                Text("Export Chronicle data for analysis or migration")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                HStack {
                    Text("Format:")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Picker("Format", selection: $exportFormat) {
                        ForEach(ExportFormat.allCases, id: \.self) { format in
                            Text(format.displayName).tag(format)
                        }
                    }
                    .pickerStyle(MenuPickerStyle())
                }
                
                VStack(alignment: .leading, spacing: 4) {
                    Text("Date Range:")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    HStack {
                        DatePicker("From", selection: .constant(exportDateRange.start), displayedComponents: .date)
                            .datePickerStyle(CompactDatePickerStyle())
                        
                        Text("to")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        
                        DatePicker("To", selection: .constant(exportDateRange.end), displayedComponents: .date)
                            .datePickerStyle(CompactDatePickerStyle())
                    }
                    .font(.caption)
                }
                
                Button(action: {
                    showingExportDialog = true
                }) {
                    HStack {
                        Image(systemName: "square.and.arrow.up")
                        Text("Export Data")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(ActionButtonStyle())
                .disabled(isExporting)
            }
            .padding(.horizontal, 8)
        }
    }
    
    private var backupHistorySection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Backup Statistics", systemImage: "chart.bar")
            
            VStack(spacing: 8) {
                StatisticRow(
                    title: "Total Backups",
                    value: "\(appState.backupStatus.totalBackups)",
                    color: .blue
                )
                
                if appState.backupStatus.averageBackupTime > 0 {
                    StatisticRow(
                        title: "Average Duration",
                        value: String(format: "%.1fs", appState.backupStatus.averageBackupTime),
                        color: .green
                    )
                }
                
                if appState.backupStatus.lastBackupSize > 0 {
                    StatisticRow(
                        title: "Last Size",
                        value: ByteCountFormatter.string(fromByteCount: appState.backupStatus.lastBackupSize, countStyle: .binary),
                        color: .orange
                    )
                }
            }
            .padding(.horizontal, 8)
        }
    }
    
    // MARK: - Helper Methods
    
    private func formatBackupDate(_ date: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.dateTimeStyle = .named
        return formatter.localizedString(for: date, relativeTo: Date())
    }
}

// MARK: - Supporting Views

struct BackupInfoRow: View {
    let title: String
    let value: String
    let systemImage: String
    
    var body: some View {
        HStack {
            Image(systemName: systemImage)
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(width: 16)
            
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
            
            Spacer()
            
            Text(value)
                .font(.caption)
                .fontWeight(.medium)
        }
    }
}

struct StatisticRow: View {
    let title: String
    let value: String
    let color: Color
    
    var body: some View {
        HStack {
            Rectangle()
                .fill(color)
                .frame(width: 3, height: 12)
            
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
            
            Spacer()
            
            Text(value)
                .font(.caption)
                .fontWeight(.medium)
                .foregroundColor(color)
        }
    }
}

// MARK: - Preview

struct BackupView_Previews: PreviewProvider {
    static var previews: some View {
        BackupView()
            .environmentObject(AppState())
            .frame(width: 350, height: 400)
    }
}