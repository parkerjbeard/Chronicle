import SwiftUI

struct CollectorToggle: View {
    let collector: CollectorStatus
    let onToggle: () -> Void
    @State private var isToggling = false
    
    var body: some View {
        HStack {
            // Collector Icon and Status
            HStack(spacing: 8) {
                StatusIndicatorComponent(
                    status: StatusIndicator.collectorStatus(from: collector),
                    size: 16
                )
                
                VStack(alignment: .leading, spacing: 2) {
                    Text(collector.name)
                        .font(.caption)
                        .fontWeight(.medium)
                    
                    Text(statusText)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            // Toggle Control
            VStack(alignment: .trailing, spacing: 2) {
                Toggle("", isOn: .constant(collector.isEnabled))
                    .toggleStyle(SwitchToggleStyle())
                    .scaleEffect(0.8)
                    .disabled(isToggling)
                    .onTapGesture {
                        toggleCollector()
                    }
                
                if isToggling {
                    ProgressView()
                        .scaleEffect(0.3)
                        .frame(width: 12, height: 12)
                } else {
                    Text("\(collector.eventCount) events")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background(
            RoundedRectangle(cornerRadius: 6)
                .fill(backgroundColorForStatus(collector.status))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .stroke(borderColorForStatus(collector.status), lineWidth: 1)
        )
    }
    
    private var statusText: String {
        if isToggling {
            return "Updating..."
        }
        
        switch collector.status {
        case .healthy:
            return collector.isEnabled ? "Running" : "Stopped"
        case .warning:
            return "Warning"
        case .error:
            return "Error"
        case .disabled:
            return "Disabled"
        }
    }
    
    private func toggleCollector() {
        isToggling = true
        
        // Add haptic feedback
        let impactFeedback = NSHapticFeedbackManager.defaultPerformer
        impactFeedback.perform(.alignment, performanceTime: .now)
        
        // Perform the toggle
        onToggle()
        
        // Reset the toggling state after a delay
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            isToggling = false
        }
    }
    
    private func backgroundColorForStatus(_ status: CollectorStatus.CollectorHealthStatus) -> Color {
        switch status {
        case .healthy:
            return Color.green.opacity(0.05)
        case .warning:
            return Color.orange.opacity(0.05)
        case .error:
            return Color.red.opacity(0.05)
        case .disabled:
            return Color.gray.opacity(0.05)
        }
    }
    
    private func borderColorForStatus(_ status: CollectorStatus.CollectorHealthStatus) -> Color {
        switch status {
        case .healthy:
            return Color.green.opacity(0.2)
        case .warning:
            return Color.orange.opacity(0.2)
        case .error:
            return Color.red.opacity(0.2)
        case .disabled:
            return Color.gray.opacity(0.2)
        }
    }
}

// MARK: - Compact Collector Toggle

struct CompactCollectorToggle: View {
    let collector: CollectorStatus
    let onToggle: () -> Void
    @State private var isToggling = false
    
    var body: some View {
        HStack {
            StatusIndicatorComponent(
                status: StatusIndicator.collectorStatus(from: collector),
                size: 12
            )
            
            Text(collector.name)
                .font(.caption2)
                .fontWeight(.medium)
            
            Spacer()
            
            if isToggling {
                ProgressView()
                    .scaleEffect(0.3)
                    .frame(width: 12, height: 12)
            } else {
                Toggle("", isOn: .constant(collector.isEnabled))
                    .toggleStyle(SwitchToggleStyle())
                    .scaleEffect(0.6)
                    .onTapGesture {
                        isToggling = true
                        onToggle()
                        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
                            isToggling = false
                        }
                    }
            }
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 4)
    }
}

// MARK: - Collector Grid Toggle

struct CollectorGridToggle: View {
    let collector: CollectorStatus
    let onToggle: () -> Void
    @State private var isToggling = false
    
    var body: some View {
        VStack(spacing: 6) {
            StatusIndicatorComponent(
                status: StatusIndicator.collectorStatus(from: collector),
                size: 20
            )
            
            Text(collector.name)
                .font(.caption2)
                .fontWeight(.medium)
                .multilineTextAlignment(.center)
                .lineLimit(2)
            
            if isToggling {
                ProgressView()
                    .scaleEffect(0.5)
                    .frame(width: 16, height: 16)
            } else {
                Toggle("", isOn: .constant(collector.isEnabled))
                    .toggleStyle(SwitchToggleStyle())
                    .scaleEffect(0.7)
                    .onTapGesture {
                        isToggling = true
                        onToggle()
                        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
                            isToggling = false
                        }
                    }
            }
        }
        .frame(width: 80, height: 80)
        .padding(8)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(backgroundColorForStatus(collector.status))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(borderColorForStatus(collector.status), lineWidth: 1)
        )
    }
    
    private func backgroundColorForStatus(_ status: CollectorStatus.CollectorHealthStatus) -> Color {
        switch status {
        case .healthy:
            return Color.green.opacity(0.05)
        case .warning:
            return Color.orange.opacity(0.05)
        case .error:
            return Color.red.opacity(0.05)
        case .disabled:
            return Color.gray.opacity(0.05)
        }
    }
    
    private func borderColorForStatus(_ status: CollectorStatus.CollectorHealthStatus) -> Color {
        switch status {
        case .healthy:
            return Color.green.opacity(0.2)
        case .warning:
            return Color.orange.opacity(0.2)
        case .error:
            return Color.red.opacity(0.2)
        case .disabled:
            return Color.gray.opacity(0.2)
        }
    }
}

// MARK: - Preview

struct CollectorToggle_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: 20) {
            CollectorToggle(
                collector: CollectorStatus(
                    id: "1",
                    name: "Keyboard Monitor",
                    isEnabled: true,
                    eventCount: 1234,
                    lastActivity: Date(),
                    status: .healthy
                ),
                onToggle: {}
            )
            
            CompactCollectorToggle(
                collector: CollectorStatus(
                    id: "2",
                    name: "File System",
                    isEnabled: false,
                    eventCount: 567,
                    lastActivity: Date(),
                    status: .warning
                ),
                onToggle: {}
            )
            
            HStack {
                CollectorGridToggle(
                    collector: CollectorStatus(
                        id: "3",
                        name: "Network",
                        isEnabled: true,
                        eventCount: 890,
                        lastActivity: Date(),
                        status: .healthy
                    ),
                    onToggle: {}
                )
                
                CollectorGridToggle(
                    collector: CollectorStatus(
                        id: "4",
                        name: "Screen",
                        isEnabled: false,
                        eventCount: 234,
                        lastActivity: Date(),
                        status: .error
                    ),
                    onToggle: {}
                )
            }
        }
        .padding()
    }
}