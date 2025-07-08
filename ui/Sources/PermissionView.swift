import SwiftUI

struct PermissionView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var permissionManager = PermissionManager()
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                // Overall Status
                overallStatusSection
                
                // Individual Permissions
                permissionsSection
                
                // Actions
                actionsSection
            }
        }
        .task {
            await permissionManager.checkAllPermissions()
        }
    }
    
    private var overallStatusSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Permission Status", systemImage: "lock.shield")
            
            HStack {
                StatusIndicatorComponent(
                    status: StatusIndicator.permissionStatus(from: appState.permissionStatus),
                    size: 24
                )
                
                VStack(alignment: .leading, spacing: 2) {
                    Text(overallStatusText)
                        .font(.caption)
                        .fontWeight(.medium)
                    
                    Text(permissionSummary)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
            }
            .padding(.horizontal, 8)
        }
    }
    
    private var permissionsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Individual Permissions", systemImage: "checklist")
            
            VStack(spacing: 8) {
                ForEach(appState.permissionStatus.permissionInfos, id: \.name) { info in
                    PermissionRow(
                        info: info,
                        onRequest: {
                            Task {
                                await requestPermission(info.name)
                            }
                        },
                        onOpenSettings: {
                            openSystemPreferences(for: info.name)
                        }
                    )
                }
            }
            .padding(.horizontal, 8)
        }
    }
    
    private var actionsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: "Actions", systemImage: "wrench.and.screwdriver")
            
            VStack(spacing: 8) {
                Button(action: {
                    Task {
                        await permissionManager.checkAllPermissions()
                        await appState.refreshAllData()
                    }
                }) {
                    HStack {
                        Image(systemName: "arrow.clockwise")
                        Text("Refresh Status")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(ActionButtonStyle())
                
                Button(action: {
                    openSystemPreferences(for: "all")
                }) {
                    HStack {
                        Image(systemName: "gearshape")
                        Text("Open System Preferences")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(ActionButtonStyle())
                
                if hasRequiredPermissions {
                    Button(action: {
                        requestAllMissingPermissions()
                    }) {
                        HStack {
                            Image(systemName: "checkmark.shield")
                            Text("Request All Permissions")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(ActionButtonStyle(isPrimary: true))
                }
            }
            .padding(.horizontal, 8)
        }
    }
    
    // MARK: - Computed Properties
    
    private var overallStatusText: String {
        switch appState.permissionStatus.overallStatus {
        case .granted:
            return "All permissions granted"
        case .denied:
            return "Some permissions denied"
        case .notDetermined:
            return "Permissions pending"
        case .unknown:
            return "Permission status unknown"
        }
    }
    
    private var permissionSummary: String {
        let granted = appState.permissionStatus.grantedCount
        let total = appState.permissionStatus.totalCount
        return "\(granted) of \(total) permissions granted"
    }
    
    private var hasRequiredPermissions: Bool {
        let requiredPermissions = appState.permissionStatus.permissionInfos.filter { $0.isRequired }
        return requiredPermissions.allSatisfy { $0.state == .granted }
    }
    
    // MARK: - Actions
    
    private func requestPermission(_ permissionName: String) async {
        guard let permissionType = PermissionType(rawValue: permissionName.lowercased()) else { return }
        
        let granted = await permissionManager.requestPermission(permissionType)
        if granted {
            await appState.refreshAllData()
        }
    }
    
    private func openSystemPreferences(for permissionName: String) {
        if permissionName == "all" {
            let url = URL(string: "x-apple.systempreferences:com.apple.preference.security")!
            NSWorkspace.shared.open(url)
        } else {
            guard let permissionType = PermissionType(rawValue: permissionName.lowercased()) else { return }
            permissionManager.openSystemPreferences(for: permissionType)
        }
    }
    
    private func requestAllMissingPermissions() {
        Task {
            let missingPermissions = appState.permissionStatus.permissionInfos.filter { 
                $0.state != .granted && $0.isRequired 
            }
            
            for permission in missingPermissions {
                guard let permissionType = PermissionType(rawValue: permission.name.lowercased()) else { continue }
                await permissionManager.requestPermission(permissionType)
            }
            
            await appState.refreshAllData()
        }
    }
}

// MARK: - Supporting Views

struct PermissionRow: View {
    let info: PermissionStatus.PermissionInfo
    let onRequest: () -> Void
    let onOpenSettings: () -> Void
    
    var body: some View {
        HStack {
            Image(systemName: info.state.systemImage)
                .font(.caption)
                .foregroundColor(info.state.color)
                .frame(width: 16)
            
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(info.name)
                        .font(.caption)
                        .fontWeight(.medium)
                    
                    if info.isRequired {
                        Text("REQUIRED")
                            .font(.caption2)
                            .fontWeight(.bold)
                            .foregroundColor(.red)
                            .padding(.horizontal, 4)
                            .padding(.vertical, 1)
                            .background(Color.red.opacity(0.1))
                            .cornerRadius(4)
                    }
                }
                
                Text(info.description)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.leading)
            }
            
            Spacer()
            
            VStack(spacing: 4) {
                Text(info.state.description)
                    .font(.caption2)
                    .foregroundColor(info.state.color)
                    .fontWeight(.medium)
                
                if info.state != .granted {
                    Button("Fix") {
                        onOpenSettings()
                    }
                    .font(.caption2)
                    .buttonStyle(PlainButtonStyle())
                    .foregroundColor(.accentColor)
                }
            }
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 8)
        .background(
            RoundedRectangle(cornerRadius: 6)
                .fill(info.state == .granted ? Color.green.opacity(0.05) : Color.red.opacity(0.05))
        )
    }
}

struct ActionButtonStyle: ButtonStyle {
    let isPrimary: Bool
    
    init(isPrimary: Bool = false) {
        self.isPrimary = isPrimary
    }
    
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.caption)
            .padding(.vertical, 8)
            .padding(.horizontal, 12)
            .background(
                RoundedRectangle(cornerRadius: 6)
                    .fill(isPrimary ? Color.accentColor : Color.gray.opacity(0.1))
            )
            .foregroundColor(isPrimary ? .white : .primary)
            .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}

// MARK: - Preview

struct PermissionView_Previews: PreviewProvider {
    static var previews: some View {
        PermissionView()
            .environmentObject(AppState())
            .frame(width: 350, height: 400)
    }
}