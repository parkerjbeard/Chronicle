import SwiftUI

struct CollectorPreferences: View {
    @EnvironmentObject var appState: AppState
    @AppStorage("keyboardCollectorEnabled") private var keyboardEnabled = true
    @AppStorage("mouseCollectorEnabled") private var mouseEnabled = true
    @AppStorage("screenCollectorEnabled") private var screenEnabled = true
    @AppStorage("audioCollectorEnabled") private var audioEnabled = false
    @AppStorage("fileCollectorEnabled") private var fileEnabled = true
    @AppStorage("networkCollectorEnabled") private var networkEnabled = true
    @AppStorage("windowCollectorEnabled") private var windowEnabled = true
    @AppStorage("clipboardCollectorEnabled") private var clipboardEnabled = true
    
    @State private var showingStopAllConfirmation = false
    @State private var showingStartAllConfirmation = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            SectionHeader(title: "Collector Settings", systemImage: "sensor.tag.radiowaves.forward")
            
            VStack(alignment: .leading, spacing: 12) {
                // Individual Collector Toggles
                collectorToggles
                
                Divider()
                
                // Bulk Actions
                bulkActions
                
                Divider()
                
                // Collector Statistics
                collectorStatistics
            }
            
            Spacer()
        }
        .confirmationModal(
            isPresented: $showingStopAllConfirmation,
            title: "Stop All Collectors?",
            message: "This will stop all active collectors. No new data will be collected until you restart them.",
            confirmText: "Stop All",
            cancelText: "Cancel",
            isDestructive: false,
            onConfirm: stopAllCollectors
        )
        .confirmationModal(
            isPresented: $showingStartAllConfirmation,
            title: "Start All Collectors?",
            message: "This will start all collectors that are currently stopped.",
            confirmText: "Start All",
            cancelText: "Cancel",
            isDestructive: false,
            onConfirm: startAllCollectors
        )
    }
    
    private var collectorToggles: some View {
        VStack(spacing: 8) {
            CollectorPreferenceRow(
                title: "Keyboard Monitor",
                description: "Track keyboard activity and key presses",
                isEnabled: $keyboardEnabled,
                isRequired: true,
                collector: findCollector("keyboard")
            )
            
            CollectorPreferenceRow(
                title: "Mouse Monitor",
                description: "Track mouse movement and clicks",
                isEnabled: $mouseEnabled,
                isRequired: true,
                collector: findCollector("mouse")
            )
            
            CollectorPreferenceRow(
                title: "Screen Monitor",
                description: "Track screen changes and capture events",
                isEnabled: $screenEnabled,
                isRequired: true,
                collector: findCollector("screen")
            )
            
            CollectorPreferenceRow(
                title: "Window Monitor",
                description: "Track window focus and application switches",
                isEnabled: $windowEnabled,
                isRequired: true,
                collector: findCollector("window")
            )
            
            CollectorPreferenceRow(
                title: "File System Monitor",
                description: "Track file system changes and operations",
                isEnabled: $fileEnabled,
                isRequired: true,
                collector: findCollector("filesystem")
            )
            
            CollectorPreferenceRow(
                title: "Network Monitor",
                description: "Track network connections and activity",
                isEnabled: $networkEnabled,
                isRequired: false,
                collector: findCollector("network")
            )
            
            CollectorPreferenceRow(
                title: "Clipboard Monitor",
                description: "Track clipboard changes and content",
                isEnabled: $clipboardEnabled,
                isRequired: false,
                collector: findCollector("clipboard")
            )
            
            CollectorPreferenceRow(
                title: "Audio Monitor",
                description: "Track audio input and output events",
                isEnabled: $audioEnabled,
                isRequired: false,
                collector: findCollector("audio")
            )
        }
    }
    
    private var bulkActions: some View {
        VStack(spacing: 8) {
            Text("Bulk Actions")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
            
            HStack(spacing: 8) {
                Button(action: {
                    showingStartAllConfirmation = true
                }) {
                    HStack {
                        Image(systemName: "play.fill")
                        Text("Start All")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(ActionButtonStyle())
                
                Button(action: {
                    showingStopAllConfirmation = true
                }) {
                    HStack {
                        Image(systemName: "stop.fill")
                        Text("Stop All")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(ActionButtonStyle())
            }
            
            Button(action: restartAllCollectors) {
                HStack {
                    Image(systemName: "arrow.clockwise")
                    Text("Restart All")
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(ActionButtonStyle())
        }
    }
    
    private var collectorStatistics: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Statistics")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
            
            VStack(spacing: 4) {
                StatisticRow(
                    title: "Active Collectors",
                    value: "\(activeCollectorCount) / \(appState.collectors.count)",
                    color: .green
                )
                
                StatisticRow(
                    title: "Total Events",
                    value: "\(totalEventCount)",
                    color: .blue
                )
                
                StatisticRow(
                    title: "Events per Second",
                    value: String(format: "%.1f", appState.ringBufferStats.eventsPerSecond),
                    color: .orange
                )
            }
        }
    }
    
    // MARK: - Computed Properties
    
    private var activeCollectorCount: Int {
        appState.collectors.filter { $0.isEnabled }.count
    }
    
    private var totalEventCount: Int {
        appState.collectors.reduce(0) { $0 + $1.eventCount }
    }
    
    // MARK: - Helper Methods
    
    private func findCollector(_ type: String) -> CollectorStatus? {
        appState.collectors.first { $0.id.lowercased().contains(type.lowercased()) }
    }
    
    private func startAllCollectors() {
        Task {
            for collector in appState.collectors.filter({ !$0.isEnabled }) {
                await appState.toggleCollector(collector.id)
            }
        }
    }
    
    private func stopAllCollectors() {
        Task {
            for collector in appState.collectors.filter({ $0.isEnabled }) {
                await appState.toggleCollector(collector.id)
            }
        }
    }
    
    private func restartAllCollectors() {
        Task {
            // First stop all collectors
            for collector in appState.collectors.filter({ $0.isEnabled }) {
                await appState.toggleCollector(collector.id)
            }
            
            // Wait a moment
            try? await Task.sleep(nanoseconds: 1_000_000_000)
            
            // Then start them again
            for collector in appState.collectors.filter({ !$0.isEnabled }) {
                await appState.toggleCollector(collector.id)
            }
        }
    }
}

// MARK: - Collector Preference Row

struct CollectorPreferenceRow: View {
    let title: String
    let description: String
    @Binding var isEnabled: Bool
    let isRequired: Bool
    let collector: CollectorStatus?
    
    var body: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(title)
                        .font(.caption)
                        .fontWeight(.medium)
                    
                    if isRequired {
                        Text("REQUIRED")
                            .font(.caption2)
                            .fontWeight(.bold)
                            .foregroundColor(.red)
                            .padding(.horizontal, 4)
                            .padding(.vertical, 1)
                            .background(Color.red.opacity(0.1))
                            .cornerRadius(3)
                    }
                }
                
                Text(description)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                
                if let collector = collector {
                    HStack {
                        StatusIndicatorComponent(
                            status: StatusIndicator.collectorStatus(from: collector),
                            size: 12
                        )
                        
                        Text("\(collector.eventCount) events")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                Toggle("", isOn: $isEnabled)
                    .toggleStyle(SwitchToggleStyle())
                    .disabled(!isRequired && collector?.status == .error)
                
                if let collector = collector {
                    Text(collector.status.rawValue.capitalized)
                        .font(.caption2)
                        .foregroundColor(StatusIndicator.collectorStatus(from: collector).color)
                }
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background(
            RoundedRectangle(cornerRadius: 6)
                .fill(isEnabled ? Color.green.opacity(0.05) : Color.gray.opacity(0.05))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .stroke(isEnabled ? Color.green.opacity(0.2) : Color.gray.opacity(0.2), lineWidth: 1)
        )
    }
}

// MARK: - Preview

struct CollectorPreferences_Previews: PreviewProvider {
    static var previews: some View {
        CollectorPreferences()
            .environmentObject(AppState())
            .frame(width: 350, height: 400)
            .padding()
    }
}