import SwiftUI

struct StatusView: View {
    @EnvironmentObject var appState: AppState
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                // System Status Section
                systemStatusSection
                
                // Collectors Section
                collectorsSection
                
                // Ring Buffer Section
                ringBufferSection
                
                // Connection Status Section
                connectionStatusSection
            }
        }
    }
    
    private var systemStatusSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "System Status", systemImage: "desktopcomputer")
            
            VStack(spacing: 12) {
                SystemMetricRow(
                    title: "CPU Usage",
                    value: appState.systemStatus.cpuUsage,
                    unit: "%",
                    color: colorForUsage(appState.systemStatus.cpuUsage)
                )
                
                SystemMetricRow(
                    title: "Memory Usage",
                    value: appState.systemStatus.memoryUsage,
                    unit: "%",
                    color: colorForUsage(appState.systemStatus.memoryUsage)
                )
                
                SystemMetricRow(
                    title: "Disk Usage",
                    value: appState.systemStatus.diskUsage,
                    unit: "%",
                    color: colorForUsage(appState.systemStatus.diskUsage)
                )
                
                HStack {
                    Text("Uptime")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Spacer()
                    
                    Text(formatUptime(appState.systemStatus.uptime))
                        .font(.caption)
                        .fontWeight(.medium)
                }
                
                HStack {
                    Text("Version")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Spacer()
                    
                    Text(appState.systemStatus.version)
                        .font(.caption)
                        .fontWeight(.medium)
                }
            }
            .padding(.horizontal, 8)
        }
    }
    
    private var collectorsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Collectors", systemImage: "sensor.tag.radiowaves.forward")
            
            if appState.collectors.isEmpty {
                Text("No collectors configured")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 8)
            } else {
                VStack(spacing: 8) {
                    ForEach(appState.collectors) { collector in
                        CollectorStatusRow(collector: collector) {
                            Task {
                                await appState.toggleCollector(collector.id)
                            }
                        }
                    }
                }
                .padding(.horizontal, 8)
            }
        }
    }
    
    private var ringBufferSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Ring Buffer", systemImage: "circle.dotted")
            
            VStack(spacing: 8) {
                RingBufferUsageView(stats: appState.ringBufferStats)
                
                HStack {
                    Text("Events/sec")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Spacer()
                    
                    Text(String(format: "%.1f", appState.ringBufferStats.eventsPerSecond))
                        .font(.caption)
                        .fontWeight(.medium)
                }
                
                if let oldest = appState.ringBufferStats.oldestEvent {
                    HStack {
                        Text("Oldest Event")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        Spacer()
                        
                        Text(formatRelativeTime(oldest))
                            .font(.caption)
                            .fontWeight(.medium)
                    }
                }
            }
            .padding(.horizontal, 8)
        }
    }
    
    private var connectionStatusSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Connection", systemImage: "network")
            
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Circle()
                        .fill(appState.isConnected ? Color.green : Color.red)
                        .frame(width: 8, height: 8)
                    
                    Text(appState.isConnected ? "Connected to Chronicle services" : "Disconnected")
                        .font(.caption)
                        .foregroundColor(appState.isConnected ? .primary : .red)
                }
                
                if let errorMessage = appState.errorMessage {
                    Text(errorMessage)
                        .font(.caption2)
                        .foregroundColor(.red)
                        .padding(.leading, 16)
                }
            }
            .padding(.horizontal, 8)
        }
    }
    
    // MARK: - Helper Methods
    
    private func colorForUsage(_ usage: Double) -> Color {
        if usage > 90 {
            return .red
        } else if usage > 70 {
            return .orange
        } else {
            return .green
        }
    }
    
    private func formatUptime(_ uptime: TimeInterval) -> String {
        let hours = Int(uptime) / 3600
        let minutes = Int(uptime) % 3600 / 60
        
        if hours > 0 {
            return "\(hours)h \(minutes)m"
        } else {
            return "\(minutes)m"
        }
    }
    
    private func formatRelativeTime(_ date: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.dateTimeStyle = .named
        return formatter.localizedString(for: date, relativeTo: Date())
    }
}

// MARK: - Supporting Views

struct SectionHeader: View {
    let title: String
    let systemImage: String
    
    var body: some View {
        HStack {
            Image(systemName: systemImage)
                .font(.caption)
                .foregroundColor(.accentColor)
            
            Text(title)
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.primary)
        }
    }
}

struct SystemMetricRow: View {
    let title: String
    let value: Double
    let unit: String
    let color: Color
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title)
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Spacer()
                
                Text(String(format: "%.1f%@", value, unit))
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundColor(color)
            }
            
            ProgressView(value: value / 100.0)
                .progressViewStyle(LinearProgressViewStyle(tint: color))
                .scaleEffect(y: 0.5)
        }
    }
}

struct CollectorStatusRow: View {
    let collector: CollectorStatus
    let onToggle: () -> Void
    
    var body: some View {
        HStack {
            Image(systemName: StatusIndicator.collectorStatus(from: collector).systemImage)
                .font(.caption)
                .foregroundColor(StatusIndicator.collectorStatus(from: collector).color)
            
            VStack(alignment: .leading, spacing: 2) {
                Text(collector.name)
                    .font(.caption)
                    .fontWeight(.medium)
                
                Text("\(collector.eventCount) events")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            Toggle("", isOn: .constant(collector.isEnabled))
                .toggleStyle(SwitchToggleStyle())
                .scaleEffect(0.7)
                .onTapGesture {
                    onToggle()
                }
        }
    }
}

struct RingBufferUsageView: View {
    let stats: RingBufferStats
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("Capacity")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Spacer()
                
                Text("\(stats.currentUsage) / \(stats.totalCapacity)")
                    .font(.caption)
                    .fontWeight(.medium)
            }
            
            ProgressView(value: stats.usagePercentage / 100.0)
                .progressViewStyle(LinearProgressViewStyle(tint: colorForBufferUsage(stats.usagePercentage)))
                .scaleEffect(y: 0.5)
            
            Text(String(format: "%.1f%% used", stats.usagePercentage))
                .font(.caption2)
                .foregroundColor(.secondary)
        }
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
}

// MARK: - Preview

struct StatusView_Previews: PreviewProvider {
    static var previews: some View {
        StatusView()
            .environmentObject(AppState())
            .frame(width: 350, height: 400)
    }
}